/** Snapshot of a ball's state — the contract between local sim and deep-space. */
export interface BallSnapshot {
  id: number; // u32 locally (incrementing), server-generated later
  x: number; // meters (physics units)
  y: number;
  vx: number; // m/s
  vy: number;
}
