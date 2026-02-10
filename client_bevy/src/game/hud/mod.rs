mod spawn;
mod systems;
mod types;

use bevy::prelude::*;

pub(crate) use types::HitCounter;
pub(crate) use types::HudUiState;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HitCounter>()
            .init_resource::<HudUiState>()
            .add_systems(Startup, spawn::spawn_hud)
            .add_systems(
                Update,
                (
                    systems::handle_button_interactions,
                    systems::update_connection_ui,
                    systems::update_hit_ui,
                    systems::update_players_ui,
                    systems::update_info_panel_ui,
                    systems::update_bot_button_ui,
                )
                    .chain(),
            );
    }
}
