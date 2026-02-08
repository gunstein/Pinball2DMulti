use serde::{Deserialize, Serialize};

use super::vec3::Vec3;

pub const CLIENT_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSpaceConfig {
    pub portal_alpha: f64,
    pub omega_min: f64,
    pub omega_max: f64,
    pub reroute_after: f64,
    pub reroute_cooldown: f64,
    pub min_age_for_capture: f64,
    pub min_age_for_reroute: f64,
    pub reroute_arrival_time_min: f64,
    pub reroute_arrival_time_max: f64,
}

impl Default for DeepSpaceConfig {
    fn default() -> Self {
        Self {
            portal_alpha: 0.15,
            omega_min: 0.5,
            omega_max: 1.0,
            reroute_after: 12.0,
            reroute_cooldown: 6.0,
            min_age_for_capture: 15.0,
            min_age_for_reroute: 2.0,
            reroute_arrival_time_min: 4.0,
            reroute_arrival_time_max: 10.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerWire {
    pub id: u32,
    pub cell_index: u32,
    pub portal_pos: [f64; 3],
    pub color: u32,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub balls_produced: u32,
    #[serde(default)]
    pub balls_in_flight: u32,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BallWire {
    pub id: u32,
    pub owner_id: u32,
    pub pos: [f64; 3],
    pub axis: [f64; 3],
    pub omega: f64,
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

impl PlayerWire {
    pub fn to_player(&self) -> Player {
        Player {
            id: self.id,
            cell_index: self.cell_index,
            portal_pos: Vec3::new(self.portal_pos[0], self.portal_pos[1], self.portal_pos[2]),
            color: self.color,
            paused: self.paused,
            balls_produced: self.balls_produced,
            balls_in_flight: self.balls_in_flight,
        }
    }
}

impl BallWire {
    pub fn to_ball(&self) -> SpaceBall3D {
        SpaceBall3D {
            id: self.id,
            owner_id: self.owner_id,
            pos: Vec3::new(self.pos[0], self.pos[1], self.pos[2]),
            axis: Vec3::new(self.axis[0], self.axis[1], self.axis[2]),
            omega: self.omega,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnected,
}
