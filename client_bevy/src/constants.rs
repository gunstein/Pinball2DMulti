pub const CANVAS_WIDTH: f32 = 400.0;
pub const CANVAS_HEIGHT: f32 = 700.0;

/// Rapier pixels_per_meter scaling factor. Rapier divides internally by this
/// so we can work in pixel coordinates everywhere.
pub const PPM: f32 = 500.0;

/// Gravity in pixel-space (Y-down in TS coords, but we negate in core.rs for Bevy Y-up).
pub const GRAVITY_Y: f32 = -300.0;

pub const BOARD_HALF_WIDTH: f32 = 175.0;
pub const BOARD_HALF_HEIGHT: f32 = 320.0;

pub const BOARD_CENTER_X: f32 = CANVAS_WIDTH / 2.0;
pub const BOARD_CENTER_Y: f32 = CANVAS_HEIGHT / 2.0;

pub const BALL_RADIUS: f32 = 10.0;
pub const BALL_RESTITUTION: f32 = 0.5;
pub const BALL_FILL_ALPHA: f32 = 0.08;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_from_hex_parses_correctly() {
        let c = color_from_hex(0xFF8040);
        // Color::srgb returns Srgba, check the components
        if let bevy::prelude::Color::Srgba(srgba) = c {
            assert!((srgba.red - 1.0).abs() < 1e-3);
            assert!((srgba.green - 0.502).abs() < 1e-2);
            assert!((srgba.blue - 0.251).abs() < 1e-2);
        } else {
            panic!("Expected Srgba color variant");
        }
    }
}
