use bevy::prelude::*;
use bevy_prototype_lyon::prelude::Shape;

use crate::constants::{color_from_hex, Colors, BALL_FILL_ALPHA};
use crate::coord::{wire_vel_to_bevy, WireVel};
use crate::shared::connection::{now_mono_secs, NetEvent, NetTransport};
use crate::shared::net_state::NetState;
use crate::shared::protocol::ServerMsg;
use crate::shared::types::{wire_to_player, SpaceBall3D};

use super::ball::{Ball, BallState, SpawnBallMessage};
use super::hud::HudUiState;
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
    mut transport: ResMut<NetTransport>,
    mut state: ResMut<NetState>,
    mut net: ResMut<NetworkState>,
    mut ball_writer: MessageWriter<SpawnBallMessage>,
    mut q_balls: Query<(&BallState, &mut Shape), With<Ball>>,
) {
    let events = transport.poll_events();

    for evt in &events {
        match evt {
            NetEvent::Connected => {
                info!("WebSocket connected");
                net.connection_label = "connected".to_string();
                state.state = crate::shared::types::ConnectionState::Connected;
                net.protocol_mismatch = false;
                state.protocol_mismatch = false;
                state.reset_interpolation();
            }
            NetEvent::Disconnected => {
                net.connection_label = "disconnected".to_string();
                state.state = crate::shared::types::ConnectionState::Disconnected;
                state.reset_interpolation();
            }
            NetEvent::ProtocolMismatch { server, client } => {
                net.protocol_mismatch = true;
                state.protocol_mismatch = true;
                net.connection_label = format!("protocol mismatch {server}!={client}");
                state.reset_interpolation();
            }
            NetEvent::Message {
                msg,
                recv_time_secs,
            } => match msg {
                ServerMsg::Welcome(w) => {
                    net.protocol_mismatch = false;
                    state.protocol_mismatch = false;
                    info!(
                        "Welcome: self_id={}, {} players",
                        w.self_id,
                        w.players.len()
                    );
                    state.self_id = w.self_id;
                    state.server_version = w.server_version.clone();
                    state.players = w.players.iter().map(|p| wire_to_player(p)).collect();
                    if let Some(me) = state.players.iter().find(|p| p.id == state.self_id) {
                        update_self_color(me.color, &mut net, &mut q_balls);
                    }
                }
                ServerMsg::PlayersState(ps) => {
                    state.players = ps.players.iter().map(|p| wire_to_player(p)).collect();
                    if let Some(me) = state.players.iter().find(|p| p.id == state.self_id) {
                        update_self_color(me.color, &mut net, &mut q_balls);
                    }
                }
                ServerMsg::SpaceState(ss) => {
                    state.push_snapshot(
                        ss.server_time,
                        *recv_time_secs,
                        decode_space_balls(&ss.balls),
                    );
                }
                ServerMsg::TransferIn(t) => {
                    let bevy_vel = wire_vel_to_bevy(WireVel::new(t.vx as f32, t.vy as f32));
                    ball_writer.write(SpawnBallMessage {
                        px: CAPTURE_SPAWN_X,
                        py: CAPTURE_SPAWN_Y,
                        vx: bevy_vel.x,
                        vy: bevy_vel.y,
                        in_launcher: false,
                        self_owned: false,
                        color: t.color,
                    });
                }
            },
        }
    }

    transport.return_event_buf(events);

    state.update_interpolation(now_mono_secs());
}

fn decode_space_balls(wire_balls: &[pinball_shared::protocol::BallWire]) -> Vec<SpaceBall3D> {
    let mut balls = Vec::with_capacity(wire_balls.len());
    for wire in wire_balls {
        balls.push(SpaceBall3D {
            id: wire.id,
            owner_id: wire.owner_id,
            pos: crate::shared::vec3::Vec3::new(wire.pos[0], wire.pos[1], wire.pos[2]),
            axis: crate::shared::vec3::Vec3::new(wire.axis[0], wire.axis[1], wire.axis[2]),
            omega: wire.omega,
        });
    }
    balls
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
    transport: Res<NetTransport>,
    hud_ui: Res<HudUiState>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    let since_sent = now - net.last_activity_sent_time;

    if hud_ui.bot_enabled {
        if since_sent >= ACTIVITY_SEND_INTERVAL {
            transport.send_activity();
            net.last_activity_sent_time = now;
        }
        return;
    }

    let since_activity = now - input.last_activity_time;

    if input.last_activity_time > 0.0
        && since_activity < ACTIVITY_TIMEOUT
        && since_sent >= ACTIVITY_SEND_INTERVAL
    {
        transport.send_activity();
        net.last_activity_sent_time = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_prototype_lyon::prelude::*;

    use crate::constants::{color_from_hex, Colors, BALL_FILL_ALPHA};
    use crate::shared::connection::{now_mono_secs, NetEvent, NetTransport};
    use crate::shared::net_state::NetState;
    use pinball_shared::config::DeepSpaceConfig;
    use pinball_shared::protocol::{
        BallWire, PlayerWire, ServerMsg, SpaceStateMsg, WelcomeMsg, PROTOCOL_VERSION,
    };

    fn assert_color_close(a: Color, e: Color) {
        let a = a.to_srgba();
        let e = e.to_srgba();
        let eps = 0.02;
        assert!((a.red - e.red).abs() < eps, "red {} != {}", a.red, e.red);
        assert!(
            (a.green - e.green).abs() < eps,
            "green {} != {}",
            a.green,
            e.green
        );
        assert!(
            (a.blue - e.blue).abs() < eps,
            "blue {} != {}",
            a.blue,
            e.blue
        );
    }

    fn make_player_wire(id: u32, color: u32) -> PlayerWire {
        PlayerWire {
            id,
            cell_index: id,
            portal_pos: [1.0, 0.0, 0.0],
            color,
            paused: false,
            balls_produced: 0,
            balls_in_flight: 0,
        }
    }

    fn spawn_test_ball(app: &mut App, color: u32, self_owned: bool) -> Entity {
        let c = color_from_hex(color);
        app.world_mut()
            .spawn((
                Ball,
                BallState {
                    in_launcher: true,
                    self_owned,
                },
                ShapeBuilder::with(&shapes::Circle {
                    radius: 10.0,
                    center: Vec2::ZERO,
                })
                .fill(c.with_alpha(BALL_FILL_ALPHA))
                .stroke((c, 2.0))
                .build(),
            ))
            .id()
    }

    fn make_test_app_with_events() -> (App, std::sync::mpsc::Sender<NetEvent>) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NetworkState>();
        app.init_resource::<NetState>();
        app.init_resource::<InputState>();

        let (transport, event_tx) = NetTransport::test_stub_with_sender();
        app.insert_resource(transport);

        app.add_systems(Update, network_event_system);
        app.add_message::<SpawnBallMessage>();

        (app, event_tx)
    }

    #[test]
    fn welcome_recolors_self_owned_launcher_ball() {
        let (mut app, event_tx) = make_test_app_with_events();

        let ball_entity = spawn_test_ball(&mut app, Colors::BALL, true);

        let real_color: u32 = 0xFF8800;
        event_tx
            .send(NetEvent::Message {
                msg: ServerMsg::Welcome(WelcomeMsg {
                    protocol_version: PROTOCOL_VERSION,
                    server_version: "test".to_string(),
                    self_id: 42,
                    players: vec![make_player_wire(42, real_color)],
                    config: DeepSpaceConfig::default(),
                }),
                recv_time_secs: 0.0,
            })
            .unwrap();

        app.update();

        let shape = app.world().get::<Shape>(ball_entity).unwrap();
        let expected = color_from_hex(real_color);
        let stroke_color = shape.stroke.as_ref().unwrap().color;
        assert_color_close(stroke_color, expected);

        let net = app.world().resource::<NetworkState>();
        assert_eq!(net.self_color, real_color);
    }

    #[test]
    fn welcome_does_not_recolor_non_self_owned_ball() {
        let (mut app, event_tx) = make_test_app_with_events();

        let captured_color: u32 = 0x00FF00;
        let captured_entity = spawn_test_ball(&mut app, captured_color, false);

        event_tx
            .send(NetEvent::Message {
                msg: ServerMsg::Welcome(WelcomeMsg {
                    protocol_version: PROTOCOL_VERSION,
                    server_version: "test".to_string(),
                    self_id: 42,
                    players: vec![make_player_wire(42, 0xFF8800)],
                    config: DeepSpaceConfig::default(),
                }),
                recv_time_secs: 0.0,
            })
            .unwrap();

        app.update();

        let shape = app.world().get::<Shape>(captured_entity).unwrap();
        let expected = color_from_hex(captured_color);
        let stroke_color = shape.stroke.as_ref().unwrap().color;
        assert_color_close(stroke_color, expected);
    }

    #[test]
    fn space_state_extrapolates_from_event_timestamp_in_same_clock_domain() {
        let (mut app, event_tx) = make_test_app_with_events();
        let recv_time_secs = now_mono_secs() - 0.1;

        event_tx
            .send(NetEvent::Message {
                msg: ServerMsg::SpaceState(SpaceStateMsg {
                    server_time: recv_time_secs,
                    balls: vec![BallWire {
                        id: 1,
                        owner_id: 99,
                        pos: [1.0, 0.0, 0.0],
                        axis: [0.0, 0.0, 1.0],
                        omega: 2.0,
                    }],
                }),
                recv_time_secs,
            })
            .unwrap();

        app.update();

        let state = app.world().resource::<NetState>();
        assert_eq!(state.interpolated_balls.len(), 1);
        let p = state.interpolated_balls[0].pos;
        assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
        // With ~100ms extrapolation and omega=2 rad/s, y should be clearly positive.
        assert!(p.y > 0.05, "expected extrapolated y > 0.05, got {}", p.y);
    }
}
