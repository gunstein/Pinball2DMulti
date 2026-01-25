import { Container, Graphics } from "pixi.js";
import { COLORS, PPM } from "../constants";
import { BallSnapshot } from "../shared/types";

const MAX_SPACE_BALLS = 200;

interface SpaceBall {
  id: number;
  x: number; // meters
  y: number;
  vx: number; // m/s
  vy: number;
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

    // Container for deep-space proxy balls (rendered above stars)
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
  }

  /** Set world transform so deep-space balls align with the board. */
  setWorldTransform(scale: number, offsetX: number, offsetY: number) {
    this.worldScale = scale;
    this.worldOffsetX = offsetX;
    this.worldOffsetY = offsetY;
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

  update(dt: number) {
    this.time += dt;

    // Twinkle stars by adjusting alpha (no redraw needed)
    for (const star of this.stars) {
      const twinkle = Math.sin(this.time * star.twinkleSpeed) * 0.3 + 0.7;
      star.graphics.alpha = star.baseAlpha * twinkle;
    }

    // Simulate deep-space balls (linear movement, no physics)
    for (const ball of this.spaceBalls.values()) {
      ball.x += ball.vx * dt;
      ball.y += ball.vy * dt;

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
