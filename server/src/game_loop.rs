use crate::config::{DeepSpaceConfig, ServerConfig};
use crate::protocol::{PlayersStateMsg, SpaceStateMsg, WelcomeMsg};
use crate::state::GameState;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};

/// Speed at which captured balls enter the board (m/s)
const CAPTURE_SPEED: f64 = 1.5;

/// Commands from client connections to the game loop
pub enum GameCommand {
    PlayerJoin {
        response: oneshot::Sender<(u32, WelcomeMsg)>,
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

/// Broadcasts from game loop to all clients
#[derive(Debug, Clone)]
pub enum GameBroadcast {
    SpaceState(SpaceStateMsg),
    PlayersState(PlayersStateMsg),
    TransferIn { player_id: u32, vx: f64, vy: f64 },
}

/// Run the main game loop. Owns all game state.
pub async fn run_game_loop(
    mut cmd_rx: mpsc::Receiver<GameCommand>,
    broadcast_tx: broadcast::Sender<GameBroadcast>,
    server_config: ServerConfig,
) {
    let deep_space_config = DeepSpaceConfig::default();
    let mut state = GameState::new(&server_config, deep_space_config);

    let tick_duration = Duration::from_secs_f64(1.0 / server_config.tick_rate_hz as f64);
    let broadcast_every_n = server_config.tick_rate_hz / server_config.broadcast_rate_hz;
    let mut tick_count: u64 = 0;

    let mut tick_interval = tokio::time::interval(tick_duration);
    tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                let dt = 1.0 / server_config.tick_rate_hz as f64;
                let captures = state.tick(dt);

                // Send transfer_in for each capture
                for cap in &captures {
                    let (vx, vy) = state.get_capture_velocity_2d(
                        &cap.ball,
                        cap.player.portal_pos,
                        CAPTURE_SPEED,
                    );
                    let _ = broadcast_tx.send(GameBroadcast::TransferIn {
                        player_id: cap.player_id,
                        vx,
                        vy,
                    });
                }

                // Broadcast space_state at lower rate
                tick_count += 1;
                if tick_count % broadcast_every_n as u64 == 0 {
                    let msg = state.get_space_state();
                    let _ = broadcast_tx.send(GameBroadcast::SpaceState(msg));
                }
            }

            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    GameCommand::PlayerJoin { response } => {
                        if let Some((player_id, _player)) = state.add_player() {
                            let welcome = WelcomeMsg {
                                self_id: player_id,
                                players: state.get_players_state().players,
                                config: state.config.clone(),
                            };
                            let _ = response.send((player_id, welcome));
                            let _ = broadcast_tx.send(GameBroadcast::PlayersState(
                                state.get_players_state(),
                            ));
                        }
                    }
                    GameCommand::PlayerLeave { id } => {
                        state.remove_player(id);
                        let _ = broadcast_tx.send(GameBroadcast::PlayersState(
                            state.get_players_state(),
                        ));
                        tracing::info!("Player {} left", id);
                    }
                    GameCommand::BallEscaped { owner_id, vx, vy } => {
                        state.ball_escaped(owner_id, vx, vy);
                    }
                }
            }

            else => break,
        }
    }

    tracing::info!("Game loop ended");
}
