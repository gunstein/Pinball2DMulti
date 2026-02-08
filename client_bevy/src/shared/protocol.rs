use serde::{Deserialize, Serialize};

use super::types::{BallWire, DeepSpaceConfig, PlayerWire};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    #[serde(rename = "welcome")]
    Welcome {
        #[serde(rename = "protocolVersion")]
        protocol_version: u32,
        #[serde(rename = "serverVersion")]
        server_version: String,
        #[serde(rename = "selfId")]
        self_id: u32,
        players: Vec<PlayerWire>,
        config: DeepSpaceConfig,
    },
    #[serde(rename = "players_state")]
    PlayersState { players: Vec<PlayerWire> },
    #[serde(rename = "space_state")]
    SpaceState { balls: Vec<BallWire> },
    #[serde(rename = "transfer_in")]
    TransferIn {
        vx: f32,
        vy: f32,
        #[serde(rename = "ownerId")]
        owner_id: u32,
        color: u32,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    #[serde(rename = "ball_escaped")]
    BallEscaped { vx: f32, vy: f32 },
    #[serde(rename = "set_paused")]
    SetPaused { paused: bool },
    #[serde(rename = "activity")]
    Activity,
}
