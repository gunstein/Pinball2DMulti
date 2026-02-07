/**
 * Client-side bot that takes over flipper and launcher control.
 *
 * Pure logic — no PixiJS, DOM, or physics engine dependencies.
 * Game.ts feeds ball state in, gets flipper/launcher commands out.
 *
 * Coordinates: physics meters (PPM=500). Y-axis positive = down.
 * Flipper Y ≈ 1.16m, board center X ≈ 0.4m.
 */

/** Minimal ball state needed by the bot. */
export interface BallInfo {
  x: number;
  y: number;
  vx: number;
  vy: number;
  inLauncher: boolean;
  inShooterLane: boolean;
}

/** Commands returned by the bot each tick. */
export interface BotOutput {
  leftFlipper: boolean;
  rightFlipper: boolean;
  launch: boolean;
}

// Flip zone: only react when ball is below this Y (in meters)
const FLIP_ZONE_Y = 1.0;
// Board center X in meters (left half → left flipper, right half → right)
const CENTER_X = 0.4;
// Minimum time between flips per flipper (seconds)
const FLIP_COOLDOWN = 0.2;
// How long to hold a flip (seconds)
const FLIP_HOLD = 0.15;

// Launcher: how long to charge before releasing (seconds)
const LAUNCH_CHARGE_MIN = 0.3;
const LAUNCH_CHARGE_MAX = 0.7;
// Cooldown after a launch before launching again
const LAUNCH_COOLDOWN = 1.0;

export class ClientBot {
  private leftHold = 0;
  private rightHold = 0;
  private leftCooldown = 0;
  private rightCooldown = 0;

  private launchTarget = 0; // how long to charge this launch
  private launchHeld = 0; // how long we've been holding
  private launching = false;
  private launchCooldown = 0;

  /** Seed for simple deterministic pseudo-random variation. */
  private seed = 1;

  private nextRandom(): number {
    // Simple LCG for light variation (no crypto needed)
    this.seed = (this.seed * 1664525 + 1013904223) & 0x7fffffff;
    return this.seed / 0x7fffffff;
  }

  update(dt: number, balls: BallInfo[]): BotOutput {
    // Decrement timers
    this.leftHold = Math.max(0, this.leftHold - dt);
    this.rightHold = Math.max(0, this.rightHold - dt);
    this.leftCooldown = Math.max(0, this.leftCooldown - dt);
    this.rightCooldown = Math.max(0, this.rightCooldown - dt);
    this.launchCooldown = Math.max(0, this.launchCooldown - dt);

    // --- Flipper logic ---
    // Find the lowest active ball (highest y) that's in the flip zone and moving down
    let flipBall: BallInfo | null = null;
    for (const b of balls) {
      if (b.inLauncher || b.inShooterLane) continue;
      if (b.y < FLIP_ZONE_Y) continue;
      if (b.vy <= 0) continue; // moving up, ignore
      if (!flipBall || b.y > flipBall.y) {
        flipBall = b;
      }
    }

    if (flipBall) {
      if (flipBall.x < CENTER_X && this.leftCooldown <= 0) {
        this.leftHold = FLIP_HOLD;
        this.leftCooldown = FLIP_COOLDOWN;
      }
      if (flipBall.x >= CENTER_X && this.rightCooldown <= 0) {
        this.rightHold = FLIP_HOLD;
        this.rightCooldown = FLIP_COOLDOWN;
      }
    }

    // --- Launcher logic ---
    const hasLaunchBall = balls.some((b) => b.inLauncher || b.inShooterLane);
    let launch = false;

    if (hasLaunchBall && this.launchCooldown <= 0) {
      if (!this.launching) {
        // Start a new launch: pick a random charge duration
        this.launching = true;
        this.launchHeld = 0;
        this.launchTarget =
          LAUNCH_CHARGE_MIN +
          this.nextRandom() * (LAUNCH_CHARGE_MAX - LAUNCH_CHARGE_MIN);
      }

      this.launchHeld += dt;

      if (this.launchHeld < this.launchTarget) {
        // Still charging: hold the button
        launch = true;
      } else {
        // Release: launcher fires on this edge (true→false handled by next tick)
        launch = false;
        this.launching = false;
        this.launchCooldown = LAUNCH_COOLDOWN;
      }
    } else if (this.launching && !hasLaunchBall) {
      // Ball left the launcher area (was launched or escaped)
      this.launching = false;
    }

    return {
      leftFlipper: this.leftHold > 0,
      rightFlipper: this.rightHold > 0,
      launch,
    };
  }

  /** Reset all state (called when bot is toggled off). */
  reset(): void {
    this.leftHold = 0;
    this.rightHold = 0;
    this.leftCooldown = 0;
    this.rightCooldown = 0;
    this.launchTarget = 0;
    this.launchHeld = 0;
    this.launching = false;
    this.launchCooldown = 0;
  }
}
