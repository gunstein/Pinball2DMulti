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
  position: Vec2;
  pivot: Vec2;
  length: number;
  width: number;
  side: "left" | "right";
}

// All positions in pixel coordinates
const cx = BOARD_CENTER_X;
const cy = BOARD_CENTER_Y;
const hw = BOARD_HALF_WIDTH; // 175
const hh = BOARD_HALF_HEIGHT; // 320

// Launcher lane width
const LANE_WIDTH = 50;
const LANE_INNER_X = cx + hw - LANE_WIDTH; // inner wall of lane

// Chamfer in upper right corner
const CHAMFER_SIZE = 60;

// Flipper area
const FLIPPER_Y = cy + hh - 70;
const DRAIN_GAP_HALF = 40; // half-width of drain opening between flippers

// Playfield polygon (the outer wall shape, clockwise from top-left)
export const playfieldPolygon: Vec2[] = [
  { x: cx - hw, y: cy - hh }, // top-left
  { x: cx + hw - CHAMFER_SIZE, y: cy - hh }, // top-right before chamfer
  { x: cx + hw, y: cy - hh + CHAMFER_SIZE }, // chamfer end (right side)
  { x: cx + hw, y: cy + hh }, // bottom-right
  { x: cx - hw, y: cy + hh }, // bottom-left
];

// Wall segments derived from polygon
export const wallSegments: Segment[] = [
  // Left wall (top-left to bottom-left)
  { from: playfieldPolygon[0], to: playfieldPolygon[4] },
  // Top wall (top-left to top-right before chamfer)
  { from: playfieldPolygon[0], to: playfieldPolygon[1] },
  // Chamfer (upper right diagonal)
  { from: playfieldPolygon[1], to: playfieldPolygon[2] },
  // Right wall (chamfer end to bottom-right)
  { from: playfieldPolygon[2], to: playfieldPolygon[3] },
  // Bottom wall - left segment (bottom-left to drain gap left)
  { from: playfieldPolygon[4], to: { x: cx - DRAIN_GAP_HALF, y: cy + hh } },
  // Bottom wall - right segment (drain gap right to bottom-right)
  { from: { x: cx + DRAIN_GAP_HALF, y: cy + hh }, to: playfieldPolygon[3] },
];

// Guide walls - go from the outer walls down to meet the flipper pivots
// These prevent the ball from going around the sides of the flippers
export const guideWalls: Segment[] = [
  // Left guide: from left wall down to left flipper pivot area
  {
    from: { x: cx - hw, y: FLIPPER_Y - 60 },
    to: { x: cx - hw + 55, y: FLIPPER_Y },
  },
  // Right guide: from launcher wall inner side down to right flipper pivot area
  {
    from: { x: LANE_INNER_X, y: FLIPPER_Y - 60 },
    to: { x: LANE_INNER_X - 55, y: FLIPPER_Y },
  },
];

// Launcher lane wall (vertical separator)
// Starts at mid-height and goes down to meet the right guide wall
export const launcherWall: Segment = {
  from: { x: LANE_INNER_X, y: cy },
  to: { x: LANE_INNER_X, y: FLIPPER_Y - 60 }, // meets the right guide wall top
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
    position: { x: cx - 55, y: FLIPPER_Y },
    pivot: { x: cx - 55 - 35, y: FLIPPER_Y },
    length: 70,
    width: 12,
    side: "left",
  },
  {
    position: { x: cx + 55, y: FLIPPER_Y },
    pivot: { x: cx + 55 + 35, y: FLIPPER_Y },
    length: 70,
    width: 12,
    side: "right",
  },
];

// Ball spawn position (in launcher lane, resting on the plunger)
export const ballSpawn: Vec2 = {
  x: cx + hw - LANE_WIDTH / 2,
  y: FLIPPER_Y - 50, // in the lane, above the guide wall
};

// Drain sensor position (below the drain gap)
export const drainPosition: Vec2 = {
  x: cx,
  y: cy + hh + 30,
};
export const DRAIN_WIDTH = DRAIN_GAP_HALF * 2 + 40;
