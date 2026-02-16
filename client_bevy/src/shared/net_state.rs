use std::collections::{HashMap, VecDeque};

use bevy::prelude::Resource;

use super::types::{ConnectionState, Player, SpaceBall3D};
use super::vec3::{rotate_normalize_in_place, slerp};

const INTERPOLATION_DELAY_SECS: f64 = 0.2;
const MAX_EXTRAPOLATION_SECS: f64 = 0.2;
const MAX_SNAPSHOT_BUFFER: usize = 8;
const SNAPSHOT_EPSILON_SECS: f64 = 1e-6;
const OFFSET_SMOOTH_UP_ALPHA: f64 = 0.02;

#[derive(Debug)]
struct Snapshot {
    server_time: f64,
    recv_time: f64,
    balls: Vec<SpaceBall3D>,
    id_to_index: HashMap<u32, usize>,
}

#[derive(Resource)]
pub struct NetState {
    pub state: ConnectionState,
    pub self_id: u32,
    pub server_version: String,
    pub protocol_mismatch: bool,

    pub players: Vec<Player>,
    pub interpolated_balls: Vec<SpaceBall3D>,

    snapshots: VecDeque<Snapshot>,
    has_server_time_offset: bool,
    server_time_offset: f64,
}

impl Default for NetState {
    fn default() -> Self {
        Self {
            state: ConnectionState::Connecting,
            self_id: 0,
            server_version: String::new(),
            protocol_mismatch: false,
            players: Vec::new(),
            interpolated_balls: Vec::new(),
            snapshots: VecDeque::new(),
            has_server_time_offset: false,
            server_time_offset: 0.0,
        }
    }
}

impl NetState {
    pub fn reset_interpolation(&mut self) {
        self.snapshots.clear();
        self.interpolated_balls.clear();
        self.has_server_time_offset = false;
        self.server_time_offset = 0.0;
    }

    pub fn push_snapshot(&mut self, server_time: f64, recv_time: f64, balls: Vec<SpaceBall3D>) {
        if !server_time.is_finite() || !recv_time.is_finite() {
            return;
        }

        if let Some(last) = self.snapshots.back() {
            if server_time < last.server_time - SNAPSHOT_EPSILON_SECS {
                // Server timeline moved backwards (e.g. reconnect/server restart).
                self.reset_interpolation();
            } else if (server_time - last.server_time).abs() <= SNAPSHOT_EPSILON_SECS {
                // Duplicate timestamp: keep only the latest payload for this time point.
                self.snapshots.pop_back();
            }
        }

        let mut id_to_index = HashMap::with_capacity(balls.len());
        for (i, ball) in balls.iter().enumerate() {
            id_to_index.insert(ball.id, i);
        }

        self.snapshots.push_back(Snapshot {
            server_time,
            recv_time,
            balls,
            id_to_index,
        });

        if self.snapshots.len() > MAX_SNAPSHOT_BUFFER {
            self.snapshots.pop_front();
        }

        self.update_server_time_offset(server_time, recv_time);
    }

    fn update_server_time_offset(&mut self, server_time: f64, recv_time: f64) {
        let sample = recv_time - server_time;
        if !sample.is_finite() {
            return;
        }

        if !self.has_server_time_offset {
            self.server_time_offset = sample;
            self.has_server_time_offset = true;
            return;
        }

        // Fast downward updates, slow upward smoothing.
        if sample < self.server_time_offset {
            self.server_time_offset = sample;
        } else {
            self.server_time_offset += (sample - self.server_time_offset) * OFFSET_SMOOTH_UP_ALPHA;
        }
    }

    pub fn update_interpolation(&mut self, now: f64) {
        if self.snapshots.is_empty() {
            self.interpolated_balls.clear();
            return;
        }

        let snapshots = &self.snapshots;
        let interpolated = &mut self.interpolated_balls;

        if snapshots.len() == 1 {
            let only = snapshots.front().expect("len checked");
            let elapsed = (now - only.recv_time).clamp(0.0, MAX_EXTRAPOLATION_SECS);
            fill_from_snapshot(interpolated, only, elapsed);
            return;
        }

        let latest = snapshots.back().expect("len checked");
        let mut render_server_time = if self.has_server_time_offset {
            now - self.server_time_offset - INTERPOLATION_DELAY_SECS
        } else {
            latest.server_time - INTERPOLATION_DELAY_SECS
        };

        let first = snapshots.front().expect("len checked");

        if render_server_time <= first.server_time {
            fill_from_snapshot(interpolated, first, 0.0);
            return;
        }

        if render_server_time >= latest.server_time {
            let extrap =
                (render_server_time - latest.server_time).clamp(0.0, MAX_EXTRAPOLATION_SECS);
            fill_from_snapshot(interpolated, latest, extrap);
            return;
        }

        let mut newer_idx = 1usize;
        while newer_idx < snapshots.len() && snapshots[newer_idx].server_time < render_server_time {
            newer_idx += 1;
        }

        if newer_idx >= snapshots.len() {
            fill_from_snapshot(interpolated, latest, MAX_EXTRAPOLATION_SECS);
            return;
        }

        let older = &snapshots[newer_idx - 1];
        let newer = &snapshots[newer_idx];
        let dt = newer.server_time - older.server_time;
        if dt <= SNAPSHOT_EPSILON_SECS {
            fill_from_snapshot(interpolated, newer, 0.0);
            return;
        }

        render_server_time = render_server_time.clamp(older.server_time, newer.server_time);
        let t = ((render_server_time - older.server_time) / dt).clamp(0.0, 1.0);
        fill_between(interpolated, older, newer, t);
    }
}

fn resize_interpolated(interpolated: &mut Vec<SpaceBall3D>, count: usize) {
    if interpolated.len() < count {
        interpolated.resize_with(count, Default::default);
    } else if interpolated.len() > count {
        interpolated.truncate(count);
    }
}

fn fill_from_snapshot(
    interpolated: &mut Vec<SpaceBall3D>,
    snapshot: &Snapshot,
    extrapolate_secs: f64,
) {
    resize_interpolated(interpolated, snapshot.balls.len());
    let extrap = extrapolate_secs.clamp(0.0, MAX_EXTRAPOLATION_SECS);

    for (dst, base) in interpolated.iter_mut().zip(snapshot.balls.iter()) {
        dst.id = base.id;
        dst.owner_id = base.owner_id;
        dst.pos = base.pos;
        dst.axis = base.axis;
        dst.omega = base.omega;

        if extrap > 0.0 {
            rotate_normalize_in_place(&mut dst.pos, dst.axis, dst.omega * extrap);
        }
    }
}

fn fill_between(interpolated: &mut Vec<SpaceBall3D>, older: &Snapshot, newer: &Snapshot, t: f64) {
    resize_interpolated(interpolated, newer.balls.len());

    for (dst, curr) in interpolated.iter_mut().zip(newer.balls.iter()) {
        dst.id = curr.id;
        dst.owner_id = curr.owner_id;
        dst.axis = curr.axis;
        dst.omega = curr.omega;

        if let Some(&older_idx) = older.id_to_index.get(&curr.id) {
            let prev = &older.balls[older_idx];
            dst.pos = slerp(prev.pos, curr.pos, t);
        } else {
            dst.pos = curr.pos;
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
        state.push_snapshot(
            1.0,
            1.0,
            vec![SpaceBall3D {
                id: 1,
                owner_id: 1,
                pos: Vec3::new(1.0, 0.0, 0.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega: 1.0,
            }],
        );

        state.update_interpolation(1.1);

        let p = state.interpolated_balls[0].pos;
        assert!(p.x < 1.0);
        assert!(p.y > 0.0);
        assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
    }

    #[test]
    fn buffered_interpolation_slerps_between_two_snapshots() {
        let mut state = NetState::default();

        // recv_time == server_time -> offset ~ 0 for easy deterministic math in test
        state.push_snapshot(
            1.0,
            1.0,
            vec![SpaceBall3D {
                id: 42,
                owner_id: 1,
                pos: Vec3::new(1.0, 0.0, 0.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega: 1.0,
            }],
        );
        state.push_snapshot(
            1.1,
            1.1,
            vec![SpaceBall3D {
                id: 42,
                owner_id: 1,
                pos: Vec3::new(0.0, 1.0, 0.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega: 1.0,
            }],
        );

        // render_server_time = now - offset - delay = now - 0.2
        // now=1.25 -> render_server_time=1.05 (halfway between 1.0 and 1.1)
        state.update_interpolation(1.25);

        let p = state.interpolated_balls[0].pos;
        let len = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
        assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
        assert!(
            (len - 1.0).abs() < 1e-6,
            "expected unit-length pos, got {}",
            len
        );
        assert!(p.x > 0.1, "expected x > 0.1, got {}", p.x);
        assert!(p.y > 0.1, "expected y > 0.1, got {}", p.y);
    }

    #[test]
    fn new_ball_shown_at_current_position_when_missing_in_older_snapshot() {
        let mut state = NetState::default();

        state.push_snapshot(1.0, 1.0, vec![]);
        state.push_snapshot(
            1.1,
            1.1,
            vec![SpaceBall3D {
                id: 99,
                owner_id: 1,
                pos: Vec3::new(0.0, 0.0, 1.0),
                axis: Vec3::new(1.0, 0.0, 0.0),
                omega: 0.5,
            }],
        );

        state.update_interpolation(1.25);

        let p = state.interpolated_balls[0].pos;
        assert!(p.z > 0.9, "new ball should stay near current position");
    }

    #[test]
    fn interpolation_stays_finite_across_many_snapshot_updates() {
        let mut state = NetState::default();
        let omega = 1.2;

        for i in 0..100u64 {
            let recv_time = i as f64 * 0.1;
            let a = i as f64 * 0.01;

            state.push_snapshot(
                recv_time,
                recv_time,
                vec![SpaceBall3D {
                    id: 1,
                    owner_id: 1,
                    pos: Vec3::new(a.cos(), a.sin(), 0.0),
                    axis: Vec3::new(0.0, 0.0, 1.0),
                    omega,
                }],
            );

            state.update_interpolation(recv_time + 0.15);

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

    #[test]
    fn duplicate_server_time_replaces_latest_snapshot() {
        let mut state = NetState::default();
        state.push_snapshot(
            1.0,
            1.0,
            vec![SpaceBall3D {
                id: 7,
                owner_id: 1,
                pos: Vec3::new(1.0, 0.0, 0.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega: 0.0,
            }],
        );
        state.push_snapshot(
            1.0,
            1.01,
            vec![SpaceBall3D {
                id: 7,
                owner_id: 1,
                pos: Vec3::new(0.0, 1.0, 0.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega: 0.0,
            }],
        );

        assert_eq!(state.snapshots.len(), 1);
        let last = state.snapshots.back().expect("snapshot");
        assert!((last.server_time - 1.0).abs() < 1e-9);
        let p = last.balls[0].pos;
        assert!((p.x - 0.0).abs() < 1e-9 && (p.y - 1.0).abs() < 1e-9);
    }

    #[test]
    fn out_of_order_server_time_resets_snapshot_timeline() {
        let mut state = NetState::default();
        state.push_snapshot(
            1.0,
            1.0,
            vec![SpaceBall3D {
                id: 1,
                owner_id: 1,
                pos: Vec3::new(1.0, 0.0, 0.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega: 0.0,
            }],
        );
        state.push_snapshot(
            1.1,
            1.1,
            vec![SpaceBall3D {
                id: 1,
                owner_id: 1,
                pos: Vec3::new(0.0, 1.0, 0.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega: 0.0,
            }],
        );
        state.push_snapshot(
            0.9,
            1.2,
            vec![SpaceBall3D {
                id: 1,
                owner_id: 1,
                pos: Vec3::new(0.0, 0.0, 1.0),
                axis: Vec3::new(0.0, 0.0, 1.0),
                omega: 0.0,
            }],
        );

        assert_eq!(state.snapshots.len(), 1);
        let last = state.snapshots.back().expect("snapshot");
        assert!((last.server_time - 0.9).abs() < 1e-9);
    }

    #[test]
    fn snapshot_buffer_is_capped() {
        let mut state = NetState::default();
        for i in 0..12_u64 {
            let t = 1.0 + i as f64 * 0.1;
            state.push_snapshot(
                t,
                t,
                vec![SpaceBall3D {
                    id: 1,
                    owner_id: 1,
                    pos: Vec3::new(1.0, 0.0, 0.0),
                    axis: Vec3::new(0.0, 0.0, 1.0),
                    omega: 0.0,
                }],
            );
        }

        assert_eq!(state.snapshots.len(), MAX_SNAPSHOT_BUFFER);
        let first = state.snapshots.front().expect("first");
        let last = state.snapshots.back().expect("last");
        assert!((first.server_time - 1.4).abs() < 1e-9);
        assert!((last.server_time - 2.1).abs() < 1e-9);
    }
}
