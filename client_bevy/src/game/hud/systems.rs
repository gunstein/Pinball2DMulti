use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::constants::color_from_hex;
use crate::game::network::NetworkState;
use crate::shared::connection::ServerConnection;
use crate::shared::types::Player;

use super::types::{
    connection_color, panel_border, HitCounter, HudBotButton, HudBotButtonText, HudConnectionDot,
    HudConnectionGlow, HudHitCountText, HudInfoButton, HudInfoPanel, HudInfoPanelBotText,
    HudInfoPanelClientText, HudInfoPanelServerText, HudMoreCountText, HudPlayerEntryDot,
    HudPlayerEntryText, HudPlayersSummaryText, HudUiState, MAX_VISIBLE_PLAYERS, UI_DIM,
};

type ButtonInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        Option<&'static HudInfoButton>,
        Option<&'static HudBotButton>,
    ),
    (Changed<Interaction>, With<Button>),
>;

type ConnectionColorsSet<'w, 's> = ParamSet<
    'w,
    's,
    (
        Query<'w, 's, &'static mut BackgroundColor, With<HudConnectionGlow>>,
        Query<'w, 's, &'static mut BackgroundColor, With<HudConnectionDot>>,
    ),
>;

type PlayersSummaryTextQuery<'w, 's> =
    Query<'w, 's, &'static mut Text, With<HudPlayersSummaryText>>;
type PlayerRowsQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static HudPlayerEntryText,
        &'static mut Text,
        &'static mut TextColor,
        &'static mut Visibility,
    ),
>;
type PlayersMoreTextQuery<'w, 's> =
    Query<'w, 's, (&'static mut Text, &'static mut Visibility), With<HudMoreCountText>>;
type PlayerDotsQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static HudPlayerEntryDot,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
        &'static mut Visibility,
    ),
>;
type PlayersTextSet<'w, 's> = ParamSet<
    'w,
    's,
    (
        PlayersSummaryTextQuery<'w, 's>,
        PlayerRowsQuery<'w, 's>,
        PlayersMoreTextQuery<'w, 's>,
        PlayerDotsQuery<'w, 's>,
    ),
>;

type InfoClientTextQuery<'w, 's> = Query<'w, 's, &'static mut Text, With<HudInfoPanelClientText>>;
type InfoServerTextQuery<'w, 's> = Query<'w, 's, &'static mut Text, With<HudInfoPanelServerText>>;
type InfoBotTextQuery<'w, 's> = Query<'w, 's, &'static mut Text, With<HudInfoPanelBotText>>;
type InfoTextSet<'w, 's> = ParamSet<
    'w,
    's,
    (
        InfoClientTextQuery<'w, 's>,
        InfoServerTextQuery<'w, 's>,
        InfoBotTextQuery<'w, 's>,
    ),
>;

#[derive(SystemParam)]
pub(super) struct ButtonQueries<'w, 's> {
    query: ButtonInteractionQuery<'w, 's>,
}

#[derive(SystemParam)]
pub(super) struct ConnectionUiQueries<'w, 's> {
    colors: ConnectionColorsSet<'w, 's>,
    dot_borders: Query<'w, 's, &'static mut BorderColor, With<HudConnectionDot>>,
}

#[derive(SystemParam)]
pub(super) struct PlayerUiQueries<'w, 's> {
    text_sets: PlayersTextSet<'w, 's>,
}

#[derive(SystemParam)]
pub(super) struct InfoPanelTextQueries<'w, 's> {
    texts: InfoTextSet<'w, 's>,
}

pub(super) fn handle_button_interactions(
    mut buttons: ButtonQueries,
    mut hud_ui: ResMut<HudUiState>,
) {
    for (interaction, info_button, bot_button) in &mut buttons.query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if info_button.is_some() {
            hud_ui.info_visible = !hud_ui.info_visible;
        }
        if bot_button.is_some() {
            hud_ui.bot_enabled = !hud_ui.bot_enabled;
        }
    }
}

pub(super) fn update_connection_ui(
    conn: Res<ServerConnection>,
    net: Res<NetworkState>,
    mut queries: ConnectionUiQueries,
) {
    let color = connection_color(conn.state.clone(), net.protocol_mismatch);
    for mut glow in &mut queries.colors.p0() {
        glow.0 = color.with_alpha(0.45);
    }
    for mut dot in &mut queries.colors.p1() {
        dot.0 = color.with_alpha(1.0);
    }
    for mut border in &mut queries.dot_borders {
        *border = BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.6));
    }
}

pub(super) fn update_hit_ui(
    hit: Res<HitCounter>,
    mut q_hit: Query<&mut Text, With<HudHitCountText>>,
) {
    if let Ok(mut text) = q_hit.single_mut() {
        text.0 = hit.count.to_string();
    }
}

pub(super) fn update_players_ui(conn: Res<ServerConnection>, mut queries: PlayerUiQueries) {
    let mut sorted_players: Vec<&Player> = conn.players.iter().collect();
    sorted_players.sort_by_key(|p| (p.id != conn.self_id, p.id));

    let active_players = conn.players.iter().filter(|p| !p.paused).count();
    if let Ok(mut summary) = queries.text_sets.p0().single_mut() {
        summary.0 = format!("{active_players}/{}", sorted_players.len());
    }

    let visible_count = sorted_players.len().min(MAX_VISIBLE_PLAYERS);
    let has_more_players = sorted_players.len() > MAX_VISIBLE_PLAYERS;

    for (entry, mut dot_fill, mut dot_border, mut visibility) in &mut queries.text_sets.p3() {
        if entry.index < visible_count {
            let player = sorted_players[entry.index];
            let alpha = if player.paused { 0.35 } else { 0.95 };
            dot_fill.0 = color_from_hex(player.color).with_alpha(alpha);
            *dot_border = BorderColor::all(if player.id == conn.self_id {
                Color::srgba(1.0, 1.0, 1.0, 0.9)
            } else {
                Color::NONE
            });
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    for (entry, mut text, mut text_color, mut visibility) in &mut queries.text_sets.p1() {
        if entry.index < visible_count {
            let player = sorted_players[entry.index];
            let self_mark = if player.id == conn.self_id { "*" } else { " " };
            text.0 = format!(
                "{self_mark}{:02} {}/{}",
                player.id, player.balls_in_flight, player.balls_produced
            );
            text_color.0 =
                color_from_hex(UI_DIM).with_alpha(if player.paused { 0.45 } else { 0.95 });
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    if let Ok((mut more_text, mut more_visibility)) = queries.text_sets.p2().single_mut() {
        if has_more_players {
            more_text.0 = format!("... {}", sorted_players.len());
            *more_visibility = Visibility::Visible;
        } else {
            *more_visibility = Visibility::Hidden;
        }
    }
}

pub(super) fn update_info_panel_ui(
    conn: Res<ServerConnection>,
    hud_ui: Res<HudUiState>,
    mut q_panel: Query<&mut Visibility, With<HudInfoPanel>>,
    mut info_texts: InfoPanelTextQueries,
) {
    if let Ok(mut visibility) = q_panel.single_mut() {
        *visibility = if hud_ui.info_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut text) = info_texts.texts.p0().single_mut() {
        text.0 = format!("Client: v{}", env!("CARGO_PKG_VERSION"));
    }

    if let Ok(mut text) = info_texts.texts.p1().single_mut() {
        if conn.server_version.is_empty() {
            text.0 = "Server: -".to_string();
        } else {
            text.0 = format!("Server: v{}", conn.server_version);
        }
    }

    if let Ok(mut text) = info_texts.texts.p2().single_mut() {
        text.0 = format!("Bot: {}", if hud_ui.bot_enabled { "ON" } else { "OFF" });
    }
}

pub(super) fn update_bot_button_ui(
    hud_ui: Res<HudUiState>,
    mut q_bot_button: Query<&mut BorderColor, With<HudBotButton>>,
    mut q_bot_text: Query<&mut TextColor, With<HudBotButtonText>>,
) {
    let alpha = if hud_ui.bot_enabled { 0.8 } else { 0.4 };
    let text_alpha = if hud_ui.bot_enabled { 1.0 } else { 0.7 };

    for mut border in &mut q_bot_button {
        *border = BorderColor::all(panel_border(alpha));
    }

    for mut text_color in &mut q_bot_text {
        text_color.0 = panel_border(text_alpha);
    }
}
