//! Integration tests for the pinball server.
//!
//! These tests start a real server instance and connect via WebSocket
//! to verify end-to-end behavior.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Re-create minimal protocol types for testing (to avoid circular deps)
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum ServerMsg {
    #[serde(rename = "welcome")]
    Welcome {
        #[serde(rename = "protocolVersion")]
        protocol_version: u32,
        #[serde(rename = "selfId")]
        self_id: u32,
        players: Vec<serde_json::Value>,
        config: serde_json::Value,
    },
    #[serde(rename = "players_state")]
    PlayersState { players: Vec<serde_json::Value> },
    #[serde(rename = "space_state")]
    SpaceState { balls: Vec<serde_json::Value> },
    #[serde(rename = "transfer_in")]
    TransferIn {
        vx: f64,
        vy: f64,
        #[serde(rename = "ownerId")]
        owner_id: u32,
        color: u32,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ClientMsg {
    #[serde(rename = "ball_escaped")]
    BallEscaped { vx: f64, vy: f64 },
    #[serde(rename = "set_paused")]
    SetPaused { paused: bool },
}

/// Start a test server on a random available port and return the WebSocket URL.
async fn start_test_server() -> String {
    use pinball_server::config::ServerConfig;
    use pinball_server::game_loop::{run_game_loop, GameBroadcast, GameCommand};
    use pinball_server::ws::AppState;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener); // Release the port so the server can bind to it

    let config = ServerConfig {
        listen_addr: addr.to_string(),
        tick_rate_hz: 60,
        broadcast_rate_hz: 10,
        cell_count: 100,
        rng_seed: 12345,
        max_velocity: 10.0,
        max_ball_escaped_per_sec: 30,
        max_connections: 100,
        max_balls_global: 1000,
        allowed_origins: vec![],
    };

    let (game_tx, game_rx) = mpsc::channel::<GameCommand>(256);
    let (broadcast_tx, _) = broadcast::channel::<GameBroadcast>(64);

    let app_state = AppState {
        game_tx,
        broadcast_tx: broadcast_tx.clone(),
        max_velocity: config.max_velocity,
        max_ball_escaped_per_sec: config.max_ball_escaped_per_sec,
        connection_semaphore: Arc::new(Semaphore::new(config.max_connections)),
        allowed_origins: vec![],
    };

    // Start game loop
    let game_config = config.clone();
    tokio::spawn(async move {
        run_game_loop(game_rx, broadcast_tx, game_config).await;
    });

    // Start HTTP/WebSocket server
    let app = axum::Router::new()
        .route("/ws", axum::routing::get(pinball_server::ws::ws_handler))
        .with_state(app_state);

    tokio::spawn(async move {
        let listener = TcpListener::bind(&config.listen_addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    format!("ws://{}/ws", addr)
}

/// Connect to the server and return the WebSocket stream.
async fn connect(
    url: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let (ws, _) = connect_async(url).await.expect("Failed to connect");
    ws
}

/// Read the next text message and parse as ServerMsg.
async fn recv_msg(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> ServerMsg {
    loop {
        match ws.next().await {
            Some(Ok(Message::Text(text))) => {
                return serde_json::from_str(&text).expect("Failed to parse server message");
            }
            Some(Ok(_)) => continue, // Skip ping/pong
            Some(Err(e)) => panic!("WebSocket error: {}", e),
            None => panic!("WebSocket closed unexpectedly"),
        }
    }
}

/// Read the next text message with a timeout.
async fn recv_msg_timeout(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    timeout: Duration,
) -> Option<ServerMsg> {
    tokio::time::timeout(timeout, recv_msg(ws)).await.ok()
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_connect_and_receive_welcome() {
    let url = start_test_server().await;
    let mut ws = connect(&url).await;

    let msg = recv_msg(&mut ws).await;
    match msg {
        ServerMsg::Welcome {
            protocol_version,
            self_id,
            players,
            ..
        } => {
            assert_eq!(protocol_version, 1);
            assert!(self_id > 0, "self_id should be positive");
            assert!(!players.is_empty(), "players should include self");
        }
        other => panic!("Expected Welcome, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiple_clients_get_unique_ids() {
    let url = start_test_server().await;

    let mut ws1 = connect(&url).await;
    let mut ws2 = connect(&url).await;

    let msg1 = recv_msg(&mut ws1).await;
    let msg2 = recv_msg(&mut ws2).await;

    let id1 = match msg1 {
        ServerMsg::Welcome { self_id, .. } => self_id,
        _ => panic!("Expected Welcome"),
    };
    let id2 = match msg2 {
        ServerMsg::Welcome { self_id, .. } => self_id,
        _ => panic!("Expected Welcome"),
    };

    assert_ne!(id1, id2, "Each client should get a unique ID");
}

#[tokio::test]
async fn test_valid_ball_escaped_is_accepted() {
    let url = start_test_server().await;
    let mut ws = connect(&url).await;

    // Get welcome
    let _welcome = recv_msg(&mut ws).await;

    // Send valid ball_escaped
    let msg = ClientMsg::BallEscaped { vx: 1.0, vy: -2.0 };
    let json = serde_json::to_string(&msg).unwrap();
    ws.send(Message::Text(json.into())).await.unwrap();

    // Wait for space_state broadcast (should include our ball eventually)
    // The server broadcasts at 10Hz, so wait up to 200ms
    let mut found_ball = false;
    for _ in 0..5 {
        if let Some(msg) = recv_msg_timeout(&mut ws, Duration::from_millis(200)).await {
            if let ServerMsg::SpaceState { balls } = msg {
                if !balls.is_empty() {
                    found_ball = true;
                    break;
                }
            }
        }
    }
    assert!(
        found_ball,
        "Ball should appear in space_state after valid ball_escaped"
    );
}

#[tokio::test]
async fn test_invalid_ball_escaped_positive_vy_ignored() {
    let url = start_test_server().await;
    let mut ws = connect(&url).await;

    // Get welcome
    let _welcome = recv_msg(&mut ws).await;

    // Send invalid ball_escaped (positive vy)
    let msg = ClientMsg::BallEscaped { vx: 1.0, vy: 2.0 };
    let json = serde_json::to_string(&msg).unwrap();
    ws.send(Message::Text(json.into())).await.unwrap();

    // Wait for space_state - should NOT contain our ball
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Check that space is empty (invalid message was ignored)
    if let Some(msg) = recv_msg_timeout(&mut ws, Duration::from_millis(200)).await {
        if let ServerMsg::SpaceState { balls } = msg {
            assert!(balls.is_empty(), "Invalid ball_escaped should be ignored");
        }
    }
}

#[tokio::test]
async fn test_set_paused_updates_player_state() {
    let url = start_test_server().await;
    let mut ws = connect(&url).await;

    // Get welcome and extract self_id
    let self_id = match recv_msg(&mut ws).await {
        ServerMsg::Welcome { self_id, .. } => self_id,
        _ => panic!("Expected Welcome"),
    };

    // Send set_paused
    let msg = ClientMsg::SetPaused { paused: true };
    let json = serde_json::to_string(&msg).unwrap();
    ws.send(Message::Text(json.into())).await.unwrap();

    // Wait for players_state broadcast
    let mut found_paused = false;
    for _ in 0..10 {
        if let Some(msg) = recv_msg_timeout(&mut ws, Duration::from_millis(200)).await {
            if let ServerMsg::PlayersState { players } = msg {
                // Find our player and check paused status
                for p in &players {
                    if let Some(id) = p.get("id").and_then(|v| v.as_u64()) {
                        if id == self_id as u64 {
                            if let Some(paused) = p.get("paused").and_then(|v| v.as_bool()) {
                                if paused {
                                    found_paused = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                if found_paused {
                    break;
                }
            }
        }
    }
    assert!(
        found_paused,
        "Player should be marked as paused after set_paused"
    );
}

#[tokio::test]
async fn test_player_disconnect_removes_from_players_state() {
    let url = start_test_server().await;

    // Connect two clients
    let mut ws1 = connect(&url).await;
    let mut ws2 = connect(&url).await;

    let id1 = match recv_msg(&mut ws1).await {
        ServerMsg::Welcome { self_id, .. } => self_id,
        _ => panic!("Expected Welcome"),
    };
    let _id2 = match recv_msg(&mut ws2).await {
        ServerMsg::Welcome { self_id, .. } => self_id,
        _ => panic!("Expected Welcome"),
    };

    // Disconnect client 1
    ws1.close(None).await.unwrap();

    // Wait for players_state update on client 2
    let mut player1_removed = false;
    for _ in 0..10 {
        if let Some(msg) = recv_msg_timeout(&mut ws2, Duration::from_millis(200)).await {
            if let ServerMsg::PlayersState { players } = msg {
                let has_player1 = players
                    .iter()
                    .any(|p| p.get("id").and_then(|v| v.as_u64()) == Some(id1 as u64));
                if !has_player1 {
                    player1_removed = true;
                    break;
                }
            }
        }
    }
    assert!(
        player1_removed,
        "Disconnected player should be removed from players_state"
    );
}

/// Start a test server with custom rate limit for testing.
async fn start_test_server_with_rate_limit(max_per_sec: u32) -> String {
    use pinball_server::config::ServerConfig;
    use pinball_server::game_loop::{run_game_loop, GameBroadcast, GameCommand};
    use pinball_server::ws::AppState;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let config = ServerConfig {
        listen_addr: addr.to_string(),
        tick_rate_hz: 60,
        broadcast_rate_hz: 10,
        cell_count: 100,
        rng_seed: 12345,
        max_velocity: 10.0,
        max_ball_escaped_per_sec: max_per_sec,
        max_connections: 100,
        max_balls_global: 1000,
        allowed_origins: vec![],
    };

    let (game_tx, game_rx) = mpsc::channel::<GameCommand>(256);
    let (broadcast_tx, _) = broadcast::channel::<GameBroadcast>(64);

    let app_state = AppState {
        game_tx,
        broadcast_tx: broadcast_tx.clone(),
        max_velocity: config.max_velocity,
        max_ball_escaped_per_sec: config.max_ball_escaped_per_sec,
        connection_semaphore: Arc::new(Semaphore::new(config.max_connections)),
        allowed_origins: vec![],
    };

    let game_config = config.clone();
    tokio::spawn(async move {
        run_game_loop(game_rx, broadcast_tx, game_config).await;
    });

    let app = axum::Router::new()
        .route("/ws", axum::routing::get(pinball_server::ws::ws_handler))
        .with_state(app_state);

    tokio::spawn(async move {
        let listener = TcpListener::bind(&config.listen_addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    format!("ws://{}/ws", addr)
}

#[tokio::test]
async fn test_oversized_message_disconnects_client() {
    let url = start_test_server().await;
    let mut ws = connect(&url).await;

    // Get welcome
    let _welcome = recv_msg(&mut ws).await;

    // Send an oversized message (> 1024 bytes)
    let huge_payload = "x".repeat(2000);
    let msg = format!(
        r#"{{"type":"ball_escaped","vx":1.0,"vy":-1.0,"extra":"{}"}}"#,
        huge_payload
    );
    let _ = ws.send(Message::Text(msg.into())).await;

    // Try to receive - server should close the connection
    let mut disconnected = false;
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        match tokio::time::timeout(Duration::from_millis(100), ws.next()).await {
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                disconnected = true;
                break;
            }
            Err(_) => {
                // Timeout - try sending to check if connection is dead
                if ws.send(Message::Ping(vec![].into())).await.is_err() {
                    disconnected = true;
                    break;
                }
            }
            _ => continue,
        }
    }
    assert!(
        disconnected,
        "Client should be disconnected after oversized message"
    );
}

#[tokio::test]
async fn test_parse_spam_disconnects_client() {
    let url = start_test_server().await;
    let mut ws = connect(&url).await;

    // Get welcome
    let _welcome = recv_msg(&mut ws).await;

    // Send multiple invalid JSON messages (parse errors)
    for _ in 0..10 {
        let _ = ws.send(Message::Text("not valid json".into())).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Try to receive - server should close the connection after MAX_PARSE_ERRORS (5)
    let mut disconnected = false;
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        match tokio::time::timeout(Duration::from_millis(100), ws.next()).await {
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                disconnected = true;
                break;
            }
            Err(_) => {
                // Timeout - try sending to check if connection is dead
                if ws.send(Message::Ping(vec![].into())).await.is_err() {
                    disconnected = true;
                    break;
                }
            }
            _ => continue,
        }
    }
    assert!(
        disconnected,
        "Client should be disconnected after too many parse errors"
    );
}

#[tokio::test]
async fn test_rate_limiting_disconnects_abusive_client() {
    // Start server with low rate limit (5 per second)
    let url = start_test_server_with_rate_limit(5).await;
    let mut ws = connect(&url).await;

    // Get welcome
    let _welcome = recv_msg(&mut ws).await;

    // Send many ball_escaped messages rapidly (more than rate limit)
    let mut send_failed = false;
    for i in 0..10 {
        let msg = ClientMsg::BallEscaped {
            vx: 1.0 + i as f64 * 0.1,
            vy: -2.0,
        };
        let json = serde_json::to_string(&msg).unwrap();
        if ws.send(Message::Text(json.into())).await.is_err() {
            // Connection was closed by server - this is expected
            send_failed = true;
            break;
        }
    }

    if send_failed {
        // Already disconnected - test passes
        return;
    }

    // Give server time to process and disconnect us
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify connection is closed by trying to send more messages
    for _ in 0..5 {
        let msg = ClientMsg::BallEscaped { vx: 1.0, vy: -1.0 };
        let json = serde_json::to_string(&msg).unwrap();
        if ws.send(Message::Text(json.into())).await.is_err() {
            // Disconnected - test passes
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // If we get here, check if we can still receive (connection should be dead)
    let result = recv_msg_timeout(&mut ws, Duration::from_millis(200)).await;
    // Connection should have been closed by server
    assert!(
        result.is_none() || ws.send(Message::Ping(vec![].into())).await.is_err(),
        "Client should be disconnected after exceeding rate limit"
    );
}
