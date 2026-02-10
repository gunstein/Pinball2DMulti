/// Client-side bot that takes over flipper and launcher control.
/// Mirrors the TypeScript bot behavior to keep gameplay feel consistent.

#[derive(Clone, Copy, Debug)]
pub(crate) struct BotBallInfo {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) vx: f32,
    pub(crate) vy: f32,
    pub(crate) in_launcher: bool,
    pub(crate) in_shooter_lane: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BotOutput {
    pub(crate) left_flipper: bool,
    pub(crate) right_flipper: bool,
    pub(crate) launch: bool,
}

const FLIP_ZONE_Y: f32 = 0.85;
const CENTER_X: f32 = 0.4;
const FLIP_COOLDOWN: f32 = 0.6;
const FLIP_HOLD: f32 = 0.2;
const MIN_BALL_SPEED: f32 = 0.3;

const LAUNCH_CHARGE_MIN: f32 = 0.7;
const LAUNCH_CHARGE_MAX: f32 = 1.0;
const LAUNCH_COOLDOWN: f32 = 1.5;

#[derive(Debug)]
pub(crate) struct ClientBot {
    left_hold: f32,
    right_hold: f32,
    left_cooldown: f32,
    right_cooldown: f32,
    launch_target: f32,
    launch_held: f32,
    launching: bool,
    launch_cooldown: f32,
    seed: u32,
}

impl Default for ClientBot {
    fn default() -> Self {
        Self {
            left_hold: 0.0,
            right_hold: 0.0,
            left_cooldown: 0.0,
            right_cooldown: 0.0,
            launch_target: 0.0,
            launch_held: 0.0,
            launching: false,
            launch_cooldown: 0.0,
            seed: 1,
        }
    }
}

impl ClientBot {
    fn next_random(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223) & 0x7fff_ffff;
        self.seed as f32 / 0x7fff_ffffu32 as f32
    }

    pub(crate) fn update(&mut self, dt: f32, balls: &[BotBallInfo]) -> BotOutput {
        self.left_hold = (self.left_hold - dt).max(0.0);
        self.right_hold = (self.right_hold - dt).max(0.0);
        self.left_cooldown = (self.left_cooldown - dt).max(0.0);
        self.right_cooldown = (self.right_cooldown - dt).max(0.0);
        self.launch_cooldown = (self.launch_cooldown - dt).max(0.0);

        let mut flip_ball: Option<&BotBallInfo> = None;
        for ball in balls {
            if ball.in_launcher || ball.in_shooter_lane {
                continue;
            }
            if ball.y < FLIP_ZONE_Y {
                continue;
            }
            let speed = (ball.vx * ball.vx + ball.vy * ball.vy).sqrt();
            if speed < MIN_BALL_SPEED {
                continue;
            }
            if flip_ball.is_none_or(|current| ball.y > current.y) {
                flip_ball = Some(ball);
            }
        }

        if let Some(ball) = flip_ball {
            let left_side = ball.x < CENTER_X - 0.05;
            let right_side = ball.x > CENTER_X + 0.05;
            if !left_side && !right_side {
                if self.left_cooldown <= 0.0 {
                    self.left_hold = FLIP_HOLD;
                    self.left_cooldown = FLIP_COOLDOWN;
                }
                if self.right_cooldown <= 0.0 {
                    self.right_hold = FLIP_HOLD;
                    self.right_cooldown = FLIP_COOLDOWN;
                }
            } else if left_side && self.left_cooldown <= 0.0 {
                self.left_hold = FLIP_HOLD;
                self.left_cooldown = FLIP_COOLDOWN;
            } else if right_side && self.right_cooldown <= 0.0 {
                self.right_hold = FLIP_HOLD;
                self.right_cooldown = FLIP_COOLDOWN;
            }
        }

        let has_launch_ball = balls.iter().any(|b| b.in_launcher || b.in_shooter_lane);
        let mut launch = false;
        if has_launch_ball && self.launch_cooldown <= 0.0 {
            if !self.launching {
                self.launching = true;
                self.launch_held = 0.0;
                self.launch_target = LAUNCH_CHARGE_MIN
                    + self.next_random() * (LAUNCH_CHARGE_MAX - LAUNCH_CHARGE_MIN);
            }

            self.launch_held += dt;
            if self.launch_held < self.launch_target {
                launch = true;
            } else {
                launch = false;
                self.launching = false;
                self.launch_cooldown = LAUNCH_COOLDOWN;
            }
        } else if self.launching && !has_launch_ball {
            self.launching = false;
        }

        BotOutput {
            left_flipper: self.left_hold > 0.0,
            right_flipper: self.right_hold > 0.0,
            launch,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.left_hold = 0.0;
        self.right_hold = 0.0;
        self.left_cooldown = 0.0;
        self.right_cooldown = 0.0;
        self.launch_target = 0.0;
        self.launch_held = 0.0;
        self.launching = false;
        self.launch_cooldown = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_flips_for_ball_in_zone() {
        let mut bot = ClientBot::default();
        let out = bot.update(
            1.0 / 60.0,
            &[BotBallInfo {
                x: 0.2,
                y: 1.0,
                vx: 0.5,
                vy: 0.5,
                in_launcher: false,
                in_shooter_lane: false,
            }],
        );
        assert!(out.left_flipper || out.right_flipper);
    }

    #[test]
    fn bot_holds_launch_while_charging() {
        let mut bot = ClientBot::default();
        let balls = [BotBallInfo {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            in_launcher: true,
            in_shooter_lane: true,
        }];
        let out = bot.update(1.0 / 60.0, &balls);
        assert!(out.launch);
    }
}
