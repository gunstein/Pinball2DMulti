use std::collections::HashMap;

use bevy::prelude::Resource;

use super::types::{ConnectionState, Player, SpaceBall3D};
use super::vec3::{rotate_normalize_in_place, slerp};

#[derive(Resource)]
pub struct NetState {
    pub state: ConnectionState,
    pub self_id: u32,
    pub server_version: String,
    pub protocol_mismatch: bool,

    pub players: Vec<Player>,
    pub snapshot_balls: Vec<SpaceBall3D>,
    pub interpolated_balls: Vec<SpaceBall3D>,

    // Buffered interpolation state
    pub prev_snapshot_balls: Vec<SpaceBall3D>,
    prev_id_to_index: HashMap<u32, usize>,
    pub prev_recv_time: f64, // monotonic secs when prev snapshot arrived
    pub curr_recv_time: f64, // monotonic secs when curr snapshot arrived

    // Keep for fallback (first snapshot before we have two)
    pub last_snapshot_time: f64,
}

impl Default for NetState {
    fn default() -> Self {
        Self {
            state: ConnectionState::Connecting,
            self_id: 0,
            server_version: String::new(),
            protocol_mismatch: false,
            players: Vec::new(),
            snapshot_balls: Vec::new(),
            interpolated_balls: Vec::new(),
            prev_snapshot_balls: Vec::new(),
            prev_id_to_index: HashMap::new(),
            prev_recv_time: 0.0,
            curr_recv_time: 0.0,
            last_snapshot_time: 0.0,
        }
    }
}

impl NetState {
    /// Shift the current snapshot to prev before loading new wire data.
    /// Called before `update_balls_from_space_state`.
    pub fn shift_snapshot(&mut self, now: f64) {
        // Swap current → prev (reuses allocations)
        std::mem::swap(&mut self.snapshot_balls, &mut self.prev_snapshot_balls);

        // Rebuild id → index map for the (now-prev) balls
        self.prev_id_to_index.clear();
        for (i, ball) in self.prev_snapshot_balls.iter().enumerate() {
            self.prev_id_to_index.insert(ball.id, i);
        }

        self.prev_recv_time = self.curr_recv_time;
        self.curr_recv_time = now;
    }

    pub fn update_interpolation(&mut self, now: f64) {
        // Resize interpolated vec
        if self.interpolated_balls.len() < self.snapshot_balls.len() {
            self.interpolated_balls
                .resize_with(self.snapshot_balls.len(), Default::default);
        } else if self.interpolated_balls.len() > self.snapshot_balls.len() {
            self.interpolated_balls.truncate(self.snapshot_balls.len());
        }

        let interval = self.curr_recv_time - self.prev_recv_time;

        if interval <= 0.0 || self.prev_snapshot_balls.is_empty() {
            // Fallback: single-snapshot extrapolation (first snapshot or bad timing)
            let elapsed = (now - self.last_snapshot_time).clamp(0.0, 0.2);
            for (dst, base) in self
                .interpolated_balls
                .iter_mut()
                .zip(self.snapshot_balls.iter())
            {
                dst.id = base.id;
                dst.owner_id = base.owner_id;
                dst.pos = base.pos;
                dst.axis = base.axis;
                dst.omega = base.omega;
                rotate_normalize_in_place(&mut dst.pos, dst.axis, dst.omega * elapsed);
            }
            return;
        }

        // Render one interval behind real-time so t stays in [0, 1] range.
        // This adds ~100ms latency but eliminates snap-backs completely.
        let render_time = now - interval;
        let t = ((render_time - self.prev_recv_time) / interval).clamp(0.0, 1.0);

        for (dst, curr) in self
            .interpolated_balls
            .iter_mut()
            .zip(self.snapshot_balls.iter())
        {
            dst.id = curr.id;
            dst.owner_id = curr.owner_id;
            dst.axis = curr.axis;
            dst.omega = curr.omega;

            if let Some(&prev_idx) = self.prev_id_to_index.get(&curr.id) {
                // Interpolate between prev and curr position
                let prev = &self.prev_snapshot_balls[prev_idx];
                dst.pos = slerp(prev.pos, curr.pos, t);
            } else {
                // New ball — show at current position
                dst.pos = curr.pos;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::SpaceBall3D;
    use crate::shared::vec3::Vec3;

    #[test]
    fn fallback_extrapolation_when_single_snapshot() {
        let mut state = NetState::default();
        state.snapshot_balls = vec![SpaceBall3D {
            id: 1,
            owner_id: 1,
            pos: Vec3::new(1.0, 0.0, 0.0),
            axis: Vec3::new(0.0, 0.0, 1.0),
            omega: 1.0,
        }];
        state.last_snapshot_time = 1.0;

        state.update_interpolation(1.1);

        let p = state.interpolated_balls[0].pos;
        assert!(p.x < 1.0);
        assert!(p.y > 0.0);
        assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
    }

    #[test]
    fn buffered_interpolation_slerps_between_two_snapshots() {
        let mut state = NetState::default();

        // First snapshot: ball at (1, 0, 0), received at t=1.0
        state.snapshot_balls = vec![SpaceBall3D {
            id: 42,
            owner_id: 1,
            pos: Vec3::new(1.0, 0.0, 0.0),
            axis: Vec3::new(0.0, 0.0, 1.0),
            omega: 1.0,
        }];
        state.curr_recv_time = 1.0;
        state.last_snapshot_time = 1.0;

        // Second snapshot: ball at (0, 1, 0), received at t=1.1
        state.shift_snapshot(1.1);
        state.snapshot_balls = vec![SpaceBall3D {
            id: 42,
            owner_id: 1,
            pos: Vec3::new(0.0, 1.0, 0.0),
            axis: Vec3::new(0.0, 0.0, 1.0),
            omega: 1.0,
        }];
        state.last_snapshot_time = 1.1;

        // With render delay of one interval (0.1s):
        // render_time = now - interval = now - 0.1
        // t = (render_time - prev_recv) / interval = (now - 0.1 - 1.0) / 0.1
        // For t=0.5: now - 0.1 - 1.0 = 0.05 → now = 1.15
        state.update_interpolation(1.15);

        let p = state.interpolated_balls[0].pos;
        let len = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
        assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
        assert!(
            (len - 1.0).abs() < 1e-6,
            "expected unit-length pos, got {}",
            len
        );
        // At t=0.5 between (1,0,0) and (0,1,0), both x and y should be positive
        assert!(p.x > 0.1, "expected x > 0.1, got {}", p.x);
        assert!(p.y > 0.1, "expected y > 0.1, got {}", p.y);
    }

    #[test]
    fn new_ball_shown_at_current_position() {
        let mut state = NetState::default();

        // First snapshot: empty, received at t=1.0
        state.curr_recv_time = 1.0;
        state.last_snapshot_time = 1.0;

        // Second snapshot: new ball appears, received at t=1.1
        state.shift_snapshot(1.1);
        state.snapshot_balls = vec![SpaceBall3D {
            id: 99,
            owner_id: 1,
            pos: Vec3::new(0.0, 0.0, 1.0),
            axis: Vec3::new(1.0, 0.0, 0.0),
            omega: 0.5,
        }];
        state.last_snapshot_time = 1.1;

        // prev_snapshot_balls is empty (first snapshot had no balls),
        // so fallback extrapolation is used. With small elapsed, pos stays near (0,0,1).
        state.update_interpolation(1.12);

        let p = state.interpolated_balls[0].pos;
        assert!(p.z > 0.9, "new ball should be near its current position");
    }

    #[test]
    fn interpolation_stays_finite_across_many_snapshot_updates() {
        let mut state = NetState::default();
        let omega = 1.2;

        for i in 0..100u64 {
            let recv_time = i as f64 * 0.1;
            let a = i as f64 * 0.01;

            if i > 0 {
                state.shift_snapshot(recv_time);
            } else {
                state.curr_recv_time = recv_time;
            }

            state.snapshot_balls = vec![SpaceBall3D {
                id: 1,
                owner_id: 1,
                pos: Vec3::new(a.cos(), a.sin(), 0.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega,
            }];
            state.last_snapshot_time = recv_time;
            state.update_interpolation(recv_time + 0.05);

            let p = state.interpolated_balls[0].pos;
            let len = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
            assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
            assert!(
                (len - 1.0).abs() < 1e-3,
                "tick {}: expected unit-length pos, got {}",
                i,
                len
            );
        }
    }
}
