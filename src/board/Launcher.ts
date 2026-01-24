import { Graphics, Container } from "pixi.js";
import { COLORS } from "../constants";
import { ballSpawn } from "./BoardGeometry";

const MAX_CHARGE = 1.0; // seconds to full charge
const MAX_IMPULSE = 0.065; // physics impulse magnitude at full charge
const COOLDOWN = 0.3; // seconds after launch before you can charge again

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

    // Only show a small charge indicator bar when charging
    if (chargePercent > 0) {
      const x = ballSpawn.x;
      const y = ballSpawn.y + 20;
      this.graphics.rect(x - 12, y, 24 * chargePercent, 3);
      this.graphics.fill({ color: COLORS.pinHit, alpha: 0.8 });
    }
  }
}
