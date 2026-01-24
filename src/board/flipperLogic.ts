export const ROTATION_SPEED_UP = 14.0; // radians/second (when active)
export const ROTATION_SPEED_DOWN = 6.0; // radians/second (when released)
export const MAX_ANGLE = 0.45; // radians

export function restAngle(side: "left" | "right"): number {
  return side === "left" ? MAX_ANGLE : -MAX_ANGLE;
}

export function activeAngle(side: "left" | "right"): number {
  return side === "left" ? -MAX_ANGLE : MAX_ANGLE;
}

export function stepFlipperAngle(
  currentAngle: number,
  dt: number,
  active: boolean,
  side: "left" | "right",
): number {
  const target = active ? activeAngle(side) : restAngle(side);
  const speed = active ? ROTATION_SPEED_UP : ROTATION_SPEED_DOWN;
  return moveTowards(currentAngle, target, speed * dt);
}

function moveTowards(
  current: number,
  target: number,
  maxDelta: number,
): number {
  const diff = target - current;
  if (Math.abs(diff) <= maxDelta) return target;
  return current + Math.sign(diff) * maxDelta;
}
