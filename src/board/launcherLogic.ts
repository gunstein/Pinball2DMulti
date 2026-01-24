export const MAX_CHARGE = 1.0; // seconds to full charge
export const MAX_LAUNCH_SPEED = 1.8; // m/s (ball velocity at full charge)
export const COOLDOWN = 0.3; // seconds after launch before you can charge again

export interface LauncherState {
  charge: number;
  cooldown: number;
  wasPressed: boolean;
}

export function initialLauncherState(): LauncherState {
  return { charge: 0, cooldown: 0, wasPressed: false };
}

export function stepLauncher(
  state: LauncherState,
  dt: number,
  active: boolean,
): { state: LauncherState; fired: number | null } {
  const s = { ...state };
  let fired: number | null = null;

  if (s.cooldown > 0) {
    s.cooldown -= dt;
    return { state: s, fired };
  }

  if (active) {
    s.charge = Math.min(MAX_CHARGE, s.charge + dt);
    s.wasPressed = true;
  } else if (s.wasPressed) {
    fired = (s.charge / MAX_CHARGE) * MAX_LAUNCH_SPEED;
    s.charge = 0;
    s.cooldown = COOLDOWN;
    s.wasPressed = false;
  }

  return { state: s, fired };
}
