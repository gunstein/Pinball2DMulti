use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use std::cmp::min;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::game_loop::{ClientEvent, GameBroadcast, GameCommand};
use crate::protocol::{ClientMsg, ServerMsg, TransferInMsg};

/// Maximum size of a text message from client (bytes)
const MAX_TEXT_MSG_BYTES: usize = 1024;
/// Maximum consecutive parse errors before disconnecting
const MAX_PARSE_ERRORS: u32 = 5;

/// Result of validating a ball_escaped message
#[derive(Debug, Clone, PartialEq)]
pub enum BallEscapedValidation {
    /// Valid velocity, clamped to max bounds
    Valid { vx: f64, vy: f64 },
    /// Invalid: NaN or Infinity
    InvalidNonFinite,
    /// Invalid: vy must be negative (ball going upward)
    InvalidVyPositive,
    /// Invalid: velocity too small
    InvalidTooSlow,
}

/// Validate and clamp a ball_escaped message.
/// Returns the validated/clamped velocity or an error.
pub fn validate_ball_escaped(vx: f64, vy: f64, max_velocity: f64) -> BallEscapedValidation {
    // Reject NaN/Inf
    if !vx.is_finite() || !vy.is_finite() {
        return BallEscapedValidation::InvalidNonFinite;
    }

    // vy must be negative (ball escaping upward from board)
    if vy >= 0.0 {
        return BallEscapedValidation::InvalidVyPositive;
    }

    // Velocity must have some magnitude (not stationary)
    let speed_sq = vx * vx + vy * vy;
    if speed_sq < 0.01 {
        return BallEscapedValidation::InvalidTooSlow;
    }

    // Clamp to max velocity
    let vx = vx.clamp(-max_velocity, max_velocity);
    let vy = vy.clamp(-max_velocity, 0.0); // vy must stay negative

    BallEscapedValidation::Valid { vx, vy }
}

/// Shared app state passed to each WebSocket handler
#[derive(Clone)]
pub struct AppState {
    pub game_tx: mpsc::Sender<GameCommand>,
    pub broadcast_tx: broadcast::Sender<GameBroadcast>,
    /// Maximum velocity component magnitude for ball_escaped
    pub max_velocity: f64,
    /// Maximum ball_escaped messages per second per client
    pub max_ball_escaped_per_sec: u32,
}

/// HTTP handler for WebSocket upgrade
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, app_state))
}

async fn handle_socket(socket: WebSocket, app_state: AppState) {
    let (mut sink, mut stream) = socket.split();

    // Create per-client channel for reliable events (TransferIn)
    let (client_tx, mut client_rx) = mpsc::channel::<ClientEvent>(32);

    // Join the game
    let (resp_tx, resp_rx) = oneshot::channel();
    if app_state
        .game_tx
        .send(GameCommand::PlayerJoin {
            response: resp_tx,
            client_tx,
        })
        .await
        .is_err()
    {
        tracing::error!("Failed to send PlayerJoin command");
        return;
    }

    let (my_id, welcome) = match resp_rx.await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            tracing::warn!("Join rejected: {}", e);
            return;
        }
        Err(_) => {
            tracing::error!("Failed to receive welcome");
            return;
        }
    };

    tracing::info!("Player {} connected", my_id);

    // Send welcome message
    let welcome_json = serde_json::to_string(&ServerMsg::Welcome(welcome)).unwrap();
    if sink.send(Message::Text(welcome_json.into())).await.is_err() {
        return;
    }

    // Subscribe to broadcasts
    let mut broadcast_rx = app_state.broadcast_tx.subscribe();

    // Rate limiting state for ball_escaped
    let mut ball_escaped_count: u32 = 0;
    let mut rate_limit_window_start = Instant::now();
    let mut parse_error_count: u32 = 0;
    let max_velocity = app_state.max_velocity;
    let max_per_sec = app_state.max_ball_escaped_per_sec;

    loop {
        tokio::select! {
            // Client -> Server
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let text_str: &str = &text;

                        // Protect against oversized messages (DoS prevention)
                        if text_str.len() > MAX_TEXT_MSG_BYTES {
                            tracing::warn!(
                                "Player {} sent oversized ws msg: {} bytes (max {}), disconnecting",
                                my_id, text_str.len(), MAX_TEXT_MSG_BYTES
                            );
                            break;
                        }

                        match serde_json::from_str::<ClientMsg>(text_str) {
                            Ok(client_msg) => {
                                parse_error_count = 0; // Reset on successful parse
                                match client_msg {
                                    ClientMsg::BallEscaped { vx, vy } => {
                                        // Validate and clamp velocity
                                        let (vx, vy) = match validate_ball_escaped(vx, vy, max_velocity) {
                                            BallEscapedValidation::Valid { vx, vy } => (vx, vy),
                                            BallEscapedValidation::InvalidNonFinite => {
                                                tracing::warn!("Player {} sent invalid velocity (NaN/Inf), ignoring", my_id);
                                                continue;
                                            }
                                            BallEscapedValidation::InvalidVyPositive => {
                                                tracing::warn!("Player {} sent invalid vy (must be negative), ignoring", my_id);
                                                continue;
                                            }
                                            BallEscapedValidation::InvalidTooSlow => {
                                                tracing::warn!("Player {} sent near-zero velocity, ignoring", my_id);
                                                continue;
                                            }
                                        };

                                        // Rate limiting
                                        let now = Instant::now();
                                        if now.duration_since(rate_limit_window_start).as_secs_f64() >= 1.0 {
                                            // Reset window
                                            rate_limit_window_start = now;
                                            ball_escaped_count = 0;
                                        }
                                        ball_escaped_count += 1;
                                        if ball_escaped_count > max_per_sec {
                                            tracing::warn!("Player {} exceeded rate limit ({} ball_escaped/sec), disconnecting", my_id, max_per_sec);
                                            break;
                                        }

                                        // Hot path - only log at trace level
                                        tracing::trace!("Player {} ball_escaped", my_id);
                                        let _ = app_state.game_tx.send(GameCommand::BallEscaped {
                                            owner_id: my_id,
                                            vx,
                                            vy,
                                        }).await;
                                    }
                                    ClientMsg::SetPaused { paused } => {
                                        tracing::trace!("Player {} set_paused={}", my_id, paused);
                                        let _ = app_state.game_tx.send(GameCommand::SetPaused {
                                            player_id: my_id,
                                            paused,
                                        }).await;
                                    }
                                }
                            }
                            Err(e) => {
                                parse_error_count += 1;
                                // Log truncated message to avoid log spam from huge payloads
                                let preview_len = min(text_str.len(), 120);
                                tracing::warn!(
                                    "Player {} parse error: {} (len={} preview={:?})",
                                    my_id, e, text_str.len(), &text_str[..preview_len]
                                );
                                if parse_error_count >= MAX_PARSE_ERRORS {
                                    tracing::warn!(
                                        "Player {} exceeded max parse errors ({}), disconnecting",
                                        my_id, MAX_PARSE_ERRORS
                                    );
                                    break;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {} // Ignore ping/pong/binary
                }
            }

            // Server -> Client (reliable per-client events like TransferIn)
            event = client_rx.recv() => {
                match event {
                    Some(ClientEvent::TransferIn { vx, vy, owner_id, color }) => {
                        let json = serde_json::to_string(&ServerMsg::TransferIn(
                            TransferInMsg { vx, vy, owner_id, color },
                        ));
                        if let Ok(json) = json {
                            if sink.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Some(ClientEvent::Disconnect) => {
                        tracing::info!("Player {} received disconnect from server", my_id);
                        break;
                    }
                    None => {
                        // Channel closed by server (client marked as dead)
                        tracing::info!("Player {} channel closed by server", my_id);
                        break;
                    }
                }
            }

            // Server -> Client (broadcast - lossy, ok to drop on lag)
            // JSON is pre-serialized as Utf8Bytes in game_loop - O(1) clone, no allocation
            result = broadcast_rx.recv() => {
                match result {
                    Ok(broadcast) => {
                        let utf8 = match broadcast {
                            GameBroadcast::SpaceState(b) => b,
                            GameBroadcast::PlayersState(b) => b,
                        };
                        if sink.send(Message::Text(utf8)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Player {} lagged by {} messages", my_id, n);
                        // Continue - space_state is stateless, dropping is fine
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    // Cleanup on disconnect
    let _ = app_state
        .game_tx
        .send(GameCommand::PlayerLeave { id: my_id })
        .await;
    tracing::info!("Player {} disconnected", my_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_VEL: f64 = 10.0;

    #[test]
    fn valid_velocity_passes() {
        let result = validate_ball_escaped(1.5, -2.0, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::Valid { vx: 1.5, vy: -2.0 });
    }

    #[test]
    fn velocity_is_clamped_to_max() {
        let result = validate_ball_escaped(15.0, -20.0, MAX_VEL);
        assert_eq!(
            result,
            BallEscapedValidation::Valid {
                vx: 10.0,
                vy: -10.0
            }
        );
    }

    #[test]
    fn negative_vx_is_clamped() {
        let result = validate_ball_escaped(-15.0, -5.0, MAX_VEL);
        assert_eq!(
            result,
            BallEscapedValidation::Valid {
                vx: -10.0,
                vy: -5.0
            }
        );
    }

    #[test]
    fn nan_vx_rejected() {
        let result = validate_ball_escaped(f64::NAN, -2.0, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::InvalidNonFinite);
    }

    #[test]
    fn nan_vy_rejected() {
        let result = validate_ball_escaped(1.0, f64::NAN, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::InvalidNonFinite);
    }

    #[test]
    fn infinity_rejected() {
        let result = validate_ball_escaped(f64::INFINITY, -2.0, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::InvalidNonFinite);
    }

    #[test]
    fn negative_infinity_rejected() {
        let result = validate_ball_escaped(1.0, f64::NEG_INFINITY, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::InvalidNonFinite);
    }

    #[test]
    fn positive_vy_rejected() {
        let result = validate_ball_escaped(1.0, 2.0, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::InvalidVyPositive);
    }

    #[test]
    fn zero_vy_rejected() {
        let result = validate_ball_escaped(1.0, 0.0, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::InvalidVyPositive);
    }

    #[test]
    fn near_zero_velocity_rejected() {
        let result = validate_ball_escaped(0.01, -0.01, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::InvalidTooSlow);
    }

    #[test]
    fn zero_velocity_rejected() {
        let result = validate_ball_escaped(0.0, -0.001, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::InvalidTooSlow);
    }

    #[test]
    fn minimum_valid_speed_passes() {
        // speed² = 0.1² = 0.01, but we need > 0.01
        // speed² = 0.11² ≈ 0.012 > 0.01
        let result = validate_ball_escaped(0.0, -0.11, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::Valid { vx: 0.0, vy: -0.11 });
    }

    #[test]
    fn edge_case_small_negative_vy() {
        // Small but valid negative vy with enough speed
        let result = validate_ball_escaped(0.5, -0.5, MAX_VEL);
        assert_eq!(result, BallEscapedValidation::Valid { vx: 0.5, vy: -0.5 });
    }
}
