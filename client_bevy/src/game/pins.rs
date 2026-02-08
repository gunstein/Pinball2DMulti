use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::board::geometry::bumpers;
use crate::constants::{color_from_hex, Colors, PPM};

use super::{to_world2, FixedSet, UpdateSet};

pub struct PinsPlugin;

#[derive(Component)]
pub(crate) struct Bumper;

#[derive(Component)]
pub(crate) struct PinHitTimer {
    pub(crate) seconds_left: f32,
}

#[derive(Component)]
pub(crate) struct PinVisual {
    pub(crate) pin: Entity,
    pub(crate) glow: bool,
}

impl Plugin for PinsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_pins)
            .add_systems(FixedUpdate, tick_pin_hit_timers.in_set(FixedSet::Simulate))
            .add_systems(Update, update_pin_visuals.in_set(UpdateSet::Visuals));
    }
}

fn spawn_pins(mut commands: Commands) {
    for def in bumpers() {
        let p = def.center / PPM;
        let pin = commands
            .spawn((
                RigidBody::Fixed,
                Collider::ball(def.radius / PPM),
                Restitution::coefficient(0.7),
                ActiveEvents::COLLISION_EVENTS,
                Transform::from_xyz(p.x, p.y, 0.0),
                GlobalTransform::default(),
                Bumper,
                PinHitTimer { seconds_left: 0.0 },
            ))
            .id();

        let world = to_world2(def.center.x, def.center.y);
        let glow = shapes::Circle {
            radius: 29.0,
            center: Vec2::ZERO,
        };
        let core = shapes::Circle {
            radius: 25.0,
            center: Vec2::ZERO,
        };

        commands.spawn((
            ShapeBuilder::with(&glow)
                .fill(color_from_hex(Colors::PIN_HIT).with_alpha(0.0))
                .build(),
            Transform::from_xyz(world.x, world.y, 2.2),
            PinVisual { pin, glow: true },
        ));

        commands.spawn((
            ShapeBuilder::with(&core)
                .stroke((color_from_hex(Colors::PIN), 2.0))
                .build(),
            Transform::from_xyz(world.x, world.y, 2.3),
            PinVisual { pin, glow: false },
        ));
    }
}

fn tick_pin_hit_timers(mut q_pins: Query<&mut PinHitTimer, With<Bumper>>, time: Res<Time<Fixed>>) {
    let dt = time.delta_secs();
    for mut timer in &mut q_pins {
        if timer.seconds_left > 0.0 {
            timer.seconds_left = (timer.seconds_left - dt).max(0.0);
        }
    }
}

fn update_pin_visuals(
    mut commands: Commands,
    q_pins: Query<(&Transform, &PinHitTimer), (With<Bumper>, Without<PinVisual>)>,
    mut q_pin_visuals: Query<
        (Entity, &PinVisual, &mut Transform, &mut Shape),
        (With<PinVisual>, Without<Bumper>),
    >,
) {
    for (visual_entity, visual, mut tf, mut shape) in &mut q_pin_visuals {
        if let Ok((pin_tf, hit)) = q_pins.get(visual.pin) {
            let px = pin_tf.translation.x * PPM;
            let py = pin_tf.translation.y * PPM;
            let world = to_world2(px, py);
            tf.translation.x = world.x;
            tf.translation.y = world.y;

            let t = hit.seconds_left.clamp(0.0, 1.0);
            if visual.glow {
                if let Some(fill) = shape.fill.as_mut() {
                    fill.color = color_from_hex(Colors::PIN_HIT).with_alpha(0.2 * t);
                }
            } else if let Some(stroke) = shape.stroke.as_mut() {
                stroke.color = if t > 0.0 {
                    color_from_hex(Colors::PIN_HIT)
                } else {
                    color_from_hex(Colors::PIN)
                };
            }
        } else {
            commands.entity(visual_entity).despawn();
        }
    }
}
