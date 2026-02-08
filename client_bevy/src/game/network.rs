use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::constants::Colors;
use crate::shared::connection::{NetEvent, ServerConnection};
use crate::shared::protocol::ServerMsg;

use super::ball::SpawnBallMessage;
use super::input::InputState;
use super::{FixedSet, UpdateSet};

const CAPTURE_SPAWN_X: f32 = 200.0;
const CAPTURE_SPAWN_Y: f32 = 80.0;
const ACTIVITY_SEND_INTERVAL: Duration = Duration::from_secs(5);

pub struct NetworkPlugin;

#[derive(Resource)]
pub(crate) struct NetworkState {
    pub(crate) self_color: u32,
    pub(crate) protocol_mismatch: bool,
    pub(crate) connection_label: String,
    pub(crate) last_activity_sent: Instant,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            self_color: Colors::BALL,
            protocol_mismatch: false,
            connection_label: "connecting".to_string(),
            last_activity_sent: Instant::now(),
        }
    }
}

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, network_event_system.in_set(UpdateSet::Network))
            .add_systems(
                FixedUpdate,
                activity_heartbeat_system.in_set(FixedSet::Simulate),
            );
    }
}

fn network_event_system(
    mut conn: ResMut<ServerConnection>,
    mut net: ResMut<NetworkState>,
    mut ball_writer: MessageWriter<SpawnBallMessage>,
) {
    for evt in conn.poll_events() {
        match evt {
            NetEvent::Connected => {
                net.connection_label = "connected".to_string();
                conn.state = crate::shared::types::ConnectionState::Connected;
            }
            NetEvent::Disconnected => {
                net.connection_label = "disconnected".to_string();
                conn.state = crate::shared::types::ConnectionState::Disconnected;
            }
            NetEvent::ProtocolMismatch { server, client } => {
                net.protocol_mismatch = true;
                conn.protocol_mismatch = true;
                net.connection_label = format!("protocol mismatch {server}!={client}");
            }
            NetEvent::Message(msg) => match msg {
                ServerMsg::Welcome {
                    self_id,
                    server_version,
                    players,
                    config,
                    ..
                } => {
                    let _ = config;
                    conn.self_id = self_id;
                    conn.server_version = server_version;
                    conn.players = players.iter().map(|p| p.to_player()).collect();
                    if let Some(me) = conn.players.iter().find(|p| p.id == conn.self_id) {
                        net.self_color = me.color;
                    }
                }
                ServerMsg::PlayersState { players } => {
                    conn.players = players.iter().map(|p| p.to_player()).collect();
                    if let Some(me) = conn.players.iter().find(|p| p.id == conn.self_id) {
                        net.self_color = me.color;
                    }
                }
                ServerMsg::SpaceState { balls } => {
                    conn.snapshot_balls = balls.iter().map(|b| b.to_ball()).collect();
                    conn.interpolated_balls = conn.snapshot_balls.clone();
                    conn.last_snapshot = Instant::now();
                }
                ServerMsg::TransferIn {
                    vx,
                    vy,
                    owner_id,
                    color,
                } => {
                    let _ = owner_id;
                    ball_writer.write(SpawnBallMessage {
                        px: CAPTURE_SPAWN_X,
                        py: CAPTURE_SPAWN_Y,
                        vx,
                        vy,
                        in_launcher: false,
                        color,
                    });
                }
            },
        }
    }

    conn.update_interpolation();
}

fn activity_heartbeat_system(
    input: Res<InputState>,
    mut net: ResMut<NetworkState>,
    conn: Res<ServerConnection>,
) {
    if let Some(last) = input.last_activity {
        if last.elapsed() < Duration::from_secs(30)
            && net.last_activity_sent.elapsed() >= ACTIVITY_SEND_INTERVAL
        {
            conn.send_activity();
            net.last_activity_sent = Instant::now();
        }
    }
}
