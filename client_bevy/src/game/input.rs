use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_rapier2d::prelude::Velocity;

use crate::board::geometry::{flippers, launcher_wall};
use crate::constants::{
    bevy_vel_to_wire, world_to_px_x, world_to_px_y, BOARD_CENTER_X, BOARD_HALF_WIDTH,
    CANVAS_HEIGHT, CANVAS_WIDTH, PPM,
};
use crate::shared::connection::ServerConnection;

use super::ball::{Ball, BallState};
use super::client_bot::{BotBallInfo, ClientBot};
use super::hud::HudUiState;

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
        app.add_systems(PreUpdate, input_system);
    }
}

fn input_system(
    mut input: ResMut<InputState>,
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_balls: Query<(&Transform, &Velocity, &BallState), With<Ball>>,
    hud_ui: Option<Res<HudUiState>>,
    conn: Res<ServerConnection>,
    time: Res<Time>,
    mut bot: Local<ClientBot>,
    mut bot_ball_infos: Local<Vec<BotBallInfo>>,
    mut last_focus: Local<Option<bool>>,
) {
    let keyboard_left = keys.pressed(KeyCode::ArrowLeft);
    let keyboard_right = keys.pressed(KeyCode::ArrowRight);
    let keyboard_launch = keys.pressed(KeyCode::Space);
    let mut touch_left = false;
    let mut touch_right = false;
    let mut touch_launch = false;

    if let Ok(window) = q_window.single() {
        let focused = window.focused;
        if let Some(prev) = *last_focus {
            if focused != prev {
                conn.send_set_paused(!focused);
            }
        }
        *last_focus = Some(focused);

        let window_size = Vec2::new(window.width(), window.height());
        if window_size.x > 0.0 && window_size.y > 0.0 {
            for touch in touches.iter() {
                let game = screen_to_game_px(touch.position(), window_size);
                match input_zone(game.x, game.y) {
                    Zone::Left => touch_left = true,
                    Zone::Right => touch_right = true,
                    Zone::Launch => touch_launch = true,
                    Zone::None => {}
                }
            }
        }
    }

    let bot_enabled = hud_ui.as_ref().is_some_and(|ui| ui.bot_enabled);
    if bot_enabled {
        bot_ball_infos.clear();
        let launcher = launcher_wall();
        let lane_right_x = BOARD_CENTER_X + BOARD_HALF_WIDTH;
        for (transform, velocity, state) in &q_balls {
            let px = world_to_px_x(transform.translation.x);
            let py = world_to_px_y(transform.translation.y);
            let (vx, vy) = bevy_vel_to_wire(velocity.linvel);
            let in_shooter_lane = px >= launcher.from.x
                && px <= lane_right_x
                && py >= launcher.from.y
                && py <= launcher.to.y;
            bot_ball_infos.push(BotBallInfo {
                x: px / PPM,
                y: py / PPM,
                vx,
                vy,
                in_launcher: state.in_launcher,
                in_shooter_lane,
            });
        }

        let out = bot.update(time.delta_secs(), &bot_ball_infos);
        input.left = out.left_flipper;
        input.right = out.right_flipper;
        input.launch = out.launch;
    } else {
        bot.reset();
        input.left = keyboard_left || touch_left;
        input.right = keyboard_right || touch_right;
        input.launch = keyboard_launch || touch_launch;
    }

    if input.left || input.right || input.launch {
        input.last_activity_time = time.elapsed_secs_f64();
    }
}

#[derive(Clone, Copy)]
enum Zone {
    Left,
    Right,
    Launch,
    None,
}

fn input_zone(game_x: f32, game_y: f32) -> Zone {
    let [left_flipper, right_flipper] = flippers();
    let launcher_x = launcher_wall().from.x;
    let flipper_center_x = (left_flipper.pivot.x + right_flipper.pivot.x) * 0.5;
    let active_zone_top = left_flipper.pivot.y - 100.0;

    if game_y < active_zone_top || game_y > CANVAS_HEIGHT {
        return Zone::None;
    }
    if game_x >= launcher_x {
        return Zone::Launch;
    }
    if game_x < flipper_center_x {
        Zone::Left
    } else {
        Zone::Right
    }
}

fn screen_to_game_px(screen: Vec2, window_size: Vec2) -> Vec2 {
    let scale_x = CANVAS_WIDTH / window_size.x;
    let scale_y = CANVAS_HEIGHT / window_size.y;
    let cam_scale = scale_x.max(scale_y).max(0.0001);
    let visible_w = window_size.x * cam_scale;
    let visible_h = window_size.y * cam_scale;

    let world_x = (screen.x / window_size.x - 0.5) * visible_w;
    let world_y = (0.5 - screen.y / window_size.y) * visible_h;
    Vec2::new(world_to_px_x(world_x), world_to_px_y(world_y))
}
