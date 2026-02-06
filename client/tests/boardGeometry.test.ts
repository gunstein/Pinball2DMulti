import { describe, it, expect } from "vitest";
import {
  flippers,
  guideWalls,
  bumpers,
  playfieldPolygon,
  escapeSlot,
  wallSegments,
  PLAYFIELD_CENTER_X,
} from "../src/board/BoardGeometry";
import {
  BOARD_CENTER_X,
  BOARD_CENTER_Y,
  BOARD_HALF_WIDTH,
} from "../src/constants";

describe("BoardGeometry symmetry", () => {
  const cx = BOARD_CENTER_X;
  const cy = BOARD_CENTER_Y;

  it("flippers are symmetric around board center X", () => {
    const left = flippers.find((f) => f.side === "left")!;
    const right = flippers.find((f) => f.side === "right")!;

    // Same Y position
    expect(left.pivot.y).toBe(right.pivot.y);

    // Symmetric X around center
    expect(left.pivot.x - cx).toBe(-(right.pivot.x - cx));

    // Same dimensions
    expect(left.length).toBe(right.length);
    expect(left.width).toBe(right.width);
    expect(left.pivotRadius).toBe(right.pivotRadius);
    expect(left.tipRadius).toBe(right.tipRadius);
  });

  it("guide walls are symmetric", () => {
    expect(guideWalls.length).toBe(2);
    const left = guideWalls[0];
    const right = guideWalls[1];

    // Same Y coordinates
    expect(left.from.y).toBe(right.from.y);
    expect(left.to.y).toBe(right.to.y);

    // End points at flipper pivot X positions (symmetric)
    const leftFlipper = flippers.find((f) => f.side === "left")!;
    const rightFlipper = flippers.find((f) => f.side === "right")!;
    expect(left.to.x).toBe(leftFlipper.pivot.x);
    expect(right.to.x).toBe(rightFlipper.pivot.x);
  });

  it("guide walls do not overlap with flippers", () => {
    const left = guideWalls[0];
    const right = guideWalls[1];
    const leftFlipper = flippers.find((f) => f.side === "left")!;
    const rightFlipper = flippers.find((f) => f.side === "right")!;

    // Guide wall ends should be above flipper top edge (pivot.y - pivotRadius)
    expect(left.to.y).toBeLessThan(
      leftFlipper.pivot.y - leftFlipper.pivotRadius,
    );
    expect(right.to.y).toBeLessThan(
      rightFlipper.pivot.y - rightFlipper.pivotRadius,
    );
  });

  it("escape slot is centered on board", () => {
    const slotCenter = (escapeSlot.xMin + escapeSlot.xMax) / 2;
    expect(slotCenter).toBe(cx);
  });

  it("playfield polygon forms a closed shape", () => {
    expect(playfieldPolygon.length).toBeGreaterThanOrEqual(4);
    // First and last points should share the same X or Y (rectangular-ish)
    const first = playfieldPolygon[0];
    const last = playfieldPolygon[playfieldPolygon.length - 1];
    expect(first.x).toBe(last.x); // both on left edge
  });

  it("PLAYFIELD_CENTER_X is between left wall and launcher wall", () => {
    const leftWall = cx - BOARD_HALF_WIDTH;
    expect(PLAYFIELD_CENTER_X).toBeGreaterThan(leftWall);
    expect(PLAYFIELD_CENTER_X).toBeLessThan(cx + BOARD_HALF_WIDTH);
  });

  it("wall segments have no zero-length walls", () => {
    for (const seg of wallSegments) {
      const dx = seg.to.x - seg.from.x;
      const dy = seg.to.y - seg.from.y;
      const length = Math.sqrt(dx * dx + dy * dy);
      expect(length).toBeGreaterThan(0);
    }
  });

  it("flipper pivotRadius is larger than tipRadius", () => {
    for (const f of flippers) {
      expect(f.pivotRadius).toBeGreaterThan(f.tipRadius);
    }
  });
});
