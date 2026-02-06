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
