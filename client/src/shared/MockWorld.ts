/**
 * MockWorld - simulates multiple players for local testing.
 * Uses sphere-based portal placement.
 */

import { Player, DeepSpaceConfig, DEFAULT_DEEP_SPACE_CONFIG } from "./types";
import { PortalPlacement } from "./sphere";

/**
 * Generate a color from player ID using golden angle hue distribution.
 */
export function colorFromId(id: number): number {
  const hue = (id * 137) % 360;
  return hsvToRgb(hue, 0.55, 0.95);
}

function hsvToRgb(h: number, s: number, v: number): number {
  const c = v * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;

  let r = 0,
    g = 0,
    b = 0;
  if (h < 60) {
    r = c;
    g = x;
  } else if (h < 120) {
    r = x;
    g = c;
  } else if (h < 180) {
    g = c;
    b = x;
  } else if (h < 240) {
    g = x;
    b = c;
  } else if (h < 300) {
    r = x;
    b = c;
  } else {
    r = c;
    b = x;
  }

  const ri = Math.round((r + m) * 255);
  const gi = Math.round((g + m) * 255);
  const bi = Math.round((b + m) * 255);

  return (ri << 16) | (gi << 8) | bi;
}

/**
 * Mock world for local testing.
 */
export class MockWorld {
  readonly config: DeepSpaceConfig;
  readonly placement: PortalPlacement;
  readonly selfId: number = 1;
  readonly players: Player[] = [];

  constructor(
    playerCount: number = 50,
    config: DeepSpaceConfig = DEFAULT_DEEP_SPACE_CONFIG,
    cellCount: number = 2048,
  ) {
    this.config = config;
    this.placement = new PortalPlacement(cellCount);

    // Create players
    for (let i = 1; i <= playerCount; i++) {
      const cellIndex = this.placement.allocate();
      if (cellIndex < 0) break; // No more cells

      this.players.push({
        id: i,
        cellIndex,
        portalPos: this.placement.portalPos(cellIndex),
        color: colorFromId(i),
        paused: false,
        ballsProduced: 0,
        ballsInFlight: 0,
      });
    }
  }

  /** Get self player */
  getSelfPlayer(): Player {
    return this.players.find((p) => p.id === this.selfId)!;
  }

  /** Get all players */
  getAllPlayers(): Player[] {
    return this.players;
  }

  /** Get other players (not self) */
  getOtherPlayers(): Player[] {
    return this.players.filter((p) => p.id !== this.selfId);
  }
}
