import { Container, Graphics } from "pixi.js";
import { COLORS, PPM } from "../constants";
import { BallSnapshot } from "../shared/types";

interface SpaceBall {
  id: string;
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
    x: number;
    y: number;
    size: number;
    alpha: number;
    twinkleSpeed: number;
  }[] = [];
  private starsGraphics: Graphics;
  private time = 0;
  private frameCounter = 0;
  private width = 800;
  private height = 600;

  private spaceBalls: Map<string, SpaceBall> = new Map();
  private ballContainer: Container;

  constructor() {
    this.container = new Container();

    // Dark background (will be resized)
    this.bg = new Graphics();
    this.container.addChild(this.bg);

    // Generate stars
    this.starsGraphics = new Graphics();
    this.container.addChild(this.starsGraphics);

    // Container for deep-space proxy balls (rendered above stars)
    this.ballContainer = new Container();
    this.container.addChild(this.ballContainer);

    this.generateStars();
  }

  private generateStars() {
    this.stars = [];
    for (let i = 0; i < 200; i++) {
      this.stars.push({
        x: Math.random() * this.width,
        y: Math.random() * this.height,
        size: Math.random() * 1.5 + 0.5,
        alpha: Math.random() * 0.6 + 0.2,
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
    this.drawStars();
  }

  /** Add a ball to deep-space from an escape snapshot. */
  addBall(snapshot: BallSnapshot) {
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
  removeBall(id: string) {
    const ball = this.spaceBalls.get(id);
    if (ball) {
      this.ballContainer.removeChild(ball.graphics);
      ball.graphics.destroy();
      this.spaceBalls.delete(id);
    }
  }

  /** Get all deep-space balls currently inside the given AABB (in pixels). */
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
    this.frameCounter++;

    // Redraw stars every 20 frames for subtle twinkling
    if (this.frameCounter % 20 === 0) {
      this.drawStars();
    }

    // Simulate deep-space balls (linear movement, no physics)
    for (const ball of this.spaceBalls.values()) {
      ball.x += ball.vx * dt;
      ball.y += ball.vy * dt;

      // Convert to screen pixels for rendering
      const px = ball.x * PPM;
      const py = ball.y * PPM;
      ball.graphics.position.set(px, py);

      // Gentle fade based on distance from center (optional visual effect)
      ball.graphics.alpha = 0.5 + 0.3 * Math.sin(this.time * 2 + ball.x);
    }
  }

  private drawStars() {
    this.starsGraphics.clear();
    for (const star of this.stars) {
      const twinkle = Math.sin(this.time * star.twinkleSpeed) * 0.3 + 0.7;
      const alpha = star.alpha * twinkle;
      this.starsGraphics.circle(star.x, star.y, star.size);
      this.starsGraphics.fill({ color: COLORS.star, alpha });
    }
  }
}
