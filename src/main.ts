import { Application } from "pixi.js";
import RAPIER from "@dimforge/rapier2d-compat";
import { CANVAS_WIDTH, CANVAS_HEIGHT } from "./constants";
import { Game } from "./game/Game";

async function main() {
  await RAPIER.init();

  const app = new Application();
  await app.init({
    resizeTo: window,
    backgroundColor: 0x050510,
    antialias: true,
    resolution: window.devicePixelRatio || 1,
    autoDensity: true,
  });
  document.body.appendChild(app.canvas);

  const game = new Game(app);

  // Initial resize
  resizeGame(app, game);

  // Resize on window change
  window.addEventListener("resize", () => resizeGame(app, game));

  game.start();
}

function resizeGame(app: Application, game: Game) {
  const screenW = app.screen.width;
  const screenH = app.screen.height;

  // Scale world container to fit, maintaining aspect ratio
  const scale = Math.min(screenW / CANVAS_WIDTH, screenH / CANVAS_HEIGHT);
  const offsetX = (screenW - CANVAS_WIDTH * scale) / 2;
  const offsetY = (screenH - CANVAS_HEIGHT * scale) / 2;

  game.resize(scale, offsetX, offsetY, screenW, screenH);
}

main().catch(console.error);
