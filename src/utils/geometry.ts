/** 2D point/vector type */
export interface Vec2 {
  x: number;
  y: number;
}

/** Find the closest point on a line segment to a given point */
export function closestPointOnSegment(
  point: Vec2,
  segStart: Vec2,
  segEnd: Vec2,
): Vec2 {
  const dx = segEnd.x - segStart.x;
  const dy = segEnd.y - segStart.y;
  const lenSq = dx * dx + dy * dy;

  if (lenSq === 0) return segStart; // Segment is a point

  // Project point onto line, clamped to [0, 1]
  let t = ((point.x - segStart.x) * dx + (point.y - segStart.y) * dy) / lenSq;
  t = Math.max(0, Math.min(1, t));

  return {
    x: segStart.x + t * dx,
    y: segStart.y + t * dy,
  };
}

/** Calculate distance between two points */
export function distance(a: Vec2, b: Vec2): number {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  return Math.sqrt(dx * dx + dy * dy);
}

/** Calculate squared distance between two points (faster, no sqrt) */
export function distanceSquared(a: Vec2, b: Vec2): number {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  return dx * dx + dy * dy;
}
