import { describe, it, expect } from "vitest";
import {
  vec3,
  dot,
  cross,
  length,
  normalize,
  scale,
  add,
  sub,
  rotateAroundAxis,
  angularDistance,
  arbitraryOrthogonal,
  buildTangentBasis,
  map2DToTangent,
  mapTangentTo2D,
} from "../src/shared/vec3";

const EPSILON = 1e-9;

function expectVec3Close(actual: { x: number; y: number; z: number }, expected: { x: number; y: number; z: number }, eps = EPSILON) {
  expect(actual.x).toBeCloseTo(expected.x, 6);
  expect(actual.y).toBeCloseTo(expected.y, 6);
  expect(actual.z).toBeCloseTo(expected.z, 6);
}

describe("vec3 basic operations", () => {
  it("vec3 creates a vector", () => {
    const v = vec3(1, 2, 3);
    expect(v).toEqual({ x: 1, y: 2, z: 3 });
  });

  it("dot product of orthogonal vectors is 0", () => {
    const a = vec3(1, 0, 0);
    const b = vec3(0, 1, 0);
    expect(dot(a, b)).toBe(0);
  });

  it("dot product of parallel vectors is product of lengths", () => {
    const a = vec3(2, 0, 0);
    const b = vec3(3, 0, 0);
    expect(dot(a, b)).toBe(6);
  });

  it("dot product of antiparallel vectors is negative", () => {
    const a = vec3(1, 0, 0);
    const b = vec3(-1, 0, 0);
    expect(dot(a, b)).toBe(-1);
  });

  it("cross product of x and y is z", () => {
    const x = vec3(1, 0, 0);
    const y = vec3(0, 1, 0);
    expectVec3Close(cross(x, y), vec3(0, 0, 1));
  });

  it("cross product of parallel vectors is zero", () => {
    const a = vec3(1, 0, 0);
    const b = vec3(2, 0, 0);
    expectVec3Close(cross(a, b), vec3(0, 0, 0));
  });

  it("length of unit vector is 1", () => {
    expect(length(vec3(1, 0, 0))).toBe(1);
    expect(length(vec3(0, 1, 0))).toBe(1);
    expect(length(vec3(0, 0, 1))).toBe(1);
  });

  it("length of (3,4,0) is 5", () => {
    expect(length(vec3(3, 4, 0))).toBe(5);
  });

  it("normalize returns unit vector", () => {
    const v = normalize(vec3(3, 4, 0));
    expect(length(v)).toBeCloseTo(1, 9);
    expectVec3Close(v, vec3(0.6, 0.8, 0));
  });

  it("normalize of zero vector returns arbitrary unit vector", () => {
    const v = normalize(vec3(0, 0, 0));
    expect(length(v)).toBeCloseTo(1, 9);
  });

  it("scale multiplies vector", () => {
    expectVec3Close(scale(vec3(1, 2, 3), 2), vec3(2, 4, 6));
  });

  it("add sums vectors", () => {
    expectVec3Close(add(vec3(1, 2, 3), vec3(4, 5, 6)), vec3(5, 7, 9));
  });

  it("sub subtracts vectors", () => {
    expectVec3Close(sub(vec3(4, 5, 6), vec3(1, 2, 3)), vec3(3, 3, 3));
  });
});

describe("rotateAroundAxis", () => {
  it("rotate x around z by 90° gives y", () => {
    const v = vec3(1, 0, 0);
    const axis = vec3(0, 0, 1);
    const result = rotateAroundAxis(v, axis, Math.PI / 2);
    expectVec3Close(result, vec3(0, 1, 0));
  });

  it("rotate x around z by 180° gives -x", () => {
    const v = vec3(1, 0, 0);
    const axis = vec3(0, 0, 1);
    const result = rotateAroundAxis(v, axis, Math.PI);
    expectVec3Close(result, vec3(-1, 0, 0));
  });

  it("rotate x around z by 360° gives x", () => {
    const v = vec3(1, 0, 0);
    const axis = vec3(0, 0, 1);
    const result = rotateAroundAxis(v, axis, 2 * Math.PI);
    expectVec3Close(result, vec3(1, 0, 0));
  });

  it("rotate around own axis does nothing", () => {
    const v = vec3(1, 0, 0);
    const axis = vec3(1, 0, 0);
    const result = rotateAroundAxis(v, axis, Math.PI / 2);
    expectVec3Close(result, vec3(1, 0, 0));
  });

  it("preserves vector length", () => {
    const v = normalize(vec3(1, 1, 1));
    const axis = normalize(vec3(1, 2, 3));
    const result = rotateAroundAxis(v, axis, 1.234);
    expect(length(result)).toBeCloseTo(1, 9);
  });
});

describe("angularDistance", () => {
  it("same point has distance 0", () => {
    const v = vec3(1, 0, 0);
    expect(angularDistance(v, v)).toBeCloseTo(0, 9);
  });

  it("orthogonal vectors have distance PI/2", () => {
    const a = vec3(1, 0, 0);
    const b = vec3(0, 1, 0);
    expect(angularDistance(a, b)).toBeCloseTo(Math.PI / 2, 9);
  });

  it("opposite points have distance PI", () => {
    const a = vec3(1, 0, 0);
    const b = vec3(-1, 0, 0);
    expect(angularDistance(a, b)).toBeCloseTo(Math.PI, 9);
  });
});

describe("arbitraryOrthogonal", () => {
  it("returns vector orthogonal to input", () => {
    const v = normalize(vec3(1, 2, 3));
    const orth = arbitraryOrthogonal(v);
    expect(dot(v, orth)).toBeCloseTo(0, 9);
  });

  it("returns unit vector", () => {
    const v = normalize(vec3(1, 2, 3));
    const orth = arbitraryOrthogonal(v);
    expect(length(orth)).toBeCloseTo(1, 9);
  });

  it("works for axis-aligned vectors", () => {
    for (const v of [vec3(1, 0, 0), vec3(0, 1, 0), vec3(0, 0, 1)]) {
      const orth = arbitraryOrthogonal(v);
      expect(dot(v, orth)).toBeCloseTo(0, 9);
      expect(length(orth)).toBeCloseTo(1, 9);
    }
  });
});

describe("buildTangentBasis", () => {
  it("returns two orthonormal vectors in tangent plane", () => {
    const u = normalize(vec3(1, 2, 3));
    const [e1, e2] = buildTangentBasis(u);

    // e1 and e2 are unit vectors
    expect(length(e1)).toBeCloseTo(1, 9);
    expect(length(e2)).toBeCloseTo(1, 9);

    // e1 and e2 are orthogonal to u
    expect(dot(u, e1)).toBeCloseTo(0, 9);
    expect(dot(u, e2)).toBeCloseTo(0, 9);

    // e1 and e2 are orthogonal to each other
    expect(dot(e1, e2)).toBeCloseTo(0, 9);
  });

  it("works for north pole", () => {
    const u = vec3(0, 1, 0);
    const [e1, e2] = buildTangentBasis(u);
    expect(dot(u, e1)).toBeCloseTo(0, 9);
    expect(dot(u, e2)).toBeCloseTo(0, 9);
    expect(dot(e1, e2)).toBeCloseTo(0, 9);
  });
});

describe("map2DToTangent and mapTangentTo2D", () => {
  it("round-trips correctly", () => {
    const u = normalize(vec3(1, 2, 3));
    const [e1, e2] = buildTangentBasis(u);

    const dx = 0.6, dy = 0.8;
    const tangent = map2DToTangent(dx, dy, e1, e2);
    const [dx2, dy2] = mapTangentTo2D(tangent, e1, e2);

    // Should recover original direction (normalized)
    const len = Math.sqrt(dx * dx + dy * dy);
    expect(dx2).toBeCloseTo(dx / len, 6);
    expect(dy2).toBeCloseTo(dy / len, 6);
  });

  it("tangent is unit vector", () => {
    const u = normalize(vec3(1, 2, 3));
    const [e1, e2] = buildTangentBasis(u);
    const tangent = map2DToTangent(3, 4, e1, e2);
    expect(length(tangent)).toBeCloseTo(1, 9);
  });

  it("tangent is orthogonal to u", () => {
    const u = normalize(vec3(1, 2, 3));
    const [e1, e2] = buildTangentBasis(u);
    const tangent = map2DToTangent(1, 1, e1, e2);
    expect(dot(u, tangent)).toBeCloseTo(0, 6);
  });
});
