use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_rapier2d::prelude::{PhysicsSet, RapierConfiguration, TimestepMode};

use crate::constants::{
    color_from_hex, Colors, CANVAS_HEIGHT, CANVAS_WIDTH, GRAVITY_Y, PHYSICS_DT,
};
use crate::shared::connection::ServerConnection;

use super::ball::{RespawnState, SpawnBallMessage};
use super::input::InputState;
use super::launcher::LauncherRuntime;
use super::network::NetworkState;

#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) enum UpdateSet {
    Input,
    Network,
    Visuals,
}

#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) enum FixedSet {
    Simulate,
    PostPhysics,
    Spawn,
}

pub struct CorePlugin {
    pub ws_url: String,
}

#[derive(Component)]
struct MainCamera;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerConnection::new(self.ws_url.clone()))
            .init_resource::<InputState>()
            .init_resource::<NetworkState>()
            .init_resource::<LauncherRuntime>()
            .init_resource::<RespawnState>()
            .add_message::<SpawnBallMessage>()
            .insert_resource(ClearColor(color_from_hex(Colors::DEEP_SPACE_BG)))
            .insert_resource(Time::<Fixed>::from_seconds(PHYSICS_DT as f64))
            .insert_resource(TimestepMode::Fixed {
                dt: PHYSICS_DT,
                substeps: 1,
            })
            .configure_sets(
                Update,
                (UpdateSet::Input, UpdateSet::Network, UpdateSet::Visuals).chain(),
            )
            .configure_sets(
                FixedUpdate,
                (FixedSet::Simulate, FixedSet::PostPhysics, FixedSet::Spawn).chain(),
            )
            .configure_sets(
                FixedUpdate,
                FixedSet::Simulate.before(PhysicsSet::SyncBackend),
            )
            .configure_sets(
                FixedUpdate,
                FixedSet::PostPhysics.after(PhysicsSet::Writeback),
            )
            .add_systems(Startup, (setup_camera, configure_rapier_gravity).chain())
            .add_systems(Update, fit_camera_to_canvas);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Msaa::Sample4, MainCamera));
}

fn configure_rapier_gravity(mut q_config: Query<&mut RapierConfiguration>) {
    for mut cfg in &mut q_config {
        cfg.gravity = Vec2::new(0.0, GRAVITY_Y);
    }
}

fn fit_camera_to_canvas(
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut q_projection: Query<&mut Projection, With<MainCamera>>,
) {
    let Ok(window) = q_window.single() else {
        return;
    };

    if window.width() <= 0.0 || window.height() <= 0.0 {
        return;
    }

    let scale_x = CANVAS_WIDTH / window.width();
    let scale_y = CANVAS_HEIGHT / window.height();
    let target_scale = scale_x.max(scale_y).max(0.0001);

    for mut projection in &mut q_projection {
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scale = target_scale;
        }
    }
}
