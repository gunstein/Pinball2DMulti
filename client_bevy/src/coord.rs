use bevy::prelude::{Vec2, Vec3};

use crate::constants::{CANVAS_HEIGHT, CANVAS_WIDTH, PPM};

/// Pixel coordinates in TS/Pixi screen space (origin top-left, Y-down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PxPos {
    pub x: f32,
    pub y: f32,
}

impl PxPos {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Velocity in wire protocol space (meters/s, Y-down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WireVel {
    pub vx: f32,
    pub vy: f32,
}

impl WireVel {
    pub const fn new(vx: f32, vy: f32) -> Self {
        Self { vx, vy }
    }
}

/// Convert TS/Pixi pixel coordinates (Y-down) to Bevy world coordinates (Y-up).
pub fn px_to_world(px: PxPos, z: f32) -> Vec3 {
    let wx = px.x - CANVAS_WIDTH * 0.5;
    let wy = (CANVAS_HEIGHT - px.y) - CANVAS_HEIGHT * 0.5;
    Vec3::new(wx, wy, z)
}

/// Convert Bevy world coordinates (Y-up) to TS/Pixi pixel coordinates (Y-down).
pub fn world_to_px(world_xy: Vec2) -> PxPos {
    PxPos {
        x: world_xy.x + CANVAS_WIDTH * 0.5,
        y: CANVAS_HEIGHT * 0.5 - world_xy.y,
    }
}

/// Convert Bevy velocity (pixels/s, Y-up) to wire velocity (meters/s, Y-down).
pub fn bevy_vel_to_wire(v: Vec2) -> WireVel {
    WireVel {
        vx: v.x / PPM,
        vy: -v.y / PPM,
    }
}

/// Convert wire velocity (meters/s, Y-down) to Bevy velocity (pixels/s, Y-up).
pub fn wire_vel_to_bevy(v: WireVel) -> Vec2 {
    Vec2::new(v.vx * PPM, -v.vy * PPM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_world_roundtrip() {
        for (x, y) in [
            (0.0, 0.0),
            (CANVAS_WIDTH, 0.0),
            (0.0, CANVAS_HEIGHT),
            (CANVAS_WIDTH, CANVAS_HEIGHT),
            (CANVAS_WIDTH * 0.5, CANVAS_HEIGHT * 0.5),
        ] {
            let world = px_to_world(PxPos::new(x, y), 0.0);
            let roundtrip = world_to_px(world.truncate());
            assert!((roundtrip.x - x).abs() < 1e-6);
            assert!((roundtrip.y - y).abs() < 1e-6);
        }
    }

    #[test]
    fn wire_bevy_velocity_roundtrip() {
        let original = Vec2::new(350.0, 125.0);
        let wire = bevy_vel_to_wire(original);
        let roundtrip = wire_vel_to_bevy(wire);
        assert!((roundtrip.x - original.x).abs() < 1e-6);
        assert!((roundtrip.y - original.y).abs() < 1e-6);
    }

    #[test]
    fn negative_wire_vy_maps_to_upward_bevy() {
        let bevy = wire_vel_to_bevy(WireVel::new(1.2, -0.8));
        assert!((bevy.x - (1.2 * PPM)).abs() < 1e-6);
        assert!((bevy.y - (0.8 * PPM)).abs() < 1e-6);
    }
}
