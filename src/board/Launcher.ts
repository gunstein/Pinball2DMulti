import { Graphics, Container } from "pixi.js";
import { COLORS } from "../constants";
import { ballSpawn } from "./BoardGeometry";
import {
  LauncherState,
  initialLauncherState,
  stepLauncher,
} from "./launcherLogic";

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

    this.chargePercent = this.state.charge / 1.0; // MAX_CHARGE = 1.0
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
