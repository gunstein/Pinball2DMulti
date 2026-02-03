/**
 * ServerDeepSpaceBackend - Server-backed deep space using WebSocket.
 * Delegates to ServerConnection for network communication.
 */

import {
  DeepSpaceBackend,
  CaptureCallback,
  CaptureEvent,
  ConnectionState,
} from "./DeepSpaceBackend";
import { ServerConnection } from "../shared/ServerConnection";
import { LocalDeepSpaceBackend } from "./LocalDeepSpaceBackend";
import { Player, SpaceBall3D, DeepSpaceConfig, DEFAULT_DEEP_SPACE_CONFIG } from "../shared/types";

/**
 * Server-backed deep space backend.
 * Uses WebSocket connection for multiplayer, with local fallback when disconnected.
 */
export class ServerDeepSpaceBackend implements DeepSpaceBackend {
  private connection: ServerConnection;
  private localFallback: LocalDeepSpaceBackend;
  private players: Player[] = [];
  private selfId = 0;
  private config: DeepSpaceConfig = DEFAULT_DEEP_SPACE_CONFIG;
  private connectionState: ConnectionState = "connecting";

  private captureCallbacks: Set<CaptureCallback> = new Set();
  private connectionStateCallbacks: Set<(state: ConnectionState) => void> = new Set();

  constructor(serverUrl: string) {
    // Create local fallback for when disconnected
    this.localFallback = new LocalDeepSpaceBackend(DEFAULT_DEEP_SPACE_CONFIG);

    // Create server connection
    this.connection = new ServerConnection(serverUrl);

    // Set up connection event handlers
    this.connection.onWelcome = (selfId, players, config) => {
      this.selfId = selfId;
      this.players = players;
      this.config = config;

      // Update local fallback with server config and self player
      const selfPlayer = players.find((p) => p.id === selfId);
      if (selfPlayer) {
        this.localFallback.updateConfig(config);
        this.localFallback.updateSelfPlayer(selfPlayer);
      }
    };

    this.connection.onPlayersState = (players) => {
      this.players = players;
    };

    this.connection.onTransferIn = (vx, vy, color) => {
      const event: CaptureEvent = {
        playerId: this.selfId,
        vx,
        vy,
        color,
      };

      // Notify all callbacks
      for (const cb of this.captureCallbacks) {
        cb(event);
      }
    };

    this.connection.onConnectionStateChange = (state) => {
      this.connectionState = state;
      for (const cb of this.connectionStateCallbacks) {
        cb(state);
      }
    };
  }

  tick(dt: number): void {
    // When disconnected, run local simulation for visual continuity
    if (this.connectionState !== "connected") {
      this.localFallback.tick(dt);

      // Forward any local captures
      // Note: This is handled by the localFallback's own capture mechanism
    }
    // When connected, server handles simulation - nothing to do client-side
  }

  getBalls(): Iterable<SpaceBall3D> {
    if (this.connectionState === "connected") {
      return this.connection.getBallIterable();
    } else {
      return this.localFallback.getBalls();
    }
  }

  getPlayers(): Player[] {
    return this.players;
  }

  getSelfPlayer(): Player | undefined {
    return this.players.find((p) => p.id === this.selfId);
  }

  getSelfId(): number {
    return this.selfId;
  }

  ballEscaped(vx: number, vy: number): void {
    if (this.connectionState === "connected") {
      this.connection.sendBallEscaped(vx, vy);
    } else {
      // Add to local fallback when disconnected
      this.localFallback.ballEscaped(vx, vy);
    }
  }

  onCapture(callback: CaptureCallback): () => void {
    this.captureCallbacks.add(callback);

    // Also subscribe to local fallback captures (for when disconnected)
    const unsubLocal = this.localFallback.onCapture(callback);

    return () => {
      this.captureCallbacks.delete(callback);
      unsubLocal();
    };
  }

  setPaused(paused: boolean): void {
    this.connection.sendSetPaused(paused);
  }

  getConnectionState(): ConnectionState {
    return this.connectionState;
  }

  onConnectionStateChange(callback: (state: ConnectionState) => void): () => void {
    this.connectionStateCallbacks.add(callback);
    return () => {
      this.connectionStateCallbacks.delete(callback);
    };
  }

  getConfig(): DeepSpaceConfig {
    return this.config;
  }

  dispose(): void {
    this.connection.close();
    this.localFallback.dispose();
    this.captureCallbacks.clear();
    this.connectionStateCallbacks.clear();
  }
}
