/**
 * Sphere-based deep space simulation.
 * Balls move on great circles on a unit sphere surface.
 */

import {
  Vec3,
  dot,
  cross,
  length,
  normalize,
  buildTangentBasis,
  map2DToTangent,
  mapTangentTo2D,
  getVelocityDirection,
  angularDistance,
  arbitraryOrthogonal,
} from "./vec3";
import {
  Player,
  SpaceBall3D,
  DeepSpaceConfig,
  DEFAULT_DEEP_SPACE_CONFIG,
} from "./types";

/**
 * Rotate pos around axis by angle, normalize, and write result back to pos.
 * Rodrigues' rotation + normalize in one pass, zero allocations.
 */
function rotateNormalizeInPlace(pos: Vec3, axis: Vec3, angle: number): void {
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
 * Sphere deep space simulation.
 */
export class SphereDeepSpace {
  readonly config: DeepSpaceConfig;
  private readonly cosPortalAlpha: number;
  private balls = new Map<number, SpaceBall3D>();
  private players: Player[] = [];
  private nextBallId = 1;

  // Reusable buffer to avoid per-tick allocations
  private captureBuffer: CaptureEvent[] = [];

  constructor(config: DeepSpaceConfig = DEFAULT_DEEP_SPACE_CONFIG) {
    this.config = config;
    this.cosPortalAlpha = Math.cos(config.portalAlpha);
  }

  /** Update player list */
  setPlayers(players: Player[]): void {
    this.players = players;
  }

  /**
   * Add a ball to deep space from an escape.
   * @param ownerId Player who owns this ball
   * @param portalPos Portal position (unit vector)
   * @param vx 2D velocity x component (m/s)
   * @param vy 2D velocity y component (m/s)
   * @returns Ball ID
   */
  addBall(ownerId: number, portalPos: Vec3, vx: number, vy: number): number {
    const id = this.nextBallId++;

    // Build tangent basis at portal
    const [e1, e2] = buildTangentBasis(portalPos);

    // Map 2D velocity to 3D tangent direction
    const tangent = map2DToTangent(vx, vy, e1, e2);

    // Compute rotation axis (perpendicular to both portal normal and velocity)
    const crossVec = cross(portalPos, tangent);
    const crossLen = length(crossVec);

    let axis: Vec3;
    if (crossLen < 0.01) {
      // Tangent parallel to portalPos (shouldn't happen, but handle it)
      axis = arbitraryOrthogonal(portalPos);
    } else {
      axis = {
        x: crossVec.x / crossLen,
        y: crossVec.y / crossLen,
        z: crossVec.z / crossLen,
      };
    }

    // Random omega within configured range
    const omega =
      this.config.omegaMin +
      Math.random() * (this.config.omegaMax - this.config.omegaMin);

    // Start at portal position. minAgeForCapture prevents instant re-capture.
    // This ensures the great circle passes through the portal, enabling return capture.
    const pos = normalize(portalPos);

    const ball: SpaceBall3D = {
      id,
      ownerId,
      pos,
      axis,
      omega,
      age: 0,
      timeSinceHit: 0,
      rerouteCooldown: 0,
    };

    this.balls.set(id, ball);
    return id;
  }

  /** Get all balls (allocates a new array) */
  getBalls(): SpaceBall3D[] {
    return Array.from(this.balls.values());
  }

  /**
   * Get an iterable view of balls without allocating a new array.
   * Useful for per-frame rendering to reduce GC pressure.
   */
  getBallIterable(): IterableIterator<SpaceBall3D> {
    return this.balls.values();
  }

  /** Get a specific ball */
  getBall(id: number): SpaceBall3D | undefined {
    return this.balls.get(id);
  }

  /**
   * Simulate one tick.
   * @param dt Delta time in seconds
   * @returns Array of capture events (ball entered a portal)
   */
  tick(dt: number): CaptureEvent[] {
    // Reuse buffer to avoid per-tick allocations
    const captures = this.captureBuffer;
    captures.length = 0;

    for (const ball of this.balls.values()) {
      // Update position in-place (rotate around axis + normalize, zero allocs)
      rotateNormalizeInPlace(ball.pos, ball.axis, ball.omega * dt);

      // Update timers
      ball.age += dt;
      ball.timeSinceHit += dt;
      ball.rerouteCooldown = Math.max(0, ball.rerouteCooldown - dt);

      // Check portal hits (only if old enough)
      // Select portal with highest dot product to avoid bias toward first player
      let captured = false;
      if (ball.age >= this.config.minAgeForCapture) {
        let bestMatch: { player: Player; dotProduct: number } | null = null;
        for (const player of this.players) {
          const p = player.portalPos;
          const d = ball.pos.x * p.x + ball.pos.y * p.y + ball.pos.z * p.z;
          if (d >= this.cosPortalAlpha) {
            if (!bestMatch || d > bestMatch.dotProduct) {
              bestMatch = { player, dotProduct: d };
            }
          }
        }
        if (bestMatch) {
          captures.push({
            ballId: ball.id,
            playerId: bestMatch.player.id,
            ball,
            player: bestMatch.player,
          });
          captured = true;
        }
      }

      // Reroute if ball hasn't hit anything for too long (skip if captured)
      if (!captured && this.shouldReroute(ball)) {
        this.rerouteBall(ball);
      }
    }

    // Remove captured balls
    for (let i = 0; i < captures.length; i++) {
      this.balls.delete(captures[i].ballId);
    }

    return captures;
  }

  /** Check if ball should be rerouted */
  private shouldReroute(ball: SpaceBall3D): boolean {
    return (
      ball.age >= this.config.minAgeForReroute &&
      ball.timeSinceHit >= this.config.rerouteAfter &&
      ball.rerouteCooldown <= 0
    );
  }

  /** Reroute ball toward a target portal */
  private rerouteBall(ball: SpaceBall3D): void {
    if (this.players.length === 0) return;

    // Choose target: random player
    const targetIdx = Math.floor(Math.random() * this.players.length);
    const target = this.players[targetIdx];
    const targetPos = target.portalPos;

    // Check if ball is already very close to target (dot ~ 1)
    const dotPosTarget = dot(ball.pos, targetPos);
    if (dotPosTarget > 0.99) {
      // Already at target, just reset cooldown
      ball.rerouteCooldown = this.config.rerouteCooldown;
      return;
    }

    // Compute axis for great circle through ball.pos and targetPos
    const crossVec = cross(ball.pos, targetPos);
    const crossLen = length(crossVec);

    if (crossLen < 0.01) {
      // Near-antiparallel (dot ~ -1): any orthogonal axis works
      ball.axis = arbitraryOrthogonal(ball.pos);
    } else {
      ball.axis = {
        x: crossVec.x / crossLen,
        y: crossVec.y / crossLen,
        z: crossVec.z / crossLen,
      };
    }

    // Compute travel time and omega
    const delta = angularDistance(ball.pos, targetPos);
    const T =
      this.config.rerouteArrivalTimeMin +
      Math.random() *
        (this.config.rerouteArrivalTimeMax - this.config.rerouteArrivalTimeMin);
    let newOmega = delta / T;

    // Clamp omega to configured range
    newOmega = Math.max(
      this.config.omegaMin,
      Math.min(this.config.omegaMax, newOmega),
    );

    // Apply reroute
    ball.omega = newOmega;
    ball.timeSinceHit = 0;
    ball.rerouteCooldown = this.config.rerouteCooldown;
  }

  /**
   * Get velocity in 2D local coordinates for a captured ball.
   * @param ball The captured ball
   * @param portalPos Portal position where ball was captured
   * @param speed2D Speed to use in 2D (m/s)
   * @returns [vx, vy] in local 2D coordinates
   */
  getCaptureVelocity2D(
    ball: SpaceBall3D,
    portalPos: Vec3,
    speed2D: number,
  ): [number, number] {
    const velDir = getVelocityDirection(ball.pos, ball.axis, ball.omega);
    const [e1, e2] = buildTangentBasis(portalPos);
    const [dx, dy] = mapTangentTo2D(velDir, e1, e2);

    // Normalize and scale to speed2D
    const len = Math.sqrt(dx * dx + dy * dy);
    if (len < 0.01) {
      return [0, speed2D]; // Default: downward into board
    }
    // vy must always be positive (downward into the board)
    // The ball enters from the top, so it always moves down
    return [(dx / len) * speed2D, Math.abs(dy / len) * speed2D];
  }
}

/** Event when a ball enters a portal */
export interface CaptureEvent {
  ballId: number;
  playerId: number;
  ball: SpaceBall3D;
  player: Player;
}
