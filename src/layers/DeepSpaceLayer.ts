import { Container, Graphics } from "pixi.js";
import { COLORS, PPM } from "../constants";
import { BallSnapshot } from "../shared/types";
import {
  Segment,
  generatePlanetRingSegments,
  PLANET_RING_CENTER,
  PLANET_RING_RADIUS,
  isInEscapeSlot,
} from "../board/BoardGeometry";
import { closestPointOnSegment, distance } from "../utils/geometry";

const MAX_SPACE_BALLS = 200;
const SPACE_BALL_RADIUS = 5; // piksel - radius for kollisjonsjekk

interface SpaceBall {
  id: number;
  x: number; // meters
  y: number;
  vx: number; // m/s
  vy: number;
  graphics: Graphics;
  hasLeftPort: boolean; // må forlate porten før den kan fanges
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

  // Planetring
  private ringSegments: Segment[] = [];
  private ringGraphics: Graphics;

  // World transform (set by Game.resize)
  private worldScale = 1;
  private worldOffsetX = 0;
  private worldOffsetY = 0;

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

  /** Draw the planet ring (subtil glød) */
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

  /** Check collision with outer boundary - circular boundary just outside the ring */
  private checkOuterBoundary(ball: SpaceBall) {
    const OUTER_RADIUS = PLANET_RING_RADIUS + 20; // pixels from center, just outside ring
    const cx = PLANET_RING_CENTER.x / PPM; // center in meters
    const cy = PLANET_RING_CENTER.y / PPM;
    const radius = OUTER_RADIUS / PPM;

    // Distance from center
    const dx = ball.x - cx;
    const dy = ball.y - cy;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (dist > radius) {
      // Ball is outside boundary - reflect it back
      const nx = dx / dist; // normal pointing outward
      const ny = dy / dist;

      // Reflect velocity: v' = v - 2(v·n)n
      const vDotN = ball.vx * nx + ball.vy * ny;
      if (vDotN > 0) {
        // Only reflect if moving outward
        ball.vx -= 2 * vDotN * nx;
        ball.vy -= 2 * vDotN * ny;

        // Push ball back inside
        ball.x = cx + nx * (radius - 0.01);
        ball.y = cy + ny * (radius - 0.01);
      }
    }
  }

  /** Check collision between a ball and the planet ring */
  private checkRingCollision(ball: SpaceBall) {
    const ballPx = ball.x * PPM;
    const ballPy = ball.y * PPM;

    for (const seg of this.ringSegments) {
      const closest = closestPointOnSegment(
        { x: ballPx, y: ballPy },
        seg.from,
        seg.to,
      );
      const dist = distance({ x: ballPx, y: ballPy }, closest);

      if (dist < SPACE_BALL_RADIUS) {
        this.reflectBall(ball, seg, closest);
        break; // Kun én kollisjon per frame
      }
    }
  }

  /** Reflect ball velocity off a wall segment */
  private reflectBall(
    ball: SpaceBall,
    seg: Segment,
    _closest: { x: number; y: number },
  ) {
    // Wall vector
    const wx = seg.to.x - seg.from.x;
    const wy = seg.to.y - seg.from.y;
    const len = Math.sqrt(wx * wx + wy * wy);
    if (len === 0) return;

    // Normal (perpendicular to wall)
    let nx = -wy / len;
    let ny = wx / len;

    // Ensure normal points inward (toward ring center)
    const midX = (seg.from.x + seg.to.x) / 2;
    const midY = (seg.from.y + seg.to.y) / 2;
    const toCenterX = PLANET_RING_CENTER.x - midX;
    const toCenterY = PLANET_RING_CENTER.y - midY;
    if (nx * toCenterX + ny * toCenterY < 0) {
      nx = -nx;
      ny = -ny;
    }

    // Convert velocity to pixels for reflection calculation
    const vxPx = ball.vx * PPM;
    const vyPx = ball.vy * PPM;

    // Reflection formula: v' = v - 2(v·n)n
    const vDotN = vxPx * nx + vyPx * ny;

    // Only reflect if moving toward the wall
    if (vDotN > 0) return;

    const newVxPx = vxPx - 2 * vDotN * nx;
    const newVyPx = vyPx - 2 * vDotN * ny;

    // Convert back to meters
    ball.vx = newVxPx / PPM;
    ball.vy = newVyPx / PPM;

    // Push ball slightly away from wall to prevent sticking
    ball.x = (ball.x * PPM + nx * 2) / PPM;
    ball.y = (ball.y * PPM + ny * 2) / PPM;
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
      graphics: g,
      hasLeftPort: false, // må forlate porten før den kan fanges
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

  /** Get all deep-space balls currently inside the given AABB (in world pixels). */
  getBallsInArea(bounds: {
    left: number;
    right: number;
    top: number;
    bottom: number;
  }): BallSnapshot[] {
    const result: BallSnapshot[] = [];
    for (const ball of this.spaceBalls.values()) {
      const px = ball.x * PPM;
      const py = ball.y * PPM;
      if (
        px >= bounds.left &&
        px <= bounds.right &&
        py >= bounds.top &&
        py <= bounds.bottom
      ) {
        result.push({
          id: ball.id,
          x: ball.x,
          y: ball.y,
          vx: ball.vx,
          vy: ball.vy,
        });
      }
    }
    return result;
  }

  /** Get deep-space balls entering through the port (moving inward) */
  getBallsEnteringPort(): BallSnapshot[] {
    const entering: BallSnapshot[] = [];

    // Board center in pixels
    const boardCenterX = PLANET_RING_CENTER.x;
    const boardCenterY = PLANET_RING_CENTER.y;
    // Minimum distance from center to be considered "in deep space"
    // Board half-height is 320, so anything beyond ~350 from center is in deep-space
    const minDeepSpaceDistance = 300; // pixels

    for (const ball of this.spaceBalls.values()) {
      const px = ball.x * PPM;
      const py = ball.y * PPM;

      // Distance from board center
      const dx = px - boardCenterX;
      const dy = py - boardCenterY;
      const distFromCenter = Math.sqrt(dx * dx + dy * dy);

      const inSlot = isInEscapeSlot(px, py);

      // Track when ball has traveled far enough from the port (truly in deep-space)
      if (distFromCenter > minDeepSpaceDistance && !ball.hasLeftPort) {
        ball.hasLeftPort = true;
      }

      // Only capture if:
      // 1. Ball has traveled far away from the port (truly was in deep-space)
      // 2. Ball is now in the escape slot area
      // 3. Ball is moving downward (into the board)
      // 4. Ball is near the board top (y around 30 pixels)
      const nearBoardTop = py > 20 && py < 60;
      if (inSlot && ball.hasLeftPort && ball.vy > 0 && nearBoardTop) {
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

    // Simulate deep-space balls (no damping - constant velocity)
    for (const ball of this.spaceBalls.values()) {
      // Move ball
      ball.x += ball.vx * dt;
      ball.y += ball.vy * dt;

      // Check collision with planet ring
      this.checkRingCollision(ball);

      // Check collision with outer boundary (keeps balls in play area)
      this.checkOuterBoundary(ball);

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
