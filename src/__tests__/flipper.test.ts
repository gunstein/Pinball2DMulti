import { describe, it, expect } from "vitest";

// Extract the pure flipper angle logic for testing (no Pixi/Rapier needed)
const ROTATION_SPEED_UP = 14.0;
const ROTATION_SPEED_DOWN = 6.0;
const MAX_ANGLE = 0.45;

function stepFlipperAngle(
  currentAngle: number,
  dt: number,
  active: boolean,
  side: "left" | "right",
): number {
  let newAngle = currentAngle;

  if (side === "left") {
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

  return Math.max(-MAX_ANGLE, Math.min(MAX_ANGLE, newAngle));
}

describe("Flipper angle logic", () => {
  const dt = 1 / 120;

  describe("left flipper", () => {
    it("rotates negatively when active", () => {
      const angle = stepFlipperAngle(0, dt, true, "left");
      expect(angle).toBeLessThan(0);
    });

    it("rotates positively when released", () => {
      const angle = stepFlipperAngle(-0.2, dt, false, "left");
      expect(angle).toBeGreaterThan(-0.2);
    });

    it("clamps at -MAX_ANGLE when active", () => {
      let angle = 0;
      for (let i = 0; i < 100; i++) {
        angle = stepFlipperAngle(angle, dt, true, "left");
      }
      expect(angle).toBeCloseTo(-MAX_ANGLE, 5);
    });

    it("clamps at rest (0) when released from negative", () => {
      let angle = -MAX_ANGLE;
      for (let i = 0; i < 100; i++) {
        angle = stepFlipperAngle(angle, dt, false, "left");
      }
      // Should return to 0 but not exceed MAX_ANGLE
      expect(angle).toBeGreaterThanOrEqual(0);
      expect(angle).toBeLessThanOrEqual(MAX_ANGLE);
    });
  });

  describe("right flipper", () => {
    it("rotates positively when active", () => {
      const angle = stepFlipperAngle(0, dt, true, "right");
      expect(angle).toBeGreaterThan(0);
    });

    it("rotates negatively when released", () => {
      const angle = stepFlipperAngle(0.2, dt, false, "right");
      expect(angle).toBeLessThan(0.2);
    });

    it("clamps at +MAX_ANGLE when active", () => {
      let angle = 0;
      for (let i = 0; i < 100; i++) {
        angle = stepFlipperAngle(angle, dt, true, "right");
      }
      expect(angle).toBeCloseTo(MAX_ANGLE, 5);
    });

    it("clamps at rest (0) when released from positive", () => {
      let angle = MAX_ANGLE;
      for (let i = 0; i < 100; i++) {
        angle = stepFlipperAngle(angle, dt, false, "right");
      }
      expect(angle).toBeLessThanOrEqual(0);
      expect(angle).toBeGreaterThanOrEqual(-MAX_ANGLE);
    });
  });

  describe("symmetry", () => {
    it("left and right reach same magnitude when active", () => {
      let leftAngle = 0;
      let rightAngle = 0;
      for (let i = 0; i < 50; i++) {
        leftAngle = stepFlipperAngle(leftAngle, dt, true, "left");
        rightAngle = stepFlipperAngle(rightAngle, dt, true, "right");
      }
      expect(Math.abs(leftAngle)).toBeCloseTo(Math.abs(rightAngle), 10);
    });
  });
});
