import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { BALL_RADIUS, BALL_RESTITUTION, COLORS } from "../constants";
import {
  ballSpawn,
  launcherStop,
  launcherWall,
  isInEscapeSlot,
} from "./BoardGeometry";
import { BallSnapshot } from "../shared/types";

const LAUNCHER_SNAP_Y_TOLERANCE = 30; // pixels above stop
const LAUNCHER_SNAP_SPEED = 0.5; // m/s threshold to consider ball stopped

// Module-level counter for generating unique ball IDs
let nextBallId = 1;

export class Ball {
  private graphics: Graphics;
  private body: RAPIER.RigidBody;
  private physics: PhysicsWorld;
  colliderHandle: number;
  private inLauncher = true;
  private active = true;

  constructor(container: Container, physics: PhysicsWorld) {
    this.physics = physics;

    // Draw ball shape once (centered at origin) - white stroke allows tint to work
    this.graphics = new Graphics();
    this.graphics.circle(0, 0, BALL_RADIUS);
    this.graphics.stroke({ color: 0xffffff, width: 2 });
    container.addChild(this.graphics);

    const { body, colliderHandle } = this.createBody(physics);
    this.body = body;
    this.colliderHandle = colliderHandle;
  }

  private createBody(physics: PhysicsWorld) {
    const bodyDesc = RAPIER.RigidBodyDesc.dynamic()
      .setTranslation(
        physics.toPhysicsX(ballSpawn.x),
        physics.toPhysicsY(ballSpawn.y),
      )
      .setCcdEnabled(true);
    const body = physics.world.createRigidBody(bodyDesc);

    const colliderDesc = RAPIER.ColliderDesc.ball(
      physics.toPhysicsSize(BALL_RADIUS),
    )
      .setRestitution(BALL_RESTITUTION)
      .setFriction(0.3)
      .setDensity(1.0)
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    const collider = physics.world.createCollider(colliderDesc, body);

    return { body, colliderHandle: collider.handle };
  }

  isActive(): boolean {
    return this.active;
  }

  /** Set the tint color of the ball (0xRRGGBB) */
  setTint(color: number) {
    this.graphics.tint = color;
  }

  /** Get the current tint color of the ball */
  getTint(): number {
    return this.graphics.tint as number;
  }

  isInLauncher(): boolean {
    return this.inLauncher;
  }

  /**
   * True if the ball is currently inside the shooter lane area (by position),
   * regardless of whether it has snapped to the launcher stop.
   * This is used for launching stacked balls reliably.
   */
  isInShooterLane(): boolean {
    if (!this.active) return false;

    const pos = this.body.translation();
    const px = this.physics.toPixelsX(pos.x);
    const py = this.physics.toPixelsY(pos.y);

    // Lane X-range is the same as the launcher stop segment (inner wall -> right wall)
    const inLaneX = px >= launcherStop.from.x && px <= launcherStop.to.x;
    // Lane Y-range spans from mid-field down to bottom (launcher wall segment)
    const inLaneY = py >= launcherWall.from.y && py <= launcherWall.to.y;

    return inLaneX && inLaneY;
  }

  getPosition(): { x: number; y: number } {
    return this.body.translation();
  }

  getVelocity(): { x: number; y: number } {
    return this.body.linvel();
  }

  setInactive() {
    this.active = false;
    this.graphics.visible = false;
    // Move body far away and freeze it
    this.body.setTranslation({ x: -100, y: -100 }, true);
    this.body.setLinvel({ x: 0, y: 0 }, true);
    this.body.setAngvel(0, true);
  }

  /** Fully remove ball from physics world and graphics. Call when ball is permanently removed. */
  destroy(container: Container) {
    this.active = false;
    // Remove graphics
    container.removeChild(this.graphics);
    this.graphics.destroy();
    // Remove physics body (this also removes attached colliders)
    this.physics.world.removeRigidBody(this.body);
  }

  respawn() {
    this.active = true;
    this.graphics.visible = true;
    this.body.setTranslation(
      {
        x: this.physics.toPhysicsX(ballSpawn.x),
        y: this.physics.toPhysicsY(ballSpawn.y),
      },
      true,
    );
    this.body.setLinvel({ x: 0, y: 0 }, true);
    this.body.setAngvel(0, true);
    this.inLauncher = true;
  }

  /** Inject ball from deep-space capture (position and velocity in physics units) */
  injectFromCapture(x: number, y: number, vx: number, vy: number) {
    this.active = true;
    this.graphics.visible = true;
    this.body.setTranslation({ x, y }, true);
    this.body.setLinvel({ x: vx, y: vy }, true);
    this.body.setAngvel(0, true);
    this.inLauncher = false;
  }

  launch(speed: number) {
    if (!this.active) return;
    // Don't require the snapped inLauncher flag; allow stacked balls in the lane.
    if (!this.isInShooterLane()) return;

    this.inLauncher = false;
    // Use impulse instead of setLinvel to properly push through stacked balls
    const mass = this.body.mass();
    this.body.applyImpulse({ x: 0, y: -speed * mass }, true);
  }

  /** Returns a snapshot if ball has escaped through the escape slot, null otherwise. */
  getEscapeSnapshot(): BallSnapshot | null {
    if (!this.active) return null;

    const pos = this.body.translation();
    const vel = this.body.linvel();
    const px = this.physics.toPixelsX(pos.x);
    const py = this.physics.toPixelsY(pos.y);

    // Escape only through the defined slot (not AABB)
    if (isInEscapeSlot(px, py)) {
      // Only escape if moving upward (negative vy in physics = moving up on screen)
      if (vel.y < 0) {
        return {
          id: nextBallId++,
          x: pos.x,
          y: pos.y,
          vx: vel.x,
          vy: vel.y,
        };
      }
    }

    return null;
  }

  fixedUpdate() {
    if (!this.active) return;

    // Check if ball has returned to launcher zone
    if (!this.inLauncher) {
      const pos = this.body.translation();
      const px = this.physics.toPixelsX(pos.x);
      const py = this.physics.toPixelsY(pos.y);
      const inLaneX = px >= launcherStop.from.x && px <= launcherStop.to.x;
      const nearStop =
        py >= launcherStop.from.y - LAUNCHER_SNAP_Y_TOLERANCE &&
        py <= launcherStop.from.y;
      const vel = this.body.linvel();
      const speed = Math.sqrt(vel.x * vel.x + vel.y * vel.y);
      if (inLaneX && nearStop && speed < LAUNCHER_SNAP_SPEED) {
        this.inLauncher = true;
      }
    }
  }

  render() {
    if (!this.active) return;

    const pos = this.body.translation();
    const px = this.physics.toPixelsX(pos.x);
    const py = this.physics.toPixelsY(pos.y);
    this.graphics.position.set(px, py);
  }
}
