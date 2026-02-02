import { Vec3 } from "./vec3";

/** Snapshot of a ball's state — the contract between local sim and deep-space. */
export interface BallSnapshot {
  id: number; // u32 locally (incrementing), server-generated later
  x: number; // meters (physics units) - local board coords
  y: number;
  vx: number; // m/s
  vy: number;
}

/** Deep-space configuration */
export interface DeepSpaceConfig {
  portalAlpha: number; // rad, portal capture radius (~0.05)
  omegaMin: number; // rad/s, min ball angular velocity
  omegaMax: number; // rad/s, max ball angular velocity
  rerouteAfter: number; // seconds without hit before reroute
  rerouteCooldown: number; // seconds between reroutes
  minAgeForCapture: number; // seconds before ball can be captured
  minAgeForReroute: number; // seconds before ball can be rerouted
  rerouteArrivalTimeMin: number; // seconds - min arrival time for reroute
  rerouteArrivalTimeMax: number; // seconds - max arrival time for reroute
}

/** Default deep-space configuration */
export const DEFAULT_DEEP_SPACE_CONFIG: DeepSpaceConfig = {
  portalAlpha: 0.15, // ~8.6 degrees
  omegaMin: 0.5, // rad/s (~12.6s per full orbit)
  omegaMax: 1.0, // rad/s (~6.3s per full orbit)
  rerouteAfter: 12.0, // seconds
  rerouteCooldown: 6.0, // seconds
  minAgeForCapture: 3.0, // seconds - ball must travel before capture
  minAgeForReroute: 2.0, // seconds
  rerouteArrivalTimeMin: 4.0, // seconds
  rerouteArrivalTimeMax: 10.0, // seconds
};

/** Player/Portal on the sphere */
export interface Player {
  id: number; // u32
  cellIndex: number; // index in PortalPlacement
  portalPos: Vec3; // unit vector on sphere
  color: number; // 0xRRGGBB
  paused: boolean; // whether player is paused (tab not visible)
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
}
