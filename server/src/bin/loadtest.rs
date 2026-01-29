//! Load test for the pinball server.
//!
//! Spawns multiple fake WebSocket clients that:
//! - Connect to the server
//! - Periodically send ball_escaped messages
//! - Receive and count space_state broadcasts
//!
//! Usage: cargo run --bin loadtest -- [OPTIONS]
//!
//! Options:
//!   --clients N      Number of clients to spawn (default: 100)
//!   --duration S     Test duration in seconds (default: 30)
//!   --escape-rate R  Ball escapes per second per client (default: 0.5)
//!   --url URL        Server URL (default: ws://127.0.0.1:9001/ws)

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// === Protocol types (minimal subset) ===

#[derive(Serialize)]
struct BallEscapedMsg {
    #[serde(rename = "type")]
    msg_type: &'static str,
    vx: f64,
    vy: f64,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ServerMsg {
    #[serde(rename = "welcome")]
    Welcome {
        #[serde(rename = "selfId")]
        self_id: u32,
    },
    #[serde(rename = "players_state")]
    PlayersState {},
    #[serde(rename = "space_state")]
    SpaceState { balls: Vec<serde_json::Value> },
    #[serde(rename = "transfer_in")]
    TransferIn { vx: f64, vy: f64 },
}

// === Metrics ===

struct Metrics {
    connected: AtomicU64,
    messages_received: AtomicU64,
    space_states_received: AtomicU64,
    transfer_ins_received: AtomicU64,
    ball_escapes_sent: AtomicU64,
    errors: AtomicU64,
    total_balls_seen: AtomicU64,
    latency_sum_ms: AtomicU64,
    latency_count: AtomicU64,
}

impl Metrics {
    fn new() -> Self {
        Self {
            connected: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            space_states_received: AtomicU64::new(0),
            transfer_ins_received: AtomicU64::new(0),
            ball_escapes_sent: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            total_balls_seen: AtomicU64::new(0),
            latency_sum_ms: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
        }
    }
}

// === Client task ===

async fn run_client(
    client_id: u32,
    url: String,
    escape_rate: f64,
    duration: Duration,
    metrics: Arc<Metrics>,
    _barrier: Arc<Barrier>,
) {
    let connect_start = Instant::now();

    let ws_result = connect_async(&url).await;
    let (mut ws, _) = match ws_result {
        Ok(conn) => {
            if client_id < 3 {
                eprintln!("Client {} connected", client_id);
            }
            conn
        }
        Err(e) => {
            if client_id < 5 {
                eprintln!("Client {} failed to connect: {}", client_id, e);
            }
            metrics.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };

    let connect_latency = connect_start.elapsed();
    metrics
        .latency_sum_ms
        .fetch_add(connect_latency.as_millis() as u64, Ordering::Relaxed);
    metrics.latency_count.fetch_add(1, Ordering::Relaxed);
    metrics.connected.fetch_add(1, Ordering::Relaxed);

    if client_id < 3 {
        eprintln!("Client {} waiting for welcome...", client_id);
    }

    // Wait for welcome message before doing anything else
    let welcome_timeout = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(msg) = ws.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if client_id < 3 {
                        eprintln!(
                            "Client {} got: {}...",
                            client_id,
                            &text.chars().take(60).collect::<String>()
                        );
                    }
                    metrics.messages_received.fetch_add(1, Ordering::Relaxed);
                    if text.contains("\"type\":\"welcome\"") {
                        return true;
                    }
                }
                Ok(Message::Close(frame)) => {
                    if client_id < 3 {
                        eprintln!("Client {} closed during welcome: {:?}", client_id, frame);
                    }
                    return false;
                }
                Err(e) => {
                    if client_id < 3 {
                        eprintln!("Client {} error during welcome: {}", client_id, e);
                    }
                    return false;
                }
                _ => {}
            }
        }
        false
    })
    .await;

    let got_welcome = match welcome_timeout {
        Ok(true) => {
            if client_id < 3 {
                eprintln!("Client {} got welcome!", client_id);
            }
            true
        }
        Ok(false) => {
            if client_id < 3 {
                eprintln!("Client {} failed to get welcome", client_id);
            }
            metrics.errors.fetch_add(1, Ordering::Relaxed);
            metrics.connected.fetch_sub(1, Ordering::Relaxed);
            return;
        }
        Err(_) => {
            if client_id < 3 {
                eprintln!("Client {} welcome timeout", client_id);
            }
            metrics.errors.fetch_add(1, Ordering::Relaxed);
            metrics.connected.fetch_sub(1, Ordering::Relaxed);
            return;
        }
    };

    if !got_welcome {
        return;
    }

    let escape_interval = if escape_rate > 0.0 {
        Duration::from_secs_f64(1.0 / escape_rate)
    } else {
        Duration::from_secs(3600) // Effectively never
    };

    let mut escape_timer = tokio::time::interval(escape_interval);
    escape_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let test_end = Instant::now() + duration;
    let mut rng_state: u64 = client_id as u64 * 12345 + 67890;

    loop {
        if Instant::now() >= test_end {
            break;
        }

        tokio::select! {
            _ = escape_timer.tick() => {
                // Simple LCG for random velocity
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let vx = ((rng_state >> 32) as f64 / u32::MAX as f64) * 4.0 - 2.0;
                let vy = -2.0 - ((rng_state >> 16) as f64 / u32::MAX as f64) * 2.0;

                let msg = BallEscapedMsg {
                    msg_type: "ball_escaped",
                    vx,
                    vy,
                };
                let json = serde_json::to_string(&msg).unwrap();
                if ws.send(Message::Text(json.into())).await.is_ok() {
                    metrics.ball_escapes_sent.fetch_add(1, Ordering::Relaxed);
                } else {
                    metrics.errors.fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }

            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if client_id < 3 {
                            eprintln!("Client {} got message: {}...", client_id, &text.chars().take(80).collect::<String>());
                        }
                        metrics.messages_received.fetch_add(1, Ordering::Relaxed);
                        if let Ok(server_msg) = serde_json::from_str::<ServerMsg>(&text) {
                            match server_msg {
                                ServerMsg::SpaceState { balls } => {
                                    metrics.space_states_received.fetch_add(1, Ordering::Relaxed);
                                    metrics.total_balls_seen.fetch_add(balls.len() as u64, Ordering::Relaxed);
                                }
                                ServerMsg::TransferIn { .. } => {
                                    metrics.transfer_ins_received.fetch_add(1, Ordering::Relaxed);
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        if client_id < 3 {
                            eprintln!("Client {} got Close: {:?}", client_id, frame);
                        }
                        break;
                    }
                    None => {
                        if client_id < 3 {
                            eprintln!("Client {} stream ended", client_id);
                        }
                        break;
                    }
                    Some(Err(e)) => {
                        if client_id < 3 {
                            eprintln!("Client {} error: {}", client_id, e);
                        }
                        metrics.errors.fetch_add(1, Ordering::Relaxed);
                        break;
                    }
                    Some(other) => {
                        if client_id < 3 {
                            eprintln!("Client {} got other message: {:?}", client_id, other);
                        }
                    }
                }
            }
        }
    }

    let _ = ws.close(None).await;
    metrics.connected.fetch_sub(1, Ordering::Relaxed);
}

// === Main ===

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut num_clients: u32 = 100;
    let mut duration_secs: u64 = 30;
    let mut escape_rate: f64 = 0.5;
    let mut url = "ws://127.0.0.1:9001/ws".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--clients" => {
                i += 1;
                num_clients = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(100);
            }
            "--duration" => {
                i += 1;
                duration_secs = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(30);
            }
            "--escape-rate" => {
                i += 1;
                escape_rate = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.5);
            }
            "--url" => {
                i += 1;
                url = args.get(i).cloned().unwrap_or(url);
            }
            _ => {}
        }
        i += 1;
    }

    println!("=== Pinball Server Load Test ===");
    println!("Clients: {}", num_clients);
    println!("Duration: {}s", duration_secs);
    println!("Escape rate: {}/s per client", escape_rate);
    println!("URL: {}", url);
    println!();

    let metrics = Arc::new(Metrics::new());
    let barrier = Arc::new(Barrier::new(num_clients as usize));
    let duration = Duration::from_secs(duration_secs);

    // Spawn all clients
    let mut handles = Vec::with_capacity(num_clients as usize);

    println!("Spawning {} clients...", num_clients);
    let spawn_start = Instant::now();

    for client_id in 0..num_clients {
        let url = url.clone();
        let metrics = Arc::clone(&metrics);
        let barrier = Arc::clone(&barrier);

        handles.push(tokio::spawn(async move {
            run_client(client_id, url, escape_rate, duration, metrics, barrier).await;
        }));

        // Stagger spawns slightly to avoid thundering herd
        if client_id % 50 == 49 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    println!("All clients spawned in {:?}", spawn_start.elapsed());
    println!();

    // Print stats periodically
    let metrics_clone = Arc::clone(&metrics);
    let stats_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let start = Instant::now();

        loop {
            interval.tick().await;
            let elapsed = start.elapsed().as_secs();
            if elapsed >= duration_secs + 5 {
                break;
            }

            let connected = metrics_clone.connected.load(Ordering::Relaxed);
            let msgs = metrics_clone.messages_received.load(Ordering::Relaxed);
            let space_states = metrics_clone.space_states_received.load(Ordering::Relaxed);
            let transfer_ins = metrics_clone.transfer_ins_received.load(Ordering::Relaxed);
            let escapes = metrics_clone.ball_escapes_sent.load(Ordering::Relaxed);
            let errors = metrics_clone.errors.load(Ordering::Relaxed);
            let balls = metrics_clone.total_balls_seen.load(Ordering::Relaxed);
            let avg_balls = if space_states > 0 {
                balls / space_states
            } else {
                0
            };

            println!(
                "[{:3}s] connected={}, msgs={}, space_states={}, transfer_ins={}, escapes={}, errors={}, avg_balls={}",
                elapsed, connected, msgs, space_states, transfer_ins, escapes, errors, avg_balls
            );
        }
    });

    // Wait for all clients to finish
    for handle in handles {
        let _ = handle.await;
    }

    stats_handle.abort();

    // Final stats
    println!();
    println!("=== Final Results ===");
    let msgs = metrics.messages_received.load(Ordering::Relaxed);
    let space_states = metrics.space_states_received.load(Ordering::Relaxed);
    let transfer_ins = metrics.transfer_ins_received.load(Ordering::Relaxed);
    let escapes = metrics.ball_escapes_sent.load(Ordering::Relaxed);
    let errors = metrics.errors.load(Ordering::Relaxed);
    let balls = metrics.total_balls_seen.load(Ordering::Relaxed);
    let latency_sum = metrics.latency_sum_ms.load(Ordering::Relaxed);
    let latency_count = metrics.latency_count.load(Ordering::Relaxed);

    println!("Total messages received: {}", msgs);
    println!("Total space_state messages: {}", space_states);
    println!("Total transfer_in messages: {}", transfer_ins);
    println!("Total ball_escaped sent: {}", escapes);
    println!("Total errors: {}", errors);
    println!(
        "Average balls in deep-space: {}",
        if space_states > 0 {
            balls / space_states
        } else {
            0
        }
    );

    if latency_count > 0 {
        println!("Average connect latency: {}ms", latency_sum / latency_count);
    }

    let msgs_per_sec = msgs as f64 / duration_secs as f64;
    let space_states_per_client = space_states as f64 / num_clients as f64;

    println!();
    println!("Messages/sec (total): {:.0}", msgs_per_sec);
    println!("Space states per client: {:.1}", space_states_per_client);
    println!(
        "Expected space states per client: {:.1}",
        duration_secs as f64 * 10.0
    ); // 10 Hz broadcast

    let delivery_rate = space_states_per_client / (duration_secs as f64 * 10.0) * 100.0;
    println!("Delivery rate: {:.1}%", delivery_rate);
}
