use axum::routing::get;
use axum::Router;
use pinball_server::config::ServerConfig;
use pinball_server::game_loop::{run_game_loop, GameBroadcast, GameCommand};
use pinball_server::ws::{ws_handler, AppState};
use tokio::sync::{broadcast, mpsc};
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = ServerConfig::default();

    // Validate configuration before starting
    if let Err(e) = config.validate() {
        eprintln!("Invalid server configuration: {}", e);
        std::process::exit(1);
    }

    let listen_addr = config.listen_addr.clone();
    let max_velocity = config.max_velocity;
    let max_ball_escaped_per_sec = config.max_ball_escaped_per_sec;

    let (game_tx, game_rx) = mpsc::channel::<GameCommand>(256);
    let (broadcast_tx, _) = broadcast::channel::<GameBroadcast>(64);

    // Spawn game loop
    let bc_tx = broadcast_tx.clone();
    tokio::spawn(async move {
        run_game_loop(game_rx, bc_tx, config).await;
    });

    // Axum app
    let app_state = AppState {
        game_tx,
        broadcast_tx,
        max_velocity,
        max_ball_escaped_per_sec,
    };
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    tracing::info!("Starting pinball server on {}", listen_addr);
    println!("Pinball server listening on {}", listen_addr);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
