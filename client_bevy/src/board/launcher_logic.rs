pub const MAX_CHARGE: f32 = 1.0;
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
