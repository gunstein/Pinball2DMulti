import { Container, Graphics } from "pixi.js";
import { COLORS, PPM } from "../constants";
import { BallSnapshot } from "../shared/types";
import {
  Segment,
  generatePlanetRingSegments,
  PLANET_RING_CENTER,
  PLANET_RING_RADIUS,
  escapeSlot,
} from "../board/BoardGeometry";

const MAX_SPACE_BALLS = 200;
const MIN_AGE_FOR_CAPTURE = 0.3; // seconds - prevents immediate re-capture

interface SpaceBall {
  id: number;
  x: number; // meters
  y: number;
  vx: number; // m/s
  vy: number;
  prevY: number; // previous y position (meters) for crossing detection
  age: number; // seconds since entering deep-space
  graphics: Graphics;
}

export class DeepSpaceLayer {
  container: Container;
  private bg: Graphics;
  private stars: {
    graphics: Graphics;
    baseAlpha: number;
    twinkleSpeed: number;
  }[] = [];
  private starsContainer: Container;
  private time = 0;
  private width = 800;
  private height = 600;

  private spaceBalls: Map<number, SpaceBall> = new Map();
  private ballContainer: Container;

  // Planetring (for rendering only - physics uses simple circle)
  private ringSegments: Segment[] = [];
  private ringGraphics: Graphics;

  // World transform (set by Game.resize)
  private worldScale = 1;
  private worldOffsetX = 0;
  private worldOffsetY = 0;

  // Ring collision constants (in meters)
  private readonly ringCenterX = PLANET_RING_CENTER.x / PPM;
  private readonly ringCenterY = PLANET_RING_CENTER.y / PPM;
  private readonly ringRadius = PLANET_RING_RADIUS / PPM;

  // Escape slot line (in meters) for crossing detection
  private readonly slotY = escapeSlot.yBottom / PPM;
  private readonly slotXMin = escapeSlot.xMin / PPM;
  private readonly slotXMax = escapeSlot.xMax / PPM;

  constructor() {
    this.container = new Container();

    // Dark background (will be resized)
    this.bg = new Graphics();
    this.container.addChild(this.bg);

    // Container for star graphics
    this.starsContainer = new Container();
    this.container.addChild(this.starsContainer);

    // Planetring (rendered above stars, below balls)
    this.ringGraphics = new Graphics();
    this.container.addChild(this.ringGraphics);
    this.ringSegments = generatePlanetRingSegments();

    // Container for deep-space proxy balls (rendered above ring)
    this.ballContainer = new Container();
    this.container.addChild(this.ballContainer);

    this.generateStars();
  }

  private generateStars() {
    // Clear old stars
    for (const star of this.stars) {
      star.graphics.destroy();
    }
    this.starsContainer.removeChildren();
    this.stars = [];

    // Create new star graphics (drawn once, only alpha changes)
    for (let i = 0; i < 200; i++) {
      const g = new Graphics();
      const x = Math.random() * this.width;
      const y = Math.random() * this.height;
      const size = Math.random() * 1.5 + 0.5;
      const baseAlpha = Math.random() * 0.6 + 0.2;

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

    // Redraw background
    this.bg.clear();
    this.bg.rect(0, 0, w, h);
    this.bg.fill({ color: COLORS.deepSpaceBg });

    // Regenerate stars for new size
    this.generateStars();

    // Redraw planet ring
    this.drawPlanetRing();
  }

  /** Set world transform so deep-space balls align with the board. */
  setWorldTransform(scale: number, offsetX: number, offsetY: number) {
    this.worldScale = scale;
    this.worldOffsetX = offsetX;
    this.worldOffsetY = offsetY;

    // Redraw ring with new transform
    this.drawPlanetRing();
  }

  /** Convert world pixels to screen pixels */
  private worldToScreen(p: { x: number; y: number }): { x: number; y: number } {
    return {
      x: this.worldOffsetX + this.worldScale * p.x,
      y: this.worldOffsetY + this.worldScale * p.y,
    };
  }

  /** Draw the planet ring (visual only - physics uses simple circle) */
  private drawPlanetRing() {
    const g = this.ringGraphics;
    g.clear();

    // Outer glow (svak)
    for (const seg of this.ringSegments) {
      const from = this.worldToScreen(seg.from);
      const to = this.worldToScreen(seg.to);
      g.moveTo(from.x, from.y);
      g.lineTo(to.x, to.y);
    }
    g.stroke({
      color: 0x4488ff,
      width: 4 * this.worldScale,
      alpha: 0.15,
    });

    // Inner line (tydeligere)
    for (const seg of this.ringSegments) {
      const from = this.worldToScreen(seg.from);
      const to = this.worldToScreen(seg.to);
      g.moveTo(from.x, from.y);
      g.lineTo(to.x, to.y);
    }
    g.stroke({
      color: 0x2266cc,
      width: 1.5 * this.worldScale,
      alpha: 0.5,
    });
  }

  /** Simple circle collision - much more efficient than segment collision */
  private checkCircleBoundary(ball: SpaceBall) {
    const dx = ball.x - this.ringCenterX;
    const dy = ball.y - this.ringCenterY;
    const distSq = dx * dx + dy * dy;
    const radiusSq = this.ringRadius * this.ringRadius;

    if (distSq > radiusSq) {
      const dist = Math.sqrt(distSq);
      const nx = dx / dist; // normal pointing outward
      const ny = dy / dist;

      // Reflect velocity: v' = v - 2(v·n)n
      const vDotN = ball.vx * nx + ball.vy * ny;
      if (vDotN > 0) {
        // Only reflect if moving outward
        ball.vx -= 2 * vDotN * nx;
        ball.vy -= 2 * vDotN * ny;

        // Push ball back inside
        ball.x = this.ringCenterX + nx * (this.ringRadius - 0.01);
        ball.y = this.ringCenterY + ny * (this.ringRadius - 0.01);
      }
    }
  }

  /** Add a ball to deep-space from an escape snapshot. */
  addBall(snapshot: BallSnapshot) {
    // Enforce cap: remove oldest if at limit
    if (this.spaceBalls.size >= MAX_SPACE_BALLS) {
      const oldestId = this.spaceBalls.keys().next().value;
      if (oldestId !== undefined) {
        this.removeBall(oldestId);
      }
    }

    const g = new Graphics();
    g.circle(0, 0, 5);
    g.fill({ color: COLORS.ballGlow, alpha: 0.7 });
    this.ballContainer.addChild(g);

    this.spaceBalls.set(snapshot.id, {
      id: snapshot.id,
      x: snapshot.x,
      y: snapshot.y,
      vx: snapshot.vx,
      vy: snapshot.vy,
      prevY: snapshot.y,
      age: 0,
      graphics: g,
    });
  }

  /** Remove a ball from deep-space (e.g. after capture). */
  removeBall(id: number) {
    const ball = this.spaceBalls.get(id);
    if (ball) {
      this.ballContainer.removeChild(ball.graphics);
      ball.graphics.destroy();
      this.spaceBalls.delete(id);
    }
  }

  /** Get deep-space balls entering through the port (using age + crossing detection) */
  getBallsEnteringPort(): BallSnapshot[] {
    const entering: BallSnapshot[] = [];

    for (const ball of this.spaceBalls.values()) {
      // Must have been in deep-space long enough
      if (ball.age < MIN_AGE_FOR_CAPTURE) continue;

      // Check if ball is within slot x-range
      if (ball.x < this.slotXMin || ball.x > this.slotXMax) continue;

      // Check for downward crossing of slot line (prevY < slotY && y >= slotY)
      if (ball.prevY < this.slotY && ball.y >= this.slotY) {
        entering.push({
          id: ball.id,
          x: ball.x,
          y: ball.y,
          vx: ball.vx,
          vy: ball.vy,
        });
      }
    }
    return entering;
  }

  update(dt: number) {
    this.time += dt;

    // Twinkle stars by adjusting alpha (no redraw needed)
    for (const star of this.stars) {
      const twinkle = Math.sin(this.time * star.twinkleSpeed) * 0.3 + 0.7;
      star.graphics.alpha = star.baseAlpha * twinkle;
    }

    // Simulate deep-space balls
    for (const ball of this.spaceBalls.values()) {
      // Store previous y for crossing detection
      ball.prevY = ball.y;

      // Update age
      ball.age += dt;

      // Move ball
      ball.x += ball.vx * dt;
      ball.y += ball.vy * dt;

      // Simple circle collision (replaces expensive segment collision)
      this.checkCircleBoundary(ball);

      // Convert to screen pixels using world transform
      const worldPx = ball.x * PPM;
      const worldPy = ball.y * PPM;
      const screenX = this.worldOffsetX + this.worldScale * worldPx;
      const screenY = this.worldOffsetY + this.worldScale * worldPy;

      ball.graphics.position.set(screenX, screenY);

      // Scale the ball graphics too
      ball.graphics.scale.set(this.worldScale);

      // Gentle fade based on distance from center (optional visual effect)
      ball.graphics.alpha = 0.5 + 0.3 * Math.sin(this.time * 2 + ball.x);
    }
  }
}
