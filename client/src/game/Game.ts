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

// WebSocket URL: use env override or derive from current host (works behind reverse proxy)
const WS_SCHEME = location.protocol === "https:" ? "wss" : "ws";
const SERVER_URL =
  import.meta.env.VITE_SERVER_URL ?? `${WS_SCHEME}://${location.host}/ws`;

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

  private accumulator = 0;
  private respawnTimer = 0;

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
      // Note: We do NOT reapply ballColor to all balls here,
      // because captured balls should keep their original owner's color.
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
      const launcherBalls = this.balls.filter((b) => b.isInShooterLane());
      const count = launcherBalls.length;
      if (count === 0) return;
      // Quadratic scaling: multiple stacked balls need more force to overcome
      // their combined weight and the friction between them
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

  private spawnBallFromCapture(vx: number, vy: number, color: number) {
    const ball = this.acquireBall();
    ball.setTint(color);
    const x = this.physics.toPhysicsX(CAPTURE_SPAWN_X);
    const y = this.physics.toPhysicsY(CAPTURE_SPAWN_Y);
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

  getServerVersion(): string {
    return this.deepSpaceClient.getServerVersion();
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

    // Send activity heartbeat if player has been active recently
    this.sendActivityHeartbeat();

    // Update deep space (handles local simulation when needed)
    this.deepSpaceClient.tick(dt);

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
    this.deepSpaceLayer.update(
      dt,
      this.deepSpaceClient.getBalls(),
      this.deepSpaceClient.getAllPlayers(),
      this.deepSpaceClient.getSelfPlayer()?.id ?? 0,
    );
  }

  private sendActivityHeartbeat() {
    const inputTime = this.input.lastActivityTime;
    if (inputTime === 0) return; // no input yet

    const now = performance.now();
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
    for (let i = this.balls.length - 1; i >= 0; i--) {
      const ball = this.balls[i];
      if (!ball.isActive()) continue;

      const snapshot = ball.getEscapeSnapshot();
      if (snapshot) {
        this.deepSpaceClient.ballEscaped(snapshot.vx, snapshot.vy);
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
            hitPin.hit();
            this.uiLayer.addHit();
          }
        }
      },
    );
  }
}
