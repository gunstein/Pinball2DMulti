use bevy::prelude::Resource;

use super::types::{ConnectionState, Player, SpaceBall3D};
use super::vec3::rotate_normalize_in_place;

#[derive(Resource)]
pub struct NetState {
    pub state: ConnectionState,
    pub self_id: u32,
    pub server_version: String,
    pub protocol_mismatch: bool,

    pub players: Vec<Player>,
    pub snapshot_balls: Vec<SpaceBall3D>,
    pub interpolated_balls: Vec<SpaceBall3D>,
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
            last_snapshot_time: 0.0,
        }
    }
}

impl NetState {
    pub fn update_interpolation(&mut self, now: f64) {
        let elapsed = (now - self.last_snapshot_time).clamp(0.0, 0.2);
        if self.interpolated_balls.len() < self.snapshot_balls.len() {
            self.interpolated_balls
                .resize_with(self.snapshot_balls.len(), Default::default);
        } else if self.interpolated_balls.len() > self.snapshot_balls.len() {
            self.interpolated_balls.truncate(self.snapshot_balls.len());
        }

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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::SpaceBall3D;
    use crate::shared::vec3::Vec3;

    #[test]
    fn interpolation_rotates_ball_forward_with_elapsed_time() {
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
    fn interpolation_stays_finite_across_many_snapshot_updates() {
        let mut state = NetState::default();
        let omega = 1.2;

        for i in 0..100 {
            let a = i as f64 * 0.01;
            state.snapshot_balls = vec![SpaceBall3D {
                id: 1,
                owner_id: 1,
                pos: Vec3::new(a.cos(), a.sin(), 0.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega,
            }];
            state.last_snapshot_time = i as f64 * 0.1;
            state.update_interpolation(state.last_snapshot_time + 0.05);

            let p = state.interpolated_balls[0].pos;
            let len = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
            assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
            assert!(
                (len - 1.0).abs() < 1e-6,
                "expected unit-length pos, got {}",
                len
            );
        }
    }
}
