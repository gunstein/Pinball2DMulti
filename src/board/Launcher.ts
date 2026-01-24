import { Graphics, Container } from "pixi.js";
import { COLORS } from "../constants";
import { ballSpawn } from "./BoardGeometry";

const MAX_CHARGE = 1.0; // seconds to full charge
const MAX_IMPULSE = 0.4; // physics impulse magnitude at full charge
const COOLDOWN = 0.3; // seconds after launch before you can charge again
const PLUNGER_TRAVEL = 25; // pixels the plunger graphic moves down when charging

export class Launcher {
  private graphics: Graphics;
  private charge = 0;
  private cooldown = 0;
  private wasPressed = false;
  private launchCallback: ((power: number) => void) | null = null;

  constructor(container: Container) {
    this.graphics = new Graphics();
    container.addChild(this.graphics);
    this.draw(0);
  }

  onLaunch(callback: (power: number) => void) {
    this.launchCallback = callback;
  }

  update(dt: number, active: boolean) {
    if (this.cooldown > 0) {
      this.cooldown -= dt;
      this.draw(0);
      return;
    }

    if (active) {
      // Charging
      this.charge = Math.min(MAX_CHARGE, this.charge + dt);
      this.wasPressed = true;
    } else if (this.wasPressed) {
      // Released - fire!
      const power = (this.charge / MAX_CHARGE) * MAX_IMPULSE;
      if (this.launchCallback) {
        this.launchCallback(power);
      }
      this.charge = 0;
      this.cooldown = COOLDOWN;
      this.wasPressed = false;
    }

    this.draw(this.charge / MAX_CHARGE);
  }

  private draw(chargePercent: number) {
    this.graphics.clear();

    const x = ballSpawn.x;
    const baseY = ballSpawn.y + 20; // plunger base position (below ball spawn)
    const plungerY = baseY + chargePercent * PLUNGER_TRAVEL;
    const plungerWidth = 20;
    const plungerHeight = 10;

    // Plunger head
    this.graphics.rect(
      x - plungerWidth / 2,
      plungerY - plungerHeight / 2,
      plungerWidth,
      plungerHeight,
    );
    this.graphics.fill({ color: 0x000000 });
    this.graphics.stroke({ color: COLORS.launcher, width: 2 });

    // Spring visual (zigzag below plunger)
    const springTop = plungerY + plungerHeight / 2;
    const springBottom = baseY + PLUNGER_TRAVEL + 20;
    const segments = 5;
    const segH = (springBottom - springTop) / segments;

    for (let i = 0; i < segments; i++) {
      const y1 = springTop + i * segH;
      const y2 = springTop + (i + 0.5) * segH;
      const xOff = i % 2 === 0 ? 4 : -4;
      this.graphics.moveTo(x + xOff, y1);
      this.graphics.lineTo(x - xOff, y2);
    }
    this.graphics.stroke({ color: COLORS.launcher, width: 1, alpha: 0.5 });

    // Charge indicator
    if (chargePercent > 0) {
      this.graphics.rect(x - 12, baseY - 25, 24 * chargePercent, 3);
      this.graphics.fill({ color: COLORS.pinHit, alpha: 0.8 });
    }
  }
}
