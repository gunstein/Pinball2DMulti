const GAME_KEYS = new Set(["ArrowLeft", "ArrowRight", "Space"]);

export class InputManager {
  private keys: Map<string, boolean> = new Map();
  private abortController: AbortController;

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
  }

  destroy() {
    this.abortController.abort();
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
