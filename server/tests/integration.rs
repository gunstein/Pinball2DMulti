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
    #[serde(rename = "activity")]
    Activity,
}

/// Configuration overrides for test servers.
#[derive(Default)]
struct TestServerOptions {
    bot_count: Option<usize>,
    max_ball_escaped_per_sec: Option<u32>,
    max_connections: Option<usize>,
    deep_space_config: Option<pinball_server::config::DeepSpaceConfig>,
}

/// Start a test server with default options.
async fn start_test_server() -> String {
    start_test_server_with_options(TestServerOptions::default()).await
}

/// Start a test server with custom options.
async fn start_test_server_with_options(opts: TestServerOptions) -> String {
    use pinball_server::config::ServerConfig;
    use pinball_server::game_loop::{
        run_game_loop, run_game_loop_with_config, GameBroadcast, GameCommand,
    };
    use pinball_server::ws::AppState;

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
        max_ball_escaped_per_sec: opts.max_ball_escaped_per_sec.unwrap_or(30),
        max_connections: opts.max_connections.unwrap_or(100),
        max_balls_global: 1000,
        allowed_origins: vec![],
        bot_count: opts.bot_count.unwrap_or(0),
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
    let ds_config = opts.deep_space_config;
    tokio::spawn(async move {
        if let Some(ds) = ds_config {
            run_game_loop_with_config(game_rx, broadcast_tx, game_config, ds).await;
        } else {
            run_game_loop(game_rx, broadcast_tx, game_config).await;
        }
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

/// Helper: get self_id from welcome message.
fn extract_self_id(msg: ServerMsg) -> u32 {
    match msg {
        ServerMsg::Welcome { self_id, .. } => self_id,
        other => panic!("Expected Welcome, got {:?}", other),
    }
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
    let url = start_test_server_with_options(TestServerOptions {
        max_ball_escaped_per_sec: Some(5),
        ..Default::default()
    })
    .await;
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

// ============================================================================
// Ball lifecycle: escape -> deep space -> capture -> transfer_in
// ============================================================================

#[tokio::test]
async fn test_ball_lifecycle_escape_to_transfer_in() {
    // Use very short min_age_for_capture so capture happens quickly.
    // Large portal_alpha so the two portals (on a 100-cell sphere) overlap easily.
    let ds_config = pinball_server::config::DeepSpaceConfig {
        portal_alpha: 1.0, // ~57 degrees — very wide portals
        omega_min: 3.0,    // fast movement
        omega_max: 3.0,
        min_age_for_capture: 0.1, // capture almost immediately
        reroute_after: 100.0,     // disable reroute
        reroute_cooldown: 100.0,
        min_age_for_reroute: 100.0,
        reroute_arrival_time_min: 4.0,
        reroute_arrival_time_max: 10.0,
    };
    let url = start_test_server_with_options(TestServerOptions {
        deep_space_config: Some(ds_config),
        ..Default::default()
    })
    .await;

    let mut ws1 = connect(&url).await;
    let mut ws2 = connect(&url).await;
    let _id1 = extract_self_id(recv_msg(&mut ws1).await);
    let _id2 = extract_self_id(recv_msg(&mut ws2).await);

    // Client 1 sends a ball into deep space
    let msg = ClientMsg::BallEscaped { vx: 1.0, vy: -2.0 };
    ws1.send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
        .await
        .unwrap();

    // Wait for client 2 to receive a transfer_in (ball captured by their portal)
    // With wide portals and fast movement, this should happen within a few seconds.
    let mut got_transfer = false;
    for _ in 0..60 {
        if let Some(msg) = recv_msg_timeout(&mut ws2, Duration::from_millis(200)).await {
            if let ServerMsg::TransferIn {
                vx,
                vy,
                owner_id,
                color,
            } = msg
            {
                assert!(vx.is_finite(), "vx should be finite");
                assert!(vy.is_finite(), "vy should be finite");
                assert!(owner_id > 0, "owner_id should be set");
                assert!(color > 0, "color should be set");
                got_transfer = true;
                break;
            }
        }
    }
    // Note: with only 2 players on a 100-cell sphere, capture may go to either.
    // If client 2 didn't get it, check client 1 (self-capture is allowed for real players).
    if !got_transfer {
        for _ in 0..30 {
            if let Some(msg) = recv_msg_timeout(&mut ws1, Duration::from_millis(200)).await {
                if let ServerMsg::TransferIn { .. } = msg {
                    got_transfer = true;
                    break;
                }
            }
        }
    }
    assert!(
        got_transfer,
        "A transfer_in should be delivered after ball escapes and is captured"
    );
}

// ============================================================================
// Multi-player ball visibility
// ============================================================================

#[tokio::test]
async fn test_other_player_sees_ball_in_space_state() {
    let url = start_test_server().await;

    let mut ws1 = connect(&url).await;
    let mut ws2 = connect(&url).await;
    let id1 = extract_self_id(recv_msg(&mut ws1).await);
    let _id2 = extract_self_id(recv_msg(&mut ws2).await);

    // Client 1 sends a ball
    let msg = ClientMsg::BallEscaped { vx: 1.0, vy: -2.0 };
    ws1.send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
        .await
        .unwrap();

    // Client 2 should see the ball in space_state with correct ownerId
    let mut found = false;
    for _ in 0..10 {
        if let Some(msg) = recv_msg_timeout(&mut ws2, Duration::from_millis(200)).await {
            if let ServerMsg::SpaceState { balls } = msg {
                for b in &balls {
                    if let Some(oid) = b.get("ownerId").and_then(|v| v.as_u64()) {
                        if oid == id1 as u64 {
                            found = true;
                            break;
                        }
                    }
                }
                if found {
                    break;
                }
            }
        }
    }
    assert!(found, "Client 2 should see client 1's ball in space_state");
}

// ============================================================================
// Bot integration
// ============================================================================

#[tokio::test]
async fn test_bots_appear_in_player_list_and_produce_balls() {
    let url = start_test_server_with_options(TestServerOptions {
        bot_count: Some(2),
        ..Default::default()
    })
    .await;

    let mut ws = connect(&url).await;
    let welcome = recv_msg(&mut ws).await;

    // Welcome should include self + 2 bots = 3 players
    let player_count = match &welcome {
        ServerMsg::Welcome { players, .. } => players.len(),
        other => panic!("Expected Welcome, got {:?}", other),
    };
    assert!(
        player_count >= 3,
        "Should have self + 2 bots, got {} players",
        player_count,
    );

    // Send activity heartbeat so bots become active
    ws.send(Message::Text(
        serde_json::to_string(&ClientMsg::Activity).unwrap().into(),
    ))
    .await
    .unwrap();

    // Wait for bots to produce balls (they send initial balls after 2-8s)
    let mut found_bot_ball = false;
    for _ in 0..100 {
        if let Some(msg) = recv_msg_timeout(&mut ws, Duration::from_millis(200)).await {
            if let ServerMsg::SpaceState { balls } = msg {
                if !balls.is_empty() {
                    found_bot_ball = true;
                    break;
                }
            }
        }
    }
    assert!(
        found_bot_ball,
        "Bots should produce balls visible in space_state"
    );
}

// ============================================================================
// Welcome includes all current players
// ============================================================================

#[tokio::test]
async fn test_welcome_includes_existing_players() {
    let url = start_test_server().await;

    let mut ws1 = connect(&url).await;
    let id1 = extract_self_id(recv_msg(&mut ws1).await);

    // Give server time to register player 1
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut ws2 = connect(&url).await;
    let welcome2 = recv_msg(&mut ws2).await;

    // Client 2's welcome should list both players
    match welcome2 {
        ServerMsg::Welcome {
            players, self_id, ..
        } => {
            let ids: Vec<u64> = players
                .iter()
                .filter_map(|p| p.get("id").and_then(|v| v.as_u64()))
                .collect();
            assert!(
                ids.contains(&(id1 as u64)),
                "Welcome should include player 1 (id {}), got {:?}",
                id1,
                ids,
            );
            assert!(
                ids.contains(&(self_id as u64)),
                "Welcome should include self (id {}), got {:?}",
                self_id,
                ids,
            );
        }
        other => panic!("Expected Welcome, got {:?}", other),
    }
}

// ============================================================================
// Activity heartbeat
// ============================================================================

#[tokio::test]
async fn test_activity_message_accepted() {
    let url = start_test_server().await;
    let mut ws = connect(&url).await;
    let _id = extract_self_id(recv_msg(&mut ws).await);

    // Send activity message
    ws.send(Message::Text(
        serde_json::to_string(&ClientMsg::Activity).unwrap().into(),
    ))
    .await
    .unwrap();

    // Connection should remain open — verify by receiving next broadcast
    let msg = recv_msg_timeout(&mut ws, Duration::from_millis(500)).await;
    assert!(
        msg.is_some(),
        "Connection should stay open after activity message"
    );
}

// ============================================================================
// Connection limit
// ============================================================================

#[tokio::test]
async fn test_connection_limit_rejects_excess_clients() {
    let url = start_test_server_with_options(TestServerOptions {
        max_connections: Some(2),
        ..Default::default()
    })
    .await;

    // Connect 2 clients (should succeed)
    let mut ws1 = connect(&url).await;
    let mut ws2 = connect(&url).await;
    let _id1 = extract_self_id(recv_msg(&mut ws1).await);
    let _id2 = extract_self_id(recv_msg(&mut ws2).await);

    // 3rd connection should be rejected (HTTP 503 or immediate close)
    let result = connect_async(&url).await;
    match result {
        Ok((mut ws3, _)) => {
            // Connection may have been upgraded but server should close it immediately
            let msg = recv_msg_timeout(&mut ws3, Duration::from_millis(500)).await;
            // Should NOT get a welcome (server rejects at semaphore)
            assert!(
                msg.is_none() || !matches!(msg, Some(ServerMsg::Welcome { .. })),
                "3rd client should not receive a welcome when at connection limit"
            );
        }
        Err(_) => {
            // Connection refused — this is the expected path
        }
    }
}
