use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::{ExternalImpulse, ReadMassProperties};

use crate::board::geometry::{ball_spawn, launcher_stop, launcher_wall};
use crate::board::launcher_logic::{step_launcher, LauncherState, MAX_CHARGE};
use crate::constants::{color_from_hex, Colors, PPM};

use super::ball::{Ball, BallState};
use super::input::InputState;
use super::{to_world2, FixedSet, UpdateSet};

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
            .add_systems(FixedUpdate, launcher_system.in_set(FixedSet::Simulate))
            .add_systems(Update, update_launcher_bar.in_set(UpdateSet::Visuals));
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
    let base_world = to_world2(spawn.x, spawn.y + 20.0);

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
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();
    let (state, fired) = step_launcher(launcher.state, dt, input.launch);
    launcher.state = state;

    if let Some(speed) = fired {
        let lane = launcher_stop();
        let mut count = 0usize;

        for (transform, _impulse, _state, _mass) in &mut q_ball {
            let px = transform.translation.x * PPM;
            let py = transform.translation.y * PPM;
            let in_lane = px >= lane.from.x
                && px <= lane.to.x
                && py >= launcher_wall().from.y
                && py <= launcher_wall().to.y;

            if in_lane {
                count += 1;
            }
        }

        if count > 0 {
            let scaled = speed * (count as f32) * (count as f32);
            for (transform, mut impulse, mut state, mass_props) in &mut q_ball {
                let px = transform.translation.x * PPM;
                let py = transform.translation.y * PPM;
                let in_lane = px >= lane.from.x
                    && px <= lane.to.x
                    && py >= launcher_wall().from.y
                    && py <= launcher_wall().to.y;
                if in_lane {
                    // Match TS behavior: impulse = launch_speed * body_mass.
                    let mass = mass_props.mass.max(0.0001);
                    impulse.impulse.y += -scaled * mass;
                    state.in_launcher = false;
                }
            }
        }
    }
}

fn update_launcher_bar(
    launcher: Res<LauncherRuntime>,
    mut q_bar: Query<(&LauncherChargeBar, &mut Transform)>,
) {
    if let Ok((bar, mut tf)) = q_bar.single_mut() {
        let charge = (launcher.state.charge / MAX_CHARGE).clamp(0.0, 1.0);
        tf.scale.x = charge;
        tf.translation.x = bar.base_world.x - bar.width * 0.5 + bar.width * 0.5 * charge;
    }
}
