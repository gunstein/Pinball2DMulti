import { Graphics, Container } from "pixi.js";
import { COLORS } from "../constants";
import { ballSpawn } from "./BoardGeometry";

const MAX_CHARGE = 1.0; // seconds to full charge
const MAX_LAUNCH_SPEED = 1.8; // m/s (ball velocity at full charge)
const COOLDOWN = 0.3; // seconds after launch before you can charge again

export class Launcher {
  private graphics: Graphics;
  private charge = 0;
  private cooldown = 0;
  private wasPressed = false;
  private chargePercent = 0;
  private launchCallback: ((speed: number) => void) | null = null;

  constructor(container: Container) {
    this.graphics = new Graphics();
    container.addChild(this.graphics);
  }

  onLaunch(callback: (speed: number) => void) {
    this.launchCallback = callback;
  }

  fixedUpdate(dt: number, active: boolean) {
    if (this.cooldown > 0) {
      this.cooldown -= dt;
      this.chargePercent = 0;
      return;
    }

    if (active) {
      // Charging
      this.charge = Math.min(MAX_CHARGE, this.charge + dt);
      this.wasPressed = true;
    } else if (this.wasPressed) {
      // Released - fire!
      const speed = (this.charge / MAX_CHARGE) * MAX_LAUNCH_SPEED;
      if (this.launchCallback) {
        this.launchCallback(speed);
      }
      this.charge = 0;
      this.cooldown = COOLDOWN;
      this.wasPressed = false;
    }

    this.chargePercent = this.charge / MAX_CHARGE;
  }

  render() {
    this.graphics.clear();

    if (this.chargePercent > 0) {
      const x = ballSpawn.x;
      const y = ballSpawn.y + 20;
      this.graphics.rect(x - 12, y, 24 * this.chargePercent, 3);
      this.graphics.fill({ color: COLORS.pinHit, alpha: 0.8 });
    }
  }
}
