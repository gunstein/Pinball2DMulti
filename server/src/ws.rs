use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::game_loop::{ClientEvent, GameBroadcast, GameCommand};
use crate::protocol::{ClientMsg, ServerMsg, TransferInMsg};

// Note: ServerMsg is still used for TransferIn serialization

/// Shared app state passed to each WebSocket handler
#[derive(Clone)]
pub struct AppState {
    pub game_tx: mpsc::Sender<GameCommand>,
    pub broadcast_tx: broadcast::Sender<GameBroadcast>,
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

    loop {
        tokio::select! {
            // Client -> Server
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let text_str: &str = &text;
                        match serde_json::from_str::<ClientMsg>(text_str) {
                            Ok(client_msg) => {
                                match client_msg {
                                    ClientMsg::BallEscaped { vx, vy } => {
                                        tracing::info!("Player {} sent ball_escaped vx={:.2}, vy={:.2}", my_id, vx, vy);
                                        let _ = app_state.game_tx.send(GameCommand::BallEscaped {
                                            owner_id: my_id,
                                            vx,
                                            vy,
                                        }).await;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse client message: {} (raw: {})", e, text_str);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {} // Ignore ping/pong/binary
                }
            }

            // Server -> Client (reliable per-client events like TransferIn)
            Some(event) = client_rx.recv() => {
                match event {
                    ClientEvent::TransferIn { vx, vy } => {
                        let json = serde_json::to_string(&ServerMsg::TransferIn(
                            TransferInMsg { vx, vy },
                        ));
                        if let Ok(json) = json {
                            if sink.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    ClientEvent::Disconnect => {
                        tracing::info!("Player {} received disconnect from server", my_id);
                        break;
                    }
                }
            }

            // Server -> Client (broadcast - lossy, ok to drop on lag)
            // JSON is pre-serialized in game_loop, just send directly
            result = broadcast_rx.recv() => {
                match result {
                    Ok(broadcast) => {
                        let json: &str = match &broadcast {
                            GameBroadcast::SpaceState(json) => json,
                            GameBroadcast::PlayersState(json) => json,
                        };

                        if sink.send(Message::Text(json.to_string().into())).await.is_err() {
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
