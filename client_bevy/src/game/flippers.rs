use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::board::flipper_logic::{rest_angle, step_flipper_angle};
use crate::board::geometry::{flippers, FlipperDef, FlipperSide};
use crate::constants::{color_from_hex, Colors, PPM};

use super::input::InputState;
use super::{to_world2, FixedSet, UpdateSet};

pub struct FlippersPlugin;

#[derive(Component)]
pub(crate) struct Flipper {
    pub(crate) side: FlipperSide,
    pub(crate) prev_angle: f32,
    pub(crate) current_angle: f32,
}

#[derive(Component)]
struct FlipperVisual {
    flipper: Entity,
}

impl Plugin for FlippersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_flippers)
            .add_systems(FixedUpdate, flipper_system.in_set(FixedSet::Simulate))
            .add_systems(Update, update_flipper_visuals.in_set(UpdateSet::Visuals));
    }
}

fn spawn_flippers(mut commands: Commands) {
    for def in flippers() {
        spawn_flipper(&mut commands, def);
    }
}

fn spawn_flipper(commands: &mut Commands, def: FlipperDef) {
    let body = commands
        .spawn((
            RigidBody::KinematicPositionBased,
            Transform::from_xyz(def.pivot.x / PPM, def.pivot.y / PPM, 0.0)
                .with_rotation(Quat::from_rotation_z(rest_angle(def.side))),
            GlobalTransform::default(),
            Friction::coefficient(0.8),
            Restitution::coefficient(0.5),
            Flipper {
                side: def.side,
                prev_angle: rest_angle(def.side),
                current_angle: rest_angle(def.side),
            },
        ))
        .id();

    let collider = build_flipper_collider(def);
    let child = commands
        .spawn((collider, Transform::default(), GlobalTransform::default()))
        .id();
    commands.entity(body).add_child(child);

    let shape = shapes::Polygon {
        points: tapered_outline_points_px(def),
        closed: true,
    };

    let world = to_world2(def.pivot.x, def.pivot.y);
    commands.spawn((
        ShapeBuilder::with(&shape)
            .stroke((color_from_hex(Colors::FLIPPER), 2.0))
            .build(),
        Transform::from_xyz(world.x, world.y, 3.0)
            .with_rotation(Quat::from_rotation_z(-rest_angle(def.side))),
        FlipperVisual { flipper: body },
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
            def.pivot_radius * angle.cos() / PPM,
            def.pivot_radius * angle.sin() / PPM,
        ));
    }

    for i in 0..segments {
        let angle = TAU * (i as f32) / (segments as f32);
        points.push(Vec2::new(
            (dir * def.length + def.tip_radius * angle.cos()) / PPM,
            (def.tip_radius * angle.sin()) / PPM,
        ));
    }

    Collider::convex_hull(&points)
        .unwrap_or_else(|| Collider::cuboid(def.length * 0.5 / PPM, def.width * 0.5 / PPM))
}

fn tapered_outline_points_px(def: FlipperDef) -> Vec<Vec2> {
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

    // Our flipper geometry is authored in TS/Pixi space (Y-down).
    // Canvas `arc(..., anticlockwise)` has opposite winding to mathematical Y-up angles.
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

        flipper.prev_angle = flipper.current_angle;
        flipper.current_angle = step_flipper_angle(flipper.current_angle, dt, active, flipper.side);
        transform.rotation = Quat::from_rotation_z(flipper.current_angle);
    }
}

fn update_flipper_visuals(
    mut commands: Commands,
    mut q_visuals: Query<(Entity, &FlipperVisual, &mut Transform), Without<Flipper>>,
    q_flippers: Query<(&Transform, &Flipper), With<Flipper>>,
    time_fixed: Res<Time<Fixed>>,
) {
    let alpha = time_fixed.overstep_fraction().clamp(0.0, 1.0);

    for (visual_entity, visual, mut tf) in &mut q_visuals {
        if let Ok((flipper_tf, flipper)) = q_flippers.get(visual.flipper) {
            let px = flipper_tf.translation.x * PPM;
            let py = flipper_tf.translation.y * PPM;
            let world = to_world2(px, py);
            tf.translation.x = world.x;
            tf.translation.y = world.y;

            let angle = flipper.prev_angle.lerp(flipper.current_angle, alpha);
            tf.rotation = Quat::from_rotation_z(-angle);
        } else {
            commands.entity(visual_entity).despawn();
        }
    }
}
