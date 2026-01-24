import { describe, it, expect, beforeAll } from "vitest";
import RAPIER from "@dimforge/rapier2d-compat";
import {
  ROTATION_SPEED_UP,
  MAX_ANGLE,
  activeAngle,
  restAngle,
  stepFlipperAngle,
} from "../src/board/flipperLogic";

const PPM = 500;
const toPhysics = (px: number) => px / PPM;

beforeAll(async () => {
  await RAPIER.init();
});

function createWorld() {
  return new RAPIER.World({ x: 0, y: toPhysics(300) }); // gravity 300px/s² down
}

describe("Ball-pin collision", () => {
  it("generates collision event when ball hits pin", () => {
    const world = createWorld();
    const eventQueue = new RAPIER.EventQueue(true);

    // Create fixed pin at (200, 200) with radius 25px
    const pinBodyDesc = RAPIER.RigidBodyDesc.fixed().setTranslation(
      toPhysics(200),
      toPhysics(200),
    );
    const pinBody = world.createRigidBody(pinBodyDesc);
    const pinColliderDesc = RAPIER.ColliderDesc.ball(toPhysics(25))
      .setRestitution(0.7)
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    const pinCollider = world.createCollider(pinColliderDesc, pinBody);

    // Create dynamic ball above pin, moving down
    const ballBodyDesc = RAPIER.RigidBodyDesc.dynamic()
      .setTranslation(toPhysics(200), toPhysics(150))
      .setCcdEnabled(true);
    const ballBody = world.createRigidBody(ballBodyDesc);
    ballBody.setLinvel({ x: 0, y: toPhysics(500) }, true); // moving down fast
    const ballColliderDesc = RAPIER.ColliderDesc.ball(toPhysics(10))
      .setRestitution(0.5)
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    const ballCollider = world.createCollider(ballColliderDesc, ballBody);

    // Step until collision
    let collisionDetected = false;
    const dt = 1 / 120;
    for (let i = 0; i < 120; i++) {
      world.timestep = dt;
      world.step(eventQueue);

      eventQueue.drainCollisionEvents((h1, h2, started) => {
        if (!started) return;
        const handles = [h1, h2];
        if (
          handles.includes(pinCollider.handle) &&
          handles.includes(ballCollider.handle)
        ) {
          collisionDetected = true;
        }
      });

      if (collisionDetected) break;
    }

    expect(collisionDetected).toBe(true);

    eventQueue.free();
    world.free();
  });

  it("ball bounces off pin (velocity reverses)", () => {
    // No gravity for a clean bounce test
    const world = new RAPIER.World({ x: 0, y: 0 });
    const eventQueue = new RAPIER.EventQueue(true);

    // Fixed pin at (200, 300)
    const pinBody = world.createRigidBody(
      RAPIER.RigidBodyDesc.fixed().setTranslation(
        toPhysics(200),
        toPhysics(300),
      ),
    );
    world.createCollider(
      RAPIER.ColliderDesc.ball(toPhysics(25)).setRestitution(1.0),
      pinBody,
    );

    // Ball above pin, moving down
    const ballBody = world.createRigidBody(
      RAPIER.RigidBodyDesc.dynamic()
        .setTranslation(toPhysics(200), toPhysics(200))
        .setCcdEnabled(true),
    );
    ballBody.setLinvel({ x: 0, y: toPhysics(400) }, true);
    world.createCollider(
      RAPIER.ColliderDesc.ball(toPhysics(10)).setRestitution(1.0),
      ballBody,
    );

    const dt = 1 / 120;
    for (let i = 0; i < 200; i++) {
      world.timestep = dt;
      world.step(eventQueue);
    }

    // Ball should have bounced and be moving upward (negative Y velocity)
    const vel = ballBody.linvel();
    expect(vel.y).toBeLessThan(0);

    eventQueue.free();
    world.free();
  });
});

describe("Ball-drain collision", () => {
  it("generates collision event when ball hits drain wall", () => {
    const world = createWorld();
    const eventQueue = new RAPIER.EventQueue(true);

    // Drain wall (horizontal at y=670px)
    const drainBodyDesc = RAPIER.RigidBodyDesc.fixed();
    const drainBody = world.createRigidBody(drainBodyDesc);
    const drainColliderDesc = RAPIER.ColliderDesc.cuboid(
      toPhysics(200),
      toPhysics(5),
    )
      .setTranslation(toPhysics(200), toPhysics(670))
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    const drainCollider = world.createCollider(drainColliderDesc, drainBody);

    // Ball above drain, falling
    const ballBodyDesc = RAPIER.RigidBodyDesc.dynamic()
      .setTranslation(toPhysics(200), toPhysics(600))
      .setCcdEnabled(true);
    const ballBody = world.createRigidBody(ballBodyDesc);
    const ballColliderDesc = RAPIER.ColliderDesc.ball(toPhysics(10))
      .setRestitution(0.5)
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    const ballCollider = world.createCollider(ballColliderDesc, ballBody);

    let drainHit = false;
    const dt = 1 / 120;
    for (let i = 0; i < 240; i++) {
      world.timestep = dt;
      world.step(eventQueue);

      eventQueue.drainCollisionEvents((h1, h2, started) => {
        if (!started) return;
        const handles = [h1, h2];
        if (
          handles.includes(drainCollider.handle) &&
          handles.includes(ballCollider.handle)
        ) {
          drainHit = true;
        }
      });

      if (drainHit) break;
    }

    expect(drainHit).toBe(true);

    eventQueue.free();
    world.free();
  });
});

describe("Ball drain-escape pipeline", () => {
  it("ball hitting drain triggers escape (not stuck on wall)", () => {
    const world = createWorld();
    const eventQueue = new RAPIER.EventQueue(true);

    // Drain wall at bottom (like Board.ts: one fixed body, collider with translation)
    const wallBody = world.createRigidBody(RAPIER.RigidBodyDesc.fixed());
    const drainColliderDesc = RAPIER.ColliderDesc.cuboid(
      toPhysics(200),
      toPhysics(5),
    )
      .setTranslation(toPhysics(200), toPhysics(670))
      .setRestitution(0.3)
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    const drainCollider = world.createCollider(drainColliderDesc, wallBody);

    // Ball above drain, falling (like a ball that missed flippers)
    const ballBody = world.createRigidBody(
      RAPIER.RigidBodyDesc.dynamic()
        .setTranslation(toPhysics(200), toPhysics(620))
        .setCcdEnabled(true),
    );
    const ballColliderDesc = RAPIER.ColliderDesc.ball(toPhysics(10))
      .setRestitution(0.5)
      .setActiveEvents(RAPIER.ActiveEvents.COLLISION_EVENTS);
    const ballCollider = world.createCollider(ballColliderDesc, ballBody);

    // Simulate the escape logic: on drain hit, record snapshot and deactivate
    let escaped = false;
    let escapeSnapshot: {
      x: number;
      y: number;
      vx: number;
      vy: number;
    } | null = null;

    const dt = 1 / 120;
    for (let i = 0; i < 240; i++) {
      world.timestep = dt;
      world.step(eventQueue);

      eventQueue.drainCollisionEvents((h1, h2, started) => {
        if (!started) return;
        const handles = [h1, h2];
        if (
          handles.includes(drainCollider.handle) &&
          handles.includes(ballCollider.handle) &&
          !escaped
        ) {
          // This is what Game.escapeBall() does:
          const pos = ballBody.translation();
          const vel = ballBody.linvel();
          escapeSnapshot = { x: pos.x, y: pos.y, vx: vel.x, vy: vel.y };
          escaped = true;
          // Deactivate: move far away, zero velocity
          ballBody.setTranslation({ x: -100, y: -100 }, true);
          ballBody.setLinvel({ x: 0, y: 0 }, true);
        }
      });

      if (escaped) break;
    }

    // Ball must have hit drain and escaped
    expect(escaped).toBe(true);
    expect(escapeSnapshot).not.toBeNull();
    // Snapshot should have downward velocity (ball was falling)
    expect(escapeSnapshot!.vy).toBeGreaterThan(0);

    // After escape, ball is deactivated (far away, stopped)
    const pos = ballBody.translation();
    expect(pos.x).toBe(-100);
    expect(pos.y).toBe(-100);
    const vel = ballBody.linvel();
    expect(vel.x).toBe(0);
    expect(vel.y).toBe(0);

    eventQueue.free();
    world.free();
  });

  it("ball on drain without escape handler stays stuck (regression)", () => {
    // This test documents the bug: if drain collision is ignored,
    // ball stays on the drain wall and never reaches escape bounds
    const world = createWorld();
    const eventQueue = new RAPIER.EventQueue(true);

    // Drain wall
    const wallBody = world.createRigidBody(RAPIER.RigidBodyDesc.fixed());
    const drainColliderDesc = RAPIER.ColliderDesc.cuboid(
      toPhysics(200),
      toPhysics(5),
    )
      .setTranslation(toPhysics(200), toPhysics(670))
      .setRestitution(0.3);
    world.createCollider(drainColliderDesc, wallBody);

    // Ball falling towards drain
    const ballBody = world.createRigidBody(
      RAPIER.RigidBodyDesc.dynamic()
        .setTranslation(toPhysics(200), toPhysics(620))
        .setCcdEnabled(true),
    );
    world.createCollider(
      RAPIER.ColliderDesc.ball(toPhysics(10)).setRestitution(0.5),
      ballBody,
    );

    // Step for 2 seconds — ball should settle on drain, never pass through
    const dt = 1 / 120;
    for (let i = 0; i < 240; i++) {
      world.timestep = dt;
      world.step(eventQueue);
    }

    // Ball Y should be at or above drain (670px) — it does NOT pass through
    const ballY = ballBody.translation().y * 500; // convert back to pixels
    expect(ballY).toBeLessThanOrEqual(670);
    // Ball speed should be very low (settled)
    const vel = ballBody.linvel();
    const speed = Math.sqrt(vel.x * vel.x + vel.y * vel.y);
    expect(speed).toBeLessThan(0.5);

    eventQueue.free();
    world.free();
  });
});

describe("Flipper-ball interaction", () => {
  it("kinematic flipper rotation imparts velocity to ball", () => {
    const world = new RAPIER.World({ x: 0, y: 0 }); // no gravity
    const eventQueue = new RAPIER.EventQueue(true);

    // Flipper: kinematic body at pivot, cuboid collider offset
    const pivotX = toPhysics(110);
    const pivotY = toPhysics(600);
    const flipperBodyDesc =
      RAPIER.RigidBodyDesc.kinematicPositionBased().setTranslation(
        pivotX,
        pivotY,
      );
    const flipperBody = world.createRigidBody(flipperBodyDesc);

    const flipperLength = toPhysics(78);
    const flipperWidth = toPhysics(12);
    const flipperColliderDesc = RAPIER.ColliderDesc.cuboid(
      flipperLength / 2,
      flipperWidth / 2,
    )
      .setTranslation(flipperLength / 2, 0) // offset from pivot
      .setRestitution(0.5)
      .setFriction(0.8);
    world.createCollider(flipperColliderDesc, flipperBody);

    // Ball resting near the flipper tip
    const ballBodyDesc = RAPIER.RigidBodyDesc.dynamic()
      .setTranslation(toPhysics(180), toPhysics(595))
      .setCcdEnabled(true);
    const ballBody = world.createRigidBody(ballBodyDesc);
    world.createCollider(
      RAPIER.ColliderDesc.ball(toPhysics(10)).setRestitution(0.5),
      ballBody,
    );

    // Record initial ball velocity
    const velBefore = ballBody.linvel();
    const speedBefore = Math.sqrt(
      velBefore.x * velBefore.x + velBefore.y * velBefore.y,
    );

    // Swing flipper from rest to active using flipperLogic constants
    const dt = 1 / 120;
    let angle = restAngle("left");
    for (let i = 0; i < 30; i++) {
      angle = stepFlipperAngle(angle, dt, true, "left");
      flipperBody.setNextKinematicRotation(angle);
      world.timestep = dt;
      world.step(eventQueue);
    }

    // Ball should have gained velocity from flipper hit
    const velAfter = ballBody.linvel();
    const speedAfter = Math.sqrt(
      velAfter.x * velAfter.x + velAfter.y * velAfter.y,
    );

    expect(speedAfter).toBeGreaterThan(speedBefore);

    eventQueue.free();
    world.free();
  });
});
