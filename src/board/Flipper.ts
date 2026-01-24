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
  private def: FlipperDef;
  private currentAngle = 0;

  constructor(container: Container, physics: PhysicsWorld, def: FlipperDef) {
    this.def = def;
    this.graphics = new Graphics();
    container.addChild(this.graphics);

    this.body = this.createBody(physics);
  }

  private createBody(physics: PhysicsWorld): RAPIER.RigidBody {
    // Body placed at pivot point
    const bodyDesc =
      RAPIER.RigidBodyDesc.kinematicPositionBased().setTranslation(
        physics.toPhysicsX(this.def.pivot.x),
        physics.toPhysicsY(this.def.pivot.y),
      );
    const body = physics.world.createRigidBody(bodyDesc);

    // Collider offset from pivot (extends outward in flipper direction)
    const dir = this.def.side === "left" ? 1 : -1;
    const colliderDesc = RAPIER.ColliderDesc.cuboid(
      physics.toPhysicsSize(this.def.length / 2),
      physics.toPhysicsSize(this.def.width / 2),
    )
      .setTranslation(physics.toPhysicsSize(dir * (this.def.length / 2)), 0)
      .setRestitution(0.5)
      .setFriction(0.8);
    physics.world.createCollider(colliderDesc, body);

    return body;
  }

  fixedUpdate(dt: number, active: boolean) {
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

    // Only rotation needed - body is at pivot, collider is offset
    this.body.setNextKinematicRotation(this.currentAngle);
  }

  render() {
    const dir = this.def.side === "left" ? 1 : -1;

    this.graphics.clear();
    this.graphics.position.set(this.def.pivot.x, this.def.pivot.y);
    this.graphics.rotation = this.currentAngle;

    this.graphics.roundRect(
      dir > 0 ? 0 : -this.def.length,
      -this.def.width / 2,
      this.def.length,
      this.def.width,
      this.def.width / 2,
    );
    this.graphics.stroke({ color: COLORS.flipper, width: 2 });
  }
}
