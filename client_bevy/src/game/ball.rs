use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::board::geometry::{ball_spawn, in_escape_slot, launcher_stop};
use crate::constants::{
    color_from_hex, px_to_world, BALL_RADIUS, BALL_RESTITUTION, PPM, RESPAWN_DELAY,
};
use crate::shared::connection::ServerConnection;

use super::network::NetworkState;
use super::pins::{Bumper, PinHitTimer};
use super::walls::Drain;
use super::{FixedSet, UpdateSet};

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
    pub(crate) color: u32,
}

#[derive(Resource, Default)]
pub(crate) struct RespawnState {
    pub(crate) seconds_left: f32,
}

#[derive(Component)]
pub(crate) struct Ball;

#[derive(Component)]
pub(crate) struct BallColor(pub(crate) u32);

#[derive(Component)]
pub(crate) struct BallVisual {
    pub(crate) ball: Entity,
}

#[derive(Component)]
pub(crate) struct BallState {
    pub(crate) in_launcher: bool,
}

#[derive(Component)]
struct BallSmoothing {
    prev_pos_m: Vec2,
    curr_pos_m: Vec2,
}

impl Plugin for BallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (emit_initial_ball, spawn_ball_system).chain())
            .add_systems(
                FixedUpdate,
                (update_launcher_snap_system, escape_system, respawn_system)
                    .chain()
                    .in_set(FixedSet::Simulate),
            )
            .add_systems(
                FixedUpdate,
                (cache_ball_smoothing_system, collision_system)
                    .chain()
                    .in_set(FixedSet::PostPhysics),
            )
            .add_systems(FixedUpdate, spawn_ball_system.in_set(FixedSet::Spawn))
            .add_systems(
                Update,
                (spawn_ball_visuals, update_ball_visuals).in_set(UpdateSet::Visuals),
            );
    }
}

fn emit_initial_ball(mut ball_writer: MessageWriter<SpawnBallMessage>) {
    let p = ball_spawn();
    ball_writer.write(SpawnBallMessage {
        px: p.x,
        py: p.y,
        vx: 0.0,
        vy: 0.0,
        in_launcher: true,
        color: crate::constants::Colors::BALL,
    });
}

fn spawn_ball(
    commands: &mut Commands,
    px: f32,
    py: f32,
    vx: f32,
    vy: f32,
    in_launcher: bool,
    color: u32,
) {
    commands
        .spawn((
            RigidBody::Dynamic,
            Collider::ball(BALL_RADIUS / PPM),
            Restitution::coefficient(BALL_RESTITUTION),
            Friction::coefficient(0.3),
            Damping {
                linear_damping: 0.0,
                angular_damping: 0.0,
            },
            ActiveEvents::COLLISION_EVENTS,
            Ccd::enabled(),
            Transform::from_xyz(px / PPM, py / PPM, 0.0),
            GlobalTransform::default(),
            Velocity::linear(Vec2::new(vx, vy)),
        ))
        .insert((
            ReadMassProperties::default(),
            ExternalImpulse::default(),
            Ball,
            BallColor(color),
            BallState { in_launcher },
            BallSmoothing {
                prev_pos_m: Vec2::new(px / PPM, py / PPM),
                curr_pos_m: Vec2::new(px / PPM, py / PPM),
            },
        ));
}

fn spawn_ball_system(mut commands: Commands, mut ball_reader: MessageReader<SpawnBallMessage>) {
    for msg in ball_reader.read() {
        spawn_ball(
            &mut commands,
            msg.px,
            msg.py,
            msg.vx,
            msg.vy,
            msg.in_launcher,
            msg.color,
        );
    }
}

fn escape_system(
    mut commands: Commands,
    conn: Res<ServerConnection>,
    q_ball: Query<(Entity, &Transform, &Velocity), With<Ball>>,
) {
    for (entity, transform, vel) in &q_ball {
        let px = transform.translation.x * PPM;
        let py = transform.translation.y * PPM;

        if in_escape_slot(px, py) && vel.linvel.y < 0.0 {
            conn.send_ball_escaped(vel.linvel.x, vel.linvel.y);
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

        let px = transform.translation.x * PPM;
        let py = transform.translation.y * PPM;
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
            let a_ball = q_ball.get(*a).is_ok();
            let b_ball = q_ball.get(*b).is_ok();
            let a_drain = q_drain.get(*a).is_ok();
            let b_drain = q_drain.get(*b).is_ok();
            let a_bumper = q_bumper.get(*a).is_ok();
            let b_bumper = q_bumper.get(*b).is_ok();

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

fn cache_ball_smoothing_system(mut q_balls: Query<(&Transform, &mut BallSmoothing), With<Ball>>) {
    for (tf, mut smooth) in &mut q_balls {
        smooth.prev_pos_m = smooth.curr_pos_m;
        smooth.curr_pos_m = tf.translation.truncate();
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
                color: net.self_color,
            });
            respawn.seconds_left = 0.0;
        }
    }
}

fn spawn_ball_visuals(
    mut commands: Commands,
    q_new_balls: Query<(Entity, &BallColor), Added<Ball>>,
) {
    for (ball, ball_color) in &q_new_balls {
        let circle = shapes::Circle {
            radius: BALL_RADIUS,
            center: Vec2::ZERO,
        };
        commands.spawn((
            ShapeBuilder::with(&circle)
                .stroke((color_from_hex(ball_color.0), 2.0))
                .build(),
            Transform::from_xyz(0.0, 0.0, 4.0),
            BallVisual { ball },
        ));
    }
}

fn update_ball_visuals(
    mut commands: Commands,
    q_balls: Query<(&BallSmoothing, &BallColor), (With<Ball>, Without<BallVisual>)>,
    mut q_visuals: Query<
        (Entity, &BallVisual, &mut Transform, &mut Shape),
        (With<BallVisual>, Without<Ball>),
    >,
    time_fixed: Res<Time<Fixed>>,
) {
    let alpha = time_fixed.overstep_fraction().clamp(0.0, 1.0);

    for (visual_entity, visual, mut visual_tf, mut shape) in &mut q_visuals {
        if let Ok((smooth, color)) = q_balls.get(visual.ball) {
            let p_m = smooth.prev_pos_m.lerp(smooth.curr_pos_m, alpha);
            let px = p_m.x * PPM;
            let py = p_m.y * PPM;
            visual_tf.translation = px_to_world(px, py, 4.0);
            if let Some(stroke) = shape.stroke.as_mut() {
                stroke.color = color_from_hex(color.0);
            }
        } else {
            commands.entity(visual_entity).despawn();
        }
    }
}
