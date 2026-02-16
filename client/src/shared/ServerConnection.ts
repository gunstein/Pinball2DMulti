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
import { rotateNormalizeInPlace, slerpTo } from "./vec3";
import type { BallWire, PlayerWire, ServerMsg } from "./generated";

/** Must match server's PROTOCOL_VERSION in protocol.rs */
const CLIENT_PROTOCOL_VERSION = 2;

/** Connection state for UI feedback */
export type ConnectionState = "connected" | "connecting" | "disconnected";

/** Reconnect configuration */
const RECONNECT_INITIAL_DELAY_MS = 1000;
const RECONNECT_MAX_DELAY_MS = 30000;
const RECONNECT_MULTIPLIER = 1.5;

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
  private serverVersion = "";
  private connectionState: ConnectionState = "connecting";

  // Reconnect state
  private reconnectDelay = RECONNECT_INITIAL_DELAY_MS;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private shouldReconnect = true;
  private protocolMismatch = false;

  // Buffered interpolation state (two-snapshot slerp)
  private prevBalls: SpaceBall3D[] = [];
  private prevBallPool: SpaceBall3D[] = [];
  private prevRecvTime = 0; // performance.now()/1000 when prev snapshot arrived
  private currRecvTime = 0; // performance.now()/1000 when curr snapshot arrived
  private prevBallIdToIndex: Map<number, number> = new Map();
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

  /** Update balls array from snapshot, shifting current to prev for interpolation */
  private updateBallsFromSnapshot(wireBalls: BallWire[]) {
    const newCount = wireBalls.length;

    // Shift current → prev (swap arrays + pools for zero-copy)
    const tmpBalls = this.prevBalls;
    const tmpPool = this.prevBallPool;
    this.prevBalls = this.balls;
    this.prevBallPool = this.ballPool;
    this.balls = tmpBalls;
    this.ballPool = tmpPool;

    // Rebuild prev id→index map
    this.prevBallIdToIndex.clear();
    for (let i = 0; i < this.prevBalls.length; i++) {
      this.prevBallIdToIndex.set(this.prevBalls[i].id, i);
    }

    // Update timing (use local receive time — no clock sync needed)
    this.prevRecvTime = this.currRecvTime;
    this.currRecvTime = performance.now() / 1000;

    // Grow pools if needed
    while (this.ballPool.length < newCount) {
      this.ballPool.push(this.createEmptyBall());
    }
    while (this.interpolatedBallPool.length < newCount) {
      this.interpolatedBallPool.push(this.createEmptyBall());
    }

    // Update balls array length
    this.balls.length = newCount;
    this.interpolatedBalls.length = newCount;

    // Copy wire data into current balls
    for (let i = 0; i < newCount; i++) {
      const wire = wireBalls[i];

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

      this.balls[i] = ball;

      // Initialize interpolated ball
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
        this.serverVersion = msg.serverVersion ?? "";
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
   * Uses buffered interpolation: slerps between two consecutive server snapshots
   * with a small render delay, providing smooth motion without snap-backs.
   * Falls back to single-snapshot extrapolation when only one snapshot is available.
   */
  getBallIterable(): Iterable<SpaceBall3D> {
    if (this.interpolatedBalls.length === 0) {
      return this.balls;
    }

    const nowSecs = performance.now() / 1000;
    const interval = this.currRecvTime - this.prevRecvTime;

    // Need two valid snapshots for interpolation
    if (interval <= 0 || this.prevBalls.length === 0) {
      // Fallback: extrapolate from current snapshot
      const elapsed = Math.min(nowSecs - this.currRecvTime, 0.2);
      for (let i = 0; i < this.interpolatedBalls.length; i++) {
        const ball = this.interpolatedBalls[i];
        const original = this.balls[i];
        ball.pos.x = original.pos.x;
        ball.pos.y = original.pos.y;
        ball.pos.z = original.pos.z;
        rotateNormalizeInPlace(
          ball.pos,
          ball.axis,
          ball.omega * Math.max(0, elapsed),
        );
      }
      return this.interpolatedBalls;
    }

    // Render one interval behind real-time so t stays in [0, 1] range.
    // This adds ~100ms latency but eliminates snap-backs completely.
    const renderTime = nowSecs - interval;
    const t = Math.min(
      Math.max((renderTime - this.prevRecvTime) / interval, 0),
      1.0,
    );

    for (let i = 0; i < this.interpolatedBalls.length; i++) {
      const ball = this.interpolatedBalls[i];
      const curr = this.balls[i];

      // Always use current axis/omega (for comet tail rendering)
      ball.axis.x = curr.axis.x;
      ball.axis.y = curr.axis.y;
      ball.axis.z = curr.axis.z;
      ball.omega = curr.omega;

      const prevIdx = this.prevBallIdToIndex.get(curr.id);
      if (prevIdx !== undefined) {
        // Interpolate between prev and curr position
        const prev = this.prevBalls[prevIdx];
        slerpTo(prev.pos, curr.pos, t, ball.pos);
      } else {
        // New ball — no prev data, show at current position
        ball.pos.x = curr.pos.x;
        ball.pos.y = curr.pos.y;
        ball.pos.z = curr.pos.z;
      }
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

  /** Get server version string (from welcome message) */
  getServerVersion(): string {
    return this.serverVersion;
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
