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
    removeChild(child: unknown) {
      const i = this.children.indexOf(child);
      if (i !== -1) this.children.splice(i, 1);
    }
    destroy = vi.fn();
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
    ticker = { add: vi.fn(), remove: vi.fn() };
  }

  return { Container, Graphics, Text, Application };
});

// --- PhysicsWorld mock ---
class PhysicsWorldMock {
  static lastInstance: PhysicsWorldMock | null = null;
  static reset() {
    PhysicsWorldMock.lastInstance = null;
  }

  world = {
    free: vi.fn(),
  };
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
    free: vi.fn(),
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
  escapeColliderHandle = 101;
  constructor() {}
}
vi.mock("../src/board/Board", () => ({ Board: BoardMock }));

// --- Flipper mock ---
class FlipperMock {
  fixedUpdate() {}
  capturePreStepPosition() {}
  capturePostStepPosition() {}
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
  velocity = { x: 0, y: -1 };

  constructor() {
    BallMock.instances.push(this);
  }

  lastTint: number | null = null;
  setTint(color: number) {
    this.lastTint = color;
  }
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
  capturePreStepPosition() {}
  capturePostStepPosition() {}
  render() {}
  destroy() {}
  setInactive() {
    this.active = false;
  }
  getEscapeSnapshot() {
    return this.active ? this.snapshot : null;
  }
  getVelocity() {
    return this.velocity;
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
    removeChild: (child: unknown) => void;
    scale: unknown;
    position: unknown;
    destroy: () => void;
  };
  constructor() {
    this.container = {
      addChild: () => {},
      removeChild: () => {},
      scale: { set: vi.fn() },
      position: { set: vi.fn() },
      destroy: () => {},
    };
  }
}
vi.mock("../src/layers/BoardLayer", () => ({ BoardLayer: BoardLayerMock }));

class UILayerMock {
  container: {
    addChild: (child: unknown) => void;
    removeChild: (child: unknown) => void;
    scale: unknown;
    position: unknown;
    destroy: () => void;
  };
  addHit = vi.fn();
  setConnectionState = vi.fn();
  setPlayers = vi.fn();
  constructor() {
    this.container = {
      addChild: () => {},
      removeChild: () => {},
      scale: { set: vi.fn() },
      position: { set: vi.fn() },
      destroy: () => {},
    };
  }
}
vi.mock("../src/layers/UILayer", () => ({ UILayer: UILayerMock }));

class SphereDeepSpaceLayerMock {
  container: {
    addChild: (child: unknown) => void;
    removeChild: (child: unknown) => void;
    destroy: () => void;
  };
  setSelfPortal = vi.fn();
  markColorsDirty = vi.fn();
  resize = vi.fn();
  setCenter = vi.fn();
  update = vi.fn();
  constructor() {
    this.container = {
      addChild: () => {},
      removeChild: () => {},
      destroy: () => {},
    };
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
  destroy = vi.fn();
}
vi.mock("../src/game/InputManager", () => ({ InputManager: InputManagerMock }));

// --- DeepSpaceClient mock ---
class DeepSpaceClientMock {
  static lastInstance: DeepSpaceClientMock | null = null;
  ballEscapedCalls: Array<{ vx: number; vy: number }> = [];
  callbacks: any = null;

  constructor(
    _useServer: boolean,
    _url: string,
    _count: number,
    callbacks: any,
  ) {
    DeepSpaceClientMock.lastInstance = this;
    this.callbacks = callbacks;
    // Simulate real DeepSpaceClient: immediately call onPlayersChanged
    // with a temporary local player that has teal color (0x4da6a6),
    // before the server welcome arrives.
    callbacks.onPlayersChanged(
      [
        {
          id: 0,
          cellIndex: 0,
          portalPos: { x: 0, y: 0, z: 1 },
          color: 0x4da6a6,
          paused: false,
          ballsProduced: 0,
          ballsInFlight: 0,
        },
      ],
      0,
    );
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
  sendActivity() {}
  dispose = vi.fn();
  getServerVersion() {
    return "";
  }
}
vi.mock("../src/shared/DeepSpaceClient", () => ({
  DeepSpaceClient: DeepSpaceClientMock,
}));

async function createGameWithApp() {
  // Ensure location exists before Game module evaluates
  (globalThis as any).location = { protocol: "http:", host: "localhost" };

  const { Container } = await import("pixi.js");
  const { Game } = await import("../src/game/Game");
  const appStub = {
    stage: new Container(),
    ticker: { add: vi.fn(), remove: vi.fn() },
  };
  return { game: new Game(appStub as any), appStub };
}

async function createGame() {
  return (await createGameWithApp()).game;
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

  it("clears stale launcher reference so empty board can respawn", async () => {
    const game = await createGame();

    const stale = (game as any).launcherBall;
    stale.active = false;
    (game as any).balls.length = 0;
    (game as any).respawnTimer = 0;

    (game as any).update(0.016);

    expect((game as any).launcherBall).toBeNull();
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
  it("start is idempotent and does not double-register ticker callback", async () => {
    const { game, appStub } = await createGameWithApp();

    game.start();
    game.start();

    expect(appStub.ticker.add).toHaveBeenCalledTimes(1);
  });

  it("destroy unregisters ticker and can be called twice safely", async () => {
    const { game, appStub } = await createGameWithApp();
    const deepSpace = DeepSpaceClientMock.lastInstance!;
    const physics = PhysicsWorldMock.lastInstance!;
    const input = (game as any).input as InputManagerMock;

    game.start();
    game.destroy();
    game.destroy();

    expect(appStub.ticker.remove).toHaveBeenCalledTimes(1);
    expect(input.destroy).toHaveBeenCalledTimes(1);
    expect(deepSpace.dispose).toHaveBeenCalledTimes(1);
    expect(physics.world.free).toHaveBeenCalledTimes(1);
    expect(physics.eventQueue.free).toHaveBeenCalledTimes(1);
    expect(appStub.stage.children.length).toBe(0);
  });

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
    const physics = PhysicsWorldMock.lastInstance!;
    const board = (game as any).board as BoardMock;
    const ball = (game as any).balls[0] as BallMock;
    const deepSpace = DeepSpaceClientMock.lastInstance!;

    ball.velocity = { x: 0.7, y: -2.3 };
    ball.snapshot = { id: 1, x: 0, y: 0, vx: 1.2, vy: -2.3 };

    physics.queueCollision(
      board.escapeColliderHandle,
      ball.colliderHandle,
      true,
    );
    (game as any).processCollisions();

    expect(deepSpace.ballEscapedCalls.length).toBe(1);
    expect(deepSpace.ballEscapedCalls[0]).toEqual({ vx: 1.2, vy: -2.3 });
    expect(ball.active).toBe(false);
    expect((game as any).balls.length).toBe(0);
  });

  it("sends velocity fallback on escape when snapshot is unavailable", async () => {
    const game = await createGame();
    const physics = PhysicsWorldMock.lastInstance!;
    const board = (game as any).board as BoardMock;
    const ball = (game as any).balls[0] as BallMock;
    const deepSpace = DeepSpaceClientMock.lastInstance!;

    ball.velocity = { x: 0.4, y: -1.1 };
    ball.snapshot = null;

    physics.queueCollision(
      board.escapeColliderHandle,
      ball.colliderHandle,
      true,
    );
    (game as any).processCollisions();

    expect(deepSpace.ballEscapedCalls.length).toBe(1);
    expect(deepSpace.ballEscapedCalls[0]).toEqual({ vx: 0.4, vy: -1.1 });
    expect(ball.active).toBe(false);
  });

  it("does not escape when sensor collision has downward velocity", async () => {
    const game = await createGame();
    const physics = PhysicsWorldMock.lastInstance!;
    const board = (game as any).board as BoardMock;
    const ball = (game as any).balls[0] as BallMock;
    const deepSpace = DeepSpaceClientMock.lastInstance!;

    ball.velocity = { x: 0.2, y: 0.6 };
    ball.snapshot = { id: 2, x: 0, y: 0, vx: 0.2, vy: 0.6 };

    physics.queueCollision(
      board.escapeColliderHandle,
      ball.colliderHandle,
      true,
    );
    (game as any).processCollisions();

    expect(deepSpace.ballEscapedCalls.length).toBe(0);
    expect(ball.active).toBe(true);
    expect((game as any).balls.length).toBe(1);
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

describe("Ball color", () => {
  it("launcher ball gets player color when welcome arrives, not temporary teal", async () => {
    // The DeepSpaceClient mock already called onPlayersChanged with a
    // temporary local player (color 0x4da6a6 / teal) during construction.
    // Then createEntities() spawned the launcher ball with that teal color.
    const game = await createGame();
    const deepSpace = DeepSpaceClientMock.lastInstance!;
    const launcherBall = (game as any).launcherBall as BallMock;

    // Before welcome: ball has the temporary teal color
    expect(launcherBall.lastTint).toBe(0x4da6a6);

    // Simulate server welcome with real player color (e.g. orange 0xff8800)
    const realPlayerColor = 0xff8800;
    deepSpace.callbacks.onPlayersChanged(
      [
        {
          id: 42,
          cellIndex: 1,
          portalPos: { x: 1, y: 0, z: 0 },
          color: realPlayerColor,
          paused: false,
          ballsProduced: 0,
          ballsInFlight: 0,
        },
      ],
      42,
    );

    // After welcome: launcher ball must have the real player color
    expect(launcherBall.lastTint).toBe(realPlayerColor);
  });

  it("captured ball keeps original owner color when players update", async () => {
    const game = await createGame();
    const deepSpace = DeepSpaceClientMock.lastInstance!;

    // First, welcome with real player color
    deepSpace.callbacks.onPlayersChanged(
      [
        {
          id: 42,
          cellIndex: 1,
          portalPos: { x: 1, y: 0, z: 0 },
          color: 0xff8800,
          paused: false,
          ballsProduced: 0,
          ballsInFlight: 0,
        },
      ],
      42,
    );

    // Simulate a captured ball from another player (different color)
    const capturedBall = new BallMock();
    capturedBall.inLauncher = false;
    (game as any).balls.push(capturedBall);
    capturedBall.setTint(0x00ff00); // green — from another player

    // Players update arrives again (e.g. new player joined)
    deepSpace.callbacks.onPlayersChanged(
      [
        {
          id: 42,
          cellIndex: 1,
          portalPos: { x: 1, y: 0, z: 0 },
          color: 0xff8800,
          paused: false,
          ballsProduced: 0,
          ballsInFlight: 0,
        },
        {
          id: 99,
          cellIndex: 2,
          portalPos: { x: 0, y: 1, z: 0 },
          color: 0x00ff00,
          paused: false,
          ballsProduced: 0,
          ballsInFlight: 0,
        },
      ],
      42,
    );

    // Captured ball must still be green (not overwritten with our orange)
    expect(capturedBall.lastTint).toBe(0x00ff00);
  });
});
