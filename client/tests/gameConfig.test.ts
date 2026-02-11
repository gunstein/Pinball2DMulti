import { describe, expect, it } from "vitest";
import { buildServerUrl, launcherStackScale } from "../src/game/gameConfig";

describe("gameConfig", () => {
  it("uses env override when provided", () => {
    expect(
      buildServerUrl(
        { protocol: "https:", host: "pinball.example.com" },
        "wss://override.example/ws",
      ),
    ).toBe("wss://override.example/ws");
  });

  it("derives ws url from location when no override", () => {
    expect(
      buildServerUrl({ protocol: "http:", host: "localhost:5173" }),
    ).toBe("ws://localhost:5173/ws");
    expect(
      buildServerUrl({ protocol: "https:", host: "pinball.example.com" }),
    ).toBe("wss://pinball.example.com/ws");
  });

  it("uses quadratic launcher stack scale", () => {
    expect(launcherStackScale(0)).toBe(1);
    expect(launcherStackScale(1)).toBe(1);
    expect(launcherStackScale(2)).toBe(4);
    expect(launcherStackScale(3)).toBe(9);
  });
});
