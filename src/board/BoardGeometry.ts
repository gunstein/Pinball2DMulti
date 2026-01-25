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

// Escape slot position on top wall (opening to deep-space)
// Åpningen er sentrert på midten av toppen
const ESCAPE_SLOT_WIDTH = 180; // bredde på åpningen
const ESCAPE_SLOT_LEFT_X = cx - ESCAPE_SLOT_WIDTH / 2; // venstre kant
const ESCAPE_SLOT_RIGHT_X = cx + ESCAPE_SLOT_WIDTH / 2; // høyre kant

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
// Purpose: prevent the ball from slipping around the outside of a flipper.
// Start from the side walls above flipper level, angle down to the upper part of the flipper.
export const guideWalls: Segment[] = [
  // Left guide: from left wall, gentle slope to left flipper pivot area
  {
    from: { x: cx - hw, y: FLIPPER_Y - 35 },
    to: { x: cx - 90, y: FLIPPER_Y - 5 },
  },
  // Right guide: from launcher wall, gentle slope inward
  {
    from: { x: LANE_INNER_X, y: FLIPPER_Y - 35 },
    to: { x: LANE_INNER_X - 50, y: FLIPPER_Y - 10 },
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
    side: "left",
  },
  {
    pivot: { x: cx + 90, y: FLIPPER_Y },
    length: 78,
    width: 12,
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

// Escape boundary: ball is "escaped" when outside this AABB (pixels)
const ESCAPE_MARGIN = 30;
export const escapeBounds = {
  left: cx - hw - ESCAPE_MARGIN,
  right: cx + hw + ESCAPE_MARGIN,
  top: cy - hh - ESCAPE_MARGIN,
  bottom: cy + hh + ESCAPE_MARGIN,
};

// Atmosphere boundary: deep-space balls inside this AABB can be captured (pixels)
const ATMOSPHERE_MARGIN = 50;
export const atmosphereBounds = {
  left: cx - hw - ATMOSPHERE_MARGIN,
  right: cx + hw + ATMOSPHERE_MARGIN,
  top: cy - hh - ATMOSPHERE_MARGIN,
  bottom: cy + hh + ATMOSPHERE_MARGIN,
};

// ============================================================
// ESCAPE SLOT - definert åpning i toppen for escape til deep-space
// ============================================================

// Escape-slot: åpning i toppveggen (sentrert)
// Slot-området er et smalt bånd rundt toppkanten (y = 30)
// yTop må være høyere enn y=30 for å unngå feilaktig capture
export const escapeSlot = {
  xMin: ESCAPE_SLOT_LEFT_X,
  xMax: ESCAPE_SLOT_RIGHT_X,
  yTop: cy - hh - 5, // øvre grense (5px over toppkanten = y=25)
  yBottom: cy - hh + 20, // nedre grense (20px under toppkanten = y=50)
};

/** Sjekk om en posisjon (i piksel) er i escape-slot området */
export function isInEscapeSlot(px: number, py: number): boolean {
  return (
    px >= escapeSlot.xMin &&
    px <= escapeSlot.xMax &&
    py >= escapeSlot.yTop &&
    py <= escapeSlot.yBottom
  );
}

// ============================================================
// PLANETRING - sirkulær ring rundt brettet med port ved escape-slot
// ============================================================

export const PLANET_RING_RADIUS = 420; // piksel fra senter
export const PLANET_RING_SEGMENTS = 48; // antall segmenter i ringen
export const PLANET_RING_CENTER = { x: cx, y: cy };

// Beregn vinkler for escape-slot (porten i ringen)
// Vinkel 0 er høyre (positiv x), vinkel går mot klokken
// Bruker toppkanten (cy - hh) som referanse for vinkelberegning
const slotY = cy - hh;
const slotLeftAngle = Math.atan2(slotY - cy, escapeSlot.xMin - cx);
const slotRightAngle = Math.atan2(slotY - cy, escapeSlot.xMax - cx);

// Port-vinkelområde (normalisert til [-PI, PI])
// Legg til litt margin på begge sider
const PORT_ANGLE_MARGIN = 0.15; // radianer

/** Sjekk om en vinkel er innenfor porten (escape-slot området) */
function isAngleInPort(angle: number): boolean {
  // Normaliser vinkelen til [-PI, PI]
  while (angle > Math.PI) angle -= 2 * Math.PI;
  while (angle < -Math.PI) angle += 2 * Math.PI;

  // slotLeftAngle er mer negativ (ca -100°), slotRightAngle er mindre negativ (ca -79°)
  // Så porten går fra slotLeftAngle til slotRightAngle
  const minAngle = slotLeftAngle - PORT_ANGLE_MARGIN;
  const maxAngle = slotRightAngle + PORT_ANGLE_MARGIN;

  return angle >= minAngle && angle <= maxAngle;
}

/** Generer planetring-segmenter (med port-åpning) */
export function generatePlanetRingSegments(): Segment[] {
  const segments: Segment[] = [];
  const angleStep = (2 * Math.PI) / PLANET_RING_SEGMENTS;

  for (let i = 0; i < PLANET_RING_SEGMENTS; i++) {
    const startAngle = i * angleStep - Math.PI;
    const endAngle = startAngle + angleStep;
    const midAngle = (startAngle + endAngle) / 2;

    // Skip segmenter som overlapper med porten
    if (isAngleInPort(midAngle)) continue;

    segments.push({
      from: {
        x: cx + PLANET_RING_RADIUS * Math.cos(startAngle),
        y: cy + PLANET_RING_RADIUS * Math.sin(startAngle),
      },
      to: {
        x: cx + PLANET_RING_RADIUS * Math.cos(endAngle),
        y: cy + PLANET_RING_RADIUS * Math.sin(endAngle),
      },
    });
  }
  return segments;
}
