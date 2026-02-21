use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, oneshot, Semaphore};

use crate::game_loop::{ClientEvent, GameBroadcast, GameCommand};
use crate::protocol::{ClientMsg, ServerMsg, TransferInMsg};

/// Maximum size of a text message from client (bytes)
const MAX_TEXT_MSG_BYTES: usize = 1024;
/// Maximum consecutive parse errors before disconnecting
const MAX_PARSE_ERRORS: u32 = 5;
/// Timeout for sending messages to client (slow consumer protection)
const SEND_TIMEOUT: Duration = Duration::from_secs(5);
/// Maximum set_paused messages per second per client
const MAX_SET_PAUSED_PER_SEC: u32 = 10;
/// Maximum activity messages per second per client
const MAX_ACTIVITY_PER_SEC: u32 = 1;

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
    /// Semaphore to limit concurrent connections
    pub connection_semaphore: Arc<Semaphore>,
    /// Allowed origins for WebSocket connections (empty = allow all)
    pub allowed_origins: Vec<String>,
}

/// Check if the Origin header is allowed
fn is_origin_allowed(headers: &HeaderMap, allowed_origins: &[String]) -> bool {
    // If no allowed origins configured, allow all (open game server)
    if allowed_origins.is_empty() {
        return true;
    }

    let origin = match headers.get("origin").and_then(|v| v.to_str().ok()) {
        Some(o) => o,
        None => {
            // No Origin header - could be same-origin or non-browser client
            // Allow for flexibility (browsers always send Origin on cross-origin)
            return true;
        }
    };

    allowed_origins.iter().any(|allowed| allowed == origin)
}

/// HTTP handler for WebSocket upgrade
pub async fn ws_handler(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    // Check Origin header for CSRF protection
    if !is_origin_allowed(&headers, &app_state.allowed_origins) {
        let origin = headers
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("<none>");
        tracing::warn!("Connection rejected: origin not allowed: {}", origin);
        return (axum::http::StatusCode::FORBIDDEN, "Origin not allowed").into_response();
    }

    // Try to acquire a connection permit
    let permit = match app_state.connection_semaphore.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            tracing::warn!("Connection rejected: max connections reached");
            // Return 503 Service Unavailable instead of upgrading
            // This avoids giving attackers free WebSocket handshake work
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "Server at max connections",
            )
                .into_response();
        }
    };
    ws.on_upgrade(|socket| handle_socket(socket, app_state, permit))
        .into_response()
}

async fn handle_socket(
    socket: WebSocket,
    app_state: AppState,
    _permit: tokio::sync::OwnedSemaphorePermit,
) {
    // _permit is held for the lifetime of this function, automatically released on drop
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

    // Send welcome message (with timeout for slow consumer protection)
    let welcome_json = match serde_json::to_string(&ServerMsg::Welcome(welcome)) {
        Ok(json) => json,
        Err(e) => {
            tracing::error!("Player {} failed to serialize welcome: {}", my_id, e);
            return;
        }
    };
    if tokio::time::timeout(SEND_TIMEOUT, sink.send(Message::Text(welcome_json.into())))
        .await
        .map_err(|_| ())
        .and_then(|r| r.map_err(|_| ()))
        .is_err()
    {
        tracing::warn!("Player {} welcome send timeout/error, disconnecting", my_id);
        return;
    }

    // Subscribe to broadcasts
    let mut broadcast_rx = app_state.broadcast_tx.subscribe();

    // Rate limiting per message type.
    // Consequences differ by severity:
    //   ball_escaped: disconnect (most exploitable — spawns balls in deep space)
    //   set_paused:   ignore excess (low risk, just a flag toggle)
    //   activity:     silently drop (heartbeat, no game effect)
    let mut ball_escaped_count: u32 = 0;
    let mut ball_escaped_window_start = Instant::now();
    let mut set_paused_count: u32 = 0;
    let mut set_paused_window_start = Instant::now();
    let mut activity_count: u32 = 0;
    let mut activity_window_start = Instant::now();
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
                                        // Rate limiting FIRST (before validation)
                                        // This prevents attackers from spamming invalid messages
                                        let now = Instant::now();
                                        if now.duration_since(ball_escaped_window_start).as_secs_f64() >= 1.0 {
                                            ball_escaped_window_start = now;
                                            ball_escaped_count = 0;
                                        }
                                        ball_escaped_count += 1;
                                        if ball_escaped_count > max_per_sec {
                                            tracing::warn!("Player {} exceeded rate limit ({} ball_escaped/sec), disconnecting", my_id, max_per_sec);
                                            break;
                                        }

                                        // Validate and clamp velocity
                                        // Use trace level to avoid log spam from invalid messages
                                        let (vx, vy) = match validate_ball_escaped(vx, vy, max_velocity) {
                                            BallEscapedValidation::Valid { vx, vy } => (vx, vy),
                                            BallEscapedValidation::InvalidNonFinite => {
                                                tracing::trace!("Player {} sent invalid velocity (NaN/Inf), ignoring", my_id);
                                                continue;
                                            }
                                            BallEscapedValidation::InvalidVyPositive => {
                                                tracing::trace!("Player {} sent invalid vy (must be negative), ignoring", my_id);
                                                continue;
                                            }
                                            BallEscapedValidation::InvalidTooSlow => {
                                                tracing::trace!("Player {} sent near-zero velocity, ignoring", my_id);
                                                continue;
                                            }
                                        };

                                        // Hot path - only log at trace level
                                        tracing::trace!("Player {} ball_escaped", my_id);
                                        let _ = app_state.game_tx.send(GameCommand::BallEscaped {
                                            owner_id: my_id,
                                            vx,
                                            vy,
                                        }).await;
                                    }
                                    ClientMsg::SetPaused { paused } => {
                                        // Rate limiting for set_paused
                                        let now = Instant::now();
                                        if now.duration_since(set_paused_window_start).as_secs_f64() >= 1.0 {
                                            set_paused_window_start = now;
                                            set_paused_count = 0;
                                        }
                                        set_paused_count += 1;
                                        if set_paused_count > MAX_SET_PAUSED_PER_SEC {
                                            tracing::warn!("Player {} exceeded set_paused rate limit, ignoring", my_id);
                                            continue;
                                        }

                                        tracing::trace!("Player {} set_paused={}", my_id, paused);
                                        let _ = app_state.game_tx.send(GameCommand::SetPaused {
                                            player_id: my_id,
                                            paused,
                                        }).await;
                                    }
                                    ClientMsg::Activity => {
                                        // Rate limiting for activity
                                        let now = Instant::now();
                                        if now.duration_since(activity_window_start).as_secs_f64() >= 1.0 {
                                            activity_window_start = now;
                                            activity_count = 0;
                                        }
                                        activity_count += 1;
                                        if activity_count > MAX_ACTIVITY_PER_SEC {
                                            continue;
                                        }

                                        let _ = app_state.game_tx.send(GameCommand::Activity {
                                            player_id: my_id,
                                        }).await;
                                    }
                                }
                            }
                            Err(e) => {
                                parse_error_count += 1;
                                // Log truncated message to avoid log spam from huge payloads
                                // Truncate at a character boundary to avoid panicking on
                                // multibyte UTF-8 sequences when slicing by byte index.
                                let preview: String = text_str.chars().take(120).collect();
                                tracing::warn!(
                                    "Player {} parse error: {} (len={} preview={:?})",
                                    my_id, e, text_str.len(), preview
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
                    Some(Ok(Message::Binary(_))) => {
                        // Binary frames are not expected - disconnect
                        tracing::warn!("Player {} sent binary frame, disconnecting", my_id);
                        break;
                    }
                    _ => {} // Ignore ping/pong
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
                            // Timeout for slow consumer protection
                            if tokio::time::timeout(SEND_TIMEOUT, sink.send(Message::Text(json.into())))
                                .await
                                .map_err(|_| ())
                                .and_then(|r| r.map_err(|_| ()))
                                .is_err()
                            {
                                tracing::warn!("Player {} send timeout/error on TransferIn, disconnecting", my_id);
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
                        // Timeout for slow consumer protection
                        if tokio::time::timeout(SEND_TIMEOUT, sink.send(Message::Text(utf8)))
                            .await
                            .map_err(|_| ())
                            .and_then(|r| r.map_err(|_| ()))
                            .is_err()
                        {
                            tracing::warn!("Player {} send timeout/error on broadcast, disconnecting", my_id);
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
    use axum::http::HeaderMap;

    // --- is_origin_allowed tests ---

    fn headers_with_origin(origin: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("origin", origin.parse().unwrap());
        h
    }

    #[test]
    fn origin_allowed_when_list_is_empty() {
        // Empty list = open server, everything allowed
        let headers = headers_with_origin("https://evil.example.com");
        assert!(is_origin_allowed(&headers, &[]));
    }

    #[test]
    fn origin_allowed_when_no_origin_header_and_list_is_empty() {
        assert!(is_origin_allowed(&HeaderMap::new(), &[]));
    }

    #[test]
    fn origin_allowed_when_exact_match() {
        let headers = headers_with_origin("https://pinball.vatnar.no");
        let allowed = vec!["https://pinball.vatnar.no".to_string()];
        assert!(is_origin_allowed(&headers, &allowed));
    }

    #[test]
    fn origin_rejected_when_not_in_list() {
        let headers = headers_with_origin("https://evil.example.com");
        let allowed = vec!["https://pinball.vatnar.no".to_string()];
        assert!(!is_origin_allowed(&headers, &allowed));
    }

    #[test]
    fn origin_allowed_when_one_of_multiple_matches() {
        let headers = headers_with_origin("https://pinballbevy.vatnar.no");
        let allowed = vec![
            "https://pinball.vatnar.no".to_string(),
            "https://pinballbevy.vatnar.no".to_string(),
        ];
        assert!(is_origin_allowed(&headers, &allowed));
    }

    #[test]
    fn missing_origin_header_allowed_even_when_list_configured() {
        // No Origin header — treated as non-browser / same-origin client
        let allowed = vec!["https://pinball.vatnar.no".to_string()];
        assert!(is_origin_allowed(&HeaderMap::new(), &allowed));
    }

    #[test]
    fn origin_match_is_exact_not_prefix() {
        // "https://pinball.vatnar.no.evil.com" must NOT match "https://pinball.vatnar.no"
        let headers = headers_with_origin("https://pinball.vatnar.no.evil.com");
        let allowed = vec!["https://pinball.vatnar.no".to_string()];
        assert!(!is_origin_allowed(&headers, &allowed));
    }

    #[test]
    fn origin_match_is_case_sensitive() {
        let headers = headers_with_origin("https://Pinball.Vatnar.No");
        let allowed = vec!["https://pinball.vatnar.no".to_string()];
        assert!(!is_origin_allowed(&headers, &allowed));
    }

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
