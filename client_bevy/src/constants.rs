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

/// Convert TS/Pixi pixel coords (top-left origin, Y-down) to Bevy world coords (center origin, Y-up).
pub fn px_to_world(x: f32, y: f32, z: f32) -> bevy::prelude::Vec3 {
    let wx = x - CANVAS_WIDTH * 0.5;
    let wy = (CANVAS_HEIGHT - y) - CANVAS_HEIGHT * 0.5;
    bevy::prelude::Vec3::new(wx, wy, z)
}

/// Convert Bevy world X back to TS/Pixi pixel X.
pub fn world_to_px_x(wx: f32) -> f32 {
    wx + CANVAS_WIDTH * 0.5
}

/// Convert Bevy world Y back to TS/Pixi pixel Y.
pub fn world_to_px_y(wy: f32) -> f32 {
    CANVAS_HEIGHT * 0.5 - wy
}

/// Convert Bevy velocity (pixel-space, Y-up) to wire protocol velocity (meter-space, Y-down).
pub fn bevy_vel_to_wire(v: bevy::prelude::Vec2) -> (f32, f32) {
    (v.x / PPM, -v.y / PPM)
}

/// Convert wire protocol velocity (meter-space, Y-down) to Bevy velocity (pixel-space, Y-up).
pub fn wire_vel_to_bevy(vx: f32, vy: f32) -> bevy::prelude::Vec2 {
    bevy::prelude::Vec2::new(vx * PPM, -vy * PPM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_to_world_roundtrip() {
        let px_x = 150.0_f32;
        let px_y = 300.0_f32;
        let w = px_to_world(px_x, px_y, 0.0);
        let back_x = world_to_px_x(w.x);
        let back_y = world_to_px_y(w.y);
        assert!((back_x - px_x).abs() < 1e-6);
        assert!((back_y - px_y).abs() < 1e-6);
    }

    #[test]
    fn px_to_world_origin_maps_to_center() {
        let w = px_to_world(CANVAS_WIDTH / 2.0, CANVAS_HEIGHT / 2.0, 0.0);
        assert!((w.x).abs() < 1e-6);
        assert!((w.y).abs() < 1e-6);
    }

    #[test]
    fn px_to_world_top_left_corner() {
        let w = px_to_world(0.0, 0.0, 0.0);
        assert!((w.x - (-CANVAS_WIDTH / 2.0)).abs() < 1e-6);
        assert!((w.y - (CANVAS_HEIGHT / 2.0)).abs() < 1e-6);
    }

    #[test]
    fn px_to_world_bottom_right_corner() {
        let w = px_to_world(CANVAS_WIDTH, CANVAS_HEIGHT, 0.0);
        assert!((w.x - (CANVAS_WIDTH / 2.0)).abs() < 1e-6);
        assert!((w.y - (-CANVAS_HEIGHT / 2.0)).abs() < 1e-6);
    }

    #[test]
    fn roundtrip_at_corners() {
        for (px_x, px_y) in [
            (0.0, 0.0),
            (CANVAS_WIDTH, 0.0),
            (0.0, CANVAS_HEIGHT),
            (CANVAS_WIDTH, CANVAS_HEIGHT),
            (BOARD_CENTER_X, BOARD_CENTER_Y),
        ] {
            let w = px_to_world(px_x, px_y, 0.0);
            let bx = world_to_px_x(w.x);
            let by = world_to_px_y(w.y);
            assert!(
                (bx - px_x).abs() < 1e-6,
                "x roundtrip failed for ({}, {})",
                px_x,
                px_y
            );
            assert!(
                (by - px_y).abs() < 1e-6,
                "y roundtrip failed for ({}, {})",
                px_x,
                px_y
            );
        }
    }

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

    #[test]
    fn wire_velocity_negative_y_maps_to_upward_bevy() {
        let bevy = wire_vel_to_bevy(1.2, -0.8);
        assert!((bevy.x - (1.2 * PPM)).abs() < 1e-6);
        assert!((bevy.y - (0.8 * PPM)).abs() < 1e-6);
    }

    #[test]
    fn bevy_wire_velocity_roundtrip() {
        let original = bevy::prelude::Vec2::new(350.0, 125.0);
        let (vx, vy) = bevy_vel_to_wire(original);
        let roundtrip = wire_vel_to_bevy(vx, vy);
        assert!((roundtrip.x - original.x).abs() < 1e-6);
        assert!((roundtrip.y - original.y).abs() < 1e-6);
    }
}
