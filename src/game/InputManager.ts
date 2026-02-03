import { getInputZone } from "../board/BoardMetrics";

const GAME_KEYS = new Set(["ArrowLeft", "ArrowRight", "Space"]);

export class InputManager {
  private keys: Map<string, boolean> = new Map();
  private abortController: AbortController;

  // Touch state
  private touchLeft = false;
  private touchRight = false;
  private touchLaunch = false;
  private activeTouches: Map<number, "left" | "right" | "launch" | "none"> =
    new Map();

  // Transform from screen to game coordinates
  private scale = 1;
  private offsetX = 0;
  private offsetY = 0;

  constructor() {
    this.abortController = new AbortController();
    const opts = { signal: this.abortController.signal };

    window.addEventListener(
      "keydown",
      (e) => {
        this.keys.set(e.code, true);
        if (GAME_KEYS.has(e.code)) {
          e.preventDefault();
        }
      },
      opts,
    );

    window.addEventListener(
      "keyup",
      (e) => {
        this.keys.set(e.code, false);
      },
      opts,
    );

    // Touch events
    window.addEventListener(
      "touchstart",
      (e) => this.handleTouchStart(e),
      opts,
    );
    window.addEventListener("touchend", (e) => this.handleTouchEnd(e), opts);
    window.addEventListener("touchcancel", (e) => this.handleTouchEnd(e), opts);
  }

  /** Update the screen-to-game coordinate transform */
  setTransform(scale: number, offsetX: number, offsetY: number) {
    this.scale = scale;
    this.offsetX = offsetX;
    this.offsetY = offsetY;
  }

  /** Convert screen coordinates to game coordinates */
  private screenToGame(
    screenX: number,
    screenY: number,
  ): { x: number; y: number } {
    return {
      x: (screenX - this.offsetX) / this.scale,
      y: (screenY - this.offsetY) / this.scale,
    };
  }

  private handleTouchStart(e: TouchEvent) {
    e.preventDefault();

    for (const touch of Array.from(e.changedTouches)) {
      const zone = this.getTouchZone(touch.clientX, touch.clientY);
      this.activeTouches.set(touch.identifier, zone);
    }
    this.updateTouchState();
  }

  private handleTouchEnd(e: TouchEvent) {
    for (const touch of Array.from(e.changedTouches)) {
      this.activeTouches.delete(touch.identifier);
    }
    this.updateTouchState();
  }

  private getTouchZone(
    screenX: number,
    screenY: number,
  ): "left" | "right" | "launch" | "none" {
    const { x: gameX, y: gameY } = this.screenToGame(screenX, screenY);
    return getInputZone(gameX, gameY);
  }

  private updateTouchState() {
    this.touchLeft = false;
    this.touchRight = false;
    this.touchLaunch = false;

    for (const zone of this.activeTouches.values()) {
      if (zone === "left") this.touchLeft = true;
      if (zone === "right") this.touchRight = true;
      if (zone === "launch") this.touchLaunch = true;
    }
  }

  destroy() {
    this.abortController.abort();
  }

  get leftFlipper(): boolean {
    return this.keys.get("ArrowLeft") || this.touchLeft;
  }

  get rightFlipper(): boolean {
    return this.keys.get("ArrowRight") || this.touchRight;
  }

  get launch(): boolean {
    return this.keys.get("Space") || this.touchLaunch;
  }
}
