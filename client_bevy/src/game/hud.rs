use bevy::prelude::*;

use crate::constants::{color_from_hex, Colors};
use crate::shared::connection::ServerConnection;
use crate::shared::types::{ConnectionState, Player};

use super::network::NetworkState;

pub struct HudPlugin;

const MAX_VISIBLE_PLAYERS: usize = 20;

const HIT_TOP: f32 = 10.0;
const PLAYERS_SUMMARY_TOP: f32 = 36.0;
const PLAYER_LIST_TOP: f32 = 60.0;
const PLAYER_ROW_SPACING: f32 = 16.0;

const INFO_BUTTON_LEFT: f32 = 12.0;
const BOT_BUTTON_LEFT: f32 = 48.0;
const BUTTON_BOTTOM: f32 = 12.0;
const BUTTON_SIZE: f32 = 28.0;

const PANEL_LEFT: f32 = 12.0;
const PANEL_BOTTOM: f32 = 48.0;
const PANEL_WIDTH: f32 = 170.0;

const STATUS_CONNECTED: u32 = 0x44ff44;
const STATUS_CONNECTING: u32 = 0xffaa00;
const STATUS_DISCONNECTED: u32 = 0xff4444;
const UI_DIM: u32 = 0x888888;

#[derive(Resource, Default)]
pub(crate) struct HitCounter {
    pub(crate) count: u32,
}

#[derive(Resource, Default)]
struct HudUiState {
    info_visible: bool,
    bot_enabled: bool,
}

#[derive(Component)]
struct HudConnectionGlow;

#[derive(Component)]
struct HudConnectionDot;

#[derive(Component)]
struct HudHitCountText;

#[derive(Component)]
struct HudPlayersSummaryText;

#[derive(Component)]
struct HudPlayerEntryText {
    index: usize,
}

#[derive(Component)]
struct HudPlayerEntryDot {
    index: usize,
}

#[derive(Component)]
struct HudMoreCountText;

#[derive(Component)]
struct HudInfoButton;

#[derive(Component)]
struct HudBotButton;

#[derive(Component)]
struct HudBotButtonText;

#[derive(Component)]
struct HudInfoPanel;

#[derive(Component)]
struct HudInfoPanelClientText;

#[derive(Component)]
struct HudInfoPanelServerText;

#[derive(Component)]
struct HudInfoPanelBotText;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HitCounter>()
            .init_resource::<HudUiState>()
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                (
                    handle_button_interactions,
                    update_connection_ui,
                    update_hit_ui,
                    update_players_ui,
                    update_info_panel_ui,
                    update_bot_button_ui,
                )
                    .chain(),
            );
    }
}

fn spawn_hud(mut commands: Commands) {
    let small = TextFont::from_font_size(10.0);
    let medium = TextFont::from_font_size(14.0);

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(30.0),
            width: Val::Px(16.0),
            height: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(color_from_hex(STATUS_CONNECTING).with_alpha(0.45)),
        BorderRadius::MAX,
        HudConnectionGlow,
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(23.0),
            top: Val::Px(33.0),
            width: Val::Px(10.0),
            height: Val::Px(10.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(color_from_hex(STATUS_CONNECTING).with_alpha(1.0)),
        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.6)),
        BorderRadius::MAX,
        HudConnectionDot,
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(44.0),
            top: Val::Px(HIT_TOP),
            ..default()
        },
        Text::new("H"),
        medium.clone(),
        TextColor(color_from_hex(Colors::WALL).with_alpha(0.95)),
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(HIT_TOP),
            ..default()
        },
        Text::new("0"),
        medium.clone(),
        TextColor(color_from_hex(Colors::WALL)),
        HudHitCountText,
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(44.0),
            top: Val::Px(PLAYERS_SUMMARY_TOP),
            ..default()
        },
        Text::new("P"),
        medium.clone(),
        TextColor(color_from_hex(UI_DIM).with_alpha(0.9)),
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(PLAYERS_SUMMARY_TOP),
            ..default()
        },
        Text::new("0/0"),
        small.clone(),
        TextColor(color_from_hex(UI_DIM)),
        HudPlayersSummaryText,
    ));

    for index in 0..MAX_VISIBLE_PLAYERS {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(PLAYER_LIST_TOP + index as f32 * PLAYER_ROW_SPACING + 2.0),
                width: Val::Px(8.0),
                height: Val::Px(8.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(color_from_hex(Colors::WALL).with_alpha(0.0)),
            BorderColor::all(Color::NONE),
            BorderRadius::MAX,
            Visibility::Hidden,
            HudPlayerEntryDot { index },
        ));

        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(26.0),
                top: Val::Px(PLAYER_LIST_TOP + index as f32 * PLAYER_ROW_SPACING),
                ..default()
            },
            Text::new(""),
            small.clone(),
            TextColor(color_from_hex(UI_DIM)),
            Visibility::Hidden,
            HudPlayerEntryText { index },
        ));
    }

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            top: Val::Px(PLAYER_LIST_TOP + MAX_VISIBLE_PLAYERS as f32 * PLAYER_ROW_SPACING + 4.0),
            ..default()
        },
        Text::new(""),
        small.clone(),
        TextColor(color_from_hex(UI_DIM)),
        Visibility::Hidden,
        HudMoreCountText,
    ));

    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(INFO_BUTTON_LEFT),
                bottom: Val::Px(BUTTON_BOTTOM),
                width: Val::Px(BUTTON_SIZE),
                height: Val::Px(BUTTON_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(panel_bg(0.6)),
            BorderColor::all(panel_border(0.4)),
            BorderRadius::MAX,
            HudInfoButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("i"),
                TextFont::from_font_size(16.0),
                TextColor(panel_border(0.7)),
            ));
        });

    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(BOT_BUTTON_LEFT),
                bottom: Val::Px(BUTTON_BOTTOM),
                width: Val::Px(BUTTON_SIZE),
                height: Val::Px(BUTTON_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(panel_bg(0.6)),
            BorderColor::all(panel_border(0.4)),
            BorderRadius::MAX,
            HudBotButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("b"),
                TextFont::from_font_size(15.0),
                TextColor(panel_border(0.7)),
                HudBotButtonText,
            ));
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(PANEL_LEFT),
                bottom: Val::Px(PANEL_BOTTOM),
                width: Val::Px(PANEL_WIDTH),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect {
                    left: Val::Px(10.0),
                    right: Val::Px(10.0),
                    top: Val::Px(8.0),
                    bottom: Val::Px(8.0),
                },
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(panel_bg(0.92)),
            BorderColor::all(panel_border(0.3)),
            BorderRadius::all(Val::Px(6.0)),
            Visibility::Hidden,
            HudInfoPanel,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                small.clone(),
                TextColor(Color::srgb(0.55, 0.8, 0.8)),
                HudInfoPanelClientText,
            ));
            parent.spawn((
                Text::new(""),
                small.clone(),
                TextColor(Color::srgb(0.55, 0.8, 0.8)),
                HudInfoPanelServerText,
            ));
            parent.spawn((
                Text::new(""),
                small.clone(),
                TextColor(Color::srgb(0.55, 0.8, 0.8)),
                HudInfoPanelBotText,
            ));
            parent.spawn((
                Text::new("github.com/gunstein/Pinball2DMulti"),
                small,
                TextColor(Color::srgb(0.55, 0.8, 0.8)),
            ));
        });
}

fn handle_button_interactions(
    mut button_query: Query<
        (&Interaction, Option<&HudInfoButton>, Option<&HudBotButton>),
        (Changed<Interaction>, With<Button>),
    >,
    mut hud_ui: ResMut<HudUiState>,
) {
    for (interaction, info_button, bot_button) in &mut button_query {
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

fn update_connection_ui(
    conn: Res<ServerConnection>,
    net: Res<NetworkState>,
    mut background_queries: ParamSet<(
        Query<&mut BackgroundColor, With<HudConnectionGlow>>,
        Query<&mut BackgroundColor, With<HudConnectionDot>>,
    )>,
    mut q_dot_border: Query<&mut BorderColor, With<HudConnectionDot>>,
) {
    let color = connection_color(conn.state.clone(), net.protocol_mismatch);
    for mut glow in &mut background_queries.p0() {
        glow.0 = color.with_alpha(0.45);
    }
    for mut dot in &mut background_queries.p1() {
        dot.0 = color.with_alpha(1.0);
    }
    for mut border in &mut q_dot_border {
        *border = BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.6));
    }
}

fn update_hit_ui(hit: Res<HitCounter>, mut q_hit: Query<&mut Text, With<HudHitCountText>>) {
    if let Ok(mut text) = q_hit.single_mut() {
        text.0 = hit.count.to_string();
    }
}

fn update_players_ui(
    conn: Res<ServerConnection>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<HudPlayersSummaryText>>,
        Query<(
            &HudPlayerEntryText,
            &mut Text,
            &mut TextColor,
            &mut Visibility,
        )>,
        Query<(&mut Text, &mut Visibility), With<HudMoreCountText>>,
        Query<(
            &HudPlayerEntryDot,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Visibility,
        )>,
    )>,
) {
    let mut sorted_players: Vec<&Player> = conn.players.iter().collect();
    sorted_players.sort_by_key(|p| (p.id != conn.self_id, p.id));

    let active_players = conn.players.iter().filter(|p| !p.paused).count();
    if let Ok(mut summary) = text_queries.p0().single_mut() {
        summary.0 = format!("{active_players}/{}", sorted_players.len());
    }

    let visible_count = sorted_players.len().min(MAX_VISIBLE_PLAYERS);
    let has_more_players = sorted_players.len() > MAX_VISIBLE_PLAYERS;

    for (entry, mut dot_fill, mut dot_border, mut visibility) in &mut text_queries.p3() {
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

    for (entry, mut text, mut text_color, mut visibility) in &mut text_queries.p1() {
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

    if let Ok((mut more_text, mut more_visibility)) = text_queries.p2().single_mut() {
        if has_more_players {
            more_text.0 = format!("... {}", sorted_players.len());
            *more_visibility = Visibility::Visible;
        } else {
            *more_visibility = Visibility::Hidden;
        }
    }
}

fn update_info_panel_ui(
    conn: Res<ServerConnection>,
    hud_ui: Res<HudUiState>,
    mut q_panel: Query<&mut Visibility, With<HudInfoPanel>>,
    mut info_texts: ParamSet<(
        Query<&mut Text, With<HudInfoPanelClientText>>,
        Query<&mut Text, With<HudInfoPanelServerText>>,
        Query<&mut Text, With<HudInfoPanelBotText>>,
    )>,
) {
    if let Ok(mut visibility) = q_panel.single_mut() {
        *visibility = if hud_ui.info_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut text) = info_texts.p0().single_mut() {
        text.0 = format!("Client: v{}", env!("CARGO_PKG_VERSION"));
    }

    if let Ok(mut text) = info_texts.p1().single_mut() {
        if conn.server_version.is_empty() {
            text.0 = "Server: -".to_string();
        } else {
            text.0 = format!("Server: v{}", conn.server_version);
        }
    }

    if let Ok(mut text) = info_texts.p2().single_mut() {
        text.0 = format!("Bot: {}", if hud_ui.bot_enabled { "ON" } else { "OFF" });
    }
}

fn update_bot_button_ui(
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

fn connection_color(state: ConnectionState, protocol_mismatch: bool) -> Color {
    if protocol_mismatch {
        return color_from_hex(STATUS_DISCONNECTED);
    }

    match state {
        ConnectionState::Connected => color_from_hex(STATUS_CONNECTED),
        ConnectionState::Connecting => color_from_hex(STATUS_CONNECTING),
        ConnectionState::Disconnected => color_from_hex(STATUS_DISCONNECTED),
    }
}

fn panel_bg(alpha: f32) -> Color {
    Color::srgba(5.0 / 255.0, 5.0 / 255.0, 16.0 / 255.0, alpha)
}

fn panel_border(alpha: f32) -> Color {
    Color::srgba(77.0 / 255.0, 166.0 / 255.0, 166.0 / 255.0, alpha)
}
