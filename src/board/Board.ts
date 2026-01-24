import RAPIER from "@dimforge/rapier2d-compat";
import { Graphics, Container } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { COLORS } from "../constants";
import {
  wallSegments,
  guideWalls,
  launcherWall,
  playfieldPolygon,
  LAUNCHER_WALL_THICKNESS,
  WALL_STROKE_WIDTH,
  drainPosition,
  DRAIN_WIDTH,
  Segment,
} from "./BoardGeometry";

const WALL_COLLIDER_THICKNESS = 5; // pixels half-thickness for segment colliders

export class Board {
  private graphics: Graphics;
  drainSensorHandle: number;

  constructor(container: Container, physics: PhysicsWorld) {
    this.graphics = new Graphics();
    container.addChild(this.graphics);

    // Create physics colliders from geometry
    for (const seg of wallSegments) {
      this.createSegmentCollider(physics, seg);
    }
    for (const seg of guideWalls) {
      this.createSegmentCollider(physics, seg);
    }
    this.createSegmentCollider(physics, launcherWall);

    // Drain sensor
    this.drainSensorHandle = this.createDrainSensor(physics);

    // Draw everything from the same geometry
    this.draw();
  }

  private createSegmentCollider(physics: PhysicsWorld, seg: Segment) {
    // Create a thin cuboid along the segment
    const mx = (seg.from.x + seg.to.x) / 2;
    const my = (seg.from.y + seg.to.y) / 2;
    const dx = seg.to.x - seg.from.x;
    const dy = seg.to.y - seg.from.y;
    const length = Math.sqrt(dx * dx + dy * dy);
    const angle = Math.atan2(dy, dx);

    const bodyDesc = RAPIER.RigidBodyDesc.fixed()
      .setTranslation(physics.toPhysicsX(mx), physics.toPhysicsY(my))
      .setRotation(angle);
    const body = physics.world.createRigidBody(bodyDesc);

    const colliderDesc = RAPIER.ColliderDesc.cuboid(
      physics.toPhysicsSize(length / 2),
      physics.toPhysicsSize(WALL_COLLIDER_THICKNESS),
    ).setRestitution(0.3);
    physics.world.createCollider(colliderDesc, body);
  }

  private createDrainSensor(physics: PhysicsWorld): number {
    const bodyDesc = RAPIER.RigidBodyDesc.fixed().setTranslation(
      physics.toPhysicsX(drainPosition.x),
      physics.toPhysicsY(drainPosition.y),
    );
    const body = physics.world.createRigidBody(bodyDesc);

    const colliderDesc = RAPIER.ColliderDesc.cuboid(
      physics.toPhysicsSize(DRAIN_WIDTH / 2),
      physics.toPhysicsSize(20),
    )
      .setSensor(true)
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    const collider = physics.world.createCollider(colliderDesc, body);
    return collider.handle;
  }

  private draw() {
    const g = this.graphics;

    // No filled background - transparent playfield

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

    // Draw launcher wall (thicker, solid-looking)
    g.moveTo(launcherWall.from.x, launcherWall.from.y);
    g.lineTo(launcherWall.to.x, launcherWall.to.y);
    g.stroke({ color: COLORS.wall, width: LAUNCHER_WALL_THICKNESS });
  }
}
