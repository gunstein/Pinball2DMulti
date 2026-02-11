import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { DeepSpaceClient } from "../src/shared/DeepSpaceClient";

const { FakeServerConnection } = vi.hoisted(() => {
  class FakeServerConnection {
    static instances: FakeServerConnection[] = [];

    onWelcome: ((selfId: number, players: any[], config: any) => void) | null =
      null;
    onPlayersState: ((players: any[]) => void) | null = null;
    onSpaceState: ((balls: any[]) => void) | null = null;
    onTransferIn: ((vx: number, vy: number, color: number) => void) | null =
      null;
    onProtocolMismatch:
      | ((serverVersion: number, clientVersion: number) => void)
      | null = null;
    onConnectionStateChange: ((state: any) => void) | null = null;

    sentEscapes: Array<{ vx: number; vy: number }> = [];
    setPausedCalls: boolean[] = [];

    constructor() {
      FakeServerConnection.instances.push(this);
    }

    sendBallEscaped(vx: number, vy: number) {
      this.sentEscapes.push({ vx, vy });
    }

    sendSetPaused(paused: boolean) {
      this.setPausedCalls.push(paused);
    }

    getBallIterable() {
      return [];
    }

    close() {}
  }

  return { FakeServerConnection };
});

vi.mock("../src/shared/ServerConnection", () => ({
  ServerConnection: FakeServerConnection,
}));

beforeEach(() => {
  FakeServerConnection.instances = [];
  (globalThis as any).document = {
    addEventListener: vi.fn(),
    visibilityState: "visible",
  };
});

afterEach(() => {
  delete (globalThis as any).document;
});

describe("DeepSpaceClient", () => {
  it("sends ballEscaped to server when connected", () => {
    const client = new DeepSpaceClient(true, "ws://test", 1, {
      onPlayersChanged: vi.fn(),
      onConnectionStateChanged: vi.fn(),
      onCapture: vi.fn(),
    });

    const server = FakeServerConnection.instances[0];
    server.onConnectionStateChange?.("connected");

    client.ballEscaped(1.5, -2.5);

    expect(server.sentEscapes).toEqual([{ vx: 1.5, vy: -2.5 }]);
  });

  it("does not simulate local fallback when disconnected in server mode", () => {
    const client = new DeepSpaceClient(true, "ws://test", 1, {
      onPlayersChanged: vi.fn(),
      onConnectionStateChanged: vi.fn(),
      onCapture: vi.fn(),
    });

    const server = FakeServerConnection.instances[0];
    server.onConnectionStateChange?.("disconnected");

    client.ballEscaped(2.0, -3.0);
    client.tick(0.5);

    expect(server.sentEscapes.length).toBe(0);
    expect(Array.from(client.getBalls())).toEqual([]);
  });
});
