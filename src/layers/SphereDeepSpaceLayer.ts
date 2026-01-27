/**
 * Sphere-based deep space rendering layer.
 * Shows a "neighborhood disk" view around the local player's portal.
 */

import { Container, Graphics } from "pixi.js";
import { COLORS } from "../constants";
import { Vec3, buildTangentBasis } from "../shared/vec3";
import { Player, SpaceBall3D } from "../shared/types";

/** Max angular distance to render (radians) */
const THETA_MAX = 0.8; // ~46 degrees
const COS_THETA_MAX = Math.cos(THETA_MAX);

/** Pixels per radian for projection */
const PIXELS_PER_RADIAN = 400;

/** Ball dot radius in pixels */
const BALL_RADIUS = 5;

/** Portal dot radius in pixels */
const PORTAL_RADIUS = 4;

export class SphereDeepSpaceLayer {
  container: Container;
  private bg: Graphics;
  private starsContainer: Container;
  private stars: {
    graphics: Graphics;
    baseAlpha: number;
    twinkleSpeed: number;
  }[] = [];
  private ballsGraphics: Graphics;
  private portalsGraphics: Graphics;

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

  constructor() {
    this.container = new Container();

    // Dark background
    this.bg = new Graphics();
    this.container.addChild(this.bg);

    // Stars container
    this.starsContainer = new Container();
    this.container.addChild(this.starsContainer);

    // Portals (other players)
    this.portalsGraphics = new Graphics();
    this.container.addChild(this.portalsGraphics);

    // Balls
    this.ballsGraphics = new Graphics();
    this.container.addChild(this.ballsGraphics);

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
  }

  /** Set the center point for rendering (aligns with board center) */
  setCenter(x: number, y: number) {
    this.centerX = x;
    this.centerY = y;
  }

  /** Set self portal position (center of view) */
  setSelfPortal(pos: Vec3) {
    this.selfPortalPos = pos;
    [this.tangentE1, this.tangentE2] = buildTangentBasis(pos);
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
      // Point is at self portal
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
   * Update and render the deep space view.
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

    // Build color map once per frame (avoids O(players*balls) find() calls)
    const colorById = new Map<number, number>();
    let selfColor = COLORS.ballGlow;
    for (const player of players) {
      colorById.set(player.id, player.color);
      if (player.id === selfId) {
        selfColor = player.color;
      }
    }

    // Clear graphics
    this.ballsGraphics.clear();
    this.portalsGraphics.clear();

    // Draw self portal marker at center (diffuse)
    this.portalsGraphics.circle(this.centerX, this.centerY, 14);
    this.portalsGraphics.stroke({
      color: selfColor,
      width: 2,
      alpha: 0.4,
    });
    this.portalsGraphics.circle(this.centerX, this.centerY, 5);
    this.portalsGraphics.fill({ color: selfColor, alpha: 0.5 });

    // Draw other player portals (diffuse)
    for (const player of players) {
      if (player.id === selfId) continue;

      if (!this.projectToScreen(player.portalPos)) continue;
      const x = this.projX;
      const y = this.projY;

      // Outer glow (soft)
      this.portalsGraphics.circle(x, y, PORTAL_RADIUS + 3);
      this.portalsGraphics.fill({ color: player.color, alpha: 0.1 });

      // Inner dot (diffuse)
      this.portalsGraphics.circle(x, y, PORTAL_RADIUS);
      this.portalsGraphics.fill({ color: player.color, alpha: 0.25 });
    }

    // Draw balls (diffuse)
    for (const ball of balls) {
      if (!this.projectToScreen(ball.pos)) continue;
      const x = this.projX;
      const y = this.projY;

      const color = colorById.get(ball.ownerId) ?? COLORS.ballGlow;

      // Outer glow (soft)
      this.ballsGraphics.circle(x, y, BALL_RADIUS + 3);
      this.ballsGraphics.fill({ color, alpha: 0.15 });

      // Core (diffuse)
      this.ballsGraphics.circle(x, y, BALL_RADIUS);
      this.ballsGraphics.fill({ color, alpha: 0.5 });
    }

    // Draw boundary circle (edge of visible area)
    const boundaryRadius = THETA_MAX * PIXELS_PER_RADIAN;
    this.portalsGraphics.circle(this.centerX, this.centerY, boundaryRadius);
    this.portalsGraphics.stroke({ color: 0x2244aa, width: 1, alpha: 0.2 });
  }
}
