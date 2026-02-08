use bevy::prelude::*;

use crate::shared::connection::ServerConnection;

use super::UpdateSet;

pub struct InputPlugin;

#[derive(Resource, Default)]
pub(crate) struct InputState {
    pub(crate) left: bool,
    pub(crate) right: bool,
    pub(crate) launch: bool,
    /// Total elapsed seconds at last input activity, or 0 if never active.
    pub(crate) last_activity_time: f64,
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
    time: Res<Time>,
) {
    input.left = keys.pressed(KeyCode::ArrowLeft);
    input.right = keys.pressed(KeyCode::ArrowRight);
    input.launch = keys.pressed(KeyCode::Space);

    if input.left || input.right || input.launch {
        input.last_activity_time = time.elapsed_secs_f64();
    }

    if keys.just_pressed(KeyCode::Tab) {
        conn.send_set_paused(false);
    }
}
