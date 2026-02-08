pub const MAX_CHARGE: f32 = 1.0;
/// Launch speed in normalized units (0..MAX_LAUNCH_SPEED).
/// Scaled to pixel-space by PPM when applied as impulse.
pub const MAX_LAUNCH_SPEED: f32 = 1.8;
pub const COOLDOWN: f32 = 0.3;

#[derive(Clone, Copy)]
pub struct LauncherState {
    pub charge: f32,
    pub cooldown: f32,
    pub was_pressed: bool,
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            charge: 0.0,
            cooldown: 0.0,
            was_pressed: false,
        }
    }
}

pub fn step_launcher(
    mut state: LauncherState,
    dt: f32,
    active: bool,
) -> (LauncherState, Option<f32>) {
    if state.cooldown > 0.0 {
        state.cooldown -= dt;
        return (state, None);
    }

    if active {
        state.charge = (state.charge + dt).min(MAX_CHARGE);
        state.was_pressed = true;
        return (state, None);
    }

    if state.was_pressed {
        let fired = (state.charge / MAX_CHARGE) * MAX_LAUNCH_SPEED;
        state.charge = 0.0;
        state.cooldown = COOLDOWN;
        state.was_pressed = false;
        (state, Some(fired))
    } else {
        (state, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    #[test]
    fn does_not_fire_when_not_pressed() {
        let (state, fired) = step_launcher(LauncherState::default(), DT, false);
        assert!(fired.is_none());
        assert_eq!(state.charge, 0.0);
    }

    #[test]
    fn charges_while_pressed() {
        let mut state = LauncherState::default();
        for _ in 0..10 {
            (state, _) = step_launcher(state, DT, true);
        }
        assert!((state.charge - 10.0 * DT).abs() < 1e-6);
        assert!(state.was_pressed);
    }

    #[test]
    fn charge_caps_at_max() {
        let mut state = LauncherState::default();
        for _ in 0..240 {
            (state, _) = step_launcher(state, DT, true);
        }
        assert_eq!(state.charge, MAX_CHARGE);
    }

    #[test]
    fn fires_on_release_with_correct_speed() {
        let mut state = LauncherState::default();
        let steps = (MAX_CHARGE / DT).round() as usize;
        for _ in 0..steps {
            (state, _) = step_launcher(state, DT, true);
        }
        let (after, fired) = step_launcher(state, DT, false);
        assert!((fired.unwrap() - MAX_LAUNCH_SPEED).abs() < 1e-4);
        assert_eq!(after.charge, 0.0);
        assert_eq!(after.cooldown, COOLDOWN);
        assert!(!after.was_pressed);
    }

    #[test]
    fn fires_with_partial_power_on_early_release() {
        let mut state = LauncherState::default();
        let steps = (0.5_f32 / DT).round() as usize;
        for _ in 0..steps {
            (state, _) = step_launcher(state, DT, true);
        }
        let (_, fired) = step_launcher(state, DT, false);
        assert!((fired.unwrap() - MAX_LAUNCH_SPEED * 0.5).abs() < 0.1);
    }

    #[test]
    fn cannot_charge_during_cooldown() {
        let mut state = LauncherState::default();
        (state, _) = step_launcher(state, DT, true);
        (state, _) = step_launcher(state, DT, false);
        assert!(state.cooldown > 0.0);

        let (during_cooldown, _) = step_launcher(state, DT, true);
        assert_eq!(during_cooldown.charge, 0.0);
        assert!(!during_cooldown.was_pressed);
    }

    #[test]
    fn cooldown_expires_and_allows_new_charge() {
        let mut state = LauncherState::default();
        (state, _) = step_launcher(state, DT, true);
        (state, _) = step_launcher(state, DT, false);

        let cooldown_steps = (COOLDOWN / DT).ceil() as usize + 1;
        for _ in 0..cooldown_steps {
            (state, _) = step_launcher(state, DT, false);
        }
        assert!(state.cooldown <= 0.0);

        (state, _) = step_launcher(state, DT, true);
        assert!(state.charge > 0.0);
        assert!(state.was_pressed);
    }
}
