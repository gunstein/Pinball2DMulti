/**
 * DeepSpaceClient - Abstracts server vs local deep-space mode.
 *
 * Provides a unified interface so Game.ts doesn't need to know
 * whether we're connected to a server or running locally.
 */

import { ServerConnection, ConnectionState } from "./ServerConnection";
import { SphereDeepSpace } from "./SphereDeepSpace";
import { MockWorld } from "./MockWorld";
import {
  Player,
  SpaceBall3D,
  DeepSpaceConfig,
  DEFAULT_DEEP_SPACE_CONFIG,
} from "./types";

// Speed for balls entering from deep space (m/s)
const CAPTURE_SPEED = 1.5;

export interface CaptureCallback {
  (vx: number, vy: number, color: number): void;
}

export interface DeepSpaceClientCallbacks {
  onPlayersChanged: (players: Player[], selfId: number) => void;
  onConnectionStateChanged: (state: ConnectionState) => void;
  onCapture: CaptureCallback;
}

/**
 * Unified client for deep-space, handling both server and local modes.
 */
export class DeepSpaceClient {
  private useServer: boolean;
  private serverUrl: string;

  // Server mode
  private serverConnection: ServerConnection | null = null;
  private localFallback: SphereDeepSpace | null = null;

  // Mock mode
  private mockWorld: MockWorld | null = null;
  private mockDeepSpace: SphereDeepSpace | null = null;

  // State
  private selfPlayer: Player | null = null;
  private allPlayers: Player[] = [];
  private connectionState: ConnectionState = "connecting";

  // Callbacks
  private callbacks: DeepSpaceClientCallbacks;

  // For cleanup
  private abortController: AbortController;

  constructor(
    useServer: boolean,
    serverUrl: string,
    mockPlayerCount: number,
    callbacks: DeepSpaceClientCallbacks,
  ) {
    this.useServer = useServer;
    this.serverUrl = serverUrl;
    this.callbacks = callbacks;
    this.abortController = new AbortController();

    if (useServer) {
      this.initServerMode();
    } else {
      this.initMockMode(mockPlayerCount);
    }
  }

  private initServerMode() {
    this.serverConnection = new ServerConnection(this.serverUrl);
    this.localFallback = new SphereDeepSpace(DEFAULT_DEEP_SPACE_CONFIG);

    // Create temporary local player before server responds
    const localPlayer: Player = {
      id: 0,
      cellIndex: 0,
      portalPos: { x: 0, y: 0, z: 1 },
      color: 0x4da6a6,
      paused: false,
      ballsProduced: 0,
      ballsInFlight: 0,
    };
    this.selfPlayer = localPlayer;
    this.allPlayers = [localPlayer];
    this.localFallback.setPlayers([localPlayer]);
    this.callbacks.onPlayersChanged([localPlayer], localPlayer.id);

    this.serverConnection.onWelcome = (selfId, players, config) => {
      this.allPlayers = players;
      this.selfPlayer = players.find((p) => p.id === selfId) || null;
      if (this.selfPlayer) {
        // Re-create local fallback with server config
        this.localFallback = new SphereDeepSpace(config);
        this.localFallback.setPlayers([this.selfPlayer]);
      }
      this.callbacks.onPlayersChanged(players, selfId);
    };

    this.serverConnection.onPlayersState = (players) => {
      this.allPlayers = players;
      if (this.selfPlayer) {
        const updated = players.find((p) => p.id === this.selfPlayer!.id);
        if (updated) this.selfPlayer = updated;
      }
      this.callbacks.onPlayersChanged(players, this.selfPlayer?.id ?? 0);
    };

    this.serverConnection.onTransferIn = (vx, vy, color) => {
      this.callbacks.onCapture(vx, vy, color);
    };

    this.serverConnection.onConnectionStateChange = (state) => {
      this.connectionState = state;
      this.callbacks.onConnectionStateChanged(state);
    };

    // Listen for tab visibility changes (with AbortController for cleanup)
    document.addEventListener(
      "visibilitychange",
      () => {
        const paused = document.visibilityState === "hidden";
        this.serverConnection?.sendSetPaused(paused);
      },
      { signal: this.abortController.signal },
    );
  }

  private initMockMode(playerCount: number) {
    this.mockWorld = new MockWorld(playerCount);
    this.mockDeepSpace = new SphereDeepSpace(this.mockWorld.config);
    this.mockDeepSpace.setPlayers(this.mockWorld.getAllPlayers());

    this.selfPlayer = this.mockWorld.getSelfPlayer();
    this.allPlayers = this.mockWorld.getAllPlayers();
    this.connectionState = "connected";

    this.callbacks.onPlayersChanged(this.allPlayers, this.selfPlayer.id);
    this.callbacks.onConnectionStateChanged("connected");
  }

  /** Get self player */
  getSelfPlayer(): Player | null {
    return this.selfPlayer;
  }

  /** Get all players */
  getAllPlayers(): Player[] {
    return this.allPlayers;
  }

  /** Get connection state */
  getConnectionState(): ConnectionState {
    return this.connectionState;
  }

  /** Get ball color for self player */
  getBallColor(): number {
    return this.selfPlayer?.color ?? 0xffffff;
  }

  /** Get self player's portal position */
  getSelfPortalPos() {
    return this.selfPlayer?.portalPos ?? { x: 0, y: 0, z: 1 };
  }

  /**
   * Tick the deep-space simulation (for local/mock mode or when disconnected).
   * Returns nothing - captures are delivered via callback.
   */
  tick(dt: number): void {
    if (this.mockDeepSpace) {
      // Mock mode: always run local simulation
      const captures = this.mockDeepSpace.tick(dt);
      this.processCaptures(captures, this.mockDeepSpace);
    } else if (this.localFallback && this.connectionState !== "connected") {
      // Server mode but disconnected: run local fallback
      const captures = this.localFallback.tick(dt);
      this.processCaptures(captures, this.localFallback);
    }
  }

  private processCaptures(
    captures: { playerId: number; ball: SpaceBall3D; player: Player }[],
    deepSpace: SphereDeepSpace,
  ) {
    if (!this.selfPlayer) return;
    for (const capture of captures) {
      if (capture.playerId === this.selfPlayer.id) {
        const [vx, vy] = deepSpace.getCaptureVelocity2D(
          capture.ball,
          capture.player.portalPos,
          CAPTURE_SPEED,
        );
        const owner = this.allPlayers.find(
          (p) => p.id === capture.ball.ownerId,
        );
        const color = owner?.color ?? this.selfPlayer.color;
        this.callbacks.onCapture(vx, vy, color);
      }
    }
  }

  /** Get balls for rendering (handles server interpolation or local state) */
  getBalls(): Iterable<SpaceBall3D> {
    if (this.serverConnection) {
      if (this.connectionState === "connected") {
        return this.serverConnection.getBallIterable();
      } else if (this.localFallback) {
        return this.localFallback.getBallIterable();
      }
    }
    if (this.mockDeepSpace) {
      return this.mockDeepSpace.getBallIterable();
    }
    return [];
  }

  /** Notify that a ball escaped from the board */
  ballEscaped(vx: number, vy: number): void {
    if (this.serverConnection) {
      if (this.connectionState === "connected") {
        this.serverConnection.sendBallEscaped(vx, vy);
      } else if (this.localFallback && this.selfPlayer) {
        this.localFallback.addBall(
          this.selfPlayer.id,
          this.selfPlayer.portalPos,
          vx,
          vy,
        );
      }
    } else if (this.mockDeepSpace && this.selfPlayer) {
      this.mockDeepSpace.addBall(
        this.selfPlayer.id,
        this.selfPlayer.portalPos,
        vx,
        vy,
      );
    }
  }

  /** Clean up resources (event listeners, connections) */
  dispose(): void {
    this.abortController.abort();
    this.serverConnection?.close();
  }
}
