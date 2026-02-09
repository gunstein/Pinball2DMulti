use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::config::DeepSpaceConfig;

/// Protocol version - increment when making breaking changes.
pub const PROTOCOL_VERSION: u32 = 1;

// === Server -> Client ===

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../client/src/shared/generated/")]
#[serde(tag = "type")]
pub enum ServerMsg {
    #[serde(rename = "welcome")]
    Welcome(WelcomeMsg),
    #[serde(rename = "players_state")]
    PlayersState(PlayersStateMsg),
    #[serde(rename = "space_state")]
    SpaceState(SpaceStateMsg),
    #[serde(rename = "transfer_in")]
    TransferIn(TransferInMsg),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../client/src/shared/generated/")]
#[serde(rename_all = "camelCase")]
pub struct WelcomeMsg {
    pub protocol_version: u32,
    pub server_version: String,
    pub self_id: u32,
    pub players: Vec<PlayerWire>,
    pub config: DeepSpaceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../client/src/shared/generated/")]
pub struct PlayersStateMsg {
    pub players: Vec<PlayerWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../client/src/shared/generated/")]
pub struct SpaceStateMsg {
    pub balls: Vec<BallWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../client/src/shared/generated/")]
#[serde(rename_all = "camelCase")]
pub struct BallWire {
    pub id: u32,
    pub owner_id: u32,
    pub pos: [f64; 3],
    pub axis: [f64; 3],
    pub omega: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../client/src/shared/generated/")]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../client/src/shared/generated/")]
#[serde(rename_all = "camelCase")]
pub struct TransferInMsg {
    pub vx: f64,
    pub vy: f64,
    pub owner_id: u32,
    pub color: u32,
}

// === Client -> Server ===

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../client/src/shared/generated/")]
#[serde(tag = "type")]
pub enum ClientMsg {
    #[serde(rename = "ball_escaped")]
    BallEscaped { vx: f64, vy: f64 },
    #[serde(rename = "set_paused")]
    SetPaused { paused: bool },
    #[serde(rename = "activity")]
    Activity,
}

// === Conversion helpers ===

/// Round to 4 decimal places (sufficient for unit vectors, saves ~50% JSON size)
#[inline]
pub fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_msg_welcome_roundtrip() {
        let msg = ServerMsg::Welcome(WelcomeMsg {
            protocol_version: PROTOCOL_VERSION,
            server_version: "0.1.0".to_string(),
            self_id: 7,
            players: vec![PlayerWire {
                id: 7,
                cell_index: 431,
                portal_pos: [0.32, 0.81, -0.49],
                color: 0xff6600,
                paused: false,
                balls_produced: 0,
                balls_in_flight: 0,
            }],
            config: DeepSpaceConfig::default(),
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"welcome\""));
        assert!(json.contains("\"protocolVersion\":1"));
        let parsed: ServerMsg = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerMsg::Welcome(w) => {
                assert_eq!(w.protocol_version, PROTOCOL_VERSION);
                assert_eq!(w.self_id, 7);
                assert_eq!(w.players.len(), 1);
            }
            _ => panic!("Expected Welcome"),
        }
    }

    #[test]
    fn server_msg_space_state_roundtrip() {
        let msg = ServerMsg::SpaceState(SpaceStateMsg {
            balls: vec![BallWire {
                id: 12,
                owner_id: 3,
                pos: [0.5, 0.7, 0.5],
                axis: [0.0, 0.0, 1.0],
                omega: 0.8,
            }],
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"space_state\""));
        let parsed: ServerMsg = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerMsg::SpaceState(s) => assert_eq!(s.balls.len(), 1),
            _ => panic!("Expected SpaceState"),
        }
    }

    #[test]
    fn server_msg_transfer_in_roundtrip() {
        let msg = ServerMsg::TransferIn(TransferInMsg {
            vx: 0.3,
            vy: 1.2,
            owner_id: 5,
            color: 0xff6600,
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"transfer_in\""));
        let parsed: ServerMsg = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerMsg::TransferIn(t) => {
                assert!((t.vx - 0.3).abs() < 1e-9);
                assert!((t.vy - 1.2).abs() < 1e-9);
            }
            _ => panic!("Expected TransferIn"),
        }
    }

    #[test]
    fn client_msg_ball_escaped_roundtrip() {
        let msg = ClientMsg::BallEscaped { vx: 0.42, vy: -1.1 };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"ball_escaped\""));
        let parsed: ClientMsg = serde_json::from_str(&json).unwrap();
        match parsed {
            ClientMsg::BallEscaped { vx, vy } => {
                assert!((vx - 0.42).abs() < 1e-9);
                assert!((vy - (-1.1)).abs() < 1e-9);
            }
            _ => panic!("Expected BallEscaped"),
        }
    }

    #[test]
    fn client_msg_set_paused_roundtrip() {
        let msg = ClientMsg::SetPaused { paused: true };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"set_paused\""));
        let parsed: ClientMsg = serde_json::from_str(&json).unwrap();
        match parsed {
            ClientMsg::SetPaused { paused } => assert!(paused),
            _ => panic!("Expected SetPaused"),
        }
    }

    #[test]
    fn players_state_roundtrip() {
        let msg = ServerMsg::PlayersState(PlayersStateMsg {
            players: vec![
                PlayerWire {
                    id: 1,
                    cell_index: 10,
                    portal_pos: [1.0, 0.0, 0.0],
                    color: 0xff0000,
                    paused: false,
                    balls_produced: 5,
                    balls_in_flight: 2,
                },
                PlayerWire {
                    id: 2,
                    cell_index: 20,
                    portal_pos: [0.0, 1.0, 0.0],
                    color: 0x00ff00,
                    paused: true,
                    balls_produced: 10,
                    balls_in_flight: 0,
                },
            ],
        });
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ServerMsg = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerMsg::PlayersState(p) => {
                assert_eq!(p.players.len(), 2);
                assert!(!p.players[0].paused);
                assert!(p.players[1].paused);
                assert_eq!(p.players[0].balls_produced, 5);
            }
            _ => panic!("Expected PlayersState"),
        }
    }
}
