import { describe, it, expect } from "vitest";
import {
  stepFlipperAngle,
  restAngle,
  activeAngle,
  MAX_ANGLE,
} from "../src/board/flipperLogic";

describe("Flipper angle logic", () => {
  const dt = 1 / 120;

  describe("left flipper", () => {
    it("rotates towards activeAngle when active", () => {
      const rest = restAngle("left");
      const angle = stepFlipperAngle(rest, dt, true, "left");
      expect(angle).toBeLessThan(rest);
    });

    it("converges to activeAngle when held", () => {
      let angle = restAngle("left");
      for (let i = 0; i < 200; i++) {
        angle = stepFlipperAngle(angle, dt, true, "left");
      }
      expect(angle).toBe(activeAngle("left"));
    });

    it("converges to restAngle when released", () => {
      let angle = activeAngle("left");
      for (let i = 0; i < 200; i++) {
        angle = stepFlipperAngle(angle, dt, false, "left");
      }
      expect(angle).toBe(restAngle("left"));
    });

    it("does not overshoot activeAngle", () => {
      let angle = restAngle("left");
      for (let i = 0; i < 200; i++) {
        angle = stepFlipperAngle(angle, dt, true, "left");
        expect(angle).toBeGreaterThanOrEqual(activeAngle("left"));
      }
    });

    it("does not overshoot restAngle", () => {
      let angle = activeAngle("left");
      for (let i = 0; i < 200; i++) {
        angle = stepFlipperAngle(angle, dt, false, "left");
        expect(angle).toBeLessThanOrEqual(restAngle("left"));
      }
    });
  });

  describe("right flipper", () => {
    it("rotates towards activeAngle when active", () => {
      const rest = restAngle("right");
      const angle = stepFlipperAngle(rest, dt, true, "right");
      expect(angle).toBeGreaterThan(rest);
    });

    it("converges to activeAngle when held", () => {
      let angle = restAngle("right");
      for (let i = 0; i < 200; i++) {
        angle = stepFlipperAngle(angle, dt, true, "right");
      }
      expect(angle).toBe(activeAngle("right"));
    });

    it("converges to restAngle when released", () => {
      let angle = activeAngle("right");
      for (let i = 0; i < 200; i++) {
        angle = stepFlipperAngle(angle, dt, false, "right");
      }
      expect(angle).toBe(restAngle("right"));
    });

    it("does not overshoot activeAngle", () => {
      let angle = restAngle("right");
      for (let i = 0; i < 200; i++) {
        angle = stepFlipperAngle(angle, dt, true, "right");
        expect(angle).toBeLessThanOrEqual(activeAngle("right"));
      }
    });

    it("does not overshoot restAngle", () => {
      let angle = activeAngle("right");
      for (let i = 0; i < 200; i++) {
        angle = stepFlipperAngle(angle, dt, false, "right");
        expect(angle).toBeGreaterThanOrEqual(restAngle("right"));
      }
    });
  });

  describe("symmetry", () => {
    it("left and right active angles are symmetric", () => {
      expect(activeAngle("left")).toBe(-MAX_ANGLE);
      expect(activeAngle("right")).toBe(MAX_ANGLE);
    });

    it("left and right rest angles are symmetric", () => {
      expect(restAngle("left")).toBe(MAX_ANGLE);
      expect(restAngle("right")).toBe(-MAX_ANGLE);
    });

    it("both sides reach target in same number of steps", () => {
      let leftAngle = restAngle("left");
      let rightAngle = restAngle("right");
      let leftSteps = 0;
      let rightSteps = 0;

      while (leftAngle !== activeAngle("left") && leftSteps < 500) {
        leftAngle = stepFlipperAngle(leftAngle, dt, true, "left");
        leftSteps++;
      }
      while (rightAngle !== activeAngle("right") && rightSteps < 500) {
        rightAngle = stepFlipperAngle(rightAngle, dt, true, "right");
        rightSteps++;
      }

      expect(leftSteps).toBe(rightSteps);
    });
  });
});
