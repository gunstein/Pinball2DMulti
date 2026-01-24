import { Container, Graphics } from 'pixi.js';
import { CANVAS_WIDTH, CANVAS_HEIGHT, COLORS } from '../constants';

export class DeepSpaceLayer {
  container: Container;
  private stars: { x: number; y: number; size: number; alpha: number; twinkleSpeed: number }[] = [];
  private starsGraphics: Graphics;
  private time = 0;

  constructor() {
    this.container = new Container();

    // Dark background
    const bg = new Graphics();
    bg.rect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
    bg.fill({ color: COLORS.deepSpaceBg });
    this.container.addChild(bg);

    // Generate stars
    this.starsGraphics = new Graphics();
    this.container.addChild(this.starsGraphics);

    for (let i = 0; i < 150; i++) {
      this.stars.push({
        x: Math.random() * CANVAS_WIDTH,
        y: Math.random() * CANVAS_HEIGHT,
        size: Math.random() * 1.5 + 0.5,
        alpha: Math.random() * 0.6 + 0.2,
        twinkleSpeed: Math.random() * 2 + 0.5,
      });
    }

    this.drawStars();
  }

  update(dt: number) {
    this.time += dt;
    // Redraw stars with twinkle every few frames
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
