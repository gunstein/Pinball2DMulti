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

const PHYSICS_DT = 1 / 60;

export class Game {
  private app: Application;
  private physics: PhysicsWorld;
  private input: InputManager;

  private deepSpaceLayer: DeepSpaceLayer;
  private boardLayer: BoardLayer;
  private uiLayer: UILayer;

  private board!: Board;
  private ball!: Ball;
  private flipperEntities: Flipper[] = [];
  private launcher!: Launcher;
  private pins: Pin[] = [];

  // Collision lookup
  private pinByHandle: Map<number, Pin> = new Map();

  // Fixed timestep accumulator
  private accumulator = 0;

  // Respawn timer
  private respawnTimer = 0;

  constructor(app: Application) {
    this.app = app;
    this.physics = new PhysicsWorld();
    this.input = new InputManager();

    this.deepSpaceLayer = new DeepSpaceLayer();
    this.boardLayer = new BoardLayer();
    this.uiLayer = new UILayer();

    this.app.stage.addChild(this.deepSpaceLayer.container);
    this.app.stage.addChild(this.boardLayer.container);
    this.app.stage.addChild(this.uiLayer.container);

    this.createEntities();
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

    // Flippers (from BoardGeometry)
    for (const def of flippers) {
      const flipper = new Flipper(container, this.physics, def);
      this.flipperEntities.push(flipper);
    }

    // Launcher (impulse-based, no physics body)
    this.launcher = new Launcher(container);
    this.launcher.onLaunch((power) => {
      this.ball.applyImpulse({ x: 0, y: -power });
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
    // Respawn timer
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

    // Variable-rate updates (rendering, effects)
    this.ball.update();
    for (const pin of this.pins) {
      pin.update(dt);
    }
    this.deepSpaceLayer.update(dt);
  }

  private fixedUpdate(dt: number) {
    // Update flippers (left = index 0, right = index 1)
    this.flipperEntities[0].update(dt, this.input.leftFlipper);
    this.flipperEntities[1].update(dt, this.input.rightFlipper);

    // Update launcher
    this.launcher.update(dt, this.input.launch);

    // Step physics
    this.physics.step();

    // Process collisions
    this.processCollisions();
  }

  private processCollisions() {
    this.physics.eventQueue.drainCollisionEvents(
      (handle1, handle2, started) => {
        if (!started) return;

        // Check drain
        if (
          handle1 === this.board.drainSensorHandle ||
          handle2 === this.board.drainSensorHandle
        ) {
          const otherHandle =
            handle1 === this.board.drainSensorHandle ? handle2 : handle1;
          if (
            otherHandle === this.ball.colliderHandle &&
            this.respawnTimer <= 0
          ) {
            this.respawnTimer = 0.3;
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
