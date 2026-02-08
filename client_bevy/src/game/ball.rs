use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::board::geometry::{ball_spawn, in_escape_slot, launcher_stop};
use crate::constants::{
    bevy_vel_to_wire, color_from_hex, px_to_world, world_to_px_x, world_to_px_y, BALL_FILL_ALPHA,
    BALL_RADIUS, BALL_RESTITUTION, RESPAWN_DELAY,
};
use crate::shared::connection::ServerConnection;

use super::network::NetworkState;
use super::pins::{Bumper, PinHitTimer};
use super::walls::Drain;
use super::FixedSet;

pub struct BallPlugin;

const LAUNCHER_SNAP_Y_TOLERANCE: f32 = 30.0;
const LAUNCHER_SNAP_SPEED: f32 = 0.5;

#[derive(Message, Clone, Copy)]
pub(crate) struct SpawnBallMessage {
    pub(crate) px: f32,
    pub(crate) py: f32,
    pub(crate) vx: f32,
    pub(crate) vy: f32,
    pub(crate) in_launcher: bool,
    pub(crate) self_owned: bool,
    pub(crate) color: u32,
}

#[derive(Resource)]
pub(crate) struct RespawnState {
    pub(crate) seconds_left: f32,
}

impl Default for RespawnState {
    fn default() -> Self {
        Self {
            seconds_left: RESPAWN_DELAY,
        }
    }
}

#[derive(Component)]
pub(crate) struct Ball;

#[derive(Component)]
pub(crate) struct BallState {
    pub(crate) in_launcher: bool,
    pub(crate) self_owned: bool,
}

impl Plugin for BallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_initial_ball)
            .add_systems(
                FixedUpdate,
                (update_launcher_snap_system, escape_system, respawn_system)
                    .chain()
                    .in_set(FixedSet::Simulate),
            )
            .add_systems(FixedUpdate, collision_system.in_set(FixedSet::PostPhysics))
            .add_systems(FixedUpdate, spawn_ball_system.in_set(FixedSet::Spawn));
    }
}

fn spawn_initial_ball(mut commands: Commands) {
    let p = ball_spawn();
    do_spawn_ball(
        &mut commands,
        SpawnBallMessage {
            px: p.x,
            py: p.y,
            vx: 0.0,
            vy: 0.0,
            in_launcher: true,
            self_owned: true,
            color: crate::constants::Colors::BALL,
        },
    );
}

fn spawn_ball_system(mut commands: Commands, mut ball_reader: MessageReader<SpawnBallMessage>) {
    for msg in ball_reader.read() {
        do_spawn_ball(&mut commands, *msg);
    }
}

fn do_spawn_ball(commands: &mut Commands, msg: SpawnBallMessage) {
    let world = px_to_world(msg.px, msg.py, 4.0);

    commands.spawn((
        // Physics
        RigidBody::Dynamic,
        Collider::ball(BALL_RADIUS),
        Restitution::coefficient(BALL_RESTITUTION),
        Friction::coefficient(0.3),
        Damping {
            linear_damping: 0.0,
            angular_damping: 0.0,
        },
        ActiveEvents::COLLISION_EVENTS,
        Ccd::enabled(),
        Velocity::linear(Vec2::new(msg.vx, msg.vy)),
        ReadMassProperties::default(),
        ExternalImpulse::default(),
        // Transform (shared by physics + visual)
        Transform::from_translation(world),
        // Visual
        ShapeBuilder::with(&shapes::Circle {
            radius: BALL_RADIUS,
            center: Vec2::ZERO,
        })
        .fill(color_from_hex(msg.color).with_alpha(BALL_FILL_ALPHA))
        .stroke((color_from_hex(msg.color), 2.0))
        .build(),
        // Game state
        Ball,
        BallState {
            in_launcher: msg.in_launcher,
            self_owned: msg.self_owned,
        },
    ));
}

fn escape_system(
    mut commands: Commands,
    conn: Res<ServerConnection>,
    q_ball: Query<(Entity, &Transform, &Velocity), With<Ball>>,
) {
    for (entity, transform, vel) in &q_ball {
        let px = world_to_px_x(transform.translation.x);
        let py = world_to_px_y(transform.translation.y);

        if in_escape_slot(px, py) && vel.linvel.y > 0.0 {
            // Protocol uses TS/Rapier coords (Y-down, meters). Convert from Bevy (Y-up, pixels).
            let (vx, vy) = bevy_vel_to_wire(vel.linvel);
            conn.send_ball_escaped(vx, vy);
            commands.entity(entity).despawn();
        }
    }
}

fn update_launcher_snap_system(
    mut q_ball: Query<(&Transform, &Velocity, &mut BallState), With<Ball>>,
) {
    let stop = launcher_stop();

    for (transform, vel, mut state) in &mut q_ball {
        if state.in_launcher {
            continue;
        }

        let px = world_to_px_x(transform.translation.x);
        let py = world_to_px_y(transform.translation.y);
        let in_lane_x = px >= stop.from.x && px <= stop.to.x;
        let near_stop = py >= stop.from.y - LAUNCHER_SNAP_Y_TOLERANCE && py <= stop.from.y;
        let speed = vel.linvel.length();

        if in_lane_x && near_stop && speed < LAUNCHER_SNAP_SPEED {
            state.in_launcher = true;
        }
    }
}

fn collision_system(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionEvent>,
    q_ball: Query<(), With<Ball>>,
    q_drain: Query<(), With<Drain>>,
    q_bumper: Query<(), With<Bumper>>,
    mut pin_timers: Query<&mut PinHitTimer>,
    mut respawn: ResMut<RespawnState>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(a, b, _) = event {
            let (a_ball, b_ball) = (q_ball.get(*a).is_ok(), q_ball.get(*b).is_ok());
            let (a_drain, b_drain) = (q_drain.get(*a).is_ok(), q_drain.get(*b).is_ok());
            let (a_bumper, b_bumper) = (q_bumper.get(*a).is_ok(), q_bumper.get(*b).is_ok());

            if (a_ball && b_drain) || (b_ball && a_drain) {
                let ball_entity = if a_ball { *a } else { *b };
                commands.entity(ball_entity).despawn();
                respawn.seconds_left = RESPAWN_DELAY;
            }

            if (a_ball && b_bumper) || (b_ball && a_bumper) {
                let pin = if a_bumper { *a } else { *b };
                if let Ok(mut timer) = pin_timers.get_mut(pin) {
                    timer.seconds_left = 1.0;
                }
            }
        }
    }
}

fn respawn_system(
    mut respawn: ResMut<RespawnState>,
    q_ball: Query<(), With<Ball>>,
    time: Res<Time<Fixed>>,
    net: Res<NetworkState>,
    mut ball_writer: MessageWriter<SpawnBallMessage>,
) {
    if q_ball.is_empty() {
        respawn.seconds_left -= time.delta_secs();
        if respawn.seconds_left <= 0.0 {
            let p = ball_spawn();
            ball_writer.write(SpawnBallMessage {
                px: p.x,
                py: p.y,
                vx: 0.0,
                vy: 0.0,
                in_launcher: true,
                self_owned: true,
                color: net.self_color,
            });
            respawn.seconds_left = 0.0;
        }
    }
}
