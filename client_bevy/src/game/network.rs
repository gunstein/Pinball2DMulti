use bevy::prelude::*;
use bevy_prototype_lyon::prelude::Shape;

use crate::constants::{color_from_hex, wire_vel_to_bevy, Colors, BALL_FILL_ALPHA};
use crate::shared::connection::{NetEvent, ServerConnection};
use crate::shared::protocol::ServerMsg;

use super::ball::{Ball, BallState, SpawnBallMessage};
use super::input::InputState;
use super::{FixedSet, UpdateSet};

const CAPTURE_SPAWN_X: f32 = 200.0;
const CAPTURE_SPAWN_Y: f32 = 80.0;
const ACTIVITY_SEND_INTERVAL: f64 = 5.0;
const ACTIVITY_TIMEOUT: f64 = 30.0;

pub struct NetworkPlugin;

#[derive(Resource)]
pub(crate) struct NetworkState {
    pub(crate) self_color: u32,
    pub(crate) protocol_mismatch: bool,
    pub(crate) connection_label: String,
    pub(crate) last_activity_sent_time: f64,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            self_color: Colors::BALL,
            protocol_mismatch: false,
            connection_label: "connecting".to_string(),
            last_activity_sent_time: 0.0,
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
    mut q_balls: Query<(&BallState, &mut Shape), With<Ball>>,
    time: Res<Time>,
) {
    for evt in conn.poll_events() {
        match evt {
            NetEvent::Connected => {
                info!("WebSocket connected");
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
                    info!("Welcome: self_id={self_id}, {} players", players.len());
                    conn.self_id = self_id;
                    conn.server_version = server_version;
                    conn.players = players.iter().map(|p| p.to_player()).collect();
                    if let Some(me) = conn.players.iter().find(|p| p.id == conn.self_id) {
                        update_self_color(me.color, &mut net, &mut q_balls);
                    }
                }
                ServerMsg::PlayersState { players } => {
                    conn.players = players.iter().map(|p| p.to_player()).collect();
                    if let Some(me) = conn.players.iter().find(|p| p.id == conn.self_id) {
                        update_self_color(me.color, &mut net, &mut q_balls);
                    }
                }
                ServerMsg::SpaceState { balls } => {
                    conn.snapshot_balls = balls.iter().map(|b| b.to_ball()).collect();
                    conn.interpolated_balls = conn.snapshot_balls.clone();
                    conn.last_snapshot_time = time.elapsed_secs_f64();
                }
                ServerMsg::TransferIn {
                    vx,
                    vy,
                    owner_id,
                    color,
                } => {
                    let _ = owner_id;
                    let bevy_vel = wire_vel_to_bevy(vx, vy);
                    ball_writer.write(SpawnBallMessage {
                        px: CAPTURE_SPAWN_X,
                        py: CAPTURE_SPAWN_Y,
                        vx: bevy_vel.x,
                        vy: bevy_vel.y,
                        in_launcher: false,
                        self_owned: false,
                        color,
                    });
                }
            },
        }
    }

    conn.update_interpolation(time.elapsed_secs_f64());
}

fn update_self_color(
    self_color: u32,
    net: &mut NetworkState,
    q_balls: &mut Query<(&BallState, &mut Shape), With<Ball>>,
) {
    net.self_color = self_color;
    let color = color_from_hex(self_color);
    for (state, mut shape) in q_balls.iter_mut() {
        if !state.self_owned {
            continue;
        }

        if let Some(fill) = shape.fill.as_mut() {
            fill.color = color.with_alpha(BALL_FILL_ALPHA);
        }
        if let Some(stroke) = shape.stroke.as_mut() {
            stroke.color = color;
        }
    }
}

fn activity_heartbeat_system(
    input: Res<InputState>,
    mut net: ResMut<NetworkState>,
    conn: Res<ServerConnection>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    let since_activity = now - input.last_activity_time;
    let since_sent = now - net.last_activity_sent_time;

    if input.last_activity_time > 0.0
        && since_activity < ACTIVITY_TIMEOUT
        && since_sent >= ACTIVITY_SEND_INTERVAL
    {
        conn.send_activity();
        net.last_activity_sent_time = now;
    }
}
