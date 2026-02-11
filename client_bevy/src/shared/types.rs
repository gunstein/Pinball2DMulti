pub use pinball_shared::protocol::{PlayerWire, PROTOCOL_VERSION as CLIENT_PROTOCOL_VERSION};

use super::vec3::Vec3;

#[derive(Debug, Clone)]
pub struct Player {
    pub id: u32,
    #[allow(dead_code)] // populated from wire protocol, not yet used in rendering
    pub cell_index: u32,
    pub portal_pos: Vec3,
    pub color: u32,
    pub paused: bool,
    pub balls_produced: u32,
    pub balls_in_flight: u32,
}

#[derive(Debug, Clone)]
pub struct SpaceBall3D {
    pub id: u32,
    pub owner_id: u32,
    pub pos: Vec3,
    pub axis: Vec3,
    pub omega: f64,
}

impl Default for SpaceBall3D {
    fn default() -> Self {
        Self {
            id: 0,
            owner_id: 0,
            pos: Vec3::new(1.0, 0.0, 0.0),
            axis: Vec3::new(0.0, 0.0, 1.0),
            omega: 0.0,
        }
    }
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

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnected,
}
