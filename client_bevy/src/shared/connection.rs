#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
#[cfg(target_arch = "wasm32")]
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

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
#[cfg(target_arch = "wasm32")]
type WasmCmdSender = Sender<ClientMsg>;

#[derive(Resource)]
pub struct ServerConnection {
    pub state: ConnectionState,
    pub self_id: u32,
    pub server_version: String,
    pub protocol_mismatch: bool,

    pub players: Vec<super::types::Player>,
    pub snapshot_balls: Vec<SpaceBall3D>,
    pub interpolated_balls: Vec<SpaceBall3D>,
    pub last_snapshot_time: f64,

    event_rx: Mutex<Receiver<NetEvent>>,

    #[cfg(not(target_arch = "wasm32"))]
    cmd_tx: Option<NativeCmdSender>,
    #[cfg(target_arch = "wasm32")]
    cmd_tx: Option<WasmCmdSender>,
}

impl ServerConnection {
    pub fn new(url: String) -> Self {
        let (event_tx, event_rx) = mpsc::channel::<NetEvent>();

        #[cfg(not(target_arch = "wasm32"))]
        let cmd_tx = Some(spawn_native_network_thread(url.clone(), event_tx));

        #[cfg(target_arch = "wasm32")]
        let cmd_tx = Some(spawn_wasm_network_runtime(url.clone(), event_tx));

        Self {
            state: ConnectionState::Connecting,
            self_id: 0,
            server_version: String::new(),
            protocol_mismatch: false,
            players: Vec::new(),
            snapshot_balls: Vec::new(),
            interpolated_balls: Vec::new(),
            last_snapshot_time: 0.0,
            event_rx: Mutex::new(event_rx),
            #[cfg(not(target_arch = "wasm32"))]
            cmd_tx,
            #[cfg(target_arch = "wasm32")]
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
            if let Some(tx) = &self.cmd_tx {
                let _ = tx.send(msg);
            }
        }
    }

    pub fn update_interpolation(&mut self, now: f64) {
        let elapsed = now - self.last_snapshot_time;
        if self.interpolated_balls.len() != self.snapshot_balls.len() {
            self.interpolated_balls = self.snapshot_balls.clone();
        }

        for (dst, base) in self
            .interpolated_balls
            .iter_mut()
            .zip(self.snapshot_balls.iter())
        {
            dst.id = base.id;
            dst.owner_id = base.owner_id;
            dst.pos = base.pos;
            dst.axis = base.axis;
            dst.omega = base.omega;
            rotate_normalize_in_place(&mut dst.pos, dst.axis, dst.omega * elapsed);
        }
    }
}

#[cfg(target_arch = "wasm32")]
const RECONNECT_MIN_DELAY_MS: u32 = 1_000;
#[cfg(target_arch = "wasm32")]
const RECONNECT_MAX_DELAY_MS: u32 = 30_000;

#[cfg(target_arch = "wasm32")]
fn next_reconnect_delay_ms(current_ms: u32) -> u32 {
    ((current_ms as f32 * 1.5) as u32).min(RECONNECT_MAX_DELAY_MS)
}

#[cfg(target_arch = "wasm32")]
fn spawn_wasm_network_runtime(url: String, event_tx: Sender<NetEvent>) -> WasmCmdSender {
    let (cmd_tx, cmd_rx) = mpsc::channel::<ClientMsg>();
    let cmd_rx = Arc::new(Mutex::new(cmd_rx));

    connect_wasm_socket(url, event_tx, cmd_rx, RECONNECT_MIN_DELAY_MS);
    cmd_tx
}

#[cfg(target_arch = "wasm32")]
fn connect_wasm_socket(
    url: String,
    event_tx: Sender<NetEvent>,
    cmd_rx: Arc<Mutex<Receiver<ClientMsg>>>,
    reconnect_delay_ms: u32,
) {
    use gloo_timers::callback::{Interval, Timeout};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{Event, MessageEvent, WebSocket};

    let _ = event_tx.send(NetEvent::Disconnected);
    let send_pump: Rc<RefCell<Option<Interval>>> = Rc::new(RefCell::new(None));

    let ws = match WebSocket::new(&url) {
        Ok(ws) => ws,
        Err(_) => {
            let next_delay_ms = next_reconnect_delay_ms(reconnect_delay_ms);
            let url_retry = url;
            let event_tx_retry = event_tx;
            let cmd_rx_retry = cmd_rx;
            Timeout::new(reconnect_delay_ms, move || {
                connect_wasm_socket(url_retry, event_tx_retry, cmd_rx_retry, next_delay_ms);
            })
            .forget();
            return;
        }
    };

    let ws_on_open = ws.clone();
    let cmd_rx_on_open = cmd_rx.clone();
    let event_tx_on_open = event_tx.clone();
    let send_pump_on_open = send_pump.clone();
    let onopen = Closure::<dyn FnMut(Event)>::new(move |_| {
        let _ = event_tx_on_open.send(NetEvent::Connected);
        *send_pump_on_open.borrow_mut() = Some(Interval::new(16, {
            let ws_send = ws_on_open.clone();
            let cmd_rx_send = cmd_rx_on_open.clone();
            move || {
                if ws_send.ready_state() != WebSocket::OPEN {
                    return;
                }
                if let Ok(rx) = cmd_rx_send.lock() {
                    while let Ok(cmd) = rx.try_recv() {
                        if let Ok(text) = serde_json::to_string(&cmd) {
                            if ws_send.send_with_str(&text).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }));
    });
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let ws_on_message = ws.clone();
    let event_tx_on_message = event_tx.clone();
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |evt| {
        let Some(txt) = evt.data().as_string() else {
            return;
        };
        let Ok(server_msg) = serde_json::from_str::<ServerMsg>(&txt) else {
            return;
        };

        if let ServerMsg::Welcome {
            protocol_version, ..
        } = &server_msg
        {
            if *protocol_version != CLIENT_PROTOCOL_VERSION {
                let _ = event_tx_on_message.send(NetEvent::ProtocolMismatch {
                    server: *protocol_version,
                    client: CLIENT_PROTOCOL_VERSION,
                });
                let _ = ws_on_message.close();
                return;
            }
        }

        let _ = event_tx_on_message.send(NetEvent::Message(server_msg));
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let event_tx_on_error = event_tx.clone();
    let onerror = Closure::<dyn FnMut(Event)>::new(move |_| {
        let _ = event_tx_on_error.send(NetEvent::Disconnected);
    });
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    let url_on_close = url;
    let event_tx_on_close = event_tx;
    let cmd_rx_on_close = cmd_rx;
    let send_pump_on_close = send_pump;
    let onclose = Closure::<dyn FnMut(Event)>::new(move |_| {
        *send_pump_on_close.borrow_mut() = None;
        let _ = event_tx_on_close.send(NetEvent::Disconnected);
        let next_delay_ms = next_reconnect_delay_ms(reconnect_delay_ms);
        let url_retry = url_on_close.clone();
        let event_tx_retry = event_tx_on_close.clone();
        let cmd_rx_retry = cmd_rx_on_close.clone();
        Timeout::new(reconnect_delay_ms, move || {
            connect_wasm_socket(url_retry, event_tx_retry, cmd_rx_retry, next_delay_ms);
        })
        .forget();
    });
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();
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
