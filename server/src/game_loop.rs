use crate::config::{DeepSpaceConfig, ServerConfig};
use crate::protocol::{ServerMsg, WelcomeMsg};
use crate::state::GameState;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};

/// Speed at which captured balls enter the board (m/s)
const CAPTURE_SPEED: f64 = 1.5;

/// Commands from client connections to the game loop
pub enum GameCommand {
    PlayerJoin {
        response: oneshot::Sender<Result<(u32, WelcomeMsg), String>>,
        /// Channel for reliable per-client messages (e.g., TransferIn)
        client_tx: mpsc::Sender<ClientEvent>,
    },
    PlayerLeave {
        id: u32,
    },
    BallEscaped {
        owner_id: u32,
        vx: f64,
        vy: f64,
    },
}

/// Per-client events sent via dedicated mpsc channel.
/// If a client's channel is full, the client is marked dead and removed.
#[derive(Debug, Clone)]
pub enum ClientEvent {
    TransferIn {
        vx: f64,
        vy: f64,
    },
    /// Server-initiated disconnect (client will receive this and close)
    Disconnect,
}

/// Broadcasts from game loop to all clients (lossy - ok to drop on lag)
/// Uses Arc<str> for pre-serialized JSON to avoid O(N clients) cloning + serialization
#[derive(Debug, Clone)]
pub enum GameBroadcast {
    /// Pre-serialized JSON for space_state
    SpaceState(Arc<str>),
    /// Pre-serialized JSON for players_state
    PlayersState(Arc<str>),
}

/// Run the main game loop. Owns all game state.
pub async fn run_game_loop(
    mut cmd_rx: mpsc::Receiver<GameCommand>,
    broadcast_tx: broadcast::Sender<GameBroadcast>,
    server_config: ServerConfig,
) {
    let deep_space_config = DeepSpaceConfig::default();
    let mut state = GameState::new(&server_config, deep_space_config);

    // Per-client channels for reliable messages (TransferIn)
    let mut client_channels: HashMap<u32, mpsc::Sender<ClientEvent>> = HashMap::new();

    let tick_duration = Duration::from_secs_f64(1.0 / server_config.tick_rate_hz as f64);
    let broadcast_every_n = (server_config.tick_rate_hz / server_config.broadcast_rate_hz).max(1);
    let mut tick_count: u64 = 0;

    let mut tick_interval = tokio::time::interval(tick_duration);
    tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                let dt = 1.0 / server_config.tick_rate_hz as f64;
                let captures = state.tick(dt);

                // Send transfer_in for each capture via dedicated client channel
                // If channel is full, mark client as dead (will be cleaned up)
                let mut dead_clients: Vec<u32> = Vec::new();
                for cap in &captures {
                    if let Some(client_tx) = client_channels.get(&cap.player_id) {
                        let (vx, vy) = state.get_capture_velocity_2d(
                            &cap.ball,
                            cap.player.portal_pos,
                            CAPTURE_SPEED,
                        );
                        if client_tx.try_send(ClientEvent::TransferIn { vx, vy }).is_err() {
                            tracing::warn!("Player {} channel full, marking as dead", cap.player_id);
                            dead_clients.push(cap.player_id);
                        }
                    }
                }
                // Remove dead clients
                for id in dead_clients {
                    client_channels.remove(&id);
                    state.remove_player(id);
                    let players_msg = state.get_players_state();
                    let json = serde_json::to_string(&ServerMsg::PlayersState(players_msg))
                        .unwrap_or_default();
                    let _ = broadcast_tx.send(GameBroadcast::PlayersState(json.into()));
                }

                // Broadcast space_state at lower rate
                tick_count += 1;
                if tick_count % broadcast_every_n as u64 == 0 {
                    let msg = state.get_space_state();
                    let ball_count = msg.balls.len();
                    let json = serde_json::to_string(&ServerMsg::SpaceState(msg))
                        .unwrap_or_default();
                    let _ = broadcast_tx.send(GameBroadcast::SpaceState(json.into()));
                    if ball_count > 0 && tick_count % (broadcast_every_n as u64 * 15) == 0 {
                        tracing::debug!("Broadcasting space_state with {} balls", ball_count);
                    }
                }
            }

            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    GameCommand::PlayerJoin { response, client_tx } => {
                        match state.add_player() {
                            Some((player_id, _player)) => {
                                // Store client channel for reliable messaging
                                client_channels.insert(player_id, client_tx);

                                let welcome = WelcomeMsg {
                                    self_id: player_id,
                                    players: state.get_players_state().players,
                                    config: state.config.clone(),
                                };
                                let _ = response.send(Ok((player_id, welcome)));
                                let players_msg = state.get_players_state();
                                let json = serde_json::to_string(&ServerMsg::PlayersState(players_msg))
                                    .unwrap_or_default();
                                let _ = broadcast_tx.send(GameBroadcast::PlayersState(json.into()));
                            }
                            None => {
                                let _ = response.send(Err("Server full".to_string()));
                            }
                        }
                    }
                    GameCommand::PlayerLeave { id } => {
                        // Remove client channel
                        client_channels.remove(&id);

                        state.remove_player(id);
                        let players_msg = state.get_players_state();
                        let json = serde_json::to_string(&ServerMsg::PlayersState(players_msg))
                            .unwrap_or_default();
                        let _ = broadcast_tx.send(GameBroadcast::PlayersState(json.into()));
                        tracing::info!("Player {} left", id);
                    }
                    GameCommand::BallEscaped { owner_id, vx, vy } => {
                        if let Some(ball_id) = state.ball_escaped(owner_id, vx, vy) {
                            tracing::info!("Ball {} escaped from player {}, vx={:.2}, vy={:.2}", ball_id, owner_id, vx, vy);
                        } else {
                            tracing::warn!("ball_escaped failed for player {} (player not found?)", owner_id);
                        }
                    }
                }
            }

            else => break,
        }
    }

    tracing::info!("Game loop ended");
}
