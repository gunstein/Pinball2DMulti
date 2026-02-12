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
