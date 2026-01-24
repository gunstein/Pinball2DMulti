/** Snapshot of a ball's state — the contract between local sim and deep-space. */
export interface BallSnapshot {
  id: string; // crypto.randomUUID() locally, server-generated later
  x: number; // meters (physics units)
  y: number;
  vx: number; // m/s
  vy: number;
}
