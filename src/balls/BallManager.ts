/**
 * BallManager - Manages ball lifecycle: spawning, pooling, escape/capture.
 * Single source of truth for ball-related logic.
 */

import { Container } from "pixi.js";
import { Ball } from "../board/Ball";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { DeepSpaceBackend, CaptureEvent } from "../deepSpace/DeepSpaceBackend";

const RESPAWN_DELAY = 0.5; // seconds

/**
 * Manages all balls in the game: spawning, pooling, escape detection, and capture handling.
 */
export class BallManager {
  private container: Container;
  private physics: PhysicsWorld;
  private backend: DeepSpaceBackend;

  // Active balls on the board
  private balls: Ball[] = [];
  // Pool of inactive balls for reuse
  private inactiveBalls: Ball[] = [];
  // Ball currently in the launcher (waiting to be launched)
  private launcherBall: Ball | null = null;
  // Map from collider handle to ball for collision detection
  private ballByHandle: Map<number, Ball> = new Map();

  // Current ball color (from player)
  private ballColor: number = 0xffffff;

  // Respawn timer
  private respawnTimer = 0;

  // Cleanup function for capture subscription
  private unsubscribeCapture: (() => void) | null = null;

  constructor(
    container: Container,
    physics: PhysicsWorld,
    backend: DeepSpaceBackend,
  ) {
    this.container = container;
    this.physics = physics;
    this.backend = backend;

    // Subscribe to capture events from deep space
    this.unsubscribeCapture = this.backend.onCapture((event) => {
      this.handleCapture(event);
    });
  }

  /**
   * Set the ball color (called when player info is received).
   */
  setBallColor(color: number): void {
    this.ballColor = color;
    // Update existing balls
    for (const ball of this.balls) {
      ball.setTint(color);
    }
  }

  /**
   * Get the ball in the launcher, if any.
   */
  getLauncherBall(): Ball | null {
    return this.launcherBall;
  }

  /**
   * Get all active balls.
   */
  getBalls(): readonly Ball[] {
    return this.balls;
  }

  /**
   * Get ball by collider handle (for collision detection).
   */
  getBallByHandle(handle: number): Ball | undefined {
    return this.ballByHandle.get(handle);
  }

  /**
   * Check if there are any active balls.
   */
  hasBalls(): boolean {
    return this.balls.length > 0;
  }

  /**
   * Spawn the initial ball in the launcher.
   */
  spawnInitialBall(): void {
    this.spawnBallInLauncher();
  }

  /**
   * Update respawn timer and check for escapes.
   * Called every frame.
   */
  tick(dt: number): void {
    // Respawn timer
    if (this.respawnTimer > 0) {
      this.respawnTimer -= dt;
      if (this.respawnTimer <= 0 && !this.launcherBall && this.balls.length === 0) {
        this.spawnBallInLauncher();
      }
    }
  }

  /**
   * Run fixed update for all balls.
   * Called at fixed physics rate.
   */
  fixedUpdate(): void {
    for (const ball of this.balls) {
      ball.fixedUpdate();
      // Check if ball returned to launcher
      if (!this.launcherBall && ball.isInLauncher()) {
        this.launcherBall = ball;
      }
    }
  }

  /**
   * Check for balls escaping through the escape slot.
   * Called after physics step.
   */
  checkEscapes(): void {
    // Iterate backwards to safely remove during iteration
    for (let i = this.balls.length - 1; i >= 0; i--) {
      const ball = this.balls[i];
      if (!ball.isActive()) continue;

      const snapshot = ball.getEscapeSnapshot();
      if (snapshot) {
        // Send to backend (server or local simulation)
        this.backend.ballEscaped(snapshot.vx, snapshot.vy);
        this.removeBall(ball);
      }
    }

    // Start respawn timer if no balls left
    if (this.balls.length === 0 && !this.launcherBall && this.respawnTimer <= 0) {
      this.respawnTimer = RESPAWN_DELAY;
    }
  }

  /**
   * Render all balls.
   */
  render(): void {
    for (const ball of this.balls) {
      ball.render();
    }
  }

  /**
   * Handle ball drain (fell off the board).
   */
  handleDrain(ball: Ball): void {
    if (ball.isActive()) {
      this.removeBall(ball);
      if (this.balls.length === 0 && !this.launcherBall && this.respawnTimer <= 0) {
        this.respawnTimer = RESPAWN_DELAY;
      }
    }
  }

  /**
   * Launch all balls in the launcher zone.
   */
  launchBalls(speed: number): void {
    const launcherBalls = this.balls.filter((b) => b.isInShooterLane());
    const count = launcherBalls.length;
    if (count === 0) return;

    // Scale speed quadratically by count to overcome friction between stacked balls
    const scaledSpeed = speed * count * count;
    for (const b of launcherBalls) {
      b.launch(scaledSpeed);
    }
    this.launcherBall = null;
  }

  /**
   * Clean up resources.
   */
  dispose(): void {
    if (this.unsubscribeCapture) {
      this.unsubscribeCapture();
      this.unsubscribeCapture = null;
    }
  }

  // --- Private methods ---

  private acquireBall(): Ball {
    let ball: Ball;
    if (this.inactiveBalls.length > 0) {
      ball = this.inactiveBalls.pop()!;
    } else {
      ball = new Ball(this.container, this.physics);
    }
    ball.setTint(this.ballColor);
    return ball;
  }

  private spawnBallInLauncher(): void {
    const ball = this.acquireBall();
    ball.respawn();
    this.balls.push(ball);
    this.ballByHandle.set(ball.colliderHandle, ball);
    this.launcherBall = ball;
  }

  private handleCapture(event: CaptureEvent): void {
    const ball = this.acquireBall();
    // Override color if provided (ball from another player)
    if (event.color !== undefined) {
      ball.setTint(event.color);
    }
    // Spawn below the escape slot so the ball doesn't immediately re-escape
    const x = this.physics.toPhysicsX(200); // center
    const y = this.physics.toPhysicsY(80); // below escape slot
    ball.injectFromCapture(x, y, event.vx, event.vy);
    this.balls.push(ball);
    this.ballByHandle.set(ball.colliderHandle, ball);
  }

  private removeBall(ball: Ball): void {
    ball.setInactive();
    this.ballByHandle.delete(ball.colliderHandle);
    const idx = this.balls.indexOf(ball);
    if (idx !== -1) {
      this.balls.splice(idx, 1);
    }
    if (this.launcherBall === ball) {
      this.launcherBall = null;
    }
    this.inactiveBalls.push(ball);
  }
}
