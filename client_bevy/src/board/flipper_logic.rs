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

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    mod left_flipper {
        use super::*;
        const SIDE: FlipperSide = FlipperSide::Left;

        #[test]
        fn rotates_towards_active_angle_when_active() {
            let rest = rest_angle(SIDE);
            let angle = step_flipper_angle(rest, DT, true, SIDE);
            assert!(angle < rest);
        }

        #[test]
        fn converges_to_active_angle_when_held() {
            let mut angle = rest_angle(SIDE);
            for _ in 0..200 {
                angle = step_flipper_angle(angle, DT, true, SIDE);
            }
            assert_eq!(angle, active_angle(SIDE));
        }

        #[test]
        fn converges_to_rest_angle_when_released() {
            let mut angle = active_angle(SIDE);
            for _ in 0..200 {
                angle = step_flipper_angle(angle, DT, false, SIDE);
            }
            assert_eq!(angle, rest_angle(SIDE));
        }

        #[test]
        fn does_not_overshoot_active_angle() {
            let mut angle = rest_angle(SIDE);
            for _ in 0..200 {
                angle = step_flipper_angle(angle, DT, true, SIDE);
                assert!(angle >= active_angle(SIDE));
            }
        }

        #[test]
        fn does_not_overshoot_rest_angle() {
            let mut angle = active_angle(SIDE);
            for _ in 0..200 {
                angle = step_flipper_angle(angle, DT, false, SIDE);
                assert!(angle <= rest_angle(SIDE));
            }
        }
    }

    mod right_flipper {
        use super::*;
        const SIDE: FlipperSide = FlipperSide::Right;

        #[test]
        fn rotates_towards_active_angle_when_active() {
            let rest = rest_angle(SIDE);
            let angle = step_flipper_angle(rest, DT, true, SIDE);
            assert!(angle > rest);
        }

        #[test]
        fn converges_to_active_angle_when_held() {
            let mut angle = rest_angle(SIDE);
            for _ in 0..200 {
                angle = step_flipper_angle(angle, DT, true, SIDE);
            }
            assert_eq!(angle, active_angle(SIDE));
        }

        #[test]
        fn converges_to_rest_angle_when_released() {
            let mut angle = active_angle(SIDE);
            for _ in 0..200 {
                angle = step_flipper_angle(angle, DT, false, SIDE);
            }
            assert_eq!(angle, rest_angle(SIDE));
        }

        #[test]
        fn does_not_overshoot_active_angle() {
            let mut angle = rest_angle(SIDE);
            for _ in 0..200 {
                angle = step_flipper_angle(angle, DT, true, SIDE);
                assert!(angle <= active_angle(SIDE));
            }
        }

        #[test]
        fn does_not_overshoot_rest_angle() {
            let mut angle = active_angle(SIDE);
            for _ in 0..200 {
                angle = step_flipper_angle(angle, DT, false, SIDE);
                assert!(angle >= rest_angle(SIDE));
            }
        }
    }

    mod symmetry {
        use super::*;

        #[test]
        fn active_angles_are_symmetric() {
            assert_eq!(active_angle(FlipperSide::Left), -MAX_ANGLE);
            assert_eq!(active_angle(FlipperSide::Right), MAX_ANGLE);
        }

        #[test]
        fn rest_angles_are_symmetric() {
            assert_eq!(rest_angle(FlipperSide::Left), MAX_ANGLE);
            assert_eq!(rest_angle(FlipperSide::Right), -MAX_ANGLE);
        }

        #[test]
        fn both_sides_reach_target_in_same_number_of_steps() {
            let mut left_angle = rest_angle(FlipperSide::Left);
            let mut right_angle = rest_angle(FlipperSide::Right);
            let mut left_steps = 0;
            let mut right_steps = 0;

            while left_angle != active_angle(FlipperSide::Left) && left_steps < 500 {
                left_angle = step_flipper_angle(left_angle, DT, true, FlipperSide::Left);
                left_steps += 1;
            }
            while right_angle != active_angle(FlipperSide::Right) && right_steps < 500 {
                right_angle = step_flipper_angle(right_angle, DT, true, FlipperSide::Right);
                right_steps += 1;
            }

            assert_eq!(left_steps, right_steps);
        }
    }
}
