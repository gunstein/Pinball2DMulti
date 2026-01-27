import { describe, it, expect, beforeEach } from "vitest";
import { SphereDeepSpace } from "../src/shared/SphereDeepSpace";
import { Player, DeepSpaceConfig } from "../src/shared/types";
import {
  vec3,
  dot,
  length,
  normalize,
  angularDistance,
} from "../src/shared/vec3";
import { PortalPlacement } from "../src/shared/sphere";

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

    it("ball starts at portal position", () => {
      const portalPos = vec3(1, 0, 0);
      const ballId = deepSpace.addBall(1, portalPos, 1, 0);
      const ball = deepSpace.getBall(ballId);
      // Ball starts at portal (minAgeForCapture prevents instant re-capture)
      expect(dot(ball!.pos, portalPos)).toBeCloseTo(1, 6);
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
      // With 4 portals at (±1,0,0), (0,±1,0), (0,0,±1), use a diagonal
      ball.pos = normalize(vec3(1, 1, 1));

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

      ball.pos = normalize(vec3(1, 1, 1)); // away from all portals
      ball.age = testConfig.rerouteAfter + 1;
      ball.timeSinceHit = testConfig.rerouteAfter + 1;
      ball.rerouteCooldown = 0;

      deepSpace.tick(0.01);
      expect(ball.rerouteCooldown).toBeGreaterThan(0);
    });

    it("reroute resets timeSinceHit", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      ball.pos = normalize(vec3(1, 1, 1)); // away from all portals
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

  describe("edge cases", () => {
    it("reroute handles near-antiparallel pos and target (cross ≈ 0)", () => {
      // Only two players: self and one at antipode
      const antipodalPlayers: Player[] = [
        { id: 1, cellIndex: 0, portalPos: vec3(1, 0, 0), color: 0xff0000 },
        { id: 2, cellIndex: 1, portalPos: vec3(-1, 0, 0), color: 0x00ff00 },
      ];
      deepSpace.setPlayers(antipodalPlayers);

      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      // Place ball very close to player 1's portal (nearly antiparallel to player 2)
      ball.pos = normalize(vec3(0.999, 0.01, 0.01));
      ball.age = testConfig.rerouteAfter + 1;
      ball.timeSinceHit = testConfig.rerouteAfter + 1;
      ball.rerouteCooldown = 0;

      // Should not crash - should produce valid axis
      deepSpace.tick(0.01);

      // Ball should still be valid (unit pos, unit axis)
      expect(length(ball.pos)).toBeCloseTo(1, 6);
      expect(length(ball.axis)).toBeCloseTo(1, 6);
      expect(Number.isNaN(ball.pos.x)).toBe(false);
      expect(Number.isNaN(ball.axis.x)).toBe(false);
    });

    it("reroute handles ball very close to target (dot ≈ 1)", () => {
      // Player 2 at (0,1,0), place ball almost there
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      // Move ball to almost exactly player 2's position
      ball.pos = normalize(vec3(0.001, 0.9999, 0.001));
      ball.age = testConfig.rerouteAfter + 1;
      ball.timeSinceHit = testConfig.rerouteAfter + 1;
      ball.rerouteCooldown = 0;

      // Should not crash
      deepSpace.tick(0.01);

      expect(Number.isNaN(ball.pos.x)).toBe(false);
      expect(Number.isNaN(ball.axis.x)).toBe(false);
      expect(length(ball.pos)).toBeCloseTo(1, 6);
    });

    it("capture at exact portal threshold (dot = cosPortalAlpha)", () => {
      const cosAlpha = Math.cos(testConfig.portalAlpha);
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      ball.age = testConfig.minAgeForCapture + 0.1;

      // Place ball exactly at the capture threshold for player 2 (0,1,0)
      // dot(ball.pos, (0,1,0)) = ball.pos.y = cosAlpha
      const sinAlpha = Math.sqrt(1 - cosAlpha * cosAlpha);
      ball.pos = normalize(vec3(sinAlpha, cosAlpha, 0));

      const captures = deepSpace.tick(0.001);
      // dot should be >= cosAlpha, so it should be captured
      expect(captures.length).toBe(1);
      expect(captures[0].playerId).toBe(2);
    });

    it("no capture just outside portal threshold", () => {
      const cosAlpha = Math.cos(testConfig.portalAlpha);
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 0);
      const ball = deepSpace.getBall(ballId)!;

      ball.age = testConfig.minAgeForCapture + 0.1;

      // Place ball just outside the capture threshold for player 2
      // Use a slightly larger angle than portalAlpha
      const outsideAngle = testConfig.portalAlpha + 0.05;
      const cosOutside = Math.cos(outsideAngle);
      const sinOutside = Math.sqrt(1 - cosOutside * cosOutside);
      ball.pos = normalize(vec3(sinOutside, cosOutside, 0));

      const captures = deepSpace.tick(0.001);
      expect(captures.length).toBe(0);
    });

    it("addBall with zero velocity produces valid ball", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 0, 0);
      const ball = deepSpace.getBall(ballId)!;

      expect(length(ball.pos)).toBeCloseTo(1, 6);
      expect(length(ball.axis)).toBeCloseTo(1, 6);
      expect(Number.isNaN(ball.omega)).toBe(false);
    });

    it("ball stays on unit sphere after many ticks", () => {
      const ballId = deepSpace.addBall(1, vec3(1, 0, 0), 1, 1);
      const ball = deepSpace.getBall(ballId)!;

      // Run 1000 ticks
      for (let i = 0; i < 1000; i++) {
        deepSpace.tick(0.016);
      }

      // If ball was captured it's gone, but if still alive it should be on sphere
      const remaining = deepSpace.getBall(ballId);
      if (remaining) {
        expect(length(remaining.pos)).toBeCloseTo(1, 6);
        expect(Number.isNaN(remaining.pos.x)).toBe(false);
      }
    });
  });
});

describe("SphereDeepSpace - end-to-end pipeline", () => {
  it("escape → travel → capture → velocity", () => {
    // Minimal config for predictable test
    const config: DeepSpaceConfig = {
      portalAlpha: 0.1,
      omegaMin: 1.0,
      omegaMax: 1.0,
      rerouteAfter: 100, // Disable reroute for this test
      rerouteCooldown: 100,
      minAgeForCapture: 0.1,
    };

    const deepSpace = new SphereDeepSpace(config);

    // Two players opposite each other
    const p1Pos = vec3(1, 0, 0);
    const p2Pos = vec3(-1, 0, 0);
    const players: Player[] = [
      { id: 1, cellIndex: 0, portalPos: p1Pos, color: 0xff0000 },
      { id: 2, cellIndex: 1, portalPos: p2Pos, color: 0x00ff00 },
    ];
    deepSpace.setPlayers(players);

    // Player 1 escapes a ball
    const ballId = deepSpace.addBall(1, p1Pos, 0, 1);
    expect(deepSpace.getBalls().length).toBe(1);

    // Simulate until capture or timeout
    let captured = false;
    let captureEvent = null;
    const maxTicks = 10000;
    for (let i = 0; i < maxTicks; i++) {
      const captures = deepSpace.tick(1 / 60);
      if (captures.length > 0) {
        captureEvent = captures[0];
        captured = true;
        break;
      }
    }

    // Ball should eventually be captured (by player 1 or 2)
    expect(captured).toBe(true);
    expect(captureEvent).not.toBeNull();

    // Ball should be removed from deep space
    expect(deepSpace.getBalls().length).toBe(0);

    // Get capture velocity - should have correct magnitude
    const speed2D = 2.0;
    const [vx, vy] = deepSpace.getCaptureVelocity2D(
      captureEvent!.ball,
      captureEvent!.player.portalPos,
      speed2D,
    );

    const actualSpeed = Math.sqrt(vx * vx + vy * vy);
    expect(actualSpeed).toBeCloseTo(speed2D, 4);
    expect(Number.isNaN(vx)).toBe(false);
    expect(Number.isNaN(vy)).toBe(false);
  });
});

describe("SphereDeepSpace - sanity long-run", () => {
  it("300 players, 200 balls, 60 seconds - no NaN, no explosion", () => {
    const config: DeepSpaceConfig = {
      portalAlpha: 0.15,
      omegaMin: 0.5,
      omegaMax: 1.0,
      rerouteAfter: 12.0,
      rerouteCooldown: 6.0,
      minAgeForCapture: 3.0,
    };

    const deepSpace = new SphereDeepSpace(config);

    // Generate 300 players on fibonacci sphere
    const placement = new PortalPlacement(2048);
    const players: Player[] = [];
    for (let i = 1; i <= 300; i++) {
      const cellIndex = placement.allocate();
      players.push({
        id: i,
        cellIndex,
        portalPos: placement.portalPos(cellIndex),
        color: 0xffffff,
      });
    }
    deepSpace.setPlayers(players);

    // Add 200 balls from random players
    for (let i = 0; i < 200; i++) {
      const owner = players[i % players.length];
      deepSpace.addBall(
        owner.id,
        owner.portalPos,
        Math.random() * 2 - 1,
        Math.random() * 2 - 1,
      );
    }

    expect(deepSpace.getBalls().length).toBe(200);

    // Simulate 60 seconds at 60 Hz
    let totalCaptures = 0;
    const dt = 1 / 60;
    const totalTicks = 60 * 60; // 60 seconds

    const startTime = performance.now();

    for (let i = 0; i < totalTicks; i++) {
      const captures = deepSpace.tick(dt);
      totalCaptures += captures.length;

      // Verify no NaN in capture events
      for (const cap of captures) {
        expect(Number.isNaN(cap.ball.pos.x)).toBe(false);
        expect(Number.isNaN(cap.ball.pos.y)).toBe(false);
        expect(Number.isNaN(cap.ball.pos.z)).toBe(false);
      }
    }

    const elapsed = performance.now() - startTime;

    // All remaining balls should have valid positions
    for (const ball of deepSpace.getBalls()) {
      expect(Number.isNaN(ball.pos.x)).toBe(false);
      expect(Number.isNaN(ball.pos.y)).toBe(false);
      expect(Number.isNaN(ball.pos.z)).toBe(false);
      expect(length(ball.pos)).toBeCloseTo(1, 3);
      expect(Number.isNaN(ball.axis.x)).toBe(false);
      expect(Number.isNaN(ball.omega)).toBe(false);
    }

    // Some balls should have been captured (with 300 players)
    expect(totalCaptures).toBeGreaterThan(0);

    // Remaining balls + captured should account for all 200
    expect(deepSpace.getBalls().length + totalCaptures).toBe(200);

    // Should finish in reasonable time (< 5 seconds for 60s sim)
    expect(elapsed).toBeLessThan(5000);
  });
});
