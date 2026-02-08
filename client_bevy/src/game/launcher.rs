use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::{ExternalImpulse, ReadMassProperties};

use crate::board::geometry::{ball_spawn, launcher_stop, launcher_wall};
use crate::board::launcher_logic::{step_launcher, LauncherState, MAX_CHARGE};
use crate::constants::{color_from_hex, px_to_world, world_to_px_x, world_to_px_y, Colors, PPM};

use super::ball::{Ball, BallState};
use super::input::InputState;
use super::FixedSet;

pub struct LauncherPlugin;
const STACKED_LAUNCH_BOOST: f32 = 0.45;

#[derive(Resource, Default)]
pub(crate) struct LauncherRuntime {
    pub(crate) state: LauncherState,
}

#[derive(Component)]
pub(crate) struct LauncherChargeBar {
    base_world: Vec2,
    width: f32,
}

impl Plugin for LauncherPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_launcher_bar)
            .add_systems(FixedUpdate, launcher_system.in_set(FixedSet::Simulate));
    }
}

fn launcher_stack_scale(count: usize) -> f32 {
    if count <= 1 {
        1.0
    } else {
        // Keep single-ball feel, but soften the quadratic boost for stacked balls.
        let c = count as f32;
        1.0 + (c * c - 1.0) * STACKED_LAUNCH_BOOST
    }
}

fn spawn_launcher_bar(mut commands: Commands) {
    let spawn = ball_spawn();
    let bar_width = 24.0;
    let bar_shape = shapes::Rectangle {
        extents: Vec2::new(bar_width, 3.0),
        origin: shapes::RectangleOrigin::Center,
        radii: None,
    };
    let base_world = px_to_world(spawn.x, spawn.y + 20.0, 0.0).truncate();

    commands.spawn((
        ShapeBuilder::with(&bar_shape)
            .fill(color_from_hex(Colors::PIN_HIT).with_alpha(0.8))
            .build(),
        Transform::from_xyz(base_world.x, base_world.y, 4.0).with_scale(Vec3::new(0.0, 1.0, 1.0)),
        LauncherChargeBar {
            base_world,
            width: bar_width,
        },
    ));
}

fn launcher_system(
    input: Res<InputState>,
    mut launcher: ResMut<LauncherRuntime>,
    mut q_ball: Query<
        (
            &Transform,
            &mut ExternalImpulse,
            &mut BallState,
            &ReadMassProperties,
        ),
        With<Ball>,
    >,
    mut q_bar: Query<(&LauncherChargeBar, &mut Transform), Without<Ball>>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();
    let (state, fired) = step_launcher(launcher.state, dt, input.launch);
    launcher.state = state;

    // Update charge bar visual
    if let Ok((bar, mut tf)) = q_bar.single_mut() {
        let charge = (launcher.state.charge / MAX_CHARGE).clamp(0.0, 1.0);
        tf.scale.x = charge;
        tf.translation.x = bar.base_world.x - bar.width * 0.5 + bar.width * 0.5 * charge;
    }

    if let Some(speed) = fired {
        let lane = launcher_stop();
        let wall = launcher_wall();

        // Count balls in launcher lane (using TS/Pixi pixel coords)
        let mut count = 0usize;
        for (transform, _, _, _) in &q_ball {
            let px = world_to_px_x(transform.translation.x);
            let py = world_to_px_y(transform.translation.y);
            if px >= lane.from.x && px <= lane.to.x && py >= wall.from.y && py <= wall.to.y {
                count += 1;
            }
        }

        if count > 0 {
            // speed is in normalized units; scale to pixel-space for Rapier impulse.
            // Softened stack boost avoids overpowered launches with 2+ balls.
            let scaled = speed * PPM * launcher_stack_scale(count);
            for (transform, mut impulse, mut ball_state, mass_props) in &mut q_ball {
                let px = world_to_px_x(transform.translation.x);
                let py = world_to_px_y(transform.translation.y);
                if px >= lane.from.x && px <= lane.to.x && py >= wall.from.y && py <= wall.to.y {
                    let mass = mass_props.mass.max(0.0001);
                    // Upward in Bevy = positive Y
                    impulse.impulse.y += scaled * mass;
                    ball_state.in_launcher = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_scale_is_one_for_single_ball() {
        assert!((launcher_stack_scale(1) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn stack_scale_grows_for_multiple_balls() {
        assert!(launcher_stack_scale(2) > 1.0);
        assert!(launcher_stack_scale(3) > launcher_stack_scale(2));
    }

    #[test]
    fn stack_scale_is_softer_than_pure_quadratic() {
        let c = 2.0_f32;
        assert!(launcher_stack_scale(2) < c * c);
    }
}
