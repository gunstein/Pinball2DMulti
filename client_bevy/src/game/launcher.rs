use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::{ExternalImpulse, ReadMassProperties};

use crate::board::geometry::{ball_spawn, launcher_stop, launcher_wall};
use crate::board::launcher_logic::{step_launcher, LauncherState, MAX_CHARGE};
use crate::constants::{color_from_hex, Colors, PPM};
use crate::coord::{px_to_world, world_to_px, PxPos};

use super::ball::{Ball, BallState};
use super::input::InputState;
use super::FixedSet;

pub struct LauncherPlugin;

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
    let c = count.max(1) as f32;
    c * c
}

fn spawn_launcher_bar(mut commands: Commands) {
    let spawn = ball_spawn();
    let bar_width = 24.0;
    let bar_shape = shapes::Rectangle {
        extents: Vec2::new(bar_width, 3.0),
        origin: shapes::RectangleOrigin::Center,
        radii: None,
    };
    let base_world = px_to_world(PxPos::new(spawn.x, spawn.y + 20.0), 0.0).truncate();

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
    q_ball_positions: Query<(Entity, &Transform), With<Ball>>,
    mut q_ball_impulses: Query<
        (&mut ExternalImpulse, &mut BallState, &ReadMassProperties),
        With<Ball>,
    >,
    mut launch_targets: Local<Vec<Entity>>,
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
        launch_targets.clear();
        for (entity, transform) in &q_ball_positions {
            let px = world_to_px(transform.translation.truncate());
            if px.x >= lane.from.x && px.x <= lane.to.x && px.y >= wall.from.y && px.y <= wall.to.y
            {
                launch_targets.push(entity);
            }
        }

        let count = launch_targets.len();
        if count > 0 {
            // Match TS client parity: quadratic launch boost for stacked balls.
            let scaled = speed * PPM * launcher_stack_scale(count);
            for entity in launch_targets.iter().copied() {
                if let Ok((mut impulse, mut ball_state, mass_props)) =
                    q_ball_impulses.get_mut(entity)
                {
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
    fn stack_scale_matches_quadratic_two_balls() {
        assert!((launcher_stack_scale(2) - 4.0).abs() < 1e-6);
    }

    #[test]
    fn stack_scale_matches_quadratic_three_balls() {
        assert!((launcher_stack_scale(3) - 9.0).abs() < 1e-6);
    }
}
