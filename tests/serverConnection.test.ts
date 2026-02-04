import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { DEFAULT_DEEP_SPACE_CONFIG } from "../src/shared/types";
import * as vec3 from "../src/shared/vec3";
import { ServerConnection } from "../src/shared/ServerConnection";

class FakeWebSocket {
  static instances: FakeWebSocket[] = [];

  url: string;
  sent: string[] = [];
  closed = false;
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;
  onerror: ((e: unknown) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }

  emitOpen() {
    this.onopen?.();
  }

  emitClose() {
    this.onclose?.();
  }

  emitMessage(data: string) {
    this.onmessage?.({ data });
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.closed = true;
    this.onclose?.();
  }
}

let consoleSpies: Array<ReturnType<typeof vi.spyOn>> = [];

beforeEach(() => {
  FakeWebSocket.instances = [];
  vi.useFakeTimers();
  vi.stubGlobal("WebSocket", FakeWebSocket as unknown as typeof WebSocket);

  if (process.env.VITEST_LOGS !== "1") {
    consoleSpies = [
      vi.spyOn(console, "log").mockImplementation(() => {}),
      vi.spyOn(console, "warn").mockImplementation(() => {}),
      vi.spyOn(console, "error").mockImplementation(() => {}),
    ];
  }
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  for (const spy of consoleSpies) {
    spy.mockRestore();
  }
  consoleSpies = [];
});

describe("ServerConnection", () => {
  it("protocol mismatch triggers callback and stops reconnect", () => {
    const onMismatch = vi.fn();
    const conn = new ServerConnection("ws://test");
    conn.onProtocolMismatch = onMismatch;

    const ws = FakeWebSocket.instances[0];
    ws.emitOpen();

    ws.emitMessage(
      JSON.stringify({
        type: "welcome",
        protocolVersion: 999,
        selfId: 1,
        players: [],
        config: DEFAULT_DEEP_SPACE_CONFIG,
      }),
    );

    expect(onMismatch).toHaveBeenCalledWith(999, 1);
    expect(ws.closed).toBe(true);

    vi.runAllTimers();
    expect(FakeWebSocket.instances.length).toBe(1);
  });

  it("sendBallEscaped clamps velocity and ignores NaN/Inf", () => {
    const conn = new ServerConnection("ws://test");
    const ws = FakeWebSocket.instances[0];
    ws.emitOpen();

    conn.sendBallEscaped(100, -100);
    expect(ws.sent.length).toBe(1);
    const sent = JSON.parse(ws.sent[0]);
    expect(sent.vx).toBe(10);
    expect(sent.vy).toBe(-10);

    conn.sendBallEscaped(Number.NaN, 1);
    expect(ws.sent.length).toBe(1);
  });

  it("clamps extrapolation dt to 0.2s", () => {
    const rotateSpy = vi.spyOn(vec3, "rotateNormalizeInPlace");

    const conn = new ServerConnection("ws://test");
    const ws = FakeWebSocket.instances[0];
    ws.emitOpen();

    const nowSpy = vi.spyOn(performance, "now");
    nowSpy.mockReturnValueOnce(0); // lastSnapshotTime

    ws.emitMessage(
      JSON.stringify({
        type: "space_state",
        balls: [
          {
            id: 1,
            ownerId: 2,
            pos: [1, 0, 0],
            axis: [0, 1, 0],
            omega: 2,
          },
        ],
      }),
    );

    nowSpy.mockReturnValueOnce(1000); // dt = 1.0s -> clamp to 0.2s

    conn.getBallIterable();

    expect(rotateSpy).toHaveBeenCalled();
    const angle = rotateSpy.mock.calls[0][2];
    expect(angle).toBeCloseTo(0.4, 6); // omega(2) * 0.2
  });
});
