/**
 * Entry point for the multiplayer pinball client.
 *
 * Each player has a 2D pinball board (PixiJS + Rapier2D physics). When a
 * ball escapes the top of the board it enters "deep space" — a shared 3D
 * unit sphere where balls travel along great circles between players.
 * When a ball reaches another player's portal it drops into their board.
 *
 * Rendering is split into three layers (back to front):
 *   1. SphereDeepSpaceLayer — full-screen star field + sphere projection
 *   2. BoardLayer — the 2D pinball table (walls, flippers, bumpers, balls)
 *   3. UILayer — score / player list overlay
 *
 * The Game class owns all game logic. This file just bootstraps PixiJS,
 * handles resize, and polls for code updates (auto-reload on deploy).
 */
import { Application } from "pixi.js";
import RAPIER from "@dimforge/rapier2d-compat";
import { CANVAS_WIDTH, CANVAS_HEIGHT } from "./constants";
import { Game } from "./game/Game";

declare const __BUILD_TIME__: string;

async function main() {
  await RAPIER.init();

  const app = new Application();
  await app.init({
    backgroundColor: 0x050510,
    antialias: true,
    resolution: window.devicePixelRatio || 1,
    autoDensity: true,
  });
  document.body.appendChild(app.canvas);

  // Prevent long-press context menu on mobile
  document.addEventListener("contextmenu", (e) => e.preventDefault());

  const game = new Game(app);

  // Resize handler
  const onResize = () => resizeGame(app, game);

  // Initial resize (deferred to ensure mobile browser has settled layout)
  onResize();
  requestAnimationFrame(onResize);

  // Resize on window change
  window.addEventListener("resize", onResize);

  // Mobile: address bar show/hide doesn't always fire 'resize'
  if (window.visualViewport) {
    window.visualViewport.addEventListener("resize", onResize);
  }

  game.start();

  // Show a banner if server/client protocol versions don't match
  game.onProtocolMismatch = () => {
    const banner = document.createElement("div");
    Object.assign(banner.style, {
      position: "fixed",
      top: "0",
      left: "0",
      right: "0",
      padding: "10px",
      background: "rgba(180, 40, 40, 0.9)",
      color: "#fff",
      fontFamily: "monospace",
      fontSize: "13px",
      textAlign: "center",
      zIndex: "2000",
      cursor: "pointer",
    });
    banner.textContent =
      "Server has been updated. Click here or refresh the page to reload.";
    banner.addEventListener("click", () => location.reload());
    document.body.appendChild(banner);
  };

  // Info icon (bottom-left corner)
  createInfoPanel(game);

  // Bot toggle (next to info icon)
  createBotToggle(game);

  // Poll for new deployments and auto-reload
  startVersionCheck();
}

const VERSION_CHECK_INTERVAL = 60_000; // 60 seconds

function startVersionCheck() {
  if (typeof __BUILD_TIME__ === "undefined") return; // dev mode
  setInterval(async () => {
    try {
      const res = await fetch("/version.json", { cache: "no-store" });
      if (!res.ok) return;
      const data = await res.json();
      if (data.t && data.t !== __BUILD_TIME__) {
        location.reload();
      }
    } catch {
      // Network error, ignore
    }
  }, VERSION_CHECK_INTERVAL);
}

// --- Screen Wake Lock (prevents screensaver while bot is active) ---
let wakeLock: WakeLockSentinel | null = null;

async function acquireWakeLock() {
  if (!("wakeLock" in navigator)) return;
  try {
    wakeLock = await navigator.wakeLock.request("screen");
    wakeLock.addEventListener("release", () => {
      wakeLock = null;
    });
  } catch {
    wakeLock = null;
  }
}

async function releaseWakeLock() {
  try {
    await wakeLock?.release();
  } finally {
    wakeLock = null;
  }
}

function createBotToggle(game: Game) {
  let botOn = false;

  // Re-acquire wake lock when tab becomes visible again
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible" && botOn) {
      void acquireWakeLock();
    }
  });

  const btn = document.createElement("button");
  // Robot icon as inline SVG — no fill, teal stroke matching info icon
  btn.innerHTML =
    '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="rgba(77,166,166,0.7)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
    '<rect x="4" y="8" width="16" height="12" rx="2"/>' +
    '<line x1="12" y1="2" x2="12" y2="8"/>' +
    '<circle cx="12" cy="2" r="1.5"/>' +
    '<circle cx="9" cy="14" r="1.5"/>' +
    '<circle cx="15" cy="14" r="1.5"/>' +
    '<line x1="1" y1="13" x2="4" y2="13"/>' +
    '<line x1="20" y1="13" x2="23" y2="13"/>' +
    "</svg>";
  Object.assign(btn.style, {
    position: "fixed",
    bottom: "12px",
    left: "48px",
    width: "28px",
    height: "28px",
    borderRadius: "50%",
    border: "1px solid rgba(77, 166, 166, 0.4)",
    background: "rgba(5, 5, 16, 0.6)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    cursor: "pointer",
    padding: "0",
    zIndex: "1000",
    transition: "box-shadow 0.2s, border-color 0.2s",
    boxShadow: "none",
  });
  document.body.appendChild(btn);

  const svg = btn.querySelector("svg")!;

  function applyState(on: boolean) {
    btn.style.borderColor = on
      ? "rgba(77, 166, 166, 0.8)"
      : "rgba(77, 166, 166, 0.4)";
    btn.style.boxShadow = on ? "0 0 8px rgba(77, 166, 166, 0.4)" : "none";
    svg.setAttribute(
      "stroke",
      on ? "rgba(77,166,166,1)" : "rgba(77,166,166,0.7)",
    );
  }

  btn.addEventListener("click", (e) => {
    e.stopPropagation();
    botOn = !game.isBotEnabled();
    game.setBotEnabled(botOn);
    applyState(botOn);
    if (botOn) {
      void acquireWakeLock();
      void document.documentElement.requestFullscreen?.();
    } else {
      void releaseWakeLock();
      if (document.fullscreenElement) {
        void document.exitFullscreen();
      }
    }
  });
}

function createInfoPanel(game: Game) {
  const GITHUB_URL = "https://github.com/gunstein/Pinball2DMulti";

  // Icon button
  const icon = document.createElement("button");
  icon.textContent = "\u24d8"; // circled i
  Object.assign(icon.style, {
    position: "fixed",
    bottom: "12px",
    left: "12px",
    width: "28px",
    height: "28px",
    borderRadius: "50%",
    border: "1px solid rgba(77, 166, 166, 0.4)",
    background: "rgba(5, 5, 16, 0.6)",
    color: "rgba(77, 166, 166, 0.7)",
    fontSize: "16px",
    lineHeight: "26px",
    textAlign: "center",
    cursor: "pointer",
    padding: "0",
    zIndex: "1000",
    fontFamily: "monospace",
    transition: "opacity 0.2s",
  });
  document.body.appendChild(icon);

  // Panel
  const panel = document.createElement("div");
  Object.assign(panel.style, {
    position: "fixed",
    bottom: "48px",
    left: "12px",
    background: "rgba(5, 5, 16, 0.92)",
    border: "1px solid rgba(77, 166, 166, 0.3)",
    borderRadius: "6px",
    padding: "10px 14px",
    fontFamily: "monospace",
    fontSize: "11px",
    color: "#8cc",
    lineHeight: "1.6",
    zIndex: "1000",
    display: "none",
    minWidth: "170px",
  });
  document.body.appendChild(panel);

  function formatBuildTime(ts: string): string {
    const ms = parseInt(ts, 10);
    if (isNaN(ms)) return ts;
    const d = new Date(ms);
    return d.toISOString().replace("T", " ").slice(0, 16) + " UTC";
  }

  function makeLine(text: string): HTMLDivElement {
    const div = document.createElement("div");
    div.style.marginBottom = "4px";
    div.textContent = text;
    return div;
  }

  function updatePanel() {
    const buildStr =
      typeof __BUILD_TIME__ !== "undefined"
        ? formatBuildTime(__BUILD_TIME__)
        : "dev";
    const serverVer = game.getServerVersion() || "-";

    panel.replaceChildren();
    panel.appendChild(makeLine(`Client: ${buildStr}`));
    panel.appendChild(makeLine(`Server: v${serverVer}`));

    const link = document.createElement("a");
    link.href = GITHUB_URL;
    link.target = "_blank";
    link.rel = "noopener";
    link.textContent = "GitHub";
    Object.assign(link.style, {
      color: "#4da6a6",
      textDecoration: "underline",
    });
    const linkDiv = document.createElement("div");
    linkDiv.appendChild(link);
    panel.appendChild(linkDiv);
  }

  icon.addEventListener("click", (e) => {
    e.stopPropagation();
    const visible = panel.style.display !== "none";
    if (visible) {
      panel.style.display = "none";
    } else {
      updatePanel();
      panel.style.display = "block";
    }
  });

  // Close panel on outside click
  document.addEventListener("click", () => {
    panel.style.display = "none";
  });
  panel.addEventListener("click", (e) => e.stopPropagation());
}

function resizeGame(app: Application, game: Game) {
  // Use visualViewport on mobile for accurate size (ignores address bar)
  const vv = window.visualViewport;
  const screenW = vv ? vv.width : window.innerWidth;
  const screenH = vv ? vv.height : window.innerHeight;

  // Resize the PixiJS renderer to match
  app.renderer.resize(screenW, screenH);

  // Scale world container to fit, maintaining aspect ratio
  const scale = Math.min(screenW / CANVAS_WIDTH, screenH / CANVAS_HEIGHT);
  const offsetX = (screenW - CANVAS_WIDTH * scale) / 2;
  const offsetY = (screenH - CANVAS_HEIGHT * scale) / 2;

  game.resize(scale, offsetX, offsetY, screenW, screenH);
}

main().catch(console.error);
