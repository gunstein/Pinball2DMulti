import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { BALL_RADIUS, BALL_RESTITUTION, COLORS } from "../constants";
import { ballSpawn, launcherStop, escapeBounds } from "./BoardGeometry";
import { BallSnapshot } from "../shared/types";

const LAUNCHER_SNAP_Y_TOLERANCE = 30; // pixels above stop
const LAUNCHER_SNAP_SPEED = 0.5; // m/s threshold to consider ball stopped

export class Ball {
  private graphics: Graphics;
  private body: RAPIER.RigidBody;
  private physics: PhysicsWorld;
  colliderHandle: number;
  private inLauncher = true;
  private active = true;

  constructor(container: Container, physics: PhysicsWorld) {
    this.physics = physics;

    // Draw ball shape once (centered at origin)
    this.graphics = new Graphics();
    this.graphics.circle(0, 0, BALL_RADIUS);
    this.graphics.stroke({ color: COLORS.ball, width: 2 });
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

  launch(speed: number) {
    if (!this.inLauncher || !this.active) return;
    this.inLauncher = false;
    this.body.setLinvel({ x: 0, y: -speed }, true);
  }

  /** Returns a snapshot if ball has escaped the playfield bounds, null otherwise. */
  getEscapeSnapshot(): BallSnapshot | null {
    if (!this.active) return null;

    const pos = this.body.translation();
    const px = this.physics.toPixelsX(pos.x);
    const py = this.physics.toPixelsY(pos.y);

    if (
      px < escapeBounds.left ||
      px > escapeBounds.right ||
      py < escapeBounds.top ||
      py > escapeBounds.bottom
    ) {
      const vel = this.body.linvel();
      return {
        id: crypto.randomUUID(),
        x: pos.x,
        y: pos.y,
        vx: vel.x,
        vy: vel.y,
      };
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
