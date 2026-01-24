import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { BALL_RADIUS, BALL_RESTITUTION, COLORS } from "../constants";
import { ballSpawn } from "./BoardGeometry";

export class Ball {
  private graphics: Graphics;
  private body: RAPIER.RigidBody;
  private physics: PhysicsWorld;
  colliderHandle: number;

  // Trail history
  private trail: { x: number; y: number }[] = [];
  private trailGraphics: Graphics;
  private readonly TRAIL_LENGTH = 15;

  constructor(container: Container, physics: PhysicsWorld) {
    this.physics = physics;
    this.graphics = new Graphics();
    this.trailGraphics = new Graphics();
    container.addChild(this.trailGraphics);
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

  respawn() {
    this.body.setTranslation(
      {
        x: this.physics.toPhysicsX(ballSpawn.x),
        y: this.physics.toPhysicsY(ballSpawn.y),
      },
      true,
    );
    this.body.setLinvel({ x: 0, y: 0 }, true);
    this.body.setAngvel(0, true);
    this.trail = [];
  }

  applyImpulse(impulse: { x: number; y: number }) {
    this.body.applyImpulse(impulse, true);
  }

  getPosition(): { x: number; y: number } {
    const pos = this.body.translation();
    return {
      x: this.physics.toPixelsX(pos.x),
      y: this.physics.toPixelsY(pos.y),
    };
  }

  update() {
    const pos = this.body.translation();
    const px = this.physics.toPixelsX(pos.x);
    const py = this.physics.toPixelsY(pos.y);

    // Update trail
    this.trail.unshift({ x: px, y: py });
    if (this.trail.length > this.TRAIL_LENGTH) {
      this.trail.pop();
    }

    // Draw trail (only when ball is moving)
    this.trailGraphics.clear();
    const vel = this.body.linvel();
    const speed = Math.sqrt(vel.x * vel.x + vel.y * vel.y);
    if (speed > 0.1) {
      for (let i = 1; i < this.trail.length; i++) {
        const alpha = 1 - i / this.trail.length;
        const radius = BALL_RADIUS * (1 - i / this.trail.length) * 0.4;
        this.trailGraphics.circle(this.trail[i].x, this.trail[i].y, radius);
        this.trailGraphics.stroke({
          color: COLORS.trail,
          width: 1,
          alpha: alpha * 0.3,
        });
      }
    }

    // Draw ball (transparent fill, teal stroke like walls)
    this.graphics.clear();
    this.graphics.circle(px, py, BALL_RADIUS);
    this.graphics.stroke({ color: COLORS.wall, width: 2 });
  }
}
