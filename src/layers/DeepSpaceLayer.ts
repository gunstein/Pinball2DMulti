import { Container, Graphics } from "pixi.js";
import { COLORS } from "../constants";

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
  private width = 800;
  private height = 600;

  constructor() {
    this.container = new Container();

    // Dark background (will be resized)
    this.bg = new Graphics();
    this.container.addChild(this.bg);

    // Generate stars
    this.starsGraphics = new Graphics();
    this.container.addChild(this.starsGraphics);

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

  update(dt: number) {
    this.time += dt;
    if (Math.floor(this.time * 10) % 3 === 0) {
      this.drawStars();
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
