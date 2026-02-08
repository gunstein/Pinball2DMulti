mod board;
mod constants;
mod game;
mod layers;
mod shared;

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use bevy_prototype_lyon::prelude::ShapePlugin;
use bevy_rapier2d::prelude::*;

use game::{
    configure_fixed_timestep, BallPlugin, CorePlugin, DeepSpacePlugin, FlippersPlugin, InputPlugin,
    LauncherPlugin, NetworkPlugin, PinsPlugin, WallsPlugin,
};

fn main() {
    let ws_url =
        std::env::var("PINBALL_WS_URL").unwrap_or_else(|_| "ws://127.0.0.1:9001/ws".to_string());

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Pinball2DMulti Bevy Client".to_string(),
            resolution: WindowResolution::new(700, 760),
            present_mode: PresentMode::AutoVsync,
            resizable: true,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(1.0).in_fixed_schedule())
    .add_plugins(ShapePlugin)
    .add_plugins(CorePlugin { ws_url })
    .add_plugins(WallsPlugin)
    .add_plugins(FlippersPlugin)
    .add_plugins(LauncherPlugin)
    .add_plugins(BallPlugin)
    .add_plugins(PinsPlugin)
    .add_plugins(DeepSpacePlugin)
    .add_plugins(InputPlugin)
    .add_plugins(NetworkPlugin);

    configure_fixed_timestep(&mut app);
    app.run();
}
