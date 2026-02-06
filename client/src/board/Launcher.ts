import { Graphics, Container } from "pixi.js";
import { COLORS } from "../constants";
import { ballSpawn } from "./BoardGeometry";
import {
  LauncherState,
  initialLauncherState,
  stepLauncher,
  MAX_CHARGE,
} from "./launcherLogic";

// Charge bar dimensions (pixels)
const CHARGE_BAR_WIDTH = 24;
const CHARGE_BAR_HEIGHT = 3;
const CHARGE_BAR_OFFSET_Y = 20; // below ball spawn point

export class Launcher {
  private graphics: Graphics;
  private state: LauncherState = initialLauncherState();
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
    const { state, fired } = stepLauncher(this.state, dt, active);
    this.state = state;

    if (fired !== null && this.launchCallback) {
      this.launchCallback(fired);
    }

    this.chargePercent = this.state.charge / MAX_CHARGE;
  }

  render() {
    this.graphics.clear();

    if (this.chargePercent > 0) {
      const x = ballSpawn.x;
      const y = ballSpawn.y + CHARGE_BAR_OFFSET_Y;
      this.graphics.rect(
        x - CHARGE_BAR_WIDTH / 2,
        y,
        CHARGE_BAR_WIDTH * this.chargePercent,
        CHARGE_BAR_HEIGHT,
      );
      this.graphics.fill({ color: COLORS.pinHit, alpha: 0.8 });
    }
  }
}
