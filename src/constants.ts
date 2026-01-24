// Canvas dimensions
export const CANVAS_WIDTH = 400;
export const CANVAS_HEIGHT = 700;

// Physics scale: pixels per meter
export const PPM = 500;

// Gravity (pixels/s², pointing down in screen-space)
export const GRAVITY_X = 0;
export const GRAVITY_Y = 300;

// Board bounds (relative to canvas center)
export const BOARD_HALF_WIDTH = 175;
export const BOARD_HALF_HEIGHT = 320;

// Board center in canvas coordinates
export const BOARD_CENTER_X = CANVAS_WIDTH / 2;
export const BOARD_CENTER_Y = CANVAS_HEIGHT / 2;

// Ball
export const BALL_RADIUS = 10;
export const BALL_RESTITUTION = 0.5;

// Colors
export const COLORS = {
  deepSpaceBg: 0x050510,
  boardBg: 0x0d1117,
  wall: 0x4da6a6,
  flipper: 0x4da6a6,
  pin: 0x4da6a6,
  pinHit: 0x44ff88,
  ball: 0xffffff,
  ballGlow: 0x88ccff,
  launcher: 0x4da6a6,
  trail: 0x4488ff,
  star: 0xffffff,
};
