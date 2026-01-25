import { Application } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { InputManager } from "./InputManager";
import { Board } from "../board/Board";
import { Ball } from "../board/Ball";
import { Flipper } from "../board/Flipper";
import { Launcher } from "../board/Launcher";
import { Pin } from "../board/Pin";
import { DeepSpaceLayer } from "../layers/DeepSpaceLayer";
import { BoardLayer } from "../layers/BoardLayer";
import { UILayer } from "../layers/UILayer";
import { bumpers, flippers } from "../board/BoardGeometry";
const PHYSICS_DT = 1 / 120;
const RESPAWN_DELAY = 0.5; // seconds after escape before new ball spawns

export class Game {
  private app: Application;
  private physics: PhysicsWorld;
  private input: InputManager;

  private deepSpaceLayer: DeepSpaceLayer;
  private boardLayer: BoardLayer;
  private uiLayer: UILayer;

  private board!: Board;
  private balls: Ball[] = [];
  private launcherBall: Ball | null = null; // ball currently in launcher
  private leftFlipper!: Flipper;
  private rightFlipper!: Flipper;
  private launcher!: Launcher;
  private pins: Pin[] = [];

  // Collision lookup
  private pinByHandle: Map<number, Pin> = new Map();
  private ballByHandle: Map<number, Ball> = new Map();

  // Fixed timestep accumulator
  private accumulator = 0;

  // Respawn timer (for spawning new ball after escape)
  private respawnTimer = 0;

  constructor(app: Application) {
    this.app = app;
    this.physics = new PhysicsWorld();
    this.input = new InputManager();

    this.deepSpaceLayer = new DeepSpaceLayer();
    this.boardLayer = new BoardLayer();
    this.uiLayer = new UILayer();

    // Deep-space is screen-space (fills whole window)
    this.app.stage.addChild(this.deepSpaceLayer.container);
    // Board + UI are world-space (scaled together)
    this.app.stage.addChild(this.boardLayer.container);
    this.app.stage.addChild(this.uiLayer.container);

    this.createEntities();
  }

  resize(
    scale: number,
    offsetX: number,
    offsetY: number,
    screenW: number,
    screenH: number,
  ) {
    // Scale and position the world layers (board + UI)
    this.boardLayer.container.scale.set(scale);
    this.boardLayer.container.position.set(offsetX, offsetY);
    this.uiLayer.container.scale.set(scale);
    this.uiLayer.container.position.set(offsetX, offsetY);

    // Deep-space fills the screen
    this.deepSpaceLayer.resize(screenW, screenH);
    // Pass world transform so deep-space balls align with the board
    this.deepSpaceLayer.setWorldTransform(scale, offsetX, offsetY);
  }

  private createEntities() {
    const container = this.boardLayer.container;

    // Board walls (from BoardGeometry)
    this.board = new Board(container, this.physics);

    // Pins/bumpers (from BoardGeometry)
    for (const def of bumpers) {
      const pin = new Pin(container, this.physics, def);
      this.pins.push(pin);
      this.pinByHandle.set(pin.colliderHandle, pin);
    }

    // Flippers (from BoardGeometry, keyed by side)
    for (const def of flippers) {
      const flipper = new Flipper(container, this.physics, def);
      if (def.side === "left") this.leftFlipper = flipper;
      else this.rightFlipper = flipper;
    }
    if (!this.leftFlipper || !this.rightFlipper) {
      throw new Error("BoardGeometry must define both left and right flippers");
    }

    // Launcher (velocity-based, no physics body)
    this.launcher = new Launcher(container);
    this.launcher.onLaunch((speed) => {
      if (this.launcherBall) {
        this.launcherBall.launch(speed);
        this.launcherBall = null; // ball is no longer in launcher
      }
    });

    // Create initial ball in launcher
    this.spawnBallInLauncher();
  }

  /** Create a new ball and place it in the launcher */
  private spawnBallInLauncher() {
    const ball = new Ball(this.boardLayer.container, this.physics);
    this.balls.push(ball);
    this.ballByHandle.set(ball.colliderHandle, ball);
    this.launcherBall = ball;
  }

  /** Create a new ball from a captured deep-space ball */
  private spawnBallFromCapture(x: number, y: number, vx: number, vy: number) {
    const ball = new Ball(this.boardLayer.container, this.physics);
    ball.injectFromCapture(x, y, vx, vy);
    this.balls.push(ball);
    this.ballByHandle.set(ball.colliderHandle, ball);
  }

  start() {
    this.app.ticker.add((ticker) => {
      const dt = Math.min(ticker.deltaMS / 1000, 0.1); // cap at 100ms
      this.update(dt);
    });
  }

  private update(dt: number) {
    // Respawn timer (spawn new ball in launcher only if no balls on board at all)
    if (this.respawnTimer > 0) {
      this.respawnTimer -= dt;
      if (
        this.respawnTimer <= 0 &&
        !this.launcherBall &&
        this.balls.length === 0
      ) {
        this.spawnBallInLauncher();
      }
    }

    // Accumulate time for fixed physics steps
    this.accumulator += dt;

    while (this.accumulator >= PHYSICS_DT) {
      this.fixedUpdate(PHYSICS_DT);
      this.accumulator -= PHYSICS_DT;
    }

    // Render all entities once per frame
    for (const ball of this.balls) {
      ball.render();
    }
    this.leftFlipper.render();
    this.rightFlipper.render();
    this.launcher.render();
    for (const pin of this.pins) {
      pin.render();
    }
    this.deepSpaceLayer.update(dt);
  }

  private fixedUpdate(dt: number) {
    // Update flippers
    this.leftFlipper.fixedUpdate(dt, this.input.leftFlipper);
    this.rightFlipper.fixedUpdate(dt, this.input.rightFlipper);

    // Update launcher
    this.launcher.fixedUpdate(dt, this.input.launch);

    // Update all balls
    for (const ball of this.balls) {
      ball.fixedUpdate();

      // Check if ball has returned to launcher
      if (!this.launcherBall && ball.isInLauncher()) {
        this.launcherBall = ball;
      }
    }

    // Update pins
    for (const pin of this.pins) {
      pin.fixedUpdate(dt);
    }

    // Step physics
    this.physics.step(dt);

    // Process collisions
    this.processCollisions();

    // Check for ball escape (all active balls)
    this.checkEscape();

    // Check for atmosphere capture
    this.checkCapture();
  }

  private checkEscape() {
    // Use a copy of the array since we may remove balls during iteration
    for (const ball of [...this.balls]) {
      if (!ball.isActive()) continue;

      const snapshot = ball.getEscapeSnapshot();
      if (snapshot) {
        this.deepSpaceLayer.addBall(snapshot);
        this.removeBall(ball);
      }
    }

    // If no balls left on board and no launcher ball, start respawn timer
    if (
      this.balls.length === 0 &&
      !this.launcherBall &&
      this.respawnTimer <= 0
    ) {
      this.respawnTimer = RESPAWN_DELAY;
    }
  }

  /** Remove a ball from the game */
  private removeBall(ball: Ball) {
    ball.setInactive();
    this.ballByHandle.delete(ball.colliderHandle);
    const idx = this.balls.indexOf(ball);
    if (idx !== -1) {
      this.balls.splice(idx, 1);
    }
    // If this was the launcher ball, clear reference
    if (this.launcherBall === ball) {
      this.launcherBall = null;
    }
  }

  private checkCapture() {
    // Check for deep-space balls entering through the port
    const entering = this.deepSpaceLayer.getBallsEnteringPort();

    for (const snap of entering) {
      this.deepSpaceLayer.removeBall(snap.id);

      // Spawn a new ball from the captured deep-space ball
      this.spawnBallFromCapture(snap.x, snap.y, snap.vx, snap.vy);
    }
  }

  private processCollisions() {
    this.physics.eventQueue.drainCollisionEvents(
      (handle1, handle2, started) => {
        if (!started) return;

        // Check drain (ball hitting bottom wall) — remove ball
        if (
          handle1 === this.board.drainColliderHandle ||
          handle2 === this.board.drainColliderHandle
        ) {
          const otherHandle =
            handle1 === this.board.drainColliderHandle ? handle2 : handle1;
          const ball = this.ballByHandle.get(otherHandle);
          if (ball && ball.isActive()) {
            // Drain removes the ball
            this.removeBall(ball);
            // If no balls left and no launcher ball, start respawn timer
            if (
              this.balls.length === 0 &&
              !this.launcherBall &&
              this.respawnTimer <= 0
            ) {
              this.respawnTimer = RESPAWN_DELAY;
            }
          }
          return;
        }

        // Check pin hits (O(1) lookup)
        const pin1 = this.pinByHandle.get(handle1);
        const pin2 = this.pinByHandle.get(handle2);
        const hitPin = pin1 || pin2;
        if (hitPin) {
          const otherHandle = pin1 ? handle2 : handle1;
          const ball = this.ballByHandle.get(otherHandle);
          if (ball) {
            hitPin.hit();
            this.uiLayer.addHit();
          }
        }
      },
    );
  }
}
