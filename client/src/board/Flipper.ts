import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { COLORS } from "../constants";
import { FlipperDef } from "./BoardGeometry";
import { stepFlipperAngle, restAngle } from "./flipperLogic";

export class Flipper {
  private graphics: Graphics;
  private body: RAPIER.RigidBody;
  private def: FlipperDef;
  private currentAngle: number;

  constructor(container: Container, physics: PhysicsWorld, def: FlipperDef) {
    this.def = def;
    this.currentAngle = restAngle(def.side);

    this.graphics = new Graphics();
    container.addChild(this.graphics);

    // Draw shape once (local coordinates relative to pivot)
    const dir = def.side === "left" ? 1 : -1;
    this.graphics.roundRect(
      dir > 0 ? 0 : -def.length,
      -def.width / 2,
      def.length,
      def.width,
      def.width / 2,
    );
    this.graphics.stroke({ color: COLORS.flipper, width: 2 });

    this.body = this.createBody(physics);
    this.render();
  }

  private createBody(physics: PhysicsWorld): RAPIER.RigidBody {
    // Body placed at pivot point
    const bodyDesc =
      RAPIER.RigidBodyDesc.kinematicPositionBased().setTranslation(
        physics.toPhysicsX(this.def.pivot.x),
        physics.toPhysicsY(this.def.pivot.y),
      );
    bodyDesc.setRotation(this.currentAngle);
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
    this.currentAngle = stepFlipperAngle(
      this.currentAngle,
      dt,
      active,
      this.def.side,
    );
    this.body.setNextKinematicRotation(this.currentAngle);
  }

  render() {
    this.graphics.position.set(this.def.pivot.x, this.def.pivot.y);
    this.graphics.rotation = this.currentAngle;
  }
}
