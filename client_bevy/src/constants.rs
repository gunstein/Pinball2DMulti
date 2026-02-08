pub const CANVAS_WIDTH: f32 = 400.0;
pub const CANVAS_HEIGHT: f32 = 700.0;
pub const PPM: f32 = 500.0;

pub const GRAVITY_Y: f32 = 300.0;

pub const BOARD_HALF_WIDTH: f32 = 175.0;
pub const BOARD_HALF_HEIGHT: f32 = 320.0;

pub const BOARD_CENTER_X: f32 = CANVAS_WIDTH / 2.0;
pub const BOARD_CENTER_Y: f32 = CANVAS_HEIGHT / 2.0;

pub const BALL_RADIUS: f32 = 10.0;
pub const BALL_RESTITUTION: f32 = 0.5;

pub const RESPAWN_DELAY: f32 = 0.5;
pub const PHYSICS_DT: f32 = 1.0 / 120.0;

#[derive(Clone, Copy)]
pub struct Colors;

impl Colors {
    pub const DEEP_SPACE_BG: u32 = 0x050510;
    pub const WALL: u32 = 0x4da6a6;
    pub const FLIPPER: u32 = 0x4da6a6;
    pub const PIN: u32 = 0x4da6a6;
    pub const PIN_HIT: u32 = 0x44ff88;
    pub const BALL: u32 = 0x4da6a6;
    pub const BALL_GLOW: u32 = 0x88ccff;
    pub const STAR: u32 = 0xffffff;
}

pub fn color_from_hex(rgb: u32) -> bevy::prelude::Color {
    let r = ((rgb >> 16) & 0xff) as f32 / 255.0;
    let g = ((rgb >> 8) & 0xff) as f32 / 255.0;
    let b = (rgb & 0xff) as f32 / 255.0;
    bevy::prelude::Color::srgb(r, g, b)
}

pub fn px_to_world(x: f32, y: f32, z: f32) -> bevy::prelude::Vec3 {
    // TS/Pixi uses top-left origin with +Y down.
    // Bevy 2D camera is centered at (0,0) with +Y up, so shift by half extents.
    let wx = x - CANVAS_WIDTH * 0.5;
    let wy = (CANVAS_HEIGHT - y) - CANVAS_HEIGHT * 0.5;
    bevy::prelude::Vec3::new(wx, wy, z)
}
