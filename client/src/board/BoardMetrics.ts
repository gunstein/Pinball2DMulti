/**
 * BoardMetrics - derived values for input zones and other game systems.
 * All values are computed from BoardGeometry to avoid hardcoded constants.
 */

import { CANVAS_HEIGHT } from "../constants";
import { flippers, launcherWall } from "./BoardGeometry";

/** Touch/input zone rectangle */
export interface InputZone {
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
}

// Derive flipper positions from BoardGeometry
const leftFlipper = flippers.find((f) => f.side === "left")!;
const rightFlipper = flippers.find((f) => f.side === "right")!;

/** Y position of flippers (from BoardGeometry) */
export const FLIPPER_Y = leftFlipper.pivot.y;

/** X position of left flipper pivot */
export const FLIPPER_LEFT_X = leftFlipper.pivot.x;

/** X position of right flipper pivot */
export const FLIPPER_RIGHT_X = rightFlipper.pivot.x;

/** X position of launcher lane inner wall */
export const LAUNCHER_LANE_X = launcherWall.from.x;

/** Center X between the two flippers (for dividing left/right touch zones) */
export const FLIPPER_CENTER_X = (FLIPPER_LEFT_X + FLIPPER_RIGHT_X) / 2;

/** Touch zone configuration */
export const inputZones = {
  /** Active touch zone starts this many pixels above flipper Y */
  activeZoneTopOffset: 100,

  /** Get the top Y of the active touch zone */
  get activeZoneTop(): number {
    return FLIPPER_Y - this.activeZoneTopOffset;
  },

  /** Bottom of active zone (bottom of canvas) */
  activeZoneBottom: CANVAS_HEIGHT,
};

/**
 * Determine which input zone a game coordinate falls into.
 * @param gameX - X coordinate in game space
 * @param gameY - Y coordinate in game space
 * @returns The zone type or "none" if outside active areas
 */
export function getInputZone(
  gameX: number,
  gameY: number,
): "left" | "right" | "launch" | "none" {
  // Touch zones are only active from flipper height down to bottom of board
  if (
    gameY < inputZones.activeZoneTop ||
    gameY > inputZones.activeZoneBottom
  ) {
    return "none";
  }

  // Launcher zone: right side of board (launcher lane area)
  if (gameX >= LAUNCHER_LANE_X) {
    return "launch";
  }

  // Flipper zones: left half vs right half of the main playfield
  return gameX < FLIPPER_CENTER_X ? "left" : "right";
}
