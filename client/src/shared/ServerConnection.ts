/**
 * WebSocket connection to the pinball server.
 * Replaces MockWorld + SphereDeepSpace for multiplayer.
 *
 * Includes client-side interpolation: ball positions are extrapolated
 * between server snapshots using axis/omega for smooth 60fps rendering.
 */

import {
  Player,
  DeepSpaceConfig,
  DEFAULT_DEEP_SPACE_CONFIG,
  SpaceBall3D,
} from "./types";
import { rotateNormalizeInPlace } from "./vec3";

/** Must match server's PROTOCOL_VERSION in protocol.rs */
const CLIENT_PROTOCOL_VERSION = 1;

/** Connection state for UI feedback */
export type ConnectionState = "connected" | "connecting" | "disconnected";

/** Reconnect configuration */
const RECONNECT_INITIAL_DELAY_MS = 1000;
const RECONNECT_MAX_DELAY_MS = 30000;
const RECONNECT_MULTIPLIER = 1.5;

// === Wire types matching server protocol.rs ===

interface PlayerWire {
  id: number;
  cellIndex: number;
  portalPos: [number, number, number];
  color: number;
  paused?: boolean;
  ballsProduced?: number;
  ballsInFlight?: number;
}

interface BallWire {
  id: number;
  ownerId: number;
  pos: [number, number, number];
  axis: [number, number, number];
  omega: number;
}

interface WelcomeMsg {
  type: "welcome";
  protocolVersion: number;
  selfId: number;
  players: PlayerWire[];
  config: DeepSpaceConfig;
}

interface PlayersStateMsg {
  type: "players_state";
  players: PlayerWire[];
}

interface SpaceStateMsg {
  type: "space_state";
  balls: BallWire[];
}

interface TransferInMsg {
  type: "transfer_in";
  vx: number;
  vy: number;
  ownerId: number;
  color: number;
}

type ServerMsg = WelcomeMsg | PlayersStateMsg | SpaceStateMsg | TransferInMsg;

function wireToPlayer(w: PlayerWire): Player {
  return {
    id: w.id,
    cellIndex: w.cellIndex,
    portalPos: { x: w.portalPos[0], y: w.portalPos[1], z: w.portalPos[2] },
    color: w.color,
    paused: w.paused ?? false,
    ballsProduced: w.ballsProduced ?? 0,
    ballsInFlight: w.ballsInFlight ?? 0,
  };
}

function wireToSpaceBall(w: BallWire): SpaceBall3D {
  return {
    id: w.id,
    ownerId: w.ownerId,
    pos: { x: w.pos[0], y: w.pos[1], z: w.pos[2] },
    axis: { x: w.axis[0], y: w.axis[1], z: w.axis[2] },
    omega: w.omega,
    age: 0,
    timeSinceHit: 0,
    rerouteCooldown: 0,
    rerouteTargetAxis: undefined,
    rerouteProgress: 0,
    rerouteTargetOmega: 0,
  };
}

/**
 * Manages the WebSocket connection to the game server.
 * Provides client-side interpolation for smooth rendering between server snapshots.
 */
export class ServerConnection {
  private ws: WebSocket | null = null;
  private url: string;
  private selfId = 0;
  private players: Player[] = [];
  private balls: SpaceBall3D[] = [];
  private config: DeepSpaceConfig = DEFAULT_DEEP_SPACE_CONFIG;
  private connectionState: ConnectionState = "connecting";

  // Reconnect state
  private reconnectDelay = RECONNECT_INITIAL_DELAY_MS;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private shouldReconnect = true;
  private protocolMismatch = false;

  // Interpolation state
  private lastSnapshotTime = 0;
  private interpolatedBalls: SpaceBall3D[] = [];

  // Object pool for balls (reused to avoid GC pressure)
  private ballPool: SpaceBall3D[] = [];
  private interpolatedBallPool: SpaceBall3D[] = [];

  // Callbacks
  onWelcome:
    | ((selfId: number, players: Player[], config: DeepSpaceConfig) => void)
    | null = null;
  onPlayersState: ((players: Player[]) => void) | null = null;
  onSpaceState: ((balls: SpaceBall3D[]) => void) | null = null;
  onTransferIn: ((vx: number, vy: number, color: number) => void) | null = null;
  onProtocolMismatch:
    | ((serverVersion: number, clientVersion: number) => void)
    | null = null;
  onConnectionStateChange: ((state: ConnectionState) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    this.connect();
  }

  private setConnectionState(state: ConnectionState) {
    if (this.connectionState !== state) {
      this.connectionState = state;
      this.onConnectionStateChange?.(state);
    }
  }

  private connect() {
    this.setConnectionState("connecting");
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      this.setConnectionState("connected");
      this.reconnectDelay = RECONNECT_INITIAL_DELAY_MS; // Reset on successful connect
      console.log("[ServerConnection] Connected to server");
    };

    this.ws.onmessage = (ev) => {
      try {
        const msg: ServerMsg = JSON.parse(ev.data);
        this.handleMessage(msg);
      } catch (e) {
        console.error("Failed to parse server message:", e);
      }
    };

    this.ws.onclose = () => {
      this.setConnectionState("disconnected");
      console.log("[ServerConnection] Disconnected from server");
      this.scheduleReconnect();
    };

    this.ws.onerror = (e) => {
      console.error("WebSocket error:", e);
    };
  }

  private scheduleReconnect() {
    // Don't reconnect if explicitly closed or protocol mismatch
    if (!this.shouldReconnect || this.protocolMismatch) {
      return;
    }

    console.log(
      `[ServerConnection] Reconnecting in ${this.reconnectDelay}ms...`,
    );

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, this.reconnectDelay);

    // Exponential backoff
    this.reconnectDelay = Math.min(
      this.reconnectDelay * RECONNECT_MULTIPLIER,
      RECONNECT_MAX_DELAY_MS,
    );
  }

  /** Update balls array from snapshot, reusing existing objects to avoid allocations */
  private updateBallsFromSnapshot(wireBalls: BallWire[]) {
    const newCount = wireBalls.length;

    // Grow pools if needed
    while (this.ballPool.length < newCount) {
      this.ballPool.push(this.createEmptyBall());
    }
    while (this.interpolatedBallPool.length < newCount) {
      this.interpolatedBallPool.push(this.createEmptyBall());
    }

    // Update balls array length (reuse array, just adjust length)
    this.balls.length = newCount;
    this.interpolatedBalls.length = newCount;

    // Update each ball in place
    for (let i = 0; i < newCount; i++) {
      const wire = wireBalls[i];

      // Reuse ball from pool
      const ball = this.ballPool[i];
      ball.id = wire.id;
      ball.ownerId = wire.ownerId;
      ball.pos.x = wire.pos[0];
      ball.pos.y = wire.pos[1];
      ball.pos.z = wire.pos[2];
      ball.axis.x = wire.axis[0];
      ball.axis.y = wire.axis[1];
      ball.axis.z = wire.axis[2];
      ball.omega = wire.omega;
      // age, timeSinceHit, rerouteCooldown not sent from server

      this.balls[i] = ball;

      // Copy to interpolated ball (reuse from pool)
      const interp = this.interpolatedBallPool[i];
      interp.id = ball.id;
      interp.ownerId = ball.ownerId;
      interp.pos.x = ball.pos.x;
      interp.pos.y = ball.pos.y;
      interp.pos.z = ball.pos.z;
      interp.axis.x = ball.axis.x;
      interp.axis.y = ball.axis.y;
      interp.axis.z = ball.axis.z;
      interp.omega = ball.omega;

      this.interpolatedBalls[i] = interp;
    }
  }

  /** Create an empty ball object for the pool */
  private createEmptyBall(): SpaceBall3D {
    return {
      id: 0,
      ownerId: 0,
      pos: { x: 0, y: 0, z: 0 },
      axis: { x: 0, y: 0, z: 1 },
      omega: 0,
      age: 0,
      timeSinceHit: 0,
      rerouteCooldown: 0,
      rerouteTargetAxis: undefined,
      rerouteProgress: 0,
      rerouteTargetOmega: 0,
    };
  }

  private handleMessage(msg: ServerMsg) {
    switch (msg.type) {
      case "welcome":
        if (msg.protocolVersion !== CLIENT_PROTOCOL_VERSION) {
          console.error(
            `[ServerConnection] Protocol version mismatch: server=${msg.protocolVersion}, client=${CLIENT_PROTOCOL_VERSION}. Please refresh the page.`,
          );
          this.protocolMismatch = true; // Prevent reconnect attempts
          this.ws?.close();
          this.onProtocolMismatch?.(
            msg.protocolVersion,
            CLIENT_PROTOCOL_VERSION,
          );
          return;
        }
        this.selfId = msg.selfId;
        this.players = msg.players.map(wireToPlayer);
        this.config = msg.config;
        this.onWelcome?.(this.selfId, this.players, this.config);
        break;

      case "players_state":
        this.players = msg.players.map(wireToPlayer);
        this.onPlayersState?.(this.players);
        break;

      case "space_state":
        this.updateBallsFromSnapshot(msg.balls);
        this.lastSnapshotTime = performance.now();
        this.onSpaceState?.(this.balls);
        break;

      case "transfer_in":
        this.onTransferIn?.(msg.vx, msg.vy, msg.color);
        break;
    }
  }

  /** Send ball_escaped to server */
  sendBallEscaped(vx: number, vy: number) {
    if (this.ws && this.connectionState === "connected") {
      // Client-side validation to avoid disconnect from server
      if (!Number.isFinite(vx) || !Number.isFinite(vy)) {
        console.warn("sendBallEscaped: ignoring NaN/Inf velocity", vx, vy);
        return;
      }
      // Clamp to server's max_velocity (default 10.0)
      const MAX_V = 10;
      vx = Math.max(-MAX_V, Math.min(MAX_V, vx));
      vy = Math.max(-MAX_V, Math.min(MAX_V, vy));

      this.ws.send(JSON.stringify({ type: "ball_escaped", vx, vy }));
    }
  }

  /** Send activity heartbeat to server */
  sendActivity() {
    if (this.ws && this.connectionState === "connected") {
      this.ws.send(JSON.stringify({ type: "activity" }));
    }
  }

  /** Send set_paused to server (when tab visibility changes) */
  sendSetPaused(paused: boolean) {
    if (this.ws && this.connectionState === "connected") {
      this.ws.send(JSON.stringify({ type: "set_paused", paused }));
    }
  }

  /**
   * Get interpolated ball positions for rendering.
   * Extrapolates positions based on time since last snapshot using axis/omega.
   * This provides smooth 60fps rendering even with 15Hz server updates.
   */
  getBallIterable(): Iterable<SpaceBall3D> {
    if (this.interpolatedBalls.length === 0) {
      return this.balls;
    }

    const now = performance.now();
    const dt = (now - this.lastSnapshotTime) / 1000; // Convert to seconds

    // Clamp dt to avoid over-extrapolation (max 200ms ahead)
    const clampedDt = Math.min(dt, 0.2);

    // Extrapolate each ball's position
    for (let i = 0; i < this.interpolatedBalls.length; i++) {
      const ball = this.interpolatedBalls[i];
      const original = this.balls[i];

      // Reset to original position before extrapolating
      ball.pos.x = original.pos.x;
      ball.pos.y = original.pos.y;
      ball.pos.z = original.pos.z;

      // Rotate by omega * dt
      rotateNormalizeInPlace(ball.pos, ball.axis, ball.omega * clampedDt);
    }

    return this.interpolatedBalls;
  }

  /** Get all players */
  getAllPlayers(): Player[] {
    return this.players;
  }

  /** Get self player ID */
  getSelfId(): number {
    return this.selfId;
  }

  /** Get self player */
  getSelfPlayer(): Player | undefined {
    return this.players.find((p) => p.id === this.selfId);
  }

  /** Is connected to server */
  isConnected(): boolean {
    return this.connectionState === "connected";
  }

  /** Get current connection state */
  getConnectionState(): ConnectionState {
    return this.connectionState;
  }

  /** Close connection and stop reconnect attempts */
  close() {
    this.shouldReconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.ws?.close();
  }
}
