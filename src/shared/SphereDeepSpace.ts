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
  rotateAroundAxis,
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
 * Sphere deep space simulation.
 */
export class SphereDeepSpace {
  readonly config: DeepSpaceConfig;
  private readonly cosPortalAlpha: number;
  private balls = new Map<number, SpaceBall3D>();
  private players: Player[] = [];
  private nextBallId = 1;

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

    // Start position: slightly offset from portal to avoid instant hit
    const startOffset = this.config.portalAlpha * 1.5;
    const pos = rotateAroundAxis(portalPos, axis, startOffset);

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

  /** Get all balls */
  getBalls(): SpaceBall3D[] {
    return Array.from(this.balls.values());
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
    const captures: CaptureEvent[] = [];
    const capturedIds = new Set<number>();

    for (const ball of this.balls.values()) {
      // Update position (rotate around axis)
      ball.pos = normalize(
        rotateAroundAxis(ball.pos, ball.axis, ball.omega * dt),
      );

      // Update timers
      ball.age += dt;
      ball.timeSinceHit += dt;
      ball.rerouteCooldown = Math.max(0, ball.rerouteCooldown - dt);

      // Check portal hits (only if old enough)
      if (ball.age >= this.config.minAgeForCapture) {
        for (const player of this.players) {
          if (this.checkPortalHit(ball, player)) {
            captures.push({
              ballId: ball.id,
              playerId: player.id,
              ball,
              player,
            });
            capturedIds.add(ball.id);
            break;
          }
        }
      }

      // Reroute if ball hasn't hit anything for too long (skip if captured)
      if (!capturedIds.has(ball.id) && this.shouldReroute(ball)) {
        this.rerouteBall(ball);
      }
    }

    // Remove captured balls
    for (const capture of captures) {
      this.balls.delete(capture.ballId);
    }

    return captures;
  }

  /** Check if ball hits a portal */
  private checkPortalHit(ball: SpaceBall3D, player: Player): boolean {
    return dot(ball.pos, player.portalPos) >= this.cosPortalAlpha;
  }

  /** Check if ball should be rerouted */
  private shouldReroute(ball: SpaceBall3D): boolean {
    return (
      ball.age >= 2.0 && // Not too young
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

    let newAxis: Vec3;
    if (crossLen < 0.01) {
      // Near-antiparallel (dot ~ -1): any orthogonal axis works
      newAxis = arbitraryOrthogonal(ball.pos);
    } else {
      newAxis = {
        x: crossVec.x / crossLen,
        y: crossVec.y / crossLen,
        z: crossVec.z / crossLen,
      };
    }

    // Compute travel time and omega
    const delta = angularDistance(ball.pos, targetPos);
    const T = 4.0 + Math.random() * 6.0; // 4-10 seconds
    let newOmega = delta / T;

    // Clamp omega to configured range
    newOmega = Math.max(
      this.config.omegaMin,
      Math.min(this.config.omegaMax, newOmega),
    );

    // Apply reroute
    ball.axis = newAxis;
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
    return [(dx / len) * speed2D, (dy / len) * speed2D];
  }
}

/** Event when a ball enters a portal */
export interface CaptureEvent {
  ballId: number;
  playerId: number;
  ball: SpaceBall3D;
  player: Player;
}
