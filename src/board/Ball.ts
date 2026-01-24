import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { BALL_RADIUS, BALL_RESTITUTION, COLORS } from "../constants";
import { ballSpawn, launcherStop } from "./BoardGeometry";

export class Ball {
  private graphics: Graphics;
  private body: RAPIER.RigidBody;
  private physics: PhysicsWorld;
  colliderHandle: number;
  private inLauncher = true;

  constructor(container: Container, physics: PhysicsWorld) {
    this.physics = physics;
    this.graphics = new Graphics();
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
    this.inLauncher = true;
  }

  launch(power: number) {
    if (!this.inLauncher) return;
    this.inLauncher = false;
    this.body.applyImpulse({ x: 0, y: -power }, true);
  }

  update() {
    const pos = this.body.translation();
    const px = this.physics.toPixelsX(pos.x);
    const py = this.physics.toPixelsY(pos.y);

    // Check if ball has returned to launcher zone
    if (!this.inLauncher) {
      const inLaneX = px >= launcherStop.from.x && px <= launcherStop.to.x;
      const nearStop =
        py >= launcherStop.from.y - 30 && py <= launcherStop.from.y;
      const vel = this.body.linvel();
      const speed = Math.sqrt(vel.x * vel.x + vel.y * vel.y);
      if (inLaneX && nearStop && speed < 0.5) {
        this.inLauncher = true;
      }
    }

    // Draw ball (transparent fill, teal stroke like walls)
    this.graphics.clear();
    this.graphics.circle(px, py, BALL_RADIUS);
    this.graphics.stroke({ color: COLORS.wall, width: 2 });
  }
}
