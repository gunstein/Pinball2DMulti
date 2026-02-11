use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::board::flipper_logic::{rest_angle, step_flipper_angle};
use crate::board::geometry::{flippers, FlipperDef, FlipperSide};
use crate::constants::{color_from_hex, Colors};
use crate::coord::{px_to_world, PxPos};

use super::input::InputState;
use super::FixedSet;

pub struct FlippersPlugin;
const FLIPPER_FRICTION: f32 = 0.2;

#[derive(Component)]
pub(crate) struct Flipper {
    pub(crate) side: FlipperSide,
    pub(crate) angle: f32,
}

impl Plugin for FlippersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_flippers)
            .add_systems(FixedUpdate, flipper_system.in_set(FixedSet::Simulate));
    }
}

fn spawn_flippers(mut commands: Commands) {
    for def in flippers() {
        spawn_flipper(&mut commands, def);
    }
}

fn spawn_flipper(commands: &mut Commands, def: FlipperDef) {
    let world = px_to_world(PxPos::new(def.pivot.x, def.pivot.y), 3.0);
    let initial_angle = rest_angle(def.side);

    let outline = tapered_outline_points(def);
    let shape = shapes::Polygon {
        points: outline,
        closed: true,
    };

    commands.spawn((
        // Visual
        ShapeBuilder::with(&shape)
            .stroke((color_from_hex(Colors::FLIPPER), 2.0))
            .build(),
        // Physics
        RigidBody::KinematicPositionBased,
        build_flipper_collider(def),
        Friction {
            coefficient: FLIPPER_FRICTION,
            combine_rule: CoefficientCombineRule::Min,
        },
        Restitution::coefficient(0.5),
        // Position at pivot, rotated to rest angle
        // Negate angle: TS uses CW-positive, Bevy uses CCW-positive
        Transform::from_translation(world).with_rotation(Quat::from_rotation_z(-initial_angle)),
        Ccd::enabled(),
        // Game state
        Flipper {
            side: def.side,
            angle: initial_angle,
        },
    ));
}

fn build_flipper_collider(def: FlipperDef) -> Collider {
    let dir = match def.side {
        FlipperSide::Left => 1.0,
        FlipperSide::Right => -1.0,
    };

    let mut points: Vec<Vec2> = Vec::new();
    let segments = 12usize;

    for i in 0..segments {
        let angle = TAU * (i as f32) / (segments as f32);
        points.push(Vec2::new(
            def.pivot_radius * angle.cos(),
            def.pivot_radius * angle.sin(),
        ));
    }

    for i in 0..segments {
        let angle = TAU * (i as f32) / (segments as f32);
        points.push(Vec2::new(
            dir * def.length + def.tip_radius * angle.cos(),
            def.tip_radius * angle.sin(),
        ));
    }

    Collider::convex_hull(&points)
        .unwrap_or_else(|| Collider::cuboid(def.length * 0.5, def.width * 0.5))
}

fn tapered_outline_points(def: FlipperDef) -> Vec<Vec2> {
    let dir = match def.side {
        FlipperSide::Left => 1.0,
        FlipperSide::Right => -1.0,
    };

    let dr = def.pivot_radius - def.tip_radius;
    let tangent_angle = (dr / def.length).asin();

    let p_up = Vec2::new(
        def.pivot_radius * tangent_angle.sin(),
        -def.pivot_radius * tangent_angle.cos(),
    );
    let p_down = Vec2::new(
        def.pivot_radius * tangent_angle.sin(),
        def.pivot_radius * tangent_angle.cos(),
    );

    let tip_x = dir * def.length;
    let t_up = Vec2::new(
        tip_x + dir * def.tip_radius * tangent_angle.sin(),
        -def.tip_radius * tangent_angle.cos(),
    );
    let t_down = Vec2::new(
        tip_x + dir * def.tip_radius * tangent_angle.sin(),
        def.tip_radius * tangent_angle.cos(),
    );

    let pivot_start = p_up.y.atan2(p_up.x);
    let pivot_end = p_down.y.atan2(p_down.x);
    let tip_start = (t_down.y).atan2(t_down.x - tip_x);
    let tip_end = (t_up.y).atan2(t_up.x - tip_x);
    let anticlockwise = dir < 0.0;

    let mut out = Vec::new();
    out.push(p_up);
    out.push(t_up);
    sample_arc_points(
        &mut out,
        Vec2::new(tip_x, 0.0),
        def.tip_radius,
        tip_end,
        tip_start,
        anticlockwise,
        8,
    );
    out.push(p_down);
    sample_arc_points(
        &mut out,
        Vec2::ZERO,
        def.pivot_radius,
        pivot_end,
        pivot_start,
        anticlockwise,
        12,
    );

    out
}

fn sample_arc_points(
    out: &mut Vec<Vec2>,
    center: Vec2,
    radius: f32,
    start: f32,
    end: f32,
    anticlockwise: bool,
    steps: usize,
) {
    if steps == 0 {
        return;
    }

    let math_ccw = !anticlockwise;
    let mut delta = end - start;
    if math_ccw {
        if delta < 0.0 {
            delta += TAU;
        }
    } else if delta > 0.0 {
        delta -= TAU;
    }

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let a = start + delta * t;
        out.push(Vec2::new(
            center.x + radius * a.cos(),
            center.y + radius * a.sin(),
        ));
    }
}

fn flipper_system(
    input: Res<InputState>,
    mut q: Query<(&mut Transform, &mut Flipper)>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut flipper) in &mut q {
        let active = match flipper.side {
            FlipperSide::Left => input.left,
            FlipperSide::Right => input.right,
        };

        flipper.angle = step_flipper_angle(flipper.angle, dt, active, flipper.side);
        transform.rotation = Quat::from_rotation_z(-flipper.angle);
    }
}
