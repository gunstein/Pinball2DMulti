import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { COLORS } from "../constants";
import { FlipperDef } from "./BoardGeometry";
import { stepFlipperAngle, restAngle } from "./flipperLogic";

const FLIPPER_FRICTION = 0.2;

export class Flipper {
  private graphics: Graphics;
  private body: RAPIER.RigidBody;
  private def: FlipperDef;
  private currentAngle: number;
  private prevAngle: number;

  constructor(container: Container, physics: PhysicsWorld, def: FlipperDef) {
    this.def = def;
    this.currentAngle = restAngle(def.side);
    this.prevAngle = this.currentAngle;

    this.graphics = new Graphics();
    container.addChild(this.graphics);

    // Draw tapered shape (local coordinates relative to pivot)
    this.drawTaperedFlipper();

    this.body = this.createBody(physics);
    this.render();
  }

  /**
   * Draw a tapered flipper: thick circle at pivot, thin at tip, connected
   * by external tangent lines. This is the classic pinball flipper shape.
   *
   * Geometry: two circles of different radii connected by their common
   * external tangents. The tangent angle is arcsin((R-r)/d) where R and r
   * are the radii and d is the distance between centers.
   */
  private drawTaperedFlipper() {
    const dir = this.def.side === "left" ? 1 : -1;
    const { length, pivotRadius, tipRadius } = this.def;
    const g = this.graphics;

    // Pivot center at origin, tip center at (dir * length, 0)
    const tipX = dir * length;

    // External tangent angle: arcsin((R - r) / distance)
    const dr = pivotRadius - tipRadius;
    const dist = length;
    const tangentAngle = Math.asin(dr / dist);

    // Upper and lower tangent points on pivot circle
    const pUpX = pivotRadius * Math.sin(tangentAngle);
    const pUpY = -pivotRadius * Math.cos(tangentAngle);
    const pDownX = pUpX;
    const pDownY = pivotRadius * Math.cos(tangentAngle);

    // Upper and lower tangent points on tip circle
    const tUpX = tipX + dir * tipRadius * Math.sin(tangentAngle);
    const tUpY = -tipRadius * Math.cos(tangentAngle);
    const tDownX = tUpX;
    const tDownY = tipRadius * Math.cos(tangentAngle);

    // Draw the shape: pivot arc -> upper tangent -> tip arc -> lower tangent
    const pivotStartAngle = Math.atan2(pUpY, pUpX);
    const pivotEndAngle = Math.atan2(pDownY, pDownX);
    const tipStartAngle = Math.atan2(tDownY, tDownX - tipX);
    const tipEndAngle = Math.atan2(tUpY, tUpX - tipX);

    g.moveTo(pUpX, pUpY);
    g.lineTo(tUpX, tUpY);
    g.arc(tipX, 0, tipRadius, tipEndAngle, tipStartAngle, dir < 0);
    g.lineTo(pDownX, pDownY);
    g.arc(0, 0, pivotRadius, pivotEndAngle, pivotStartAngle, dir < 0);
    g.closePath();
    g.stroke({ color: COLORS.flipper, width: 2 });
  }

  /**
   * Generate sample points on the two circles (pivot + tip) and let
   * Rapier's convexHull() compute the outer boundary for collision.
   */
  private generateHullPoints(physics: PhysicsWorld): Float32Array {
    const dir = this.def.side === "left" ? 1 : -1;
    const { length, pivotRadius, tipRadius } = this.def;
    const points: number[] = [];
    const segments = 12; // points per full circle

    // Full circle around pivot center (0, 0)
    for (let i = 0; i < segments; i++) {
      const a = (2 * Math.PI * i) / segments;
      points.push(
        physics.toPhysicsSize(pivotRadius * Math.cos(a)),
        physics.toPhysicsSize(pivotRadius * Math.sin(a)),
      );
    }

    // Full circle around tip center (dir * length, 0)
    for (let i = 0; i < segments; i++) {
      const a = (2 * Math.PI * i) / segments;
      points.push(
        physics.toPhysicsSize(dir * length + tipRadius * Math.cos(a)),
        physics.toPhysicsSize(tipRadius * Math.sin(a)),
      );
    }

    // Rapier's convexHull computes the outer hull from all points
    return new Float32Array(points);
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

    // Convex hull collider matching the tapered shape
    const hullPoints = this.generateHullPoints(physics);
    const colliderDesc = RAPIER.ColliderDesc.convexHull(hullPoints);
    if (colliderDesc) {
      colliderDesc
        .setRestitution(0.5)
        .setFriction(FLIPPER_FRICTION)
        .setFrictionCombineRule(RAPIER.CoefficientCombineRule.Min);
      physics.world.createCollider(colliderDesc, body);
    }

    return body;
  }

  fixedUpdate(dt: number, active: boolean) {
    this.prevAngle = this.currentAngle;
    this.currentAngle = stepFlipperAngle(
      this.currentAngle,
      dt,
      active,
      this.def.side,
    );
    this.body.setNextKinematicRotation(this.currentAngle);
  }

  render(alpha = 1) {
    const clamped = Math.max(0, Math.min(1, alpha));
    const angle =
      this.prevAngle + (this.currentAngle - this.prevAngle) * clamped;
    this.graphics.position.set(this.def.pivot.x, this.def.pivot.y);
    this.graphics.rotation = angle;
  }
}
