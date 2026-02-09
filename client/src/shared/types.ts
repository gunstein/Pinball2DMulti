import { Vec3 } from "./vec3";
import type { DeepSpaceConfig } from "./generated";

export type { DeepSpaceConfig };

/** Snapshot of a ball's state — the contract between local sim and deep-space. */
export interface BallSnapshot {
  id: number; // u32 locally (incrementing), server-generated later
  x: number; // meters (physics units) - local board coords
  y: number;
  vx: number; // m/s
  vy: number;
}

/**
 * Default deep-space configuration.
 * IMPORTANT: These values should match server/src/config.rs DeepSpaceConfig::default()
 * to ensure consistent behavior between client (offline/mock) and server modes.
 */
export const DEFAULT_DEEP_SPACE_CONFIG: DeepSpaceConfig = {
  portalAlpha: 0.15, // ~8.6 degrees
  omegaMin: 0.5, // rad/s (~12.6s per full orbit)
  omegaMax: 1.0, // rad/s (~6.3s per full orbit)
  rerouteAfter: 12.0, // seconds
  rerouteCooldown: 6.0, // seconds
  minAgeForCapture: 15.0, // seconds - ball must travel before capture (matches server)
  minAgeForReroute: 2.0, // seconds
  rerouteArrivalTimeMin: 4.0, // seconds
  rerouteArrivalTimeMax: 10.0, // seconds (4.0 + 6.0)
};

/** Player/Portal on the sphere */
export interface Player {
  id: number; // u32
  cellIndex: number; // index in PortalPlacement
  portalPos: Vec3; // unit vector on sphere
  color: number; // 0xRRGGBB
  paused: boolean; // whether player is paused (tab not visible)
  ballsProduced: number; // total balls sent to deep space
  ballsInFlight: number; // current balls in deep space
}

/** Deep-space ball moving on sphere surface */
export interface SpaceBall3D {
  id: number; // u32
  ownerId: number; // player who last owned this ball

  pos: Vec3; // unit vector: current position on sphere
  axis: Vec3; // unit vector: rotation axis for great circle
  omega: number; // rad/s: angular velocity

  age: number; // seconds since spawn
  timeSinceHit: number; // seconds since last portal hit or reroute
  rerouteCooldown: number; // seconds until reroute allowed

  // Smooth reroute transition fields
  rerouteTargetAxis?: Vec3; // target axis for smooth transition
  rerouteProgress: number; // 0.0 to 1.0 transition progress
  rerouteTargetOmega: number; // target omega for smooth transition
}
