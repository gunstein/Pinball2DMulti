/**
 * Sphere utilities: Fibonacci sphere generation and portal placement.
 */

import { Vec3, vec3, normalize, dot } from "./vec3";

/** Golden angle in radians */
const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));

/**
 * Generate M evenly-distributed points on a unit sphere using Fibonacci spiral.
 * @param m Number of points
 * @returns Array of unit vectors
 */
export function fibonacciSphere(m: number): Vec3[] {
  const points: Vec3[] = [];

  for (let i = 0; i < m; i++) {
    // y goes from ~1 to ~-1
    const y = 1 - (2 * (i + 0.5)) / m;
    const r = Math.sqrt(1 - y * y);
    const phi = i * GOLDEN_ANGLE;

    const x = Math.cos(phi) * r;
    const z = Math.sin(phi) * r;

    points.push(normalize(vec3(x, y, z)));
  }

  return points;
}

/**
 * Fisher-Yates shuffle (in-place).
 */
function shuffle<T>(arr: T[]): T[] {
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

/**
 * Portal placement manager.
 * Manages cell allocation for players on the sphere.
 */
export class PortalPlacement {
  readonly cellCenters: Vec3[];
  private freeCells: number[];
  private tokenToCell = new Map<string, number>();

  /**
   * @param cellCount Number of cells (M)
   */
  constructor(cellCount: number = 2048) {
    this.cellCenters = fibonacciSphere(cellCount);

    // Initialize all cells as free, then shuffle for even distribution
    this.freeCells = Array.from({ length: cellCount }, (_, i) => i);
    shuffle(this.freeCells);
  }

  /**
   * Allocate a cell for a player.
   * @param resumeToken Optional token to resume previous cell
   * @returns Cell index, or -1 if no cells available
   */
  allocate(resumeToken?: string): number {
    // Try to resume previous cell
    if (resumeToken && this.tokenToCell.has(resumeToken)) {
      const prevCell = this.tokenToCell.get(resumeToken)!;
      // Check if it's still free
      const freeIdx = this.freeCells.indexOf(prevCell);
      if (freeIdx !== -1) {
        this.freeCells.splice(freeIdx, 1);
        return prevCell;
      }
    }

    // Allocate from shuffled pool
    if (this.freeCells.length === 0) {
      return -1; // No cells available
    }

    const cellIndex = this.freeCells.pop()!;

    // Store token mapping if provided
    if (resumeToken) {
      this.tokenToCell.set(resumeToken, cellIndex);
    }

    return cellIndex;
  }

  /**
   * Release a cell back to the pool.
   */
  release(cellIndex: number): void {
    if (!this.freeCells.includes(cellIndex)) {
      this.freeCells.push(cellIndex);
    }
  }

  /**
   * Get portal position for a cell.
   */
  portalPos(cellIndex: number): Vec3 {
    return this.cellCenters[cellIndex];
  }

  /**
   * Find K nearest occupied cells to a position.
   * @param pos Position to search from
   * @param k Number of nearest to return
   * @param occupiedCells Set of occupied cell indices
   */
  findNearestOccupied(
    pos: Vec3,
    k: number,
    occupiedCells: Set<number>,
  ): number[] {
    const withDist: Array<{ idx: number; d: number }> = [];

    for (const idx of occupiedCells) {
      withDist.push({ idx, d: dot(pos, this.cellCenters[idx]) });
    }

    withDist.sort((a, b) => b.d - a.d);
    return withDist.slice(0, k).map((x) => x.idx);
  }

  /** Number of available cells */
  get availableCount(): number {
    return this.freeCells.length;
  }

  /** Total cell count */
  get totalCount(): number {
    return this.cellCenters.length;
  }
}
