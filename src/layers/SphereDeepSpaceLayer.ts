/**
 * Sphere-based deep space rendering layer.
 * Shows a "neighborhood disk" view around the local player's portal.
 */

import { Container, Graphics } from "pixi.js";
import { COLORS } from "../constants";
import { Vec3, dot, sub, scale, buildTangentBasis } from "../shared/vec3";
import { Player, SpaceBall3D } from "../shared/types";

/** Max angular distance to render (radians) */
const THETA_MAX = 0.8; // ~46 degrees

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
   * @returns [x, y, visible] where visible is true if within THETA_MAX
   */
  private projectToScreen(pos: Vec3): [number, number, boolean] {
    const d = dot(this.selfPortalPos, pos);
    const dClamped = Math.max(-1, Math.min(1, d));
    const theta = Math.acos(dClamped);

    if (theta > THETA_MAX) {
      return [0, 0, false];
    }

    // Project to tangent plane
    const v = sub(pos, scale(this.selfPortalPos, d));
    const vLen = Math.sqrt(dot(v, v));

    if (vLen < 1e-6) {
      // Point is at self portal
      return [this.centerX, this.centerY, true];
    }

    const dir = scale(v, 1 / vLen);
    const r = theta * PIXELS_PER_RADIAN;

    const dx = dot(dir, this.tangentE1);
    const dy = dot(dir, this.tangentE2);

    return [this.centerX + dx * r, this.centerY + dy * r, true];
  }

  /**
   * Update and render the deep space view.
   */
  update(dt: number, balls: SpaceBall3D[], players: Player[], selfId: number) {
    this.time += dt;

    // Twinkle stars
    for (const star of this.stars) {
      const twinkle = Math.sin(this.time * star.twinkleSpeed) * 0.3 + 0.7;
      star.graphics.alpha = star.baseAlpha * twinkle;
    }

    // Clear graphics
    this.ballsGraphics.clear();
    this.portalsGraphics.clear();

    // Draw self portal marker at center (diffuse)
    const selfPlayer = players.find((p) => p.id === selfId);
    if (selfPlayer) {
      // Outer ring
      this.portalsGraphics.circle(this.centerX, this.centerY, 14);
      this.portalsGraphics.stroke({
        color: selfPlayer.color,
        width: 2,
        alpha: 0.4,
      });
      // Inner dot
      this.portalsGraphics.circle(this.centerX, this.centerY, 5);
      this.portalsGraphics.fill({ color: selfPlayer.color, alpha: 0.5 });
    }

    // Draw other player portals (diffuse)
    for (const player of players) {
      if (player.id === selfId) continue;

      const [x, y, visible] = this.projectToScreen(player.portalPos);
      if (!visible) continue;

      // Outer glow (soft)
      this.portalsGraphics.circle(x, y, PORTAL_RADIUS + 3);
      this.portalsGraphics.fill({ color: player.color, alpha: 0.1 });

      // Inner dot (diffuse)
      this.portalsGraphics.circle(x, y, PORTAL_RADIUS);
      this.portalsGraphics.fill({ color: player.color, alpha: 0.25 });
    }

    // Draw balls (diffuse)
    for (const ball of balls) {
      const [x, y, visible] = this.projectToScreen(ball.pos);
      if (!visible) continue;

      // Find owner color
      const owner = players.find((p) => p.id === ball.ownerId);
      const color = owner?.color ?? COLORS.ballGlow;

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
