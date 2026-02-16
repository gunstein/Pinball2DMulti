/**
 * WebSocket connection to the pinball server.
 * Replaces MockWorld + SphereDeepSpace for multiplayer.
 *
 * Includes client-side snapshot interpolation for smooth 60fps rendering.
 */

import {
  Player,
  DeepSpaceConfig,
  DEFAULT_DEEP_SPACE_CONFIG,
  SpaceBall3D,
} from "./types";
import { rotateNormalizeInPlace, slerpTo, type Vec3 } from "./vec3";
import type { BallWire, PlayerWire, ServerMsg } from "./generated";

/** Must match server's PROTOCOL_VERSION in protocol.rs */
const CLIENT_PROTOCOL_VERSION = 2;

/** Connection state for UI feedback */
export type ConnectionState = "connected" | "connecting" | "disconnected";

/** Reconnect configuration */
const RECONNECT_INITIAL_DELAY_MS = 1000;
const RECONNECT_MAX_DELAY_MS = 30000;
const RECONNECT_MULTIPLIER = 1.5;

/** Snapshot interpolation tuning */
const INTERPOLATION_DELAY_SECS = 0.2;
const MAX_EXTRAPOLATION_SECS = 0.2;
const MAX_SNAPSHOT_BUFFER = 8;
const SNAPSHOT_EPSILON_SECS = 1e-6;
const OFFSET_SMOOTH_UP_ALPHA = 0.02;

interface Snapshot {
  serverTime: number;
  recvTime: number;
  balls: BallWire[];
  idToIndex: Map<number, number>;
}

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

  // Snapshot interpolation state
  private snapshots: Snapshot[] = [];
  private hasServerTimeOffset = false;
  private serverTimeOffset = 0;

  // Object pools for reusable output objects
  private ballPool: SpaceBall3D[] = [];
  private interpolatedBalls: SpaceBall3D[] = [];
  private interpolatedBallPool: SpaceBall3D[] = [];

  // Reused scratch vectors (avoid per-ball allocations)
  private scratchPrevPos: Vec3 = { x: 0, y: 0, z: 0 };
  private scratchCurrPos: Vec3 = { x: 0, y: 0, z: 0 };

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
      this.resetInterpolationState();
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
      this.resetInterpolationState();
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

  private resetInterpolationState() {
    this.snapshots.length = 0;
    this.hasServerTimeOffset = false;
    this.serverTimeOffset = 0;
    this.balls.length = 0;
    this.interpolatedBalls.length = 0;
  }

  /** Update snapshots and latest ball cache from a new server snapshot. */
  private updateBallsFromSnapshot(wireBalls: BallWire[], serverTime: number) {
    if (!Number.isFinite(serverTime)) {
      return;
    }

    const recvTime = performance.now() / 1000;

    const last = this.snapshots[this.snapshots.length - 1];
    if (last) {
      if (serverTime < last.serverTime - SNAPSHOT_EPSILON_SECS) {
        // Server timeline moved backwards (likely reconnect / restart).
        this.resetInterpolationState();
      } else if (Math.abs(serverTime - last.serverTime) <= SNAPSHOT_EPSILON_SECS) {
        // Duplicate timestamp: keep the latest payload for this time point.
        this.snapshots.pop();
      }
    }

    const idToIndex: Map<number, number> = new Map();
    for (let i = 0; i < wireBalls.length; i++) {
      idToIndex.set(wireBalls[i].id, i);
    }

    this.snapshots.push({
      serverTime,
      recvTime,
      balls: wireBalls,
      idToIndex,
    });

    if (this.snapshots.length > MAX_SNAPSHOT_BUFFER) {
      this.snapshots.shift();
    }

    this.updateServerTimeOffset(serverTime, recvTime);
    this.copyWireBallsToArray(wireBalls, this.balls, this.ballPool);
  }

  private updateServerTimeOffset(serverTime: number, recvTime: number) {
    const sample = recvTime - serverTime;
    if (!Number.isFinite(sample)) {
      return;
    }

    if (!this.hasServerTimeOffset) {
      this.serverTimeOffset = sample;
      this.hasServerTimeOffset = true;
      return;
    }

    // Fast downward updates (better path), slow upward smoothing (temporary queueing/jitter).
    if (sample < this.serverTimeOffset) {
      this.serverTimeOffset = sample;
    } else {
      this.serverTimeOffset +=
        (sample - this.serverTimeOffset) * OFFSET_SMOOTH_UP_ALPHA;
    }
  }

  private ensureBallPoolSize(pool: SpaceBall3D[], count: number) {
    while (pool.length < count) {
      pool.push(this.createEmptyBall());
    }
  }

  private copyWireBallsToArray(
    wireBalls: BallWire[],
    out: SpaceBall3D[],
    pool: SpaceBall3D[],
  ) {
    const count = wireBalls.length;
    this.ensureBallPoolSize(pool, count);

    out.length = count;
    for (let i = 0; i < count; i++) {
      const wire = wireBalls[i];
      const ball = pool[i];
      ball.id = wire.id;
      ball.ownerId = wire.ownerId;
      ball.pos.x = wire.pos[0];
      ball.pos.y = wire.pos[1];
      ball.pos.z = wire.pos[2];
      ball.axis.x = wire.axis[0];
      ball.axis.y = wire.axis[1];
      ball.axis.z = wire.axis[2];
      ball.omega = wire.omega;
      out[i] = ball;
    }
  }

  private writeWirePosToVec(pos: [number, number, number], out: Vec3) {
    out.x = pos[0];
    out.y = pos[1];
    out.z = pos[2];
  }

  private fillFromSnapshot(snapshot: Snapshot, extrapolateSecs: number) {
    const count = snapshot.balls.length;
    this.ensureBallPoolSize(this.interpolatedBallPool, count);
    this.interpolatedBalls.length = count;

    const clampedExtrap = Math.max(0, Math.min(MAX_EXTRAPOLATION_SECS, extrapolateSecs));

    for (let i = 0; i < count; i++) {
      const wire = snapshot.balls[i];
      const out = this.interpolatedBallPool[i];

      out.id = wire.id;
      out.ownerId = wire.ownerId;
      out.pos.x = wire.pos[0];
      out.pos.y = wire.pos[1];
      out.pos.z = wire.pos[2];
      out.axis.x = wire.axis[0];
      out.axis.y = wire.axis[1];
      out.axis.z = wire.axis[2];
      out.omega = wire.omega;

      if (clampedExtrap > 0) {
        rotateNormalizeInPlace(out.pos, out.axis, out.omega * clampedExtrap);
      }

      this.interpolatedBalls[i] = out;
    }

    return this.interpolatedBalls;
  }

  private fillByInterpolation(older: Snapshot, newer: Snapshot, t: number) {
    const count = newer.balls.length;
    this.ensureBallPoolSize(this.interpolatedBallPool, count);
    this.interpolatedBalls.length = count;

    const clampedT = Math.max(0, Math.min(1, t));

    for (let i = 0; i < count; i++) {
      const newerWire = newer.balls[i];
      const out = this.interpolatedBallPool[i];

      out.id = newerWire.id;
      out.ownerId = newerWire.ownerId;
      out.axis.x = newerWire.axis[0];
      out.axis.y = newerWire.axis[1];
      out.axis.z = newerWire.axis[2];
      out.omega = newerWire.omega;

      const olderIdx = older.idToIndex.get(newerWire.id);
      if (olderIdx !== undefined) {
        const olderWire = older.balls[olderIdx];
        this.writeWirePosToVec(olderWire.pos, this.scratchPrevPos);
        this.writeWirePosToVec(newerWire.pos, this.scratchCurrPos);
        slerpTo(this.scratchPrevPos, this.scratchCurrPos, clampedT, out.pos);
      } else {
        out.pos.x = newerWire.pos[0];
        out.pos.y = newerWire.pos[1];
        out.pos.z = newerWire.pos[2];
      }

      this.interpolatedBalls[i] = out;
    }

    return this.interpolatedBalls;
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
        this.updateBallsFromSnapshot(msg.balls, msg.serverTime);
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
   * Uses a timestamped snapshot buffer keyed by serverTime.
   */
  getBallIterable(): Iterable<SpaceBall3D> {
    if (this.snapshots.length === 0) {
      return this.balls;
    }

    const nowSecs = performance.now() / 1000;

    if (this.snapshots.length === 1) {
      const only = this.snapshots[0];
      const elapsed = Math.max(0, nowSecs - only.recvTime);
      return this.fillFromSnapshot(only, elapsed);
    }

    let renderServerTime: number;
    if (this.hasServerTimeOffset) {
      renderServerTime =
        nowSecs - this.serverTimeOffset - INTERPOLATION_DELAY_SECS;
    } else {
      const latest = this.snapshots[this.snapshots.length - 1];
      renderServerTime = latest.serverTime - INTERPOLATION_DELAY_SECS;
    }

    const first = this.snapshots[0];
    const latest = this.snapshots[this.snapshots.length - 1];

    if (renderServerTime <= first.serverTime) {
      return this.fillFromSnapshot(first, 0);
    }

    if (renderServerTime >= latest.serverTime) {
      return this.fillFromSnapshot(latest, renderServerTime - latest.serverTime);
    }

    let newerIndex = 1;
    while (
      newerIndex < this.snapshots.length &&
      this.snapshots[newerIndex].serverTime < renderServerTime
    ) {
      newerIndex++;
    }

    if (newerIndex >= this.snapshots.length) {
      return this.fillFromSnapshot(latest, MAX_EXTRAPOLATION_SECS);
    }

    const older = this.snapshots[newerIndex - 1];
    const newer = this.snapshots[newerIndex];
    const dt = newer.serverTime - older.serverTime;

    if (dt <= SNAPSHOT_EPSILON_SECS) {
      return this.fillFromSnapshot(newer, 0);
    }

    const t = (renderServerTime - older.serverTime) / dt;
    return this.fillByInterpolation(older, newer, t);
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
