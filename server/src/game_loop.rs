use crate::config::{DeepSpaceConfig, ServerConfig};
use crate::protocol::{ServerMsg, WelcomeMsg, PROTOCOL_VERSION};
use crate::state::GameState;
use axum::extract::ws::Utf8Bytes;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};

/// Speed at which captured balls enter the board (m/s).
/// This is passed to SphereDeepSpace so it can compute vx/vy at capture time.
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
    SetPaused {
        player_id: u32,
        paused: bool,
    },
    Activity {
        player_id: u32,
    },
}

/// Per-client events sent via dedicated mpsc channel.
/// If a client's channel is full, the client is marked dead and removed.
#[derive(Debug, Clone)]
pub enum ClientEvent {
    TransferIn {
        vx: f64,
        vy: f64,
        owner_id: u32,
        color: u32,
    },
    /// Server-initiated disconnect (client will receive this and close)
    Disconnect,
}

/// Broadcasts from game loop to all clients (lossy - ok to drop on lag)
/// Uses Utf8Bytes for pre-serialized JSON - O(1) clone, no allocation per client
/// UTF-8 validation happens once in game_loop, not per client
#[derive(Debug, Clone)]
pub enum GameBroadcast {
    /// Pre-serialized JSON for space_state
    SpaceState(Utf8Bytes),
    /// Pre-serialized JSON for players_state
    PlayersState(Utf8Bytes),
}

/// Run the main game loop. Owns all game state.
pub async fn run_game_loop(
    cmd_rx: mpsc::Receiver<GameCommand>,
    broadcast_tx: broadcast::Sender<GameBroadcast>,
    server_config: ServerConfig,
) {
    run_game_loop_with_config(
        cmd_rx,
        broadcast_tx,
        server_config,
        DeepSpaceConfig::default(),
    )
    .await;
}

/// Game loop with custom deep space config (used by integration tests).
pub async fn run_game_loop_with_config(
    mut cmd_rx: mpsc::Receiver<GameCommand>,
    broadcast_tx: broadcast::Sender<GameBroadcast>,
    server_config: ServerConfig,
    deep_space_config: DeepSpaceConfig,
) {
    let mut state = GameState::new(&server_config, deep_space_config, CAPTURE_SPEED);

    // Per-client channels for reliable messages (TransferIn)
    let mut client_channels: HashMap<u32, mpsc::Sender<ClientEvent>> = HashMap::new();

    let tick_duration = Duration::from_secs_f64(1.0 / server_config.tick_rate_hz as f64);
    let broadcast_every_n = (server_config.tick_rate_hz / server_config.broadcast_rate_hz).max(1);
    // Players state broadcasts at 2 Hz for stats updates (much lower than space_state)
    let players_broadcast_every_n = (server_config.tick_rate_hz / 2).max(1);
    let mut tick_count: u64 = 0;
    // Dirty flag for immediate players_state broadcast on join/leave/pause
    let mut players_dirty = false;

    let mut tick_interval = tokio::time::interval(tick_duration);
    // Skip missed ticks rather than bursting to catch up. Under load the
    // simulation slows down in wall-clock time instead of spiking CPU with
    // a burst of catch-up ticks. This keeps frame timing smooth at the cost
    // of briefly running slower than real-time.
    tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                let dt = 1.0 / server_config.tick_rate_hz as f64;
                let captures = state.tick(dt);

                // Send transfer_in for each capture via dedicated client channel
                // vx/vy are pre-computed in deep_space - no cloning needed
                // If channel is full, mark client as dead (will be cleaned up)
                let mut dead_clients: Vec<u32> = Vec::new();
                for cap in &captures {
                    if let Some(client_tx) = client_channels.get(&cap.player_id) {
                        if client_tx.try_send(ClientEvent::TransferIn {
                            vx: cap.vx,
                            vy: cap.vy,
                            owner_id: cap.ball_owner_id,
                            color: cap.ball_color,
                        }).is_err() {
                            tracing::warn!("Player {} channel full, marking as dead", cap.player_id);
                            dead_clients.push(cap.player_id);
                        }
                    }
                }
                // Remove dead clients (mark players_dirty for broadcast)
                for id in dead_clients {
                    client_channels.remove(&id);
                    state.remove_player(id);
                    players_dirty = true;
                }

                // Broadcast space_state at 10 Hz
                tick_count += 1;
                if tick_count % broadcast_every_n as u64 == 0 {
                    let msg = state.get_space_state();
                    let ball_count = msg.balls.len();
                    match serde_json::to_string(&ServerMsg::SpaceState(msg)) {
                        Ok(json) => { let _ = broadcast_tx.send(GameBroadcast::SpaceState(json.into())); }
                        Err(e) => tracing::error!("Failed to serialize SpaceState: {}", e),
                    }

                    if ball_count > 0 && tick_count % (broadcast_every_n as u64 * 15) == 0 {
                        tracing::debug!("Broadcasting space_state with {} balls", ball_count);
                    }
                }

                // Broadcast players_state only when dirty OR at low rate (2 Hz) for stats
                if players_dirty || tick_count % players_broadcast_every_n as u64 == 0 {
                    let players_msg = state.get_players_state();
                    match serde_json::to_string(&ServerMsg::PlayersState(players_msg)) {
                        Ok(json) => { let _ = broadcast_tx.send(GameBroadcast::PlayersState(json.into())); }
                        Err(e) => tracing::error!("Failed to serialize PlayersState: {}", e),
                    }
                    players_dirty = false;
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
                                    protocol_version: PROTOCOL_VERSION,
                                    self_id: player_id,
                                    players: state.get_players_state().players,
                                    config: state.config.clone(),
                                };
                                let _ = response.send(Ok((player_id, welcome)));
                                // Broadcast immediately so other players see the new player
                                players_dirty = true;
                            }
                            None => {
                                let _ = response.send(Err("Server full".to_string()));
                            }
                        }
                    }
                    GameCommand::PlayerLeave { id } => {
                        client_channels.remove(&id);
                        state.remove_player(id);
                        players_dirty = true;
                        tracing::info!("Player {} left", id);
                    }
                    GameCommand::BallEscaped { owner_id, vx, vy } => {
                        if state.ball_escaped(owner_id, vx, vy).is_none() {
                            tracing::warn!("ball_escaped failed for player {} (player not found?)", owner_id);
                        }
                    }
                    GameCommand::SetPaused { player_id, paused } => {
                        if state.set_player_paused(player_id, paused) {
                            tracing::debug!("Player {} paused={}", player_id, paused);
                            players_dirty = true;
                        }
                    }
                    GameCommand::Activity { player_id } => {
                        state.player_activity(player_id);
                    }
                }
            }

            else => break,
        }
    }

    tracing::info!("Game loop ended");
}
