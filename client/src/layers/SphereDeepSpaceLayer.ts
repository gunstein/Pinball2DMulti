/**
 * Sphere-based deep space rendering layer.
 * Shows a "neighborhood disk" view around the local player's portal.
 * Uses pooled per-object Graphics (not clear/redraw each frame) for portals
 * and balls to avoid per-frame geometry rebuild and GC pressure.
 */

import { Container, Graphics } from "pixi.js";
import { COLORS } from "../constants";
import { Vec3, buildTangentBasis, rotateAroundAxisTo } from "../shared/vec3";
import { Player, SpaceBall3D } from "../shared/types";

/** Max angular distance to render (radians) */
const THETA_MAX = 0.8; // ~46 degrees
const COS_THETA_MAX = Math.cos(THETA_MAX);

/** Pixels per radian for projection */
const PIXELS_PER_RADIAN = 400;

/** Ball dot radius in pixels */
const BALL_RADIUS = 5;

/** Portal dot radius in pixels */
const PORTAL_RADIUS = 6;

/** Max pooled dot objects for portals and balls */
const MAX_PORTAL_DOTS = 60;
const MAX_BALL_DOTS = 60;

/** Comet tail settings */
const TAIL_SEGMENTS = 4;
const TAIL_TIME_STEP = 0.08; // seconds back per segment
const TAIL_START_ALPHA = 0.3;
const TAIL_END_ALPHA = 0.05;

/** A pooled dot with pre-drawn glow+core (never redrawn, only moved/tinted) */
interface PooledDot {
  graphics: Graphics;
  currentTint: number;
}

/** A pooled ball graphic that supports comet tail (redrawn each frame) */
interface PooledBall {
  graphics: Graphics;
  currentTint: number;
}

function createDot(radius: number, container: Container): PooledDot {
  const g = new Graphics();
  // Outer glow
  g.circle(0, 0, radius + 3);
  g.fill({ color: 0xffffff, alpha: 0.3 });
  // Inner core
  g.circle(0, 0, radius);
  g.fill({ color: 0xffffff, alpha: 1.0 });
  g.visible = false;
  container.addChild(g);
  return { graphics: g, currentTint: 0xffffff };
}

function createBallGraphics(container: Container): PooledBall {
  const g = new Graphics();
  g.visible = false;
  container.addChild(g);
  return { graphics: g, currentTint: 0xffffff };
}

export class SphereDeepSpaceLayer {
  container: Container;
  private bg: Graphics;
  private starsContainer: Container;
  private stars: {
    graphics: Graphics;
    baseAlpha: number;
    twinkleSpeed: number;
  }[] = [];

  // Per-object Graphics pools (drawn once, moved each frame)
  private portalsContainer: Container;
  private ballsContainer: Container;
  private portalDots: PooledDot[] = [];
  private ballPool: PooledBall[] = [];

  // Self portal marker (drawn once, updated on center/color change)
  private selfMarker: Graphics;

  // Boundary circle (drawn once, updated on resize/center change)
  private boundaryGraphics: Graphics;

  private time = 0;
  private width = 800;
  private height = 600;

  // Self portal for projection center
  private selfPortalPos: Vec3 = { x: 1, y: 0, z: 0 };
  private tangentE1: Vec3 = { x: 0, y: 1, z: 0 };
  private tangentE2: Vec3 = { x: 0, y: 0, z: 1 };

  // Screen center offset (to align with board)
  private centerX = 400;
  private centerY = 350;

  // Reused projection output (avoid per-call tuple allocations)
  private projX = 0;
  private projY = 0;

  // Scratch Vec3 for tail rotation (avoid per-segment allocation)
  private scratchVec3: Vec3 = { x: 0, y: 0, z: 0 };

  // Cached color lookup (rebuilt only when players change)
  private colorById: number[] = [];
  private selfColor = COLORS.ballGlow;
  private colorsDirty = true;

  constructor() {
    this.container = new Container();

    // Dark background
    this.bg = new Graphics();
    this.container.addChild(this.bg);

    // Stars container
    this.starsContainer = new Container();
    this.container.addChild(this.starsContainer);

    // Boundary circle
    this.boundaryGraphics = new Graphics();
    this.container.addChild(this.boundaryGraphics);

    // Portals container
    this.portalsContainer = new Container();
    this.container.addChild(this.portalsContainer);

    // Self portal marker
    this.selfMarker = new Graphics();
    this.container.addChild(this.selfMarker);

    // Balls container
    this.ballsContainer = new Container();
    this.container.addChild(this.ballsContainer);

    // Pre-allocate dot pools
    for (let i = 0; i < MAX_PORTAL_DOTS; i++) {
      this.portalDots.push(createDot(PORTAL_RADIUS, this.portalsContainer));
    }
    for (let i = 0; i < MAX_BALL_DOTS; i++) {
      this.ballPool.push(createBallGraphics(this.ballsContainer));
    }

    this.generateStars();
  }

  private generateStars() {
    for (const star of this.stars) {
      star.graphics.destroy();
    }
    this.starsContainer.removeChildren();
    this.stars = [];

    for (let i = 0; i < 150; i++) {
      const g = new Graphics();
      const x = Math.random() * this.width;
      const y = Math.random() * this.height;
      const size = Math.random() * 1.5 + 0.5;
      const baseAlpha = Math.random() * 0.5 + 0.1;

      g.circle(0, 0, size);
      g.fill({ color: COLORS.star });
      g.position.set(x, y);
      g.alpha = baseAlpha;

      this.starsContainer.addChild(g);
      this.stars.push({
        graphics: g,
        baseAlpha,
        twinkleSpeed: Math.random() * 2 + 0.5,
      });
    }
  }

  resize(w: number, h: number) {
    this.width = w;
    this.height = h;

    this.bg.clear();
    this.bg.rect(0, 0, w, h);
    this.bg.fill({ color: COLORS.deepSpaceBg });

    this.generateStars();
    this.drawBoundary();
  }

  private drawBoundary() {
    this.boundaryGraphics.clear();
    const r = THETA_MAX * PIXELS_PER_RADIAN;

    // Soft glow fill in the disk
    this.boundaryGraphics.circle(this.centerX, this.centerY, r);
    this.boundaryGraphics.fill({ color: 0x2244aa, alpha: 0.05 });

    // Stronger edge stroke
    this.boundaryGraphics.circle(this.centerX, this.centerY, r);
    this.boundaryGraphics.stroke({ color: 0x66aaff, width: 2, alpha: 0.25 });
  }

  private drawSelfMarker() {
    this.selfMarker.clear();
    this.selfMarker.circle(this.centerX, this.centerY, 14);
    this.selfMarker.stroke({
      color: this.selfColor,
      width: 2,
      alpha: 0.7,
    });
    this.selfMarker.circle(this.centerX, this.centerY, 5);
    this.selfMarker.fill({ color: this.selfColor, alpha: 0.8 });
  }

  /** Set the center point for rendering (aligns with board center) */
  setCenter(x: number, y: number) {
    this.centerX = x;
    this.centerY = y;
    this.drawBoundary();
    this.drawSelfMarker();
  }

  /** Set self portal position (center of view) */
  setSelfPortal(pos: Vec3) {
    this.selfPortalPos = pos;
    [this.tangentE1, this.tangentE2] = buildTangentBasis(pos);
  }

  /** Mark color cache as dirty (call when players change) */
  markColorsDirty(): void {
    this.colorsDirty = true;
  }

  /**
   * Project a sphere point to 2D screen coordinates.
   * Uses azimuthal equidistant projection centered on self portal.
   * Writes to this.projX/this.projY to avoid allocations.
   * @returns true if the point is visible (within THETA_MAX)
   */
  private projectToScreen(pos: Vec3): boolean {
    const sx = this.selfPortalPos.x;
    const sy = this.selfPortalPos.y;
    const sz = this.selfPortalPos.z;
    const px = pos.x;
    const py = pos.y;
    const pz = pos.z;
    const d = sx * px + sy * py + sz * pz;

    // Fast reject without acos
    if (d < COS_THETA_MAX) {
      return false;
    }

    const dClamped = Math.max(-1, Math.min(1, d));
    const theta = Math.acos(dClamped);

    // Project to tangent plane: v = pos - self*d (scalar math, no Vec3 alloc)
    const vx = px - sx * d;
    const vy = py - sy * d;
    const vz = pz - sz * d;
    const vLen = Math.sqrt(vx * vx + vy * vy + vz * vz);

    if (vLen < 1e-6) {
      this.projX = this.centerX;
      this.projY = this.centerY;
      return true;
    }

    const invVLen = 1 / vLen;
    const dirx = vx * invVLen;
    const diry = vy * invVLen;
    const dirz = vz * invVLen;

    const e1x = this.tangentE1.x;
    const e1y = this.tangentE1.y;
    const e1z = this.tangentE1.z;
    const e2x = this.tangentE2.x;
    const e2y = this.tangentE2.y;
    const e2z = this.tangentE2.z;

    const dx = dirx * e1x + diry * e1y + dirz * e1z;
    const dy = dirx * e2x + diry * e2y + dirz * e2z;

    const r = theta * PIXELS_PER_RADIAN;
    this.projX = this.centerX + dx * r;
    this.projY = this.centerY + dy * r;
    return true;
  }

  /**
   * Apply tint to a pooled dot (only if changed, to avoid Pixi internals).
   */
  private applyTint(dot: PooledDot, color: number, alpha: number): void {
    if (dot.currentTint !== color) {
      dot.graphics.tint = color;
      dot.currentTint = color;
    }
    dot.graphics.alpha = alpha;
  }

  /**
   * Update and render the deep space view.
   * No Graphics clear/redraw — just move pre-drawn dot objects.
   */
  update(
    dt: number,
    balls: Iterable<SpaceBall3D>,
    players: Player[],
    selfId: number,
  ) {
    this.time += dt;

    // Twinkle stars
    for (const star of this.stars) {
      const twinkle = Math.sin(this.time * star.twinkleSpeed) * 0.3 + 0.7;
      star.graphics.alpha = star.baseAlpha * twinkle;
    }

    // Rebuild color lookup only when players change
    if (this.colorsDirty) {
      this.colorById.length = 0;
      this.selfColor = COLORS.ballGlow;
      for (const player of players) {
        this.colorById[player.id] = player.color;
        if (player.id === selfId) {
          this.selfColor = player.color;
        }
      }
      this.colorsDirty = false;
      this.drawSelfMarker();
    }

    // --- Portals: move pooled dots ---
    let portalIdx = 0;
    for (const player of players) {
      if (player.id === selfId) continue;
      if (portalIdx >= this.portalDots.length) break;
      if (!this.projectToScreen(player.portalPos)) continue;

      const dot = this.portalDots[portalIdx];
      dot.graphics.position.set(this.projX, this.projY);
      // Paused players are dimmer
      const alpha = player.paused ? 0.2 : 0.6;
      this.applyTint(dot, player.color, alpha);
      dot.graphics.visible = true;
      portalIdx++;
    }
    // Hide unused
    for (let i = portalIdx; i < this.portalDots.length; i++) {
      this.portalDots[i].graphics.visible = false;
    }

    // --- Balls: draw with comet tail ---
    let ballIdx = 0;
    for (const ball of balls) {
      if (ballIdx >= this.ballPool.length) break;
      if (!this.projectToScreen(ball.pos)) continue;

      const color = this.colorById[ball.ownerId] ?? COLORS.ballGlow;
      const pooledBall = this.ballPool[ballIdx];
      const g = pooledBall.graphics;

      // Clear and redraw with tail
      g.clear();

      // Draw tail segments (going backwards in time)
      // Rotate backwards along the great circle to find previous positions
      for (let t = TAIL_SEGMENTS; t >= 1; t--) {
        const timeBack = t * TAIL_TIME_STEP;
        // Rotate backwards: negative angle (writes to scratch, zero alloc)
        rotateAroundAxisTo(
          ball.pos,
          ball.axis,
          -ball.omega * timeBack,
          this.scratchVec3,
        );

        if (this.projectToScreen(this.scratchVec3)) {
          const tailX = this.projX;
          const tailY = this.projY;

          // Interpolate alpha from start to end
          const alpha =
            TAIL_START_ALPHA +
            ((TAIL_END_ALPHA - TAIL_START_ALPHA) * t) / TAIL_SEGMENTS;
          // Smaller radius for tail segments
          const radius = BALL_RADIUS * (1 - t * 0.15);

          g.circle(tailX, tailY, radius);
          g.fill({ color, alpha });
        }
      }

      // Draw main ball (current position)
      if (this.projectToScreen(ball.pos)) {
        // Outer glow
        g.circle(this.projX, this.projY, BALL_RADIUS + 3);
        g.fill({ color, alpha: 0.3 });
        // Inner core
        g.circle(this.projX, this.projY, BALL_RADIUS);
        g.fill({ color, alpha: 0.8 });
      }

      g.visible = true;
      ballIdx++;
    }
    // Hide unused
    for (let i = ballIdx; i < this.ballPool.length; i++) {
      this.ballPool[i].graphics.visible = false;
    }
  }
}
