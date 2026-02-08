mod ball;
mod core;
mod deep_space;
mod flippers;
mod input;
mod launcher;
mod network;
mod pins;
mod walls;

pub use ball::BallPlugin;
pub use core::{configure_fixed_timestep, CorePlugin};
pub(crate) use core::{FixedSet, UpdateSet};
pub use deep_space::DeepSpacePlugin;
pub use flippers::FlippersPlugin;
pub use input::InputPlugin;
pub use launcher::LauncherPlugin;
pub use network::NetworkPlugin;
pub use pins::PinsPlugin;
pub use walls::WallsPlugin;

use bevy::prelude::Vec2;

use crate::constants::px_to_world;

pub(crate) fn to_world2(px: f32, py: f32) -> Vec2 {
    px_to_world(px, py, 0.0).truncate()
}
