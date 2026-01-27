import { describe, it, expect, beforeEach } from "vitest";
import { SphereDeepSpace } from "../src/shared/SphereDeepSpace";
import { Player, DeepSpaceConfig } from "../src/shared/types";
import { vec3, dot, length, normalize } from "../src/shared/vec3";

// Test config with faster movement for quicker tests
const testConfig: DeepSpaceConfig = {
  portalAlpha: 0.1, // ~5.7 degrees - larger for easier testing
  omegaMin: 1.0,
  omegaMax: 1.0, // Fixed omega for predictable tests
  rerouteAfter: 10.0,
  rerouteCooldown: 5.0,
  minAgeForCapture: 0.5,
};

function createTestPlayers(): Player[] {
  return [
    { id: 1, cellIndex: 0, portalPos: vec3(1, 0, 0), color: 0xff0000 },
    { id: 2, cellIndex: 1, portalPos: vec3(0, 1, 0), color: 0x00ff00 },
    { id: 3, cellIndex: 2, portalPos: vec3(0, 0, 1), color: 0x0000ff },
    { id: 4, cellIndex: 3, portalPos: vec3(-1, 0, 0), color: 0xffff00 },
  ];
}

describe("SphereDeepSpace", () => {
  let deepSpace: SphereDeepSpace;
  let players: Player[];

  beforeEach(() => {
    deepSpace = new SphereDeepSpace(testConfig);
    players = createTestPlayers();
    deepSpace.setPlayers(players);
  });

  describe("addBall", () => {
    it("creates a ball with correct owner", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId);
      expect(ball).toBeDefined();
      expect(ball!.ownerId).toBe(1);
    });

    it("ball position is unit vector", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId);
      expect(length(ball!.pos)).toBeCloseTo(1, 9);
    });

    it("ball axis is unit vector", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId);
      expect(length(ball!.axis)).toBeCloseTo(1, 9);
    });

    it("ball starts with age 0", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId);
      expect(ball!.age).toBe(0);
    });

    it("ball position is offset from portal", () => {
      const portalPos = vec3(1, 0, 0);
      const ballId = deepSpace.addBall(1, portalPos, 1, 0);
      const ball = deepSpace.getBall(ballId);
      // Ball should not be exactly at portal
      expect(dot(ball!.pos, portalPos)).toBeLessThan(1);
    });
  });

  describe("tick - movement", () => {
    it("ball moves on great circle", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 0, 1);
      const ball = deepSpace.getBall(ballId)!;
      const initialPos = { ...ball.pos };

      deepSpace.tick(0.1);

      // Position should have changed
      expect(ball.pos.x).not.toBeCloseTo(initialPos.x, 3);
      // But still on unit sphere
      expect(length(ball.pos)).toBeCloseTo(1, 9);
    });

    it("ball age increases", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      deepSpace.tick(0.5);
      const ball = deepSpace.getBall(ballId)!;
      expect(ball.age).toBeCloseTo(0.5, 9);
    });

    it("multiple ticks accumulate age", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      deepSpace.tick(0.1);
      deepSpace.tick(0.1);
      deepSpace.tick(0.1);
      const ball = deepSpace.getBall(ballId)!;
      expect(ball.age).toBeCloseTo(0.3, 9);
    });
  });

  describe("tick - capture", () => {
    it("ball is not captured before minAgeForCapture", () => {
      // Place ball very close to player 2's portal
      const ballId = deepSpace.addBall(1, vec3(0, 1, 0), 0.01, 0);
      const ball = deepSpace.getBall(ballId)!;
      // Manually move ball to portal position
      ball.pos = normalize(vec3(0, 1, 0));

      const captures = deepSpace.tick(0.1);
      expect(captures.length).toBe(0);
      expect(ball.age).toBeLessThan(testConfig.minAgeForCapture);
    });

    it("ball is captured when at portal and old enough", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      // Age the ball past minAgeForCapture
      ball.age = testConfig.minAgeForCapture + 0.1;
      // Move ball to player 2's portal
      ball.pos = normalize(vec3(0, 1, 0));

      const captures = deepSpace.tick(0.01);
      expect(captures.length).toBe(1);
      expect(captures[0].playerId).toBe(2);
      expect(captures[0].ballId).toBe(ballId);
    });

    it("captured ball is removed", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;
      ball.age = testConfig.minAgeForCapture + 0.1;
      ball.pos = normalize(vec3(0, 1, 0));

      deepSpace.tick(0.01);
      expect(deepSpace.getBall(ballId)).toBeUndefined();
    });

    it("capture event contains ball data", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;
      ball.age = testConfig.minAgeForCapture + 0.1;
      ball.pos = normalize(vec3(0, 0, 1)); // Player 3's portal

      const captures = deepSpace.tick(0.01);
      expect(captures[0].ball.ownerId).toBe(1);
      expect(captures[0].player.id).toBe(3);
    });
  });

  describe("tick - captured balls are not rerouted", () => {
    it("captured ball axis is not mutated by reroute", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      // Set up for capture
      ball.age = testConfig.minAgeForCapture + 0.1;
      ball.pos = normalize(vec3(0, 1, 0));

      // Also set up conditions for reroute (shouldn't happen)
      ball.timeSinceHit = testConfig.rerouteAfter + 1;
      ball.rerouteCooldown = 0;

      const axisBefore = { ...ball.axis };
      const captures = deepSpace.tick(0.01);

      // Ball was captured
      expect(captures.length).toBe(1);
      // Axis should not have been mutated by reroute
      expect(captures[0].ball.axis.x).toBeCloseTo(axisBefore.x, 9);
      expect(captures[0].ball.axis.y).toBeCloseTo(axisBefore.y, 9);
      expect(captures[0].ball.axis.z).toBeCloseTo(axisBefore.z, 9);
    });
  });

  describe("reroute", () => {
    it("ball is rerouted after rerouteAfter seconds", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      // Set up for reroute: age >= 2.0 AND timeSinceHit >= rerouteAfter
      ball.age = testConfig.rerouteAfter + 1;
      ball.timeSinceHit = testConfig.rerouteAfter + 1;
      ball.rerouteCooldown = 0;
      // Move ball away from any portal to avoid capture
      ball.pos = normalize(vec3(0.5, 0.5, 0.707));

      const axisBefore = { ...ball.axis };
      deepSpace.tick(0.01);

      // Axis should have changed (rerouted)
      const axisChanged =
        ball.axis.x !== axisBefore.x ||
        ball.axis.y !== axisBefore.y ||
        ball.axis.z !== axisBefore.z;
      expect(axisChanged).toBe(true);
    });

    it("reroute sets cooldown", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      ball.age = testConfig.rerouteAfter + 1;
      ball.timeSinceHit = testConfig.rerouteAfter + 1;
      ball.rerouteCooldown = 0;

      deepSpace.tick(0.01);
      expect(ball.rerouteCooldown).toBeGreaterThan(0);
    });

    it("reroute resets timeSinceHit", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      ball.age = testConfig.rerouteAfter + 1;
      ball.timeSinceHit = testConfig.rerouteAfter + 1;
      ball.rerouteCooldown = 0;

      deepSpace.tick(0.01);
      expect(ball.timeSinceHit).toBeLessThan(1);
    });
  });

  describe("getBalls", () => {
    it("returns all balls", () => {
      deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      deepSpace.addBall(2, vec3(0, 1, 0), 0, 1);

      const balls = deepSpace.getBalls();
      expect(balls.length).toBe(2);
    });

    it("returns empty array when no balls", () => {
      expect(deepSpace.getBalls()).toEqual([]);
    });
  });

  describe("getCaptureVelocity2D", () => {
    it("returns velocity with correct magnitude", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      const speed2D = 2.5;
      const [vx, vy] = deepSpace.getCaptureVelocity2D(
        ball,
        vec3(0, 1, 0),
        speed2D,
      );

      const actualSpeed = Math.sqrt(vx * vx + vy * vy);
      expect(actualSpeed).toBeCloseTo(speed2D, 6);
    });
  });
});
