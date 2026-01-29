/**
 * WebSocket connection to the pinball server.
 * Replaces MockWorld + SphereDeepSpace for multiplayer.
 */

import {
  Player,
  DeepSpaceConfig,
  DEFAULT_DEEP_SPACE_CONFIG,
  SpaceBall3D,
} from "./types";
import { Vec3 } from "./vec3";

// === Wire types matching server protocol.rs ===

interface PlayerWire {
  id: number;
  cellIndex: number;
  portalPos: [number, number, number];
  color: number;
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
}

type ServerMsg = WelcomeMsg | PlayersStateMsg | SpaceStateMsg | TransferInMsg;

function wireToPlayer(w: PlayerWire): Player {
  return {
    id: w.id,
    cellIndex: w.cellIndex,
    portalPos: { x: w.portalPos[0], y: w.portalPos[1], z: w.portalPos[2] },
    color: w.color,
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
  };
}

/**
 * Manages the WebSocket connection to the game server.
 */
export class ServerConnection {
  private ws: WebSocket | null = null;
  private selfId = 0;
  private players: Player[] = [];
  private balls: SpaceBall3D[] = [];
  private config: DeepSpaceConfig = DEFAULT_DEEP_SPACE_CONFIG;
  private connected = false;

  // Callbacks
  onWelcome:
    | ((selfId: number, players: Player[], config: DeepSpaceConfig) => void)
    | null = null;
  onPlayersState: ((players: Player[]) => void) | null = null;
  onSpaceState: ((balls: SpaceBall3D[]) => void) | null = null;
  onTransferIn: ((vx: number, vy: number) => void) | null = null;

  constructor(url: string) {
    this.connect(url);
  }

  private connect(url: string) {
    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      this.connected = true;
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
      this.connected = false;
      console.log("[ServerConnection] Disconnected from server");
    };

    this.ws.onerror = (e) => {
      console.error("WebSocket error:", e);
    };
  }

  private handleMessage(msg: ServerMsg) {
    switch (msg.type) {
      case "welcome":
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
        this.balls = msg.balls.map(wireToSpaceBall);
        this.onSpaceState?.(this.balls);
        break;

      case "transfer_in":
        this.onTransferIn?.(msg.vx, msg.vy);
        break;
    }
  }

  /** Send ball_escaped to server */
  sendBallEscaped(vx: number, vy: number) {
    if (this.ws && this.connected) {
      this.ws.send(JSON.stringify({ type: "ball_escaped", vx, vy }));
    }
  }

  /** Get latest ball positions from server (for rendering) */
  getBallIterable(): Iterable<SpaceBall3D> {
    return this.balls;
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
    return this.connected;
  }
}
