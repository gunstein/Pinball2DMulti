import { Application } from "pixi.js";
import { PhysicsWorld } from "../physics/PhysicsWorld";
import { InputManager } from "./InputManager";
import { Board } from "../board/Board";
import { Ball } from "../board/Ball";
import { Flipper } from "../board/Flipper";
import { Launcher } from "../board/Launcher";
import { Pin } from "../board/Pin";
import { SphereDeepSpaceLayer } from "../layers/SphereDeepSpaceLayer";
import { BoardLayer } from "../layers/BoardLayer";
import { UILayer } from "../layers/UILayer";
import { bumpers, flippers } from "../board/BoardGeometry";
import { MockWorld } from "../shared/MockWorld";
import { SphereDeepSpace, CaptureEvent } from "../shared/SphereDeepSpace";
import { ServerConnection } from "../shared/ServerConnection";
import { Player, SpaceBall3D } from "../shared/types";
import { PPM } from "../constants";

const PHYSICS_DT = 1 / 120;
const MAX_PHYSICS_STEPS = 8;
const RESPAWN_DELAY = 0.5;
const MOCK_PLAYER_COUNT = 50;

// Speed for balls entering from deep space (m/s)
const CAPTURE_SPEED = 1.5;

// Set to true to use server, false for offline mock mode
const USE_SERVER = true;
const SERVER_URL = "ws://localhost:9001/ws";

export class Game {
  private app: Application;
  private physics: PhysicsWorld;
  private input: InputManager;

  private deepSpaceLayer: SphereDeepSpaceLayer;
  private boardLayer: BoardLayer;
  private uiLayer: UILayer;

  // Server connection (multiplayer mode)
  private serverConnection: ServerConnection | null = null;

  // Mock world (offline mode)
  private mockWorld: MockWorld | null = null;
  private sphereDeepSpace: SphereDeepSpace | null = null;

  // Current player state
  private selfPlayer: Player | null = null;
  private allPlayers: Player[] = [];
  private ballColor: number = 0xffffff; // Default white, set from selfPlayer.color

  private board!: Board;
  private balls: Ball[] = [];
  private launcherBall: Ball | null = null;
  private leftFlipper!: Flipper;
  private rightFlipper!: Flipper;
  private launcher!: Launcher;
  private pins: Pin[] = [];

  private pinByHandle: Map<number, Pin> = new Map();
  private ballByHandle: Map<number, Ball> = new Map();
  private inactiveBalls: Ball[] = [];

  private accumulator = 0;
  private respawnTimer = 0;

  constructor(app: Application) {
    this.app = app;
    this.physics = new PhysicsWorld();
    this.input = new InputManager();

    // Create layers
    this.deepSpaceLayer = new SphereDeepSpaceLayer();
    this.boardLayer = new BoardLayer();
    this.uiLayer = new UILayer();

    // Add layers to stage
    this.app.stage.addChild(this.deepSpaceLayer.container);
    this.app.stage.addChild(this.boardLayer.container);
    this.app.stage.addChild(this.uiLayer.container);

    if (USE_SERVER) {
      this.initServerMode();
    } else {
      this.initMockMode();
    }

    this.createEntities();
  }

  private initServerMode() {
    this.serverConnection = new ServerConnection(SERVER_URL);

    this.serverConnection.onWelcome = (selfId, players, config) => {
      this.allPlayers = players;
      this.selfPlayer = players.find((p) => p.id === selfId) || null;
      if (this.selfPlayer) {
        this.deepSpaceLayer.setSelfPortal(this.selfPlayer.portalPos);
        this.ballColor = this.selfPlayer.color;
        // Apply color to existing balls
        for (const ball of this.balls) {
          ball.setTint(this.ballColor);
        }
      }
      this.deepSpaceLayer.markColorsDirty();
      console.log(`Joined as player ${selfId} with ${players.length} players`);
    };

    this.serverConnection.onPlayersState = (players) => {
      this.allPlayers = players;
      if (this.selfPlayer) {
        const updated = players.find((p) => p.id === this.selfPlayer!.id);
        if (updated) this.selfPlayer = updated;
      }
      this.deepSpaceLayer.markColorsDirty();
    };

    this.serverConnection.onTransferIn = (vx, vy) => {
      this.spawnBallFromCapture(vx, vy);
    };
  }

  private initMockMode() {
    this.mockWorld = new MockWorld(MOCK_PLAYER_COUNT);
    this.sphereDeepSpace = new SphereDeepSpace(this.mockWorld.config);
    this.sphereDeepSpace.setPlayers(this.mockWorld.getAllPlayers());

    this.selfPlayer = this.mockWorld.getSelfPlayer();
    this.allPlayers = this.mockWorld.getAllPlayers();
    this.deepSpaceLayer.setSelfPortal(this.selfPlayer.portalPos);
  }

  resize(
    scale: number,
    offsetX: number,
    offsetY: number,
    screenW: number,
    screenH: number,
  ) {
    // Scale and position board + UI
    this.boardLayer.container.scale.set(scale);
    this.boardLayer.container.position.set(offsetX, offsetY);
    this.uiLayer.container.scale.set(scale);
    this.uiLayer.container.position.set(offsetX, offsetY);

    // Deep space fills screen
    this.deepSpaceLayer.resize(screenW, screenH);

    // Set center for deep space projection (aligned with board center)
    const boardCenterX = offsetX + 200 * scale;
    const boardCenterY = offsetY + 350 * scale;
    this.deepSpaceLayer.setCenter(boardCenterX, boardCenterY);
  }

  private createEntities() {
    const container = this.boardLayer.container;

    this.board = new Board(container, this.physics);

    for (const def of bumpers) {
      const pin = new Pin(container, this.physics, def);
      this.pins.push(pin);
      this.pinByHandle.set(pin.colliderHandle, pin);
    }

    for (const def of flippers) {
      const flipper = new Flipper(container, this.physics, def);
      if (def.side === "left") this.leftFlipper = flipper;
      else this.rightFlipper = flipper;
    }
    if (!this.leftFlipper || !this.rightFlipper) {
      throw new Error("BoardGeometry must define both left and right flippers");
    }

    this.launcher = new Launcher(container);
    this.launcher.onLaunch((speed) => {
      // Launch all balls in the launcher zone, scaling speed quadratically by count
      // to overcome friction and collision forces between stacked balls
      const launcherBalls = this.balls.filter((b) => b.isInShooterLane());
      const count = launcherBalls.length;
      if (count === 0) return;
      const scaledSpeed = speed * count * count;
      for (const b of launcherBalls) {
        b.launch(scaledSpeed);
      }
      this.launcherBall = null;
    });

    this.spawnBallInLauncher();
  }

  private acquireBall(): Ball {
    let ball: Ball;
    if (this.inactiveBalls.length > 0) {
      ball = this.inactiveBalls.pop()!;
    } else {
      ball = new Ball(this.boardLayer.container, this.physics);
    }
    ball.setTint(this.ballColor);
    return ball;
  }

  private spawnBallInLauncher() {
    const ball = this.acquireBall();
    ball.respawn();
    this.balls.push(ball);
    this.ballByHandle.set(ball.colliderHandle, ball);
    this.launcherBall = ball;
  }

  private spawnBallFromCapture(vx: number, vy: number) {
    const ball = this.acquireBall();
    // Spawn below the escape slot so the ball doesn't immediately re-escape
    const x = this.physics.toPhysicsX(200); // center
    const y = this.physics.toPhysicsY(80); // below escape slot (yBottom=50)
    // Server guarantees vy is positive (downward into the board)
    ball.injectFromCapture(x, y, vx, vy);
    this.balls.push(ball);
    this.ballByHandle.set(ball.colliderHandle, ball);
  }

  start() {
    this.app.ticker.add((ticker) => {
      const dt = Math.min(ticker.deltaMS / 1000, 0.1);
      this.update(dt);
    });
  }

  private update(dt: number) {
    // Respawn timer
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

    // Physics steps
    this.accumulator += dt;
    let steps = 0;
    while (this.accumulator >= PHYSICS_DT && steps < MAX_PHYSICS_STEPS) {
      this.fixedUpdate(PHYSICS_DT);
      this.accumulator -= PHYSICS_DT;
      steps++;
    }
    if (steps >= MAX_PHYSICS_STEPS) {
      this.accumulator = 0;
    }

    // Update deep space (mock mode only - server mode handles via callbacks)
    if (this.sphereDeepSpace) {
      const captures = this.sphereDeepSpace.tick(dt);
      this.handleCaptures(captures);
    }

    // Render
    for (const ball of this.balls) {
      ball.render();
    }
    this.leftFlipper.render();
    this.rightFlipper.render();
    this.launcher.render();
    for (const pin of this.pins) {
      pin.render();
    }

    // Render deep space
    const spaceBalls = this.getSpaceBalls();
    const selfId = this.selfPlayer?.id ?? 0;
    this.deepSpaceLayer.update(dt, spaceBalls, this.allPlayers, selfId);
  }

  private getSpaceBalls(): Iterable<SpaceBall3D> {
    if (this.serverConnection) {
      return this.serverConnection.getBallIterable();
    }
    if (this.sphereDeepSpace) {
      return this.sphereDeepSpace.getBallIterable();
    }
    return [];
  }

  private handleCaptures(captures: CaptureEvent[]) {
    if (!this.selfPlayer) return;
    for (const capture of captures) {
      if (capture.playerId === this.selfPlayer.id) {
        // Ball captured by us - spawn on our board
        const [vx, vy] = this.sphereDeepSpace!.getCaptureVelocity2D(
          capture.ball,
          capture.player.portalPos,
          CAPTURE_SPEED,
        );
        this.spawnBallFromCapture(vx, vy);
      }
      // For other players, the ball just disappears (they would handle it on their client)
    }
  }

  private fixedUpdate(dt: number) {
    this.leftFlipper.fixedUpdate(dt, this.input.leftFlipper);
    this.rightFlipper.fixedUpdate(dt, this.input.rightFlipper);
    this.launcher.fixedUpdate(dt, this.input.launch);

    for (const ball of this.balls) {
      ball.fixedUpdate();
      if (!this.launcherBall && ball.isInLauncher()) {
        this.launcherBall = ball;
      }
    }

    for (const pin of this.pins) {
      pin.fixedUpdate(dt);
    }

    this.physics.step(dt);
    this.processCollisions();
    this.checkEscape();
  }

  private checkEscape() {
    // Iterate backwards to safely remove during iteration
    for (let i = this.balls.length - 1; i >= 0; i--) {
      const ball = this.balls[i];
      if (!ball.isActive()) continue;

      const snapshot = ball.getEscapeSnapshot();
      if (snapshot) {
        // Send to server or local simulation
        if (this.serverConnection) {
          this.serverConnection.sendBallEscaped(snapshot.vx, snapshot.vy);
        } else if (this.sphereDeepSpace && this.selfPlayer) {
          this.sphereDeepSpace.addBall(
            this.selfPlayer.id,
            this.selfPlayer.portalPos,
            snapshot.vx,
            snapshot.vy,
          );
        }
        this.removeBall(ball);
      }
    }

    if (
      this.balls.length === 0 &&
      !this.launcherBall &&
      this.respawnTimer <= 0
    ) {
      this.respawnTimer = RESPAWN_DELAY;
    }
  }

  private removeBall(ball: Ball) {
    ball.setInactive();
    this.ballByHandle.delete(ball.colliderHandle);
    const idx = this.balls.indexOf(ball);
    if (idx !== -1) {
      this.balls.splice(idx, 1);
    }
    if (this.launcherBall === ball) {
      this.launcherBall = null;
    }
    this.inactiveBalls.push(ball);
  }

  private processCollisions() {
    this.physics.eventQueue.drainCollisionEvents(
      (handle1, handle2, started) => {
        if (!started) return;

        if (
          handle1 === this.board.drainColliderHandle ||
          handle2 === this.board.drainColliderHandle
        ) {
          const otherHandle =
            handle1 === this.board.drainColliderHandle ? handle2 : handle1;
          const ball = this.ballByHandle.get(otherHandle);
          if (ball && ball.isActive()) {
            this.removeBall(ball);
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
