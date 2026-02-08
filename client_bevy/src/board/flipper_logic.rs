use super::geometry::FlipperSide;

pub const ROTATION_SPEED_UP: f32 = 14.0;
pub const ROTATION_SPEED_DOWN: f32 = 6.0;
pub const MAX_ANGLE: f32 = 0.45;

pub fn rest_angle(side: FlipperSide) -> f32 {
    match side {
        FlipperSide::Left => MAX_ANGLE,
        FlipperSide::Right => -MAX_ANGLE,
    }
}

pub fn active_angle(side: FlipperSide) -> f32 {
    match side {
        FlipperSide::Left => -MAX_ANGLE,
        FlipperSide::Right => MAX_ANGLE,
    }
}

pub fn step_flipper_angle(current_angle: f32, dt: f32, active: bool, side: FlipperSide) -> f32 {
    let target = if active {
        active_angle(side)
    } else {
        rest_angle(side)
    };
    let speed = if active {
        ROTATION_SPEED_UP
    } else {
        ROTATION_SPEED_DOWN
    };
    move_towards(current_angle, target, speed * dt)
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    let diff = target - current;
    if diff.abs() <= max_delta {
        target
    } else {
        current + diff.signum() * max_delta
    }
}
