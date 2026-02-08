use std::time::Instant;

use bevy::prelude::*;

use crate::shared::connection::ServerConnection;

use super::UpdateSet;

pub struct InputPlugin;

#[derive(Resource, Default)]
pub(crate) struct InputState {
    pub(crate) left: bool,
    pub(crate) right: bool,
    pub(crate) launch: bool,
    pub(crate) last_activity: Option<Instant>,
}

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, input_system.in_set(UpdateSet::Input));
    }
}

fn input_system(
    mut input: ResMut<InputState>,
    keys: Res<ButtonInput<KeyCode>>,
    conn: Res<ServerConnection>,
) {
    input.left = keys.pressed(KeyCode::ArrowLeft);
    input.right = keys.pressed(KeyCode::ArrowRight);
    input.launch = keys.pressed(KeyCode::Space);

    if input.left || input.right || input.launch {
        input.last_activity = Some(Instant::now());
    }

    if keys.just_pressed(KeyCode::Tab) {
        conn.send_set_paused(false);
    }
}
