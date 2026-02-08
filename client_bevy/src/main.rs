mod board;
mod constants;
mod game;
mod shared;

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use bevy_prototype_lyon::prelude::ShapePlugin;
use bevy_rapier2d::prelude::*;

use constants::PPM;
use game::{
    BallPlugin, CorePlugin, DeepSpacePlugin, FlippersPlugin, HudPlugin, InputPlugin,
    LauncherPlugin, NetworkPlugin, PinsPlugin, WallsPlugin,
};

fn main() {
    let ws_url = ws_url_from_env_or_location();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Pinball2DMulti Bevy Client".to_string(),
                resolution: WindowResolution::new(700, 760),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(PPM).in_fixed_schedule())
        .add_plugins(ShapePlugin)
        .add_plugins(CorePlugin { ws_url })
        .add_plugins(WallsPlugin)
        .add_plugins(FlippersPlugin)
        .add_plugins(LauncherPlugin)
        .add_plugins(BallPlugin)
        .add_plugins(PinsPlugin)
        .add_plugins(DeepSpacePlugin)
        .add_plugins(InputPlugin)
        .add_plugins(NetworkPlugin)
        .add_plugins(HudPlugin)
        .run();
}

#[cfg(not(target_arch = "wasm32"))]
fn ws_url_from_env_or_location() -> String {
    std::env::var("PINBALL_WS_URL").unwrap_or_else(|_| "ws://127.0.0.1:9001/ws".to_string())
}

#[cfg(target_arch = "wasm32")]
fn ws_url_from_env_or_location() -> String {
    let Some(window) = web_sys::window() else {
        return "ws://127.0.0.1:9001/ws".to_string();
    };

    let location = window.location();
    let host = location
        .host()
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "127.0.0.1:9001".to_string());
    let ws_scheme = if location.protocol().ok().as_deref() == Some("https:") {
        "wss"
    } else {
        "ws"
    };

    format!("{ws_scheme}://{host}/ws")
}
