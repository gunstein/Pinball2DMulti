const GAME_KEYS = new Set(["ArrowLeft", "ArrowRight", "Space"]);

export class InputManager {
  private keys: Map<string, boolean> = new Map();
  private abortController: AbortController;

  // Touch state
  private touchLeft = false;
  private touchRight = false;
  private touchLaunch = false;
  private activeTouches: Map<number, "left" | "right" | "launch"> = new Map();

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

  private handleTouchStart(e: TouchEvent) {
    e.preventDefault();
    const screenWidth = window.innerWidth;
    const screenHeight = window.innerHeight;

    for (const touch of Array.from(e.changedTouches)) {
      const zone = this.getTouchZone(
        touch.clientX,
        touch.clientY,
        screenWidth,
        screenHeight,
      );
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
    x: number,
    y: number,
    screenWidth: number,
    screenHeight: number,
  ): "left" | "right" | "launch" {
    // Bottom center = launcher zone
    const launcherWidth = screenWidth * 0.3;
    const launcherHeight = screenHeight * 0.25;
    const launcherLeft = (screenWidth - launcherWidth) / 2;

    if (
      y > screenHeight - launcherHeight &&
      x > launcherLeft &&
      x < launcherLeft + launcherWidth
    ) {
      return "launch";
    }

    // Left/right halves for flippers
    return x < screenWidth / 2 ? "left" : "right";
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
