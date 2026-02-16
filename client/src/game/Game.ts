/**
 * Core game controller.
 *
 * Owns the three rendering layers, the Rapier2D physics world, and the
 * DeepSpaceClient (which abstracts over live server / offline mock mode).
 *
 * Each frame:
 *   1. Run fixed-timestep physics (flippers, launcher, ball movement)
 *   2. Check for ball escapes (top of board → deep space)
 *   3. Process collisions (drain, bumpers)
 *   4. Tick the deep-space client (ball movement on sphere, captures)
 *   5. Render all layers
 *
 * Balls are pooled: inactive balls go into `inactiveBalls` for reuse,
 * avoiding repeated construction of PixiJS Graphics + Rapier colliders.
 */
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
import { bumpers, flippers, PLAYFIELD_CENTER_X } from "../board/BoardGeometry";
import { DeepSpaceClient } from "../shared/DeepSpaceClient";
import { Player } from "../shared/types";
import { ClientBot } from "./ClientBot";
import type { BallInfo } from "./ClientBot";
import { buildServerUrl, launcherStackScale } from "./gameConfig";

const PHYSICS_DT = 1 / 120;
const MAX_PHYSICS_STEPS = 8;
const RESPAWN_DELAY = 0.5;
const MOCK_PLAYER_COUNT = 50;
const ACTIVITY_SEND_INTERVAL = 5; // seconds between activity heartbeats
const ACTIVITY_TIMEOUT = 30; // seconds of inactivity before stopping heartbeats
const MAX_POOLED_BALLS = 10;

/** Y coordinate of the visual center of the playfield (pixels) */
const BOARD_CENTER_Y = 350;

/** Where captured balls spawn when entering the board from deep space (pixels) */
const CAPTURE_SPAWN_X = 200;
const CAPTURE_SPAWN_Y = 80;

// Set to true to use server, false for offline mock mode
const USE_SERVER = true;

// WebSocket URL: env override first, otherwise derive from current host.
const SERVER_URL = buildServerUrl(location, import.meta.env.VITE_SERVER_URL);

export class Game {
  private app: Application;
  private physics: PhysicsWorld;
  private input: InputManager;

  private deepSpaceLayer: SphereDeepSpaceLayer;
  private boardLayer: BoardLayer;
  private uiLayer: UILayer;

  // Deep-space client (handles server/local mode)
  private deepSpaceClient!: DeepSpaceClient;

  // Public callback for protocol mismatch (wired up by main.ts)
  onProtocolMismatch:
    | ((serverVersion: number, clientVersion: number) => void)
    | null = null;

  // Ball color (from self player)
  private ballColor: number = 0xffffff;

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

  private tickerCallback: ((ticker: { deltaMS: number }) => void) | null = null;
  private destroyed = false;
  private accumulator = 0;
  private respawnTimer = 0;

  // Client-side bot (screensaver mode)
  private clientBot = new ClientBot();
  private botBallInfos: BallInfo[] = [];
  private botEnabled = false;

  // Activity heartbeat state
  private lastActivitySent = 0; // last input time we sent a heartbeat for
  private lastActivitySendTime = 0; // performance.now() when we last sent

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

    // Create deep-space client
    this.deepSpaceClient = new DeepSpaceClient(
      USE_SERVER,
      SERVER_URL,
      MOCK_PLAYER_COUNT,
      {
        onPlayersChanged: (players, selfId) =>
          this.handlePlayersChanged(players, selfId),
        onConnectionStateChanged: (state) =>
          this.uiLayer.setConnectionState(state),
        onCapture: (vx, vy, color) => this.spawnBallFromCapture(vx, vy, color),
        onProtocolMismatch: (serverVer, clientVer) =>
          this.onProtocolMismatch?.(serverVer, clientVer),
      },
    );

    this.createEntities();
  }

  private handlePlayersChanged(players: Player[], selfId: number) {
    const selfPlayer = players.find((p) => p.id === selfId);
    if (selfPlayer) {
      this.deepSpaceLayer.setSelfPortal(selfPlayer.portalPos);
      this.ballColor = selfPlayer.color;
      // Recolor the launcher ball (it's ours), but not captured balls
      // (those keep the original owner's color).
      if (this.launcherBall) {
        this.launcherBall.setTint(this.ballColor);
      }
    }
    this.deepSpaceLayer.markColorsDirty();
    this.uiLayer.setPlayers(players, selfId);
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

    // Set center for deep space projection (aligned with playfield center, excluding launcher)
    const boardCenterX = offsetX + PLAYFIELD_CENTER_X * scale;
    const boardCenterY = offsetY + BOARD_CENTER_Y * scale;
    this.deepSpaceLayer.setCenter(boardCenterX, boardCenterY);

    // Update input manager transform for touch zones
    this.input.setTransform(scale, offsetX, offsetY);
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
      let count = 0;
      for (const b of this.balls) {
        if (b.isInShooterLane()) {
          count++;
        }
      }
      if (count === 0) return;
      const scaledSpeed = speed * launcherStackScale(count);
      for (const b of this.balls) {
        if (b.isInShooterLane()) {
          b.launch(scaledSpeed);
        }
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

  private spawnBallFromCapture(vx: number, vy: number, color: number) {
    const ball = this.acquireBall();
    ball.setTint(color);
    const x = this.physics.toPhysicsX(CAPTURE_SPAWN_X);
    const y = this.physics.toPhysicsY(CAPTURE_SPAWN_Y);
    ball.injectFromCapture(x, y, vx, vy);
    this.balls.push(ball);
    this.ballByHandle.set(ball.colliderHandle, ball);
  }

  /**
   * Start the frame loop.
   * Idempotent: subsequent calls are ignored.
   * Terminal: calling start after destroy is a no-op.
   */
  start() {
    if (this.destroyed || this.tickerCallback) {
      return;
    }
    this.tickerCallback = (ticker: { deltaMS: number }) => {
      const dt = Math.min(ticker.deltaMS / 1000, 0.1);
      this.update(dt);
    };
    this.app.ticker.add(this.tickerCallback);
  }

  /**
   * Tear down all runtime resources owned by Game.
   * Idempotent: safe to call multiple times.
   */
  destroy() {
    if (this.destroyed) {
      return;
    }
    this.destroyed = true;

    if (this.tickerCallback) {
      this.app.ticker.remove(this.tickerCallback);
      this.tickerCallback = null;
    }

    this.deepSpaceClient.dispose();
    this.input.destroy();

    const uniqueBalls = new Set<Ball>([...this.balls, ...this.inactiveBalls]);
    for (const ball of uniqueBalls) {
      ball.destroy(this.boardLayer.container);
    }

    // Detach layer roots from stage before dropping references.
    this.app.stage.removeChild(this.deepSpaceLayer.container);
    this.app.stage.removeChild(this.boardLayer.container);
    this.app.stage.removeChild(this.uiLayer.container);

    this.balls = [];
    this.inactiveBalls = [];
    this.launcherBall = null;
    this.pinByHandle.clear();
    this.ballByHandle.clear();
    this.pins = [];
    this.botEnabled = false;
    this.clientBot.reset();
    this.respawnTimer = 0;
    this.accumulator = 0;
    this.lastActivitySent = 0;
    this.lastActivitySendTime = 0;

    // Destroy layer trees so Pixi resources can be released promptly.
    this.deepSpaceLayer.container.destroy({ children: true });
    this.boardLayer.container.destroy({ children: true });
    this.uiLayer.container.destroy({ children: true });

    this.physics.world.free();
    this.physics.eventQueue.free();
  }

  getServerVersion(): string {
    return this.deepSpaceClient.getServerVersion();
  }

  setBotEnabled(on: boolean): void {
    this.botEnabled = on;
    if (!on) {
      this.clientBot.reset();
    }
  }

  isBotEnabled(): boolean {
    return this.botEnabled;
  }

  private update(dt: number) {
    this.reconcileBallState();

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
    } else if (this.balls.length === 0 && !this.launcherBall) {
      // Fallback: if we somehow end up with no balls and no timer, start one
      this.respawnTimer = RESPAWN_DELAY;
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
    const renderAlpha = this.accumulator / PHYSICS_DT;

    // Send activity heartbeat if player has been active recently
    this.sendActivityHeartbeat();

    // Update deep space (handles local simulation when needed)
    this.deepSpaceClient.tick(dt);

    // Render
    for (const ball of this.balls) {
      ball.render(renderAlpha);
    }
    this.leftFlipper.render(renderAlpha);
    this.rightFlipper.render(renderAlpha);
    this.launcher.render();
    for (const pin of this.pins) {
      pin.render();
    }

    // Render deep space
    this.deepSpaceLayer.update(
      dt,
      this.deepSpaceClient.getBalls(),
      this.deepSpaceClient.getAllPlayers(),
      this.deepSpaceClient.getSelfPlayer()?.id ?? 0,
    );
  }

  /**
   * Keep runtime ball ownership coherent even if unexpected event ordering
   * leaves stale references behind.
   */
  private reconcileBallState() {
    if (this.launcherBall) {
      const handle = this.launcherBall.colliderHandle;
      if (!this.launcherBall.isActive() || !this.ballByHandle.has(handle)) {
        this.launcherBall = null;
      }
    }

    for (let i = this.balls.length - 1; i >= 0; i--) {
      const ball = this.balls[i];
      if (ball.isActive()) continue;

      this.ballByHandle.delete(ball.colliderHandle);
      const last = this.balls.length - 1;
      if (i !== last) {
        this.balls[i] = this.balls[last];
      }
      this.balls.pop();

      if (this.inactiveBalls.length < MAX_POOLED_BALLS) {
        this.inactiveBalls.push(ball);
      }
    }
  }

  private sendActivityHeartbeat() {
    const inputTime = this.input.lastActivityTime;
    const now = performance.now();

    // Bot counts as active (keeps server-side bots alive)
    if (this.botEnabled) {
      if (now - this.lastActivitySendTime < ACTIVITY_SEND_INTERVAL * 1000)
        return;
      this.lastActivitySendTime = now;
      this.deepSpaceClient.sendActivity();
      return;
    }

    if (inputTime === 0) return; // no input yet

    // Don't send if input is too old (player inactive)
    if (now - inputTime > ACTIVITY_TIMEOUT * 1000) return;
    // Don't send more often than the interval
    if (now - this.lastActivitySendTime < ACTIVITY_SEND_INTERVAL * 1000) return;
    // Don't re-send for the same input event
    if (inputTime === this.lastActivitySent) return;

    this.lastActivitySent = inputTime;
    this.lastActivitySendTime = now;
    this.deepSpaceClient.sendActivity();
  }

  private fixedUpdate(dt: number) {
    let left = this.input.leftFlipper;
    let right = this.input.rightFlipper;
    let launch = this.input.launch;

    if (this.botEnabled) {
      let infoCount = 0;
      for (const b of this.balls) {
        if (!b.isActive()) continue;

        const pos = b.getPosition();
        const vel = b.getVelocity();
        let info = this.botBallInfos[infoCount];
        if (!info) {
          info = {
            x: 0,
            y: 0,
            vx: 0,
            vy: 0,
            inLauncher: false,
            inShooterLane: false,
          };
          this.botBallInfos[infoCount] = info;
        }

        info.x = pos.x;
        info.y = pos.y;
        info.vx = vel.x;
        info.vy = vel.y;
        info.inLauncher = b.isInLauncher();
        info.inShooterLane = b.isInShooterLane();
        infoCount++;
      }
      this.botBallInfos.length = infoCount;
      const cmd = this.clientBot.update(dt, this.botBallInfos);
      left = cmd.leftFlipper;
      right = cmd.rightFlipper;
      launch = cmd.launch;
    }

    this.leftFlipper.fixedUpdate(dt, left);
    this.rightFlipper.fixedUpdate(dt, right);
    this.launcher.fixedUpdate(dt, launch);

    for (const ball of this.balls) {
      ball.fixedUpdate();
      if (!this.launcherBall && ball.isInLauncher()) {
        this.launcherBall = ball;
      }
    }

    for (const pin of this.pins) {
      pin.fixedUpdate(dt);
    }

    for (const ball of this.balls) {
      ball.capturePreStepPosition();
    }
    this.physics.step(dt);
    for (const ball of this.balls) {
      ball.capturePostStepPosition();
    }
    this.processCollisions();
  }

  private removeBall(ball: Ball) {
    ball.setInactive();
    this.ballByHandle.delete(ball.colliderHandle);
    // O(1) swap-and-pop removal (ball order doesn't matter)
    const idx = this.balls.indexOf(ball);
    if (idx !== -1) {
      const last = this.balls.length - 1;
      if (idx !== last) {
        this.balls[idx] = this.balls[last];
      }
      this.balls.pop();
    }
    if (this.launcherBall === ball) {
      this.launcherBall = null;
    }
    if (this.inactiveBalls.length < MAX_POOLED_BALLS) {
      this.inactiveBalls.push(ball);
    }
  }

  private processCollisions() {
    this.physics.eventQueue.drainCollisionEvents(
      (handle1, handle2, started) => {
        if (!started) return;

        // Check escape-slot sensor collision
        if (
          handle1 === this.board.escapeColliderHandle ||
          handle2 === this.board.escapeColliderHandle
        ) {
          const otherHandle =
            handle1 === this.board.escapeColliderHandle ? handle2 : handle1;
          const ball = this.ballByHandle.get(otherHandle);
          if (ball && ball.isActive()) {
            const vel = ball.getVelocity();
            // Escape sensor defines the valid area; only require upward travel.
            if (vel.y < 0) {
              const snapshot = ball.getEscapeSnapshot();
              if (snapshot) {
                this.deepSpaceClient.ballEscaped(snapshot.vx, snapshot.vy);
              } else {
                this.deepSpaceClient.ballEscaped(vel.x, vel.y);
              }
              this.removeBall(ball);
              if (
                this.balls.length === 0 &&
                !this.launcherBall &&
                this.respawnTimer <= 0
              ) {
                this.respawnTimer = RESPAWN_DELAY;
              }
            }
          }
          return;
        }

        // Check drain collision
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

        // Check pin collision
        const pin1 = this.pinByHandle.get(handle1);
        const pin2 = this.pinByHandle.get(handle2);
        const hitPin = pin1 || pin2;
        if (hitPin) {
          const otherHandle = pin1 ? handle2 : handle1;
          const ball = this.ballByHandle.get(otherHandle);
          if (ball) {
            hitPin.hit(ball.getTint());
            this.uiLayer.addHit();
          }
        }
      },
    );
  }
}
