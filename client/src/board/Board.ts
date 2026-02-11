import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { COLORS } from "../constants";
import {
  wallSegments,
  guideWalls,
  launcherWall,
  launcherStop,
  escapeSlot,
  LAUNCHER_WALL_THICKNESS,
  WALL_STROKE_WIDTH,
  BOTTOM_WALL_INDEX,
  Segment,
} from "./BoardGeometry";

const WALL_COLLIDER_THICKNESS = 5; // pixels half-thickness for segment colliders

export class Board {
  private graphics: Graphics;
  drainColliderHandle: number = -1;
  escapeColliderHandle: number = -1;

  constructor(container: Container, physics: PhysicsWorld) {
    this.graphics = new Graphics();
    container.addChild(this.graphics);

    // Single fixed body for all wall colliders (less world clutter)
    const bodyDesc = RAPIER.RigidBodyDesc.fixed();
    const wallBody = physics.world.createRigidBody(bodyDesc);

    // Create colliders on the shared body
    for (let i = 0; i < wallSegments.length; i++) {
      const handle = this.createSegmentCollider(
        physics,
        wallBody,
        wallSegments[i],
        i === BOTTOM_WALL_INDEX,
      );
      if (i === BOTTOM_WALL_INDEX) {
        this.drainColliderHandle = handle;
      }
    }
    for (const seg of guideWalls) {
      this.createSegmentCollider(physics, wallBody, seg);
    }
    this.createSegmentCollider(physics, wallBody, launcherWall);
    this.createSegmentCollider(physics, wallBody, launcherStop);
    this.escapeColliderHandle = this.createEscapeSensor(physics, wallBody);

    this.draw();
  }

  private createSegmentCollider(
    physics: PhysicsWorld,
    body: RAPIER.RigidBody,
    seg: Segment,
    activeEvents = false,
  ): number {
    const mx = (seg.from.x + seg.to.x) / 2;
    const my = (seg.from.y + seg.to.y) / 2;
    const dx = seg.to.x - seg.from.x;
    const dy = seg.to.y - seg.from.y;
    const length = Math.sqrt(dx * dx + dy * dy);
    const angle = Math.atan2(dy, dx);

    const colliderDesc = RAPIER.ColliderDesc.cuboid(
      physics.toPhysicsSize(length / 2),
      physics.toPhysicsSize(WALL_COLLIDER_THICKNESS),
    )
      .setTranslation(physics.toPhysicsX(mx), physics.toPhysicsY(my))
      .setRotation(angle)
      .setRestitution(0.3);

    if (activeEvents) {
      colliderDesc.setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    }

    const collider = physics.world.createCollider(colliderDesc, body);
    return collider.handle;
  }

  private createEscapeSensor(
    physics: PhysicsWorld,
    body: RAPIER.RigidBody,
  ): number {
    const width = escapeSlot.xMax - escapeSlot.xMin;
    const height = escapeSlot.yBottom - escapeSlot.yTop;
    const centerX = (escapeSlot.xMin + escapeSlot.xMax) / 2;
    const centerY = (escapeSlot.yTop + escapeSlot.yBottom) / 2;

    const sensorDesc = RAPIER.ColliderDesc.cuboid(
      physics.toPhysicsSize(width / 2),
      physics.toPhysicsSize(height / 2),
    )
      .setTranslation(physics.toPhysicsX(centerX), physics.toPhysicsY(centerY))
      .setSensor(true)
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);

    return physics.world.createCollider(sensorDesc, body).handle;
  }

  private draw() {
    const g = this.graphics;

    // Draw wall segments
    for (const seg of wallSegments) {
      g.moveTo(seg.from.x, seg.from.y);
      g.lineTo(seg.to.x, seg.to.y);
    }
    g.stroke({ color: COLORS.wall, width: WALL_STROKE_WIDTH });

    // Draw guide walls
    for (const seg of guideWalls) {
      g.moveTo(seg.from.x, seg.from.y);
      g.lineTo(seg.to.x, seg.to.y);
    }
    g.stroke({ color: COLORS.wall, width: WALL_STROKE_WIDTH });

    // Draw launcher wall
    g.moveTo(launcherWall.from.x, launcherWall.from.y);
    g.lineTo(launcherWall.to.x, launcherWall.to.y);
    g.stroke({ color: COLORS.wall, width: LAUNCHER_WALL_THICKNESS });

    // Draw shooter-lane stop
    g.moveTo(launcherStop.from.x, launcherStop.from.y);
    g.lineTo(launcherStop.to.x, launcherStop.to.y);
    g.stroke({ color: COLORS.wall, width: LAUNCHER_WALL_THICKNESS });
  }
}
