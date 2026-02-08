use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::board::geometry::bumpers;
use crate::constants::{color_from_hex, px_to_world, Colors};

use super::{FixedSet, UpdateSet};

pub struct PinsPlugin;

#[derive(Component)]
pub(crate) struct Bumper {
    pub(crate) glow: Entity,
}

#[derive(Component)]
pub(crate) struct PinHitTimer {
    pub(crate) seconds_left: f32,
}

#[derive(Component)]
struct PinGlow;

impl Plugin for PinsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_pins)
            .add_systems(FixedUpdate, tick_pin_hit_timers.in_set(FixedSet::Simulate))
            .add_systems(Update, update_pin_visuals.in_set(UpdateSet::Visuals));
    }
}

fn spawn_pins(mut commands: Commands) {
    for def in bumpers() {
        let world = px_to_world(def.center.x, def.center.y, 0.0);

        // Glow ring (spawned first to get entity ID)
        let glow = commands
            .spawn((
                ShapeBuilder::with(&shapes::Circle {
                    radius: 29.0,
                    center: Vec2::ZERO,
                })
                .fill(color_from_hex(Colors::PIN_HIT).with_alpha(0.0))
                .build(),
                Transform::from_xyz(world.x, world.y, 2.2),
                PinGlow,
            ))
            .id();

        // Pin: physics + visual on same entity
        commands.spawn((
            RigidBody::Fixed,
            Collider::ball(def.radius),
            Restitution::coefficient(0.7),
            ActiveEvents::COLLISION_EVENTS,
            Transform::from_xyz(world.x, world.y, 2.3),
            ShapeBuilder::with(&shapes::Circle {
                radius: 25.0,
                center: Vec2::ZERO,
            })
            .stroke((color_from_hex(Colors::PIN), 2.0))
            .build(),
            Bumper { glow },
            PinHitTimer { seconds_left: 0.0 },
        ));
    }
}

fn tick_pin_hit_timers(mut q_pins: Query<&mut PinHitTimer>, time: Res<Time<Fixed>>) {
    let dt = time.delta_secs();
    for mut timer in &mut q_pins {
        if timer.seconds_left > 0.0 {
            timer.seconds_left = (timer.seconds_left - dt).max(0.0);
        }
    }
}

fn update_pin_visuals(
    mut q_pins: Query<(&PinHitTimer, &Bumper, &mut Shape)>,
    mut q_glows: Query<&mut Shape, (With<PinGlow>, Without<Bumper>)>,
) {
    for (hit, bumper, mut shape) in &mut q_pins {
        let t = hit.seconds_left.clamp(0.0, 1.0);

        if let Some(stroke) = shape.stroke.as_mut() {
            stroke.color = if t > 0.0 {
                color_from_hex(Colors::PIN_HIT)
            } else {
                color_from_hex(Colors::PIN)
            };
        }

        if let Ok(mut glow_shape) = q_glows.get_mut(bumper.glow) {
            if let Some(fill) = glow_shape.fill.as_mut() {
                fill.color = color_from_hex(Colors::PIN_HIT).with_alpha(0.2 * t);
            }
        }
    }
}
