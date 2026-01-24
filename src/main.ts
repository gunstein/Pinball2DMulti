import { Application } from 'pixi.js';
import RAPIER from '@dimforge/rapier2d-compat';
import { CANVAS_WIDTH, CANVAS_HEIGHT } from './constants';
import { Game } from './game/Game';

async function main() {
  await RAPIER.init();

  const app = new Application();
  await app.init({
    width: CANVAS_WIDTH,
    height: CANVAS_HEIGHT,
    backgroundColor: 0x050510,
    antialias: true,
    resolution: window.devicePixelRatio || 1,
    autoDensity: true,
  });
  document.body.appendChild(app.canvas);

  const game = new Game(app);
  game.start();
}

main().catch(console.error);
