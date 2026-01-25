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
import { bumpers, flippers, atmosphereBounds } from "../board/BoardGeometry";
import { BallSnapshot } from "../shared/types";

const PHYSICS_DT = 1 / 120;
const RESPAWN_DELAY = 0.5; // seconds after escape before new ball spawns

// Module-level counter for generating unique ball IDs (from escapeBall)
let nextBallId = 1000; // Start higher to avoid collision with Ball.ts counter

export class Game {
  private app: Application;
  private physics: PhysicsWorld;
  private input: InputManager;

  private deepSpaceLayer: DeepSpaceLayer;
  private boardLayer: BoardLayer;
  private uiLayer: UILayer;

  private board!: Board;
  private ball!: Ball;
  private leftFlipper!: Flipper;
  private rightFlipper!: Flipper;
  private launcher!: Launcher;
  private pins: Pin[] = [];

  // Collision lookup
  private pinByHandle: Map<number, Pin> = new Map();

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
      this.ball.launch(speed);
    });

    // Ball
    this.ball = new Ball(container, this.physics);
  }

  start() {
    this.app.ticker.add((ticker) => {
      const dt = Math.min(ticker.deltaMS / 1000, 0.1); // cap at 100ms
      this.update(dt);
    });
  }

  private update(dt: number) {
    // Respawn timer (spawn new ball after escape)
    if (this.respawnTimer > 0) {
      this.respawnTimer -= dt;
      if (this.respawnTimer <= 0) {
        this.ball.respawn();
      }
    }

    // Accumulate time for fixed physics steps
    this.accumulator += dt;

    while (this.accumulator >= PHYSICS_DT) {
      this.fixedUpdate(PHYSICS_DT);
      this.accumulator -= PHYSICS_DT;
    }

    // Render all entities once per frame
    this.ball.render();
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

    // Update ball state
    this.ball.fixedUpdate();

    // Update pins
    for (const pin of this.pins) {
      pin.fixedUpdate(dt);
    }

    // Step physics
    this.physics.step(dt);

    // Process collisions
    this.processCollisions();

    // Check for ball escape
    this.checkEscape();

    // Check for atmosphere capture
    this.checkCapture();
  }

  private checkEscape() {
    // Backup: if ball somehow leaves escape bounds without hitting drain
    const snapshot = this.ball.getEscapeSnapshot();
    if (snapshot) {
      this.deepSpaceLayer.addBall(snapshot);
      this.ball.setInactive();
      this.respawnTimer = RESPAWN_DELAY;
    }
  }

  private escapeBall() {
    const pos = this.ball.getPosition();
    const vel = this.ball.getVelocity();
    const snapshot: BallSnapshot = {
      id: nextBallId++,
      x: pos.x,
      y: pos.y,
      vx: vel.x,
      vy: vel.y,
    };
    this.deepSpaceLayer.addBall(snapshot);
    this.ball.setInactive();
    this.respawnTimer = RESPAWN_DELAY;
  }

  private checkCapture() {
    // Only capture if the local ball is active (don't stack multiple balls)
    if (!this.ball.isActive()) return;

    const captured = this.deepSpaceLayer.getBallsInArea(atmosphereBounds);
    if (captured.length > 0) {
      // Capture the first ball that enters atmosphere
      const snap = captured[0];
      this.deepSpaceLayer.removeBall(snap.id);
      // For now, captured balls just disappear (they re-enter as a visual effect)
      // Later: spawn as additional local Rapier ball
    }
  }

  private processCollisions() {
    this.physics.eventQueue.drainCollisionEvents(
      (handle1, handle2, started) => {
        if (!started) return;

        // Check drain (ball hitting bottom wall) — trigger escape
        if (
          handle1 === this.board.drainColliderHandle ||
          handle2 === this.board.drainColliderHandle
        ) {
          const otherHandle =
            handle1 === this.board.drainColliderHandle ? handle2 : handle1;
          if (
            otherHandle === this.ball.colliderHandle &&
            this.ball.isActive() &&
            this.respawnTimer <= 0
          ) {
            this.escapeBall();
          }
          return;
        }

        // Check pin hits (O(1) lookup)
        const pin1 = this.pinByHandle.get(handle1);
        const pin2 = this.pinByHandle.get(handle2);
        const hitPin = pin1 || pin2;
        if (hitPin) {
          const otherHandle = pin1 ? handle2 : handle1;
          if (otherHandle === this.ball.colliderHandle) {
            hitPin.hit();
            this.uiLayer.addHit();
          }
        }
      },
    );
  }
}
