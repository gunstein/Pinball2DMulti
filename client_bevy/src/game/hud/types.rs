use bevy::prelude::*;

use crate::constants::color_from_hex;
use crate::shared::types::ConnectionState;

pub(super) const MAX_VISIBLE_PLAYERS: usize = 20;

pub(super) const HIT_TOP: f32 = 10.0;
pub(super) const PLAYERS_SUMMARY_TOP: f32 = 36.0;
pub(super) const PLAYER_LIST_TOP: f32 = 60.0;
pub(super) const PLAYER_ROW_SPACING: f32 = 16.0;

pub(super) const INFO_BUTTON_LEFT: f32 = 12.0;
pub(super) const BOT_BUTTON_LEFT: f32 = 48.0;
pub(super) const BUTTON_BOTTOM: f32 = 12.0;
pub(super) const BUTTON_SIZE: f32 = 28.0;

pub(super) const PANEL_LEFT: f32 = 12.0;
pub(super) const PANEL_BOTTOM: f32 = 48.0;
pub(super) const PANEL_WIDTH: f32 = 170.0;

pub(super) const STATUS_CONNECTED: u32 = 0x44ff44;
pub(super) const STATUS_CONNECTING: u32 = 0xffaa00;
pub(super) const STATUS_DISCONNECTED: u32 = 0xff4444;
pub(super) const UI_DIM: u32 = 0x888888;

#[derive(Resource, Default)]
pub(crate) struct HitCounter {
    pub(crate) count: u32,
}

#[derive(Resource, Default)]
pub(crate) struct HudUiState {
    pub(crate) info_visible: bool,
    pub(crate) bot_enabled: bool,
}

#[derive(Component)]
pub(super) struct HudConnectionGlow;

#[derive(Component)]
pub(super) struct HudConnectionDot;

#[derive(Component)]
pub(super) struct HudHitCountText;

#[derive(Component)]
pub(super) struct HudPlayersSummaryText;

#[derive(Component)]
pub(super) struct HudPlayerEntryText {
    pub(super) index: usize,
}

#[derive(Component)]
pub(super) struct HudPlayerEntryDot {
    pub(super) index: usize,
}

#[derive(Component)]
pub(super) struct HudMoreCountText;

#[derive(Component)]
pub(super) struct HudInfoButton;

#[derive(Component)]
pub(super) struct HudBotButton;

#[derive(Component)]
pub(super) struct HudBotButtonText;

#[derive(Component)]
pub(super) struct HudInfoPanel;

#[derive(Component)]
pub(super) struct HudInfoPanelClientText;

#[derive(Component)]
pub(super) struct HudInfoPanelServerText;

#[derive(Component)]
pub(super) struct HudInfoPanelBotText;

pub(super) fn connection_color(state: ConnectionState, protocol_mismatch: bool) -> Color {
    if protocol_mismatch {
        return color_from_hex(STATUS_DISCONNECTED);
    }

    match state {
        ConnectionState::Connected => color_from_hex(STATUS_CONNECTED),
        ConnectionState::Connecting => color_from_hex(STATUS_CONNECTING),
        ConnectionState::Disconnected => color_from_hex(STATUS_DISCONNECTED),
    }
}

pub(super) fn panel_bg(alpha: f32) -> Color {
    Color::srgba(5.0 / 255.0, 5.0 / 255.0, 16.0 / 255.0, alpha)
}

pub(super) fn panel_border(alpha: f32) -> Color {
    Color::srgba(77.0 / 255.0, 166.0 / 255.0, 166.0 / 255.0, alpha)
}
