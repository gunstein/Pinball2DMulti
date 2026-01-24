import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { COLORS } from "../constants";
import { CircleDef } from "./BoardGeometry";

const PIN_RESTITUTION = 0.7;

export class Pin {
  private pinGraphics: Graphics;
  private glowGraphics: Graphics;
  private def: CircleDef;
  private hitTimer = 0;
  private readonly HIT_DURATION = 1.0;
  colliderHandle: number;

  constructor(container: Container, physics: PhysicsWorld, def: CircleDef) {
    this.def = def;

    // Glow circle (drawn once, visibility controlled via alpha)
    this.glowGraphics = new Graphics();
    this.glowGraphics.circle(def.center.x, def.center.y, def.radius + 4);
    this.glowGraphics.fill({ color: COLORS.pinHit, alpha: 1 });
    this.glowGraphics.alpha = 0;
    container.addChild(this.glowGraphics);

    // Pin outline (drawn once, color swapped on hit via tint)
    this.pinGraphics = new Graphics();
    this.pinGraphics.circle(def.center.x, def.center.y, def.radius);
    this.pinGraphics.stroke({ color: COLORS.pin, width: 2 });
    container.addChild(this.pinGraphics);

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
    // Glow alpha fades from 0.2 to 0 over HIT_DURATION
    this.glowGraphics.alpha =
      this.hitTimer > 0 ? 0.2 * (this.hitTimer / this.HIT_DURATION) : 0;

    // Tint pin outline green when hit
    this.pinGraphics.tint = this.hitTimer > 0 ? COLORS.pinHit : 0xffffff;
  }
}
