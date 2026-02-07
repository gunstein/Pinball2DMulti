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

  it("does nothing when ball is too slow (sitting in cradle)", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vx: 0, vy: 0 })]);
    expect(cmd.leftFlipper).toBe(false);
    expect(cmd.rightFlipper).toBe(false);
  });

  it("flips when ball is in zone moving up (enough speed)", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: -1 })]);
    expect(cmd.leftFlipper).toBe(true);
  });

  it("flips left flipper when ball is in left flip zone", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    expect(cmd.leftFlipper).toBe(true);
    expect(cmd.rightFlipper).toBe(false);
  });

  it("flips right flipper when ball is in right flip zone", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ x: 0.55, y: 1.1, vy: 1 })]);
    expect(cmd.leftFlipper).toBe(false);
    expect(cmd.rightFlipper).toBe(true);
  });

  it("flips both flippers when ball is near center", () => {
    const bot = new ClientBot();
    const cmd = bot.update(DT, [makeBall({ x: 0.4, y: 1.1, vy: 1 })]);
    expect(cmd.leftFlipper).toBe(true);
    expect(cmd.rightFlipper).toBe(true);
  });

  it("respects cooldown between flips", () => {
    const bot = new ClientBot();
    // First flip triggers
    const cmd1 = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    expect(cmd1.leftFlipper).toBe(true);

    // Immediately after, hold still active
    const cmd2 = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    expect(cmd2.leftFlipper).toBe(true);

    // After hold expires (0.2s = 24 ticks) but within cooldown (0.6s = 72 ticks)
    for (let i = 0; i < 30; i++) bot.update(DT, []);
    const cmd3 = bot.update(DT, [makeBall({ x: 0.2, y: 1.1, vy: 1 })]);
    expect(cmd3.leftFlipper).toBe(false);

    // After cooldown expires (need ~72 ticks total, we've done ~32)
    for (let i = 0; i < 45; i++) bot.update(DT, []);
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
    // Two balls: one at y=0.95 on left, one at y=1.15 on right — both moving
    const cmd = bot.update(DT, [
      makeBall({ x: 0.2, y: 0.95, vy: 1 }),
      makeBall({ x: 0.55, y: 1.15, vy: 1 }),
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
    // Charge for enough ticks to exceed max charge time (1.0s = 120 ticks)
    let released = false;
    for (let i = 0; i < 150; i++) {
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
    for (let i = 0; i < 150; i++) {
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
