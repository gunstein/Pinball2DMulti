import {
  BOARD_CENTER_X,
  BOARD_CENTER_Y,
  BOARD_HALF_WIDTH,
  BOARD_HALF_HEIGHT,
} from "../constants";

export interface Vec2 {
  x: number;
  y: number;
}

export interface Segment {
  from: Vec2;
  to: Vec2;
}

export interface CircleDef {
  center: Vec2;
  radius: number;
}

export interface FlipperDef {
  pivot: Vec2;
  length: number;
  width: number;
  pivotRadius: number;
  tipRadius: number;
  side: "left" | "right";
}

// All positions in pixel coordinates
const cx = BOARD_CENTER_X;
const cy = BOARD_CENTER_Y;
const hw = BOARD_HALF_WIDTH; // 175
const hh = BOARD_HALF_HEIGHT; // 320

// Launcher lane width
const LANE_WIDTH = 35;
const LANE_INNER_X = cx + hw - LANE_WIDTH; // inner wall of lane

// Playfield center X (excluding launcher lane, for deep space centering)
// Nudged slightly right since launcher wall only covers lower half
export const PLAYFIELD_CENTER_X = (cx - hw + LANE_INNER_X) / 2 + 14;

// Chamfer in upper right corner
const CHAMFER_SIZE = 60;

// Flipper area
const FLIPPER_Y = cy + hh - 70;

// Useful derived Y coordinates
const BOTTOM_Y = cy + hh;

// Shooter lane: where the ball should rest (top of the little "stop" block)
const LAUNCHER_STOP_Y = BOTTOM_Y - 20;

// Playfield polygon (the outer wall shape, clockwise from top-left)
export const playfieldPolygon: Vec2[] = [
  { x: cx - hw, y: cy - hh }, // top-left
  { x: cx + hw - CHAMFER_SIZE, y: cy - hh }, // top-right before chamfer
  { x: cx + hw, y: cy - hh + CHAMFER_SIZE }, // chamfer end (right side)
  { x: cx + hw, y: cy + hh }, // bottom-right
  { x: cx - hw, y: cy + hh }, // bottom-left
];

// Escape slot: centered opening in the top wall (balls exit here to deep space)
const ESCAPE_SLOT_WIDTH = 180;
const ESCAPE_SLOT_LEFT_X = cx - ESCAPE_SLOT_WIDTH / 2;
const ESCAPE_SLOT_RIGHT_X = cx + ESCAPE_SLOT_WIDTH / 2;

// Wall segments derived from polygon
export const wallSegments: Segment[] = [
  // Left wall (top-left to bottom-left)
  { from: playfieldPolygon[0], to: playfieldPolygon[4] },
  // Top wall LEFT part (top-left to escape slot left edge)
  { from: playfieldPolygon[0], to: { x: ESCAPE_SLOT_LEFT_X, y: cy - hh } },
  // ESCAPE SLOT (open - no wall)
  // Top wall RIGHT part (escape slot right edge to chamfer)
  { from: { x: ESCAPE_SLOT_RIGHT_X, y: cy - hh }, to: playfieldPolygon[1] },
  // Chamfer (upper right diagonal)
  { from: playfieldPolygon[1], to: playfieldPolygon[2] },
  // Right wall (chamfer end to bottom-right)
  { from: playfieldPolygon[2], to: playfieldPolygon[3] },
  // Bottom wall - solid (ball hitting this triggers respawn)
  { from: playfieldPolygon[4], to: playfieldPolygon[3] },
];

// Guide walls near the flippers.
// Purpose: funnel the ball down towards the flippers from higher up.
// Start from the side walls well above flipper level, angle inward to flipper pivots.
export const guideWalls: Segment[] = [
  // Left guide: from left wall, funnels to left flipper pivot area
  {
    from: { x: cx - hw, y: FLIPPER_Y - 80 },
    to: { x: cx - 90, y: FLIPPER_Y - 14 },
  },
  // Right guide: from launcher wall, funnels to right flipper pivot area
  {
    from: { x: LANE_INNER_X, y: FLIPPER_Y - 80 },
    to: { x: cx + 90, y: FLIPPER_Y - 14 },
  },
];

// Launcher lane wall (vertical separator)
// Extends to the bottom so the ball can't drop out of the shooter lane
export const launcherWall: Segment = {
  from: { x: LANE_INNER_X, y: cy },
  to: { x: LANE_INNER_X, y: BOTTOM_Y },
};

// Shooter-lane stop (horizontal "floor" that the ball rests on)
export const launcherStop: Segment = {
  from: { x: LANE_INNER_X, y: LAUNCHER_STOP_Y },
  to: { x: cx + hw, y: LAUNCHER_STOP_Y },
};

// Wall/stroke thickness for rendering
export const WALL_STROKE_WIDTH = 3;
export const LAUNCHER_WALL_THICKNESS = 6;

// Bumpers/pins (3 circles) - spread out more
export const bumpers: CircleDef[] = [
  { center: { x: cx - 70, y: cy - 120 }, radius: 25 }, // upper-left
  { center: { x: cx + 50, y: cy - 150 }, radius: 25 }, // upper-right
  { center: { x: cx - 20, y: cy + 10 }, radius: 25 }, // lower-middle
];

// Flippers
// Positioned so inner ends leave a drain gap of ~30px between them
// Left flipper: pivot at left end, swings right end up
// Right flipper: pivot at right end, swings left end up
export const flippers: FlipperDef[] = [
  {
    pivot: { x: cx - 90, y: FLIPPER_Y },
    length: 78,
    width: 12,
    pivotRadius: 10,
    tipRadius: 4,
    side: "left",
  },
  {
    pivot: { x: cx + 90, y: FLIPPER_Y },
    length: 78,
    width: 12,
    pivotRadius: 10,
    tipRadius: 4,
    side: "right",
  },
];

// Ball spawn position (in launcher lane, resting just above the stop)
export const ballSpawn: Vec2 = {
  x: cx + hw - LANE_WIDTH / 2,
  y: LAUNCHER_STOP_Y - 12,
};

// Bottom wall is the drain - its index in wallSegments
// (0: left, 1: top-left, 2: top-right, 3: chamfer, 4: right, 5: bottom)
export const BOTTOM_WALL_INDEX = 5;

// Escape slot bounds (used for detecting when a ball leaves the board)
export const escapeSlot = {
  xMin: ESCAPE_SLOT_LEFT_X,
  xMax: ESCAPE_SLOT_RIGHT_X,
  yTop: cy - hh - 5,
  yBottom: cy - hh,
};

/** Check if a pixel position is inside the escape slot area */
export function isInEscapeSlot(px: number, py: number): boolean {
  return (
    px >= escapeSlot.xMin &&
    px <= escapeSlot.xMax &&
    py >= escapeSlot.yTop &&
    py <= escapeSlot.yBottom
  );
}
