/**
 * 3D vector utilities for sphere-based deep-space.
 * All vectors are assumed to be unit vectors (on the sphere surface) unless noted.
 */

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

/** Create a Vec3 */
export function vec3(x: number, y: number, z: number): Vec3 {
  return { x, y, z };
}

/** Dot product */
export function dot(a: Vec3, b: Vec3): number {
  return a.x * b.x + a.y * b.y + a.z * b.z;
}

/** Cross product */
export function cross(a: Vec3, b: Vec3): Vec3 {
  return {
    x: a.y * b.z - a.z * b.y,
    y: a.z * b.x - a.x * b.z,
    z: a.x * b.y - a.y * b.x,
  };
}

/** Vector length */
export function length(v: Vec3): number {
  return Math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z);
}

/** Normalize vector to unit length */
export function normalize(v: Vec3): Vec3 {
  const len = length(v);
  if (len < 1e-10) {
    // Return arbitrary unit vector if input is near-zero
    return { x: 1, y: 0, z: 0 };
  }
  return { x: v.x / len, y: v.y / len, z: v.z / len };
}

/** Scale vector by scalar */
export function scale(v: Vec3, s: number): Vec3 {
  return { x: v.x * s, y: v.y * s, z: v.z * s };
}

/** Add two vectors */
export function add(a: Vec3, b: Vec3): Vec3 {
  return { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z };
}

/** Subtract vectors (a - b) */
export function sub(a: Vec3, b: Vec3): Vec3 {
  return { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z };
}

/**
 * Rotate vector around axis by angle (Rodrigues' rotation formula).
 * @param v Vector to rotate (should be unit)
 * @param axis Rotation axis (must be unit)
 * @param angle Rotation angle in radians
 */
export function rotateAroundAxis(v: Vec3, axis: Vec3, angle: number): Vec3 {
  const cosA = Math.cos(angle);
  const sinA = Math.sin(angle);
  const oneMinusCos = 1 - cosA;

  // v' = v*cos(a) + (axis × v)*sin(a) + axis*(axis·v)*(1-cos(a))
  const crossAV = cross(axis, v);
  const dotAV = dot(axis, v);

  return {
    x: v.x * cosA + crossAV.x * sinA + axis.x * dotAV * oneMinusCos,
    y: v.y * cosA + crossAV.y * sinA + axis.y * dotAV * oneMinusCos,
    z: v.z * cosA + crossAV.z * sinA + axis.z * dotAV * oneMinusCos,
  };
}

/**
 * Rotate pos around axis by angle, normalize, and write result back to pos.
 * Rodrigues' rotation + normalize in one pass, zero allocations.
 * Used for client-side interpolation of ball positions.
 */
export function rotateNormalizeInPlace(
  pos: Vec3,
  axis: Vec3,
  angle: number,
): void {
  const cosA = Math.cos(angle);
  const sinA = Math.sin(angle);
  const oneMinusCos = 1 - cosA;

  // cross(axis, pos)
  const cx = axis.y * pos.z - axis.z * pos.y;
  const cy = axis.z * pos.x - axis.x * pos.z;
  const cz = axis.x * pos.y - axis.y * pos.x;

  // dot(axis, pos)
  const d = axis.x * pos.x + axis.y * pos.y + axis.z * pos.z;

  // v' = pos*cos + cross*sin + axis*(dot)*(1-cos)
  const rx = pos.x * cosA + cx * sinA + axis.x * d * oneMinusCos;
  const ry = pos.y * cosA + cy * sinA + axis.y * d * oneMinusCos;
  const rz = pos.z * cosA + cz * sinA + axis.z * d * oneMinusCos;

  // normalize in-place
  const len = Math.sqrt(rx * rx + ry * ry + rz * rz);
  if (len < 1e-10) {
    pos.x = 1;
    pos.y = 0;
    pos.z = 0;
  } else {
    const inv = 1 / len;
    pos.x = rx * inv;
    pos.y = ry * inv;
    pos.z = rz * inv;
  }
}

/**
 * Get angular distance between two unit vectors (in radians).
 */
export function angularDistance(a: Vec3, b: Vec3): number {
  const d = dot(a, b);
  // Clamp to [-1, 1] to handle numerical errors
  return Math.acos(Math.max(-1, Math.min(1, d)));
}

/**
 * Find an arbitrary vector orthogonal to v.
 * Used when we need a reference direction.
 */
export function arbitraryOrthogonal(v: Vec3): Vec3 {
  // Choose a reference that's not parallel to v
  const ref = Math.abs(v.y) < 0.9 ? vec3(0, 1, 0) : vec3(1, 0, 0);
  return normalize(cross(ref, v));
}

/**
 * Build a local tangent basis (e1, e2) for a point on the sphere.
 * e1 and e2 are orthonormal vectors in the tangent plane at u.
 * @param u Unit vector (point on sphere)
 * @returns [e1, e2] orthonormal basis vectors
 */
export function buildTangentBasis(u: Vec3): [Vec3, Vec3] {
  // Choose a reference vector not parallel to u
  const ref =
    Math.abs(dot(u, vec3(0, 1, 0))) < 0.9 ? vec3(0, 1, 0) : vec3(1, 0, 0);

  const e1 = normalize(cross(ref, u));
  const e2 = cross(u, e1); // Already unit since u and e1 are unit and orthogonal

  return [e1, e2];
}

/**
 * Map a 2D direction to a 3D tangent direction on the sphere.
 * @param dx 2D x component
 * @param dy 2D y component
 * @param e1 First basis vector of tangent plane
 * @param e2 Second basis vector of tangent plane
 * @returns Normalized 3D tangent direction
 */
export function map2DToTangent(
  dx: number,
  dy: number,
  e1: Vec3,
  e2: Vec3,
): Vec3 {
  const len = Math.sqrt(dx * dx + dy * dy);
  if (len < 1e-10) {
    return e1; // Default to e1 if no direction
  }
  const nx = dx / len;
  const ny = dy / len;
  return normalize(add(scale(e1, nx), scale(e2, ny)));
}

/**
 * Map a 3D tangent direction back to 2D components.
 * @param tangent 3D tangent direction
 * @param e1 First basis vector of tangent plane
 * @param e2 Second basis vector of tangent plane
 * @returns [dx, dy] 2D components
 */
export function mapTangentTo2D(
  tangent: Vec3,
  e1: Vec3,
  e2: Vec3,
): [number, number] {
  return [dot(tangent, e1), dot(tangent, e2)];
}

/**
 * Get the velocity direction of a ball moving on a great circle.
 * @param pos Current position (unit vector)
 * @param axis Rotation axis (unit vector)
 * @param omega Angular velocity (rad/s), positive = counter-clockwise around axis
 * @returns Velocity direction (unit tangent vector)
 */
export function getVelocityDirection(
  pos: Vec3,
  axis: Vec3,
  omega: number,
): Vec3 {
  // Velocity direction is axis × pos (normalized), sign depends on omega
  const dir = normalize(cross(axis, pos));
  return omega >= 0 ? dir : scale(dir, -1);
}
