pub use pinball_shared::protocol::{
    BallWire, PlayerWire, PROTOCOL_VERSION as CLIENT_PROTOCOL_VERSION,
};

use super::vec3::Vec3;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Player {
    pub id: u32,
    pub cell_index: u32,
    pub portal_pos: Vec3,
    pub color: u32,
    pub paused: bool,
    pub balls_produced: u32,
    pub balls_in_flight: u32,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SpaceBall3D {
    pub id: u32,
    pub owner_id: u32,
    pub pos: Vec3,
    pub axis: Vec3,
    pub omega: f64,
}

pub fn wire_to_player(w: &PlayerWire) -> Player {
    Player {
        id: w.id,
        cell_index: w.cell_index,
        portal_pos: Vec3::new(w.portal_pos[0], w.portal_pos[1], w.portal_pos[2]),
        color: w.color,
        paused: w.paused,
        balls_produced: w.balls_produced,
        balls_in_flight: w.balls_in_flight,
    }
}

pub fn wire_to_ball(w: &BallWire) -> SpaceBall3D {
    SpaceBall3D {
        id: w.id,
        owner_id: w.owner_id,
        pos: Vec3::new(w.pos[0], w.pos[1], w.pos[2]),
        axis: Vec3::new(w.axis[0], w.axis[1], w.axis[2]),
        omega: w.omega,
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnected,
}
