import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { COLORS } from "../constants";
import { CircleDef } from "./BoardGeometry";

const PIN_RESTITUTION = 0.7;

export class Pin {
  private graphics: Graphics;
  private def: CircleDef;
  private hitTimer = 0;
  private readonly HIT_DURATION = 1.0;
  colliderHandle: number;

  constructor(container: Container, physics: PhysicsWorld, def: CircleDef) {
    this.def = def;
    this.graphics = new Graphics();
    container.addChild(this.graphics);
    this.colliderHandle = this.createBody(physics);
  }

  private createBody(physics: PhysicsWorld): number {
    const bodyDesc = RAPIER.RigidBodyDesc.fixed().setTranslation(
      physics.toPhysicsX(this.def.center.x),
      physics.toPhysicsY(this.def.center.y),
    );
    const body = physics.world.createRigidBody(bodyDesc);

    const colliderDesc = RAPIER.ColliderDesc.ball(
      physics.toPhysicsSize(this.def.radius),
    )
      .setRestitution(PIN_RESTITUTION)
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    const collider = physics.world.createCollider(colliderDesc, body);
    return collider.handle;
  }

  hit() {
    this.hitTimer = this.HIT_DURATION;
  }

  fixedUpdate(dt: number) {
    if (this.hitTimer > 0) {
      this.hitTimer -= dt;
      if (this.hitTimer <= 0) this.hitTimer = 0;
    }
  }

  render() {
    this.graphics.clear();
    const color = this.hitTimer > 0 ? COLORS.pinHit : COLORS.pin;

    if (this.hitTimer > 0) {
      this.graphics.circle(
        this.def.center.x,
        this.def.center.y,
        this.def.radius + 4,
      );
      this.graphics.fill({
        color: COLORS.pinHit,
        alpha: 0.2 * (this.hitTimer / this.HIT_DURATION),
      });
    }

    this.graphics.circle(this.def.center.x, this.def.center.y, this.def.radius);
    this.graphics.stroke({ color, width: 2 });
  }
}
