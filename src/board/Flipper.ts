import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { COLORS } from "../constants";
import { FlipperDef } from "./BoardGeometry";

const ROTATION_SPEED_UP = 14.0; // radians/second
const ROTATION_SPEED_DOWN = 6.0; // radians/second
const MAX_ANGLE = 0.45; // radians

export class Flipper {
  private graphics: Graphics;
  private body: RAPIER.RigidBody;
  private physics: PhysicsWorld;
  private def: FlipperDef;
  private currentAngle = 0;

  constructor(container: Container, physics: PhysicsWorld, def: FlipperDef) {
    this.physics = physics;
    this.def = def;
    this.graphics = new Graphics();
    container.addChild(this.graphics);

    this.body = this.createBody(physics);
    this.draw();
  }

  private createBody(physics: PhysicsWorld): RAPIER.RigidBody {
    const bodyDesc =
      RAPIER.RigidBodyDesc.kinematicPositionBased().setTranslation(
        physics.toPhysicsX(this.def.position.x),
        physics.toPhysicsY(this.def.position.y),
      );
    const body = physics.world.createRigidBody(bodyDesc);

    const colliderDesc = RAPIER.ColliderDesc.cuboid(
      physics.toPhysicsSize(this.def.length / 2),
      physics.toPhysicsSize(this.def.width / 2),
    )
      .setRestitution(0.5)
      .setFriction(0.8);
    physics.world.createCollider(colliderDesc, body);

    return body;
  }

  update(dt: number, active: boolean) {
    let newAngle = this.currentAngle;

    if (this.def.side === "left") {
      if (active) {
        newAngle -= ROTATION_SPEED_UP * dt;
      } else {
        newAngle += ROTATION_SPEED_DOWN * dt;
      }
    } else {
      if (active) {
        newAngle += ROTATION_SPEED_UP * dt;
      } else {
        newAngle -= ROTATION_SPEED_DOWN * dt;
      }
    }

    newAngle = Math.max(-MAX_ANGLE, Math.min(MAX_ANGLE, newAngle));
    this.currentAngle = newAngle;

    // Compute position from pivot + angle directly (no drift)
    const restDx = this.def.position.x - this.def.pivot.x;
    const restDy = this.def.position.y - this.def.pivot.y;
    const cos = Math.cos(this.currentAngle);
    const sin = Math.sin(this.currentAngle);
    const targetX = this.def.pivot.x + restDx * cos - restDy * sin;
    const targetY = this.def.pivot.y + restDx * sin + restDy * cos;

    // Set next kinematic target - Rapier computes velocity internally for collisions
    this.body.setNextKinematicTranslation({
      x: this.physics.toPhysicsX(targetX),
      y: this.physics.toPhysicsY(targetY),
    });
    this.body.setNextKinematicRotation(this.currentAngle);

    this.draw();
  }

  private draw() {
    const pos = this.body.translation();
    const px = this.physics.toPixelsX(pos.x);
    const py = this.physics.toPixelsY(pos.y);

    this.graphics.clear();
    this.graphics.position.set(px, py);
    this.graphics.rotation = this.currentAngle;

    this.graphics.roundRect(
      -this.def.length / 2,
      -this.def.width / 2,
      this.def.length,
      this.def.width,
      this.def.width / 2,
    );
    this.graphics.stroke({ color: COLORS.flipper, width: 2 });
  }
}
