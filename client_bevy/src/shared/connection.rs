use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use bevy::prelude::Resource;

use super::protocol::{ClientMsg, ServerMsg};
use super::types::{ConnectionState, SpaceBall3D, CLIENT_PROTOCOL_VERSION};
use super::vec3::rotate_normalize_in_place;

#[derive(Debug, Clone)]
pub enum NetEvent {
    Connected,
    Disconnected,
    Message(ServerMsg),
    ProtocolMismatch { server: u32, client: u32 },
}

#[cfg(not(target_arch = "wasm32"))]
type NativeCmdSender = tokio::sync::mpsc::UnboundedSender<ClientMsg>;

#[derive(Resource)]
pub struct ServerConnection {
    pub state: ConnectionState,
    pub self_id: u32,
    pub server_version: String,
    pub protocol_mismatch: bool,

    pub players: Vec<super::types::Player>,
    pub snapshot_balls: Vec<SpaceBall3D>,
    pub interpolated_balls: Vec<SpaceBall3D>,
    pub last_snapshot: Instant,

    event_rx: Mutex<Receiver<NetEvent>>,

    #[cfg(not(target_arch = "wasm32"))]
    cmd_tx: Option<NativeCmdSender>,
}

impl ServerConnection {
    pub fn new(url: String) -> Self {
        let (event_tx, event_rx) = mpsc::channel::<NetEvent>();

        #[cfg(not(target_arch = "wasm32"))]
        let cmd_tx = Some(spawn_native_network_thread(url.clone(), event_tx));

        #[cfg(target_arch = "wasm32")]
        let _ = event_tx;

        Self {
            state: ConnectionState::Connecting,
            self_id: 0,
            server_version: String::new(),
            protocol_mismatch: false,
            players: Vec::new(),
            snapshot_balls: Vec::new(),
            interpolated_balls: Vec::new(),
            last_snapshot: Instant::now(),
            event_rx: Mutex::new(event_rx),
            #[cfg(not(target_arch = "wasm32"))]
            cmd_tx,
        }
    }

    pub fn poll_events(&mut self) -> Vec<NetEvent> {
        let mut out = Vec::new();
        if let Ok(rx) = self.event_rx.lock() {
            while let Ok(evt) = rx.try_recv() {
                out.push(evt);
            }
        }
        out
    }

    pub fn send_ball_escaped(&self, vx: f32, vy: f32) {
        self.send(ClientMsg::BallEscaped { vx, vy });
    }

    pub fn send_set_paused(&self, paused: bool) {
        self.send(ClientMsg::SetPaused { paused });
    }

    pub fn send_activity(&self) {
        self.send(ClientMsg::Activity);
    }

    fn send(&self, msg: ClientMsg) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(tx) = &self.cmd_tx {
                let _ = tx.send(msg);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = msg;
        }
    }

    pub fn update_interpolation(&mut self) {
        let elapsed = self.last_snapshot.elapsed().as_secs_f64();
        let mut i = 0usize;
        while i < self.interpolated_balls.len() {
            let base = &self.snapshot_balls[i];
            let mut b = base.clone();
            rotate_normalize_in_place(&mut b.pos, b.axis, b.omega * elapsed);
            self.interpolated_balls[i] = b;
            i += 1;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_native_network_thread(url: String, event_tx: Sender<NetEvent>) -> NativeCmdSender {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<ClientMsg>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("failed to build tokio runtime");

        rt.block_on(async move {
            let mut reconnect_delay = Duration::from_millis(1000);
            let max_delay = Duration::from_millis(30_000);

            loop {
                let _ = event_tx.send(NetEvent::Disconnected);

                let connect = tokio_tungstenite::connect_async(url.as_str()).await;

                let (ws_stream, _) = match connect {
                    Ok(x) => x,
                    Err(_) => {
                        tokio::time::sleep(reconnect_delay).await;
                        reconnect_delay = (reconnect_delay.mul_f32(1.5)).min(max_delay);
                        continue;
                    }
                };

                reconnect_delay = Duration::from_millis(1000);
                let _ = event_tx.send(NetEvent::Connected);

                let (mut write, mut read) = ws_stream.split();

                loop {
                    tokio::select! {
                        biased;

                        Some(cmd) = cmd_rx.recv() => {
                            if let Ok(text) = serde_json::to_string(&cmd) {
                                if write.send(Message::Text(text.into())).await.is_err() {
                                    break;
                                }
                            }
                        }

                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(txt))) => {
                                    if let Ok(server_msg) = serde_json::from_str::<ServerMsg>(&txt) {
                                        if let ServerMsg::Welcome { protocol_version, .. } = &server_msg {
                                            if *protocol_version != CLIENT_PROTOCOL_VERSION {
                                                let _ = event_tx.send(NetEvent::ProtocolMismatch {
                                                    server: *protocol_version,
                                                    client: CLIENT_PROTOCOL_VERSION,
                                                });
                                                let _ = write.close().await;
                                                break;
                                            }
                                        }
                                        let _ = event_tx.send(NetEvent::Message(server_msg));
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    break;
                                }
                                Some(Ok(_)) => {}
                                Some(Err(_)) => {
                                    break;
                                }
                                None => {
                                    break;
                                }
                            }
                        }
                    }
                }

                let _ = event_tx.send(NetEvent::Disconnected);
                tokio::time::sleep(reconnect_delay).await;
                reconnect_delay = (reconnect_delay.mul_f32(1.5)).min(max_delay);
            }
        });
    });

    cmd_tx
}
