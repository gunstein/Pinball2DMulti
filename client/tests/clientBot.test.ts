import { describe, it, expect } from "vitest";
import { ClientBot, BallInfo } from "../src/game/ClientBot";

const DT = 1 / 120;

function makeBall(overrides: Partial<BallInfo> = {}): BallInfo {
  return {
    x: 0.4,
    y: 0.5,
    vx: 0,
    vy: 0,
    inLauncher: false,
    inShooterLane: false,
    ...overrides,
  };
}

describe("ClientBot flipper logic", () => {
  it("does nothing when no balls", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, []);
    expect(cmd.leftFlipper).toBe(false);
    expect(cmd.rightFlipper).toBe(false);
    expect(cmd.launch).toBe(false);
  });

  it("does nothing when ball is above flip zone", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ y: 0.5, vy: 1 })]);
    expect(cmd.leftFlipper).toBe(false);
    expect(cmd.rightFlipper).toBe(false);
  });

  it("does nothing when ball is moving up", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ y: 1.1, vy: -1 })]);
    expect(cmd.leftFlipper).toBe(false);
    expect(cmd.rightFlipper).toBe(false);
  });

  it("flips left flipper when ball is in left flip zone moving down", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    expect(cmd.leftFlipper).toBe(true);
    expect(cmd.rightFlipper).toBe(false);
  });

  it("flips right flipper when ball is in right flip zone moving down", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ x: 0.5, y: 1.1, vy: 1 })]);
    expect(cmd.leftFlipper).toBe(false);
    expect(cmd.rightFlipper).toBe(true);
  });

  it("respects cooldown between flips", () => {
    const bot = new ClientBot();
    // First flip triggers
    const cmd1 = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    expect(cmd1.leftFlipper).toBe(true);

    // Immediately after, cooldown prevents re-trigger (hold still active though)
    const cmd2 = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    expect(cmd2.leftFlipper).toBe(true); // still holding from first flip

    // After hold expires but within cooldown
    for (let i = 0; i < 20; i++) bot.update(DT, []);
    const cmd3 = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    // Hold has expired, cooldown still active → no flip
    expect(cmd3.leftFlipper).toBe(false);

    // After cooldown expires (0.2s = 24 ticks at 120Hz)
    for (let i = 0; i < 10; i++) bot.update(DT, []);
    const cmd4 = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    expect(cmd4.leftFlipper).toBe(true);
  });

  it("ignores balls in launcher for flipper decisions", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [
      makeBall({ x: 0.2, y: 1.1, vy: 1, inLauncher: true }),
    ]);
    expect(cmd.leftFlipper).toBe(false);
  });

  it("picks the lowest ball in flip zone", () => {
    const bot = new ClientBot();
    // Two balls: one at y=1.05 on left, one at y=1.15 on right
    const cmd = bot.update(DT, [
      makeBall({ x: 0.2, y: 1.05, vy: 1 }),
      makeBall({ x: 0.5, y: 1.15, vy: 1 }),
    ]);
    // Should pick the lower ball (y=1.15) which is on the right
    expect(cmd.leftFlipper).toBe(false);
    expect(cmd.rightFlipper).toBe(true);
  });
});

describe("ClientBot launcher logic", () => {
  it("starts charging when ball is in launcher", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ inLauncher: true })]);
    expect(cmd.launch).toBe(true);
  });

  it("releases after charge duration", () => {
    const bot = new ClientBot();
    // Charge for enough ticks to exceed max charge time (0.7s = 84 ticks)
    let released = false;
    for (let i = 0; i < 120; i++) {
      const cmd = bot.update(DT, [makeBall({ inLauncher: true })]);
      if (!cmd.launch && i > 10) {
        released = true;
        break;
      }
    }
    expect(released).toBe(true);
  });

  it("has cooldown after launch", () => {
    const bot = new ClientBot();
    // Run until launch completes
    for (let i = 0; i < 120; i++) {
      bot.update(DT, [makeBall({ inLauncher: true })]);
    }
    // Immediately try again — should not charge due to cooldown
    const cmd = bot.update(DT, [makeBall({ inLauncher: true })]);
    expect(cmd.launch).toBe(false);
  });

  it("launches balls in shooter lane too", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ inShooterLane: true })]);
    expect(cmd.launch).toBe(true);
  });
});

describe("ClientBot reset", () => {
  it("clears all state on reset", () => {
    const bot = new ClientBot();
    // Trigger some state
    bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    bot.update(DT, [makeBall({ inLauncher: true })]);

    bot.reset();

    // After reset, no outputs without new stimuli
    const cmd = bot.update(DT, []);
    expect(cmd.leftFlipper).toBe(false);
    expect(cmd.rightFlipper).toBe(false);
    expect(cmd.launch).toBe(false);
  });
});
