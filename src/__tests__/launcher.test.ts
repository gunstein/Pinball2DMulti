import { describe, it, expect } from "vitest";

// Extract the pure launcher state machine for testing (no Pixi needed)
const MAX_CHARGE = 1.0;
const MAX_LAUNCH_SPEED = 1.8;
const COOLDOWN = 0.3;

interface LauncherState {
  charge: number;
  cooldown: number;
  wasPressed: boolean;
}

function stepLauncher(
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

function initialState(): LauncherState {
  return { charge: 0, cooldown: 0, wasPressed: false };
}

describe("Launcher state machine", () => {
  const dt = 1 / 120;

  it("does not fire when not pressed", () => {
    const { state, fired } = stepLauncher(initialState(), dt, false);
    expect(fired).toBeNull();
    expect(state.charge).toBe(0);
  });

  it("charges while pressed", () => {
    let state = initialState();
    for (let i = 0; i < 10; i++) {
      ({ state } = stepLauncher(state, dt, true));
    }
    expect(state.charge).toBeCloseTo(10 * dt, 10);
    expect(state.wasPressed).toBe(true);
  });

  it("charge caps at MAX_CHARGE", () => {
    let state = initialState();
    // Hold for 2 seconds (much more than MAX_CHARGE=1.0)
    for (let i = 0; i < 240; i++) {
      ({ state } = stepLauncher(state, dt, true));
    }
    expect(state.charge).toBe(MAX_CHARGE);
  });

  it("fires on release with correct speed", () => {
    let state = initialState();
    // Charge for exactly MAX_CHARGE seconds
    const steps = Math.round(MAX_CHARGE / dt);
    for (let i = 0; i < steps; i++) {
      ({ state } = stepLauncher(state, dt, true));
    }
    // Release
    const { state: afterRelease, fired } = stepLauncher(state, dt, false);
    expect(fired).toBeCloseTo(MAX_LAUNCH_SPEED, 5);
    expect(afterRelease.charge).toBe(0);
    expect(afterRelease.cooldown).toBe(COOLDOWN);
    expect(afterRelease.wasPressed).toBe(false);
  });

  it("fires with partial power on early release", () => {
    let state = initialState();
    // Charge for 0.5s (half)
    const steps = Math.round(0.5 / dt);
    for (let i = 0; i < steps; i++) {
      ({ state } = stepLauncher(state, dt, true));
    }
    const { fired } = stepLauncher(state, dt, false);
    expect(fired).toBeCloseTo(MAX_LAUNCH_SPEED * 0.5, 1);
  });

  it("cannot charge during cooldown", () => {
    let state = initialState();
    // Charge and release
    state = stepLauncher(state, dt, true).state;
    state = stepLauncher(state, dt, false).state;
    expect(state.cooldown).toBeGreaterThan(0);

    // Try to charge during cooldown
    const { state: duringCooldown } = stepLauncher(state, dt, true);
    expect(duringCooldown.charge).toBe(0);
    expect(duringCooldown.wasPressed).toBe(false);
  });

  it("cooldown expires and allows new charge", () => {
    let state = initialState();
    // Charge and release
    state = stepLauncher(state, dt, true).state;
    state = stepLauncher(state, dt, false).state;

    // Wait out cooldown
    const cooldownSteps = Math.ceil(COOLDOWN / dt) + 1;
    for (let i = 0; i < cooldownSteps; i++) {
      ({ state } = stepLauncher(state, dt, false));
    }
    expect(state.cooldown).toBeLessThanOrEqual(0);

    // Should be able to charge again
    ({ state } = stepLauncher(state, dt, true));
    expect(state.charge).toBeGreaterThan(0);
    expect(state.wasPressed).toBe(true);
  });
});
