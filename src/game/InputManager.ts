const GAME_KEYS = new Set(["ArrowLeft", "ArrowRight", "Space"]);

export class InputManager {
  private keys: Map<string, boolean> = new Map();

  constructor() {
    window.addEventListener("keydown", (e) => {
      this.keys.set(e.code, true);
      if (GAME_KEYS.has(e.code)) {
        e.preventDefault();
      }
    });
    window.addEventListener("keyup", (e) => {
      this.keys.set(e.code, false);
    });
  }

  get leftFlipper(): boolean {
    return this.keys.get("ArrowLeft") || false;
  }

  get rightFlipper(): boolean {
    return this.keys.get("ArrowRight") || false;
  }

  get launch(): boolean {
    return this.keys.get("Space") || false;
  }
}
