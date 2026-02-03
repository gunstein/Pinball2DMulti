/**
 * LocalDeepSpaceBackend - Local simulation for offline/mock mode.
 * Runs SphereDeepSpace locally without a server.
 */

import {
  DeepSpaceBackend,
  CaptureCallback,
  CaptureEvent,
  ConnectionState,
} from "./DeepSpaceBackend";
import { SphereDeepSpace } from "../shared/SphereDeepSpace";
import { Player, SpaceBall3D, DeepSpaceConfig, DEFAULT_DEEP_SPACE_CONFIG } from "../shared/types";

// Speed for balls entering from deep space (m/s)
const CAPTURE_SPEED = 1.5;

/**
 * Local deep space backend using SphereDeepSpace simulation.
 * Used for offline play and mock mode.
 */
export class LocalDeepSpaceBackend implements DeepSpaceBackend {
  private deepSpace: SphereDeepSpace;
  private players: Player[] = [];
  private selfPlayer: Player;
  private captureCallbacks: Set<CaptureCallback> = new Set();
  private config: DeepSpaceConfig;

  constructor(config: DeepSpaceConfig = DEFAULT_DEEP_SPACE_CONFIG, selfPlayer?: Player) {
    this.config = config;
    this.deepSpace = new SphereDeepSpace(config);

    // Create a default self player if not provided
    this.selfPlayer = selfPlayer ?? {
      id: 1,
      cellIndex: 0,
      portalPos: { x: 0, y: 0, z: 1 }, // Front of sphere
      color: 0x4da6a6,
      paused: false,
      ballsProduced: 0,
      ballsInFlight: 0,
    };

    this.players = [this.selfPlayer];
    this.deepSpace.setPlayers(this.players);
  }

  tick(dt: number): void {
    const captures = this.deepSpace.tick(dt);

    // Process captures
    for (const capture of captures) {
      if (capture.playerId === this.selfPlayer.id) {
        const [vx, vy] = this.deepSpace.getCaptureVelocity2D(
          capture.ball,
          capture.player.portalPos,
          CAPTURE_SPEED,
        );

        // Get color from ball owner
        const owner = this.players.find((p) => p.id === capture.ball.ownerId);
        const color = owner?.color ?? this.selfPlayer.color;

        const event: CaptureEvent = {
          playerId: capture.playerId,
          vx,
          vy,
          color,
        };

        // Notify all callbacks
        for (const cb of this.captureCallbacks) {
          cb(event);
        }
      }
    }
  }

  getBalls(): Iterable<SpaceBall3D> {
    return this.deepSpace.getBallIterable();
  }

  getPlayers(): Player[] {
    return this.players;
  }

  getSelfPlayer(): Player | undefined {
    return this.selfPlayer;
  }

  getSelfId(): number {
    return this.selfPlayer.id;
  }

  ballEscaped(vx: number, vy: number): void {
    this.deepSpace.addBall(
      this.selfPlayer.id,
      this.selfPlayer.portalPos,
      vx,
      vy,
    );
  }

  onCapture(callback: CaptureCallback): () => void {
    this.captureCallbacks.add(callback);
    return () => {
      this.captureCallbacks.delete(callback);
    };
  }

  setPaused(_paused: boolean): void {
    // Local mode doesn't need pause handling
  }

  getConnectionState(): ConnectionState {
    return "connected"; // Local is always "connected"
  }

  onConnectionStateChange(_callback: (state: ConnectionState) => void): () => void {
    // Local mode never changes connection state
    return () => {};
  }

  getConfig(): DeepSpaceConfig {
    return this.config;
  }

  /**
   * Update the self player (e.g., when server provides real player data).
   */
  updateSelfPlayer(player: Player): void {
    this.selfPlayer = player;
    this.players = [player];
    this.deepSpace.setPlayers(this.players);
  }

  /**
   * Update config (e.g., when server provides config).
   * Creates a new SphereDeepSpace with the new config.
   */
  updateConfig(config: DeepSpaceConfig): void {
    this.config = config;
    this.deepSpace = new SphereDeepSpace(config);
    this.deepSpace.setPlayers(this.players);
  }

  dispose(): void {
    this.captureCallbacks.clear();
  }
}
