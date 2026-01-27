import { describe, it, expect } from "vitest";
import { fibonacciSphere, PortalPlacement } from "../src/shared/sphere";
import { length, dot } from "../src/shared/vec3";

describe("fibonacciSphere", () => {
  it("generates correct number of points", () => {
    const points = fibonacciSphere(100);
    expect(points.length).toBe(100);
  });

  it("all points are unit vectors", () => {
    const points = fibonacciSphere(50);
    for (const p of points) {
      expect(length(p)).toBeCloseTo(1, 9);
    }
  });

  it("points are reasonably distributed (no clustering)", () => {
    const points = fibonacciSphere(100);

    // Check that no two points are too close together
    // With 100 points on unit sphere, average spacing is about 0.35 rad
    const minExpectedDist = 0.1; // Very conservative threshold

    for (let i = 0; i < points.length; i++) {
      for (let j = i + 1; j < points.length; j++) {
        const d = dot(points[i], points[j]);
        const angularDist = Math.acos(Math.max(-1, Math.min(1, d)));
        expect(angularDist).toBeGreaterThan(minExpectedDist);
      }
    }
  });

  it("covers both hemispheres", () => {
    const points = fibonacciSphere(100);

    let hasPositiveZ = false;
    let hasNegativeZ = false;

    for (const p of points) {
      if (p.z > 0.5) hasPositiveZ = true;
      if (p.z < -0.5) hasNegativeZ = true;
    }

    expect(hasPositiveZ).toBe(true);
    expect(hasNegativeZ).toBe(true);
  });
});

describe("PortalPlacement", () => {
  it("allocates unique cell indices", () => {
    const placement = new PortalPlacement(100);
    const allocated = new Set<number>();

    for (let i = 0; i < 50; i++) {
      const idx = placement.allocate();
      expect(idx).toBeGreaterThanOrEqual(0);
      expect(allocated.has(idx)).toBe(false);
      allocated.add(idx);
    }
  });

  it("returns -1 when all cells are allocated", () => {
    const placement = new PortalPlacement(10);

    for (let i = 0; i < 10; i++) {
      const idx = placement.allocate();
      expect(idx).toBeGreaterThanOrEqual(0);
    }

    // 11th allocation should fail
    expect(placement.allocate()).toBe(-1);
  });

  it("portalPos returns unit vector", () => {
    const placement = new PortalPlacement(100);
    const idx = placement.allocate();
    const pos = placement.portalPos(idx);
    expect(length(pos)).toBeCloseTo(1, 9);
  });

  it("availableCount decreases with allocation", () => {
    const placement = new PortalPlacement(100);
    expect(placement.availableCount).toBe(100);

    placement.allocate();
    expect(placement.availableCount).toBe(99);

    placement.allocate();
    expect(placement.availableCount).toBe(98);
  });

  it("totalCount returns cell count", () => {
    const placement = new PortalPlacement(200);
    expect(placement.totalCount).toBe(200);
  });

  it("shuffle distributes first allocations across sphere", () => {
    // This tests that first N allocations are spread out (not clustered at north pole)
    const placement = new PortalPlacement(1000);
    const positions = [];

    // Allocate first 10 players
    for (let i = 0; i < 10; i++) {
      const idx = placement.allocate();
      positions.push(placement.portalPos(idx));
    }

    // Check that z-coordinates are spread out (not all near 1.0)
    const zValues = positions.map((p) => p.z);
    const minZ = Math.min(...zValues);
    const maxZ = Math.max(...zValues);

    // With shuffle, first 10 should span a good range of z values
    expect(maxZ - minZ).toBeGreaterThan(0.5);
  });

  it("resume token can reclaim released cell", () => {
    const placement = new PortalPlacement(100);
    const idx1 = placement.allocate("player-123");

    // Release the cell
    placement.release(idx1);

    // Allocate with same token - should get same cell back
    const idx2 = placement.allocate("player-123");
    expect(idx1).toBe(idx2);
  });

  it("different resume tokens get different indices", () => {
    const placement = new PortalPlacement(100);
    const idx1 = placement.allocate("player-1");
    const idx2 = placement.allocate("player-2");
    expect(idx1).not.toBe(idx2);
  });
});
