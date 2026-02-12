use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::constants::color_from_hex;
use crate::game::network::NetworkState;
use crate::shared::net_state::NetState;
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
    state: Res<NetState>,
    net: Res<NetworkState>,
    mut queries: ConnectionUiQueries,
) {
    let color = connection_color(state.state.clone(), net.protocol_mismatch);
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

pub(super) fn update_players_ui(
    state: Res<NetState>,
    mut queries: PlayerUiQueries,
    mut last_signature: Local<Option<u64>>,
) {
    let mut signature = (state.self_id as u64).wrapping_mul(0x9e3779b185ebca87);
    for player in &state.players {
        signature = signature
            .wrapping_mul(0x9e3779b185ebca87)
            .wrapping_add(player.id as u64)
            .wrapping_add((player.color as u64) << 8)
            .wrapping_add((player.paused as u64) << 40)
            .wrapping_add((player.balls_in_flight as u64) << 20)
            .wrapping_add((player.balls_produced as u64) << 28);
    }

    if *last_signature == Some(signature) {
        return;
    }
    *last_signature = Some(signature);

    let mut sorted_players: Vec<&Player> = state.players.iter().collect();
    sorted_players.sort_by_key(|p| (p.id != state.self_id, p.id));

    let active_players = state.players.iter().filter(|p| !p.paused).count();
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
            *dot_border = BorderColor::all(if player.id == state.self_id {
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
            let self_mark = if player.id == state.self_id { "*" } else { " " };
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
    state: Res<NetState>,
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
    if !hud_ui.info_visible {
        return;
    }

    if let Ok(mut text) = info_texts.texts.p0().single_mut() {
        text.0 = format!("Client: v{}", env!("CARGO_PKG_VERSION"));
    }

    if let Ok(mut text) = info_texts.texts.p1().single_mut() {
        if state.server_version.is_empty() {
            text.0 = "Server: -".to_string();
        } else {
            text.0 = format!("Server: v{}", state.server_version);
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

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::game::network::NetworkState;
    use crate::shared::net_state::NetState;
    use crate::shared::types::{ConnectionState, Player};
    use crate::shared::vec3::Vec3;

    use super::*;

    fn assert_color_close(actual: Color, expected: Color) {
        let a = actual.to_srgba();
        let e = expected.to_srgba();
        let eps = 1e-4;
        assert!((a.red - e.red).abs() < eps, "red {} != {}", a.red, e.red);
        assert!(
            (a.green - e.green).abs() < eps,
            "green {} != {}",
            a.green,
            e.green
        );
        assert!(
            (a.blue - e.blue).abs() < eps,
            "blue {} != {}",
            a.blue,
            e.blue
        );
        assert!(
            (a.alpha - e.alpha).abs() < eps,
            "alpha {} != {}",
            a.alpha,
            e.alpha
        );
    }

    fn make_player(id: u32, paused: bool, in_flight: u32, produced: u32, color: u32) -> Player {
        Player {
            id,
            cell_index: id,
            portal_pos: Vec3::new(1.0, 0.0, 0.0),
            color,
            paused,
            balls_produced: produced,
            balls_in_flight: in_flight,
        }
    }

    fn make_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NetState>();
        app.insert_resource(NetworkState::default());
        app.init_resource::<HitCounter>();
        app.init_resource::<HudUiState>();
        app
    }

    #[test]
    fn connection_ui_uses_state_color() {
        let mut app = make_test_app();
        app.add_systems(Update, update_connection_ui);

        let glow = app
            .world_mut()
            .spawn((HudConnectionGlow, BackgroundColor(Color::NONE)))
            .id();
        let dot = app
            .world_mut()
            .spawn((
                HudConnectionDot,
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
            ))
            .id();

        {
            let mut conn = app.world_mut().resource_mut::<NetState>();
            conn.state = ConnectionState::Connected;
        }

        app.update();

        let expected = connection_color(ConnectionState::Connected, false);
        let glow_color = app.world().get::<BackgroundColor>(glow).unwrap().0;
        let dot_color = app.world().get::<BackgroundColor>(dot).unwrap().0;
        let dot_border = app.world().get::<BorderColor>(dot).unwrap();
        assert_color_close(glow_color, expected.with_alpha(0.45));
        assert_color_close(dot_color, expected.with_alpha(1.0));
        assert_color_close(dot_border.top, Color::srgba(1.0, 1.0, 1.0, 0.6));
        assert_color_close(dot_border.right, Color::srgba(1.0, 1.0, 1.0, 0.6));
        assert_color_close(dot_border.bottom, Color::srgba(1.0, 1.0, 1.0, 0.6));
        assert_color_close(dot_border.left, Color::srgba(1.0, 1.0, 1.0, 0.6));
    }

    #[test]
    fn connection_ui_protocol_mismatch_forces_disconnected_color() {
        let mut app = make_test_app();
        app.add_systems(Update, update_connection_ui);

        let dot = app
            .world_mut()
            .spawn((
                HudConnectionDot,
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
            ))
            .id();
        app.world_mut()
            .spawn((HudConnectionGlow, BackgroundColor(Color::NONE)));

        {
            let mut conn = app.world_mut().resource_mut::<NetState>();
            conn.state = ConnectionState::Connected;
        }
        {
            let mut net = app.world_mut().resource_mut::<NetworkState>();
            net.protocol_mismatch = true;
        }

        app.update();

        let expected = connection_color(ConnectionState::Connected, true);
        let dot_color = app.world().get::<BackgroundColor>(dot).unwrap().0;
        assert_color_close(dot_color, expected.with_alpha(1.0));
    }

    #[test]
    fn players_ui_formats_rows_and_self_marker() {
        let mut app = make_test_app();
        app.add_systems(Update, update_players_ui);

        let summary = app
            .world_mut()
            .spawn((HudPlayersSummaryText, Text::new(""), TextColor(Color::NONE)))
            .id();
        let more = app
            .world_mut()
            .spawn((HudMoreCountText, Text::new(""), Visibility::Hidden))
            .id();

        let dot0 = app
            .world_mut()
            .spawn((
                HudPlayerEntryDot { index: 0 },
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
                Visibility::Hidden,
            ))
            .id();
        let row0 = app
            .world_mut()
            .spawn((
                HudPlayerEntryText { index: 0 },
                Text::new(""),
                TextColor(Color::NONE),
                Visibility::Hidden,
            ))
            .id();
        let row1 = app
            .world_mut()
            .spawn((
                HudPlayerEntryText { index: 1 },
                Text::new(""),
                TextColor(Color::NONE),
                Visibility::Hidden,
            ))
            .id();
        let row2 = app
            .world_mut()
            .spawn((
                HudPlayerEntryText { index: 2 },
                Text::new(""),
                TextColor(Color::NONE),
                Visibility::Hidden,
            ))
            .id();

        {
            let mut conn = app.world_mut().resource_mut::<NetState>();
            conn.self_id = 2;
            conn.players = vec![
                make_player(5, false, 1, 8, 0x33ccaa),
                make_player(2, false, 3, 9, 0xe5f26d),
                make_player(1, true, 0, 4, 0xaa66ff),
            ];
        }

        app.update();

        assert_eq!(&app.world().get::<Text>(summary).unwrap().0, "2/3");
        assert_eq!(&app.world().get::<Text>(row0).unwrap().0, "*02 3/9");
        assert_eq!(&app.world().get::<Text>(row1).unwrap().0, " 01 0/4");
        assert_eq!(&app.world().get::<Text>(row2).unwrap().0, " 05 1/8");
        assert_eq!(
            *app.world().get::<Visibility>(row0).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<Visibility>(row1).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<Visibility>(row2).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<Visibility>(more).unwrap(),
            Visibility::Hidden
        );

        let self_border = app.world().get::<BorderColor>(dot0).unwrap();
        assert_color_close(self_border.top, Color::srgba(1.0, 1.0, 1.0, 0.9));
    }

    #[test]
    fn players_ui_shows_more_count_when_overflowing() {
        let mut app = make_test_app();
        app.add_systems(Update, update_players_ui);

        app.world_mut()
            .spawn((HudPlayersSummaryText, Text::new(""), TextColor(Color::NONE)));
        app.world_mut().spawn((
            HudPlayerEntryDot { index: 0 },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Visibility::Hidden,
        ));
        app.world_mut().spawn((
            HudPlayerEntryText { index: 0 },
            Text::new(""),
            TextColor(Color::NONE),
            Visibility::Hidden,
        ));
        let more = app
            .world_mut()
            .spawn((HudMoreCountText, Text::new(""), Visibility::Hidden))
            .id();

        {
            let mut conn = app.world_mut().resource_mut::<NetState>();
            conn.players = (1..=21)
                .map(|id| make_player(id, false, id, id + 10, 0x44ff44))
                .collect();
        }

        app.update();

        assert_eq!(&app.world().get::<Text>(more).unwrap().0, "... 21");
        assert_eq!(
            *app.world().get::<Visibility>(more).unwrap(),
            Visibility::Visible
        );
    }

    #[test]
    fn button_interactions_toggle_info_and_bot_state() {
        let mut app = make_test_app();
        app.add_systems(Update, handle_button_interactions);

        app.world_mut()
            .spawn((Button, Interaction::Pressed, HudInfoButton));
        app.world_mut()
            .spawn((Button, Interaction::Pressed, HudBotButton));

        app.update();

        let ui = app.world().resource::<HudUiState>();
        assert!(ui.info_visible);
        assert!(ui.bot_enabled);
    }

    #[test]
    fn info_panel_ui_shows_versions_and_bot_state() {
        let mut app = make_test_app();
        app.add_systems(Update, update_info_panel_ui);

        let panel = app
            .world_mut()
            .spawn((HudInfoPanel, Visibility::Hidden))
            .id();
        let client = app
            .world_mut()
            .spawn((HudInfoPanelClientText, Text::new("")))
            .id();
        let server = app
            .world_mut()
            .spawn((HudInfoPanelServerText, Text::new("")))
            .id();
        let bot = app
            .world_mut()
            .spawn((HudInfoPanelBotText, Text::new("")))
            .id();

        {
            let mut conn = app.world_mut().resource_mut::<NetState>();
            conn.server_version = "1.2.3".to_string();
        }
        {
            let mut ui = app.world_mut().resource_mut::<HudUiState>();
            ui.info_visible = true;
            ui.bot_enabled = true;
        }

        app.update();

        assert_eq!(
            *app.world().get::<Visibility>(panel).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            &app.world().get::<Text>(client).unwrap().0,
            &format!("Client: v{}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(
            &app.world().get::<Text>(server).unwrap().0,
            "Server: v1.2.3"
        );
        assert_eq!(&app.world().get::<Text>(bot).unwrap().0, "Bot: ON");
    }

    #[test]
    fn bot_button_ui_updates_border_and_text_alpha() {
        let mut app = make_test_app();
        app.add_systems(Update, update_bot_button_ui);

        let bot_button = app
            .world_mut()
            .spawn((HudBotButton, BorderColor::all(Color::NONE)))
            .id();
        let bot_text = app
            .world_mut()
            .spawn((HudBotButtonText, TextColor(Color::NONE)))
            .id();

        {
            let mut ui = app.world_mut().resource_mut::<HudUiState>();
            ui.bot_enabled = true;
        }
        app.update();

        let border_on = app.world().get::<BorderColor>(bot_button).unwrap();
        let text_on = app.world().get::<TextColor>(bot_text).unwrap().0;
        assert_color_close(border_on.top, panel_border(0.8));
        assert_color_close(text_on, panel_border(1.0));

        {
            let mut ui = app.world_mut().resource_mut::<HudUiState>();
            ui.bot_enabled = false;
        }
        app.update();

        let border_off = app.world().get::<BorderColor>(bot_button).unwrap();
        let text_off = app.world().get::<TextColor>(bot_text).unwrap().0;
        assert_color_close(border_off.top, panel_border(0.4));
        assert_color_close(text_off, panel_border(0.7));
    }
}
