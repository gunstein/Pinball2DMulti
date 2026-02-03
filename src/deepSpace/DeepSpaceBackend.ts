/**
 * DeepSpaceBackend - Interface for deep space simulation.
 * Abstracts the difference between server mode and local/mock mode.
 */

import { Player, SpaceBall3D, DeepSpaceConfig } from "../shared/types";

/** Event emitted when a ball is captured by a player's portal */
export interface CaptureEvent {
  playerId: number;
  vx: number;
  vy: number;
  color: number;
}

/** Callback for capture events */
export type CaptureCallback = (event: CaptureEvent) => void;

/** Connection state (for server backend) */
export type ConnectionState = "connected" | "connecting" | "disconnected";

/**
 * Interface for deep space backend implementations.
 * Both server and local backends implement this interface,
 * allowing the game to work the same way in both modes.
 */
export interface DeepSpaceBackend {
  /**
   * Update the simulation (called every frame).
   * For server mode, this handles client-side interpolation.
   * For local mode, this runs the actual simulation.
   */
  tick(dt: number): void;

  /**
   * Get all balls for rendering.
   * Returns interpolated positions for smooth rendering.
   */
  getBalls(): Iterable<SpaceBall3D>;

  /**
   * Get all players.
   */
  getPlayers(): Player[];

  /**
   * Get self player (the local player).
   */
  getSelfPlayer(): Player | undefined;

  /**
   * Get self player ID.
   */
  getSelfId(): number;

  /**
   * Notify that a ball escaped from the local board.
   * @param vx - X velocity component
   * @param vy - Y velocity component (should be negative for upward escape)
   */
  ballEscaped(vx: number, vy: number): void;

  /**
   * Register a callback for when a ball is captured.
   * The callback receives the velocity to inject the ball back into the board.
   * @returns Unsubscribe function
   */
  onCapture(callback: CaptureCallback): () => void;

  /**
   * Notify pause state change (for tab visibility).
   */
  setPaused(paused: boolean): void;

  /**
   * Get current connection state.
   * For local backend, always returns "connected".
   */
  getConnectionState(): ConnectionState;

  /**
   * Register a callback for connection state changes.
   * @returns Unsubscribe function
   */
  onConnectionStateChange(callback: (state: ConnectionState) => void): () => void;

  /**
   * Get the deep space config.
   */
  getConfig(): DeepSpaceConfig;

  /**
   * Clean up resources.
   */
  dispose(): void;
}
