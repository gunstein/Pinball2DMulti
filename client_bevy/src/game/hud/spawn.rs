use bevy::prelude::*;

use crate::constants::{color_from_hex, Colors};

use super::types::{
    panel_bg, panel_border, HudBotButton, HudBotButtonText, HudConnectionDot, HudConnectionGlow,
    HudHitCountText, HudInfoButton, HudInfoPanel, HudInfoPanelBotText, HudInfoPanelClientText,
    HudInfoPanelServerText, HudMoreCountText, HudPlayerEntryDot, HudPlayerEntryText,
    HudPlayersSummaryText, BOT_BUTTON_LEFT, BUTTON_BOTTOM, BUTTON_SIZE, HIT_TOP, INFO_BUTTON_LEFT,
    MAX_VISIBLE_PLAYERS, PANEL_BOTTOM, PANEL_LEFT, PANEL_WIDTH, PLAYERS_SUMMARY_TOP,
    PLAYER_LIST_TOP, PLAYER_ROW_SPACING, STATUS_CONNECTING, UI_DIM,
};

pub(super) fn spawn_hud(mut commands: Commands) {
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
