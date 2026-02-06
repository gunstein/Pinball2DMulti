import { describe, it, expect, beforeEach, vi } from "vitest";
import type { BallSnapshot } from "../src/shared/types";

// --- Pixi stub (so Game can be imported without a DOM/canvas) ---
vi.mock("pixi.js", () => {
  class Container {
    children: unknown[] = [];
    scale = { set: vi.fn() };
    position = { set: vi.fn() };
    addChild(child: unknown) {
      this.children.push(child);
    }
  }

  class Graphics {}
  class Text {
    text: string;
    anchor = { set: vi.fn() };
    x = 0;
    y = 0;
    constructor(opts?: { text?: string }) {
      this.text = opts?.text ?? "";
    }
  }

  class Application {
    stage = new Container();
    ticker = { add: vi.fn() };
  }

  return { Container, Graphics, Text, Application };
});

// --- PhysicsWorld mock ---
class PhysicsWorldMock {
  static lastInstance: PhysicsWorldMock | null = null;
  static reset() {
    PhysicsWorldMock.lastInstance = null;
  }

  world = {};
  collisions: Array<[number, number, boolean]> = [];
  eventQueue = {
    drainCollisionEvents: (
      cb: (h1: number, h2: number, started: boolean) => void,
    ) => {
      for (const [h1, h2, started] of this.collisions) {
        cb(h1, h2, started);
      }
      this.collisions = [];
    },
  };

  constructor() {
    PhysicsWorldMock.lastInstance = this;
  }

  step(_: number) {}

  queueCollision(h1: number, h2: number, started = true) {
    this.collisions.push([h1, h2, started]);
  }
}

vi.mock("../src/physics/PhysicsWorld", () => ({
  PhysicsWorld: PhysicsWorldMock,
}));

// --- Board mock ---
class BoardMock {
  drainColliderHandle = 100;
  constructor() {}
}
vi.mock("../src/board/Board", () => ({ Board: BoardMock }));

// --- Flipper mock ---
class FlipperMock {
  fixedUpdate() {}
  render() {}
}
vi.mock("../src/board/Flipper", () => ({ Flipper: FlipperMock }));

// --- Launcher mock ---
class LauncherMock {
  private cb: ((speed: number) => void) | null = null;
  onLaunch(cb: (speed: number) => void) {
    this.cb = cb;
  }
  fixedUpdate() {}
  render() {}
  trigger(speed: number) {
    this.cb?.(speed);
  }
}
vi.mock("../src/board/Launcher", () => ({ Launcher: LauncherMock }));

// --- Pin mock ---
let nextPinHandle = 200;
class PinMock {
  colliderHandle = nextPinHandle++;
  constructor() {}
  fixedUpdate() {}
  render() {}
  hit() {}
}
vi.mock("../src/board/Pin", () => ({ Pin: PinMock }));

// --- Ball mock ---
let nextBallHandle = 1;
class BallMock {
  static instances: BallMock[] = [];
  static reset() {
    BallMock.instances = [];
    nextBallHandle = 1;
  }

  colliderHandle = nextBallHandle++;
  active = true;
  inLauncher = true;
  inShooterLane = true;
  snapshot: BallSnapshot | null = null;
  lastLaunchSpeed: number | null = null;

  constructor() {
    BallMock.instances.push(this);
  }

  setTint() {}
  respawn() {
    this.active = true;
    this.inLauncher = true;
  }
  injectFromCapture() {
    this.active = true;
    this.inLauncher = false;
  }
  isActive() {
    return this.active;
  }
  isInLauncher() {
    return this.inLauncher;
  }
  isInShooterLane() {
    return this.inShooterLane;
  }
  fixedUpdate() {}
  render() {}
  setInactive() {
    this.active = false;
  }
  getEscapeSnapshot() {
    return this.active ? this.snapshot : null;
  }
  launch(speed: number) {
    this.lastLaunchSpeed = speed;
  }
}
vi.mock("../src/board/Ball", () => ({ Ball: BallMock }));

// --- Layers mocks ---
class BoardLayerMock {
  container: {
    addChild: (child: unknown) => void;
    scale: unknown;
    position: unknown;
  };
  constructor() {
    this.container = {
      addChild: () => {},
      scale: { set: vi.fn() },
      position: { set: vi.fn() },
    };
  }
}
vi.mock("../src/layers/BoardLayer", () => ({ BoardLayer: BoardLayerMock }));

class UILayerMock {
  container: {
    addChild: (child: unknown) => void;
    scale: unknown;
    position: unknown;
  };
  addHit = vi.fn();
  setConnectionState = vi.fn();
  setPlayers = vi.fn();
  constructor() {
    this.container = {
      addChild: () => {},
      scale: { set: vi.fn() },
      position: { set: vi.fn() },
    };
  }
}
vi.mock("../src/layers/UILayer", () => ({ UILayer: UILayerMock }));

class SphereDeepSpaceLayerMock {
  container: { addChild: (child: unknown) => void };
  setSelfPortal = vi.fn();
  markColorsDirty = vi.fn();
  resize = vi.fn();
  setCenter = vi.fn();
  update = vi.fn();
  constructor() {
    this.container = { addChild: () => {} };
  }
}
vi.mock("../src/layers/SphereDeepSpaceLayer", () => ({
  SphereDeepSpaceLayer: SphereDeepSpaceLayerMock,
}));

// --- InputManager mock ---
class InputManagerMock {
  leftFlipper = false;
  rightFlipper = false;
  launch = false;
  setTransform() {}
}
vi.mock("../src/game/InputManager", () => ({ InputManager: InputManagerMock }));

// --- DeepSpaceClient mock ---
class DeepSpaceClientMock {
  static lastInstance: DeepSpaceClientMock | null = null;
  ballEscapedCalls: Array<{ vx: number; vy: number }> = [];

  constructor() {
    DeepSpaceClientMock.lastInstance = this;
  }

  tick() {}
  getBalls() {
    return [];
  }
  getAllPlayers() {
    return [];
  }
  getSelfPlayer() {
    return null;
  }
  ballEscaped(vx: number, vy: number) {
    this.ballEscapedCalls.push({ vx, vy });
  }
}
vi.mock("../src/shared/DeepSpaceClient", () => ({
  DeepSpaceClient: DeepSpaceClientMock,
}));

async function createGame() {
  // Ensure location exists before Game module evaluates
  (globalThis as any).location = { protocol: "http:", host: "localhost" };

  const { Container } = await import("pixi.js");
  const { Game } = await import("../src/game/Game");
  const appStub = {
    stage: new Container(),
    ticker: { add: vi.fn() },
  };
  return new Game(appStub as any);
}

beforeEach(() => {
  BallMock.reset();
  PhysicsWorldMock.reset();
  DeepSpaceClientMock.lastInstance = null;
});

describe("Game respawn flow", () => {
  it("respawns ball after timer expires when no balls remain", async () => {
    const game = await createGame();
    const physics = PhysicsWorldMock.lastInstance!;
    const board = (game as any).board as BoardMock;
    const ball = (game as any).balls[0] as BallMock;

    // Drain the ball
    physics.queueCollision(
      board.drainColliderHandle,
      ball.colliderHandle,
      true,
    );
    (game as any).processCollisions();

    expect((game as any).balls.length).toBe(0);
    expect((game as any).respawnTimer).toBeGreaterThan(0);

    // Simulate update ticks until respawn timer expires
    const respawnDelay = (game as any).respawnTimer;
    (game as any).update(respawnDelay + 0.01);

    // A new ball should have been spawned
    expect((game as any).balls.length).toBe(1);
    expect((game as any).launcherBall).not.toBeNull();
  });

  it("fallback creates respawn timer when no balls and no timer", async () => {
    const game = await createGame();

    // Force empty state: no balls, no launcher ball, no timer
    (game as any).balls.length = 0;
    (game as any).launcherBall = null;
    (game as any).respawnTimer = 0;

    // A single update should set the respawn timer
    (game as any).update(0.016);

    expect((game as any).respawnTimer).toBeGreaterThan(0);
  });

  it("does not respawn while balls still exist", async () => {
    const game = await createGame();

    // Ball exists, respawn timer should not be active
    expect((game as any).balls.length).toBe(1);
    expect((game as any).respawnTimer).toBe(0);

    // Update should not change anything
    (game as any).update(1.0);
    expect((game as any).balls.length).toBeGreaterThanOrEqual(1);
  });

  it("caps inactive ball pool at 10", async () => {
    const game = await createGame();
    const physics = PhysicsWorldMock.lastInstance!;
    const board = (game as any).board as BoardMock;

    // Drain many balls to fill the pool
    for (let i = 0; i < 15; i++) {
      const ball = new BallMock();
      (game as any).balls.push(ball);
      (game as any).ballByHandle.set(ball.colliderHandle, ball);

      physics.queueCollision(
        board.drainColliderHandle,
        ball.colliderHandle,
        true,
      );
      (game as any).processCollisions();
    }

    // Pool should be capped
    expect((game as any).inactiveBalls.length).toBeLessThanOrEqual(10);
  });
});

describe("Game lifecycle", () => {
  it("removes ball on drain collision and schedules respawn", async () => {
    const game = await createGame();
    const physics = PhysicsWorldMock.lastInstance!;
    const board = (game as any).board as BoardMock;
    const ball = (game as any).balls[0] as BallMock;

    physics.queueCollision(
      board.drainColliderHandle,
      ball.colliderHandle,
      true,
    );

    (game as any).processCollisions();

    expect(ball.active).toBe(false);
    expect((game as any).balls.length).toBe(0);
    expect((game as any).respawnTimer).toBeGreaterThan(0);
  });

  it("sends escape snapshot to deep-space and removes ball", async () => {
    const game = await createGame();
    const ball = (game as any).balls[0] as BallMock;
    const deepSpace = DeepSpaceClientMock.lastInstance!;

    ball.snapshot = { id: 1, x: 0, y: 0, vx: 1.2, vy: -2.3 };

    (game as any).checkEscape();

    expect(deepSpace.ballEscapedCalls.length).toBe(1);
    expect(deepSpace.ballEscapedCalls[0]).toEqual({ vx: 1.2, vy: -2.3 });
    expect(ball.active).toBe(false);
    expect((game as any).balls.length).toBe(0);
  });

  it("scales launcher speed by count squared for stacked balls", async () => {
    const game = await createGame();
    const launcher = (game as any).launcher as LauncherMock;

    // Add two balls to the shooter lane
    const extraBall = new BallMock();
    extraBall.inShooterLane = true;
    (game as any).balls.push(extraBall);

    // Trigger a launch with base speed 2
    launcher.trigger(2);

    const balls = (game as any).balls as BallMock[];
    // Count is 2 -> scale = 2 * 2^2 = 8
    for (const b of balls) {
      expect(b.lastLaunchSpeed).toBe(8);
    }
  });
});
