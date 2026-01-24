# Pinball2DMulti

A "cozy" 2D pinball game in the browser with a shared deep-space background connecting all players.

## Concept

Each player has their own local pinball board with transparent background, revealing a shared deep-space behind it. Balls that escape one player's board float through deep-space and can enter another player's board. The game has no end — play as long as you want. Focus is on calm flow, nice colors, and satisfying collisions.

## Tech Stack

- **PixiJS** — 2D rendering
- **Rapier2D** — physics simulation (WASM)
- **Vite** — dev server and bundler
- **Vitest** — testing
- **TypeScript**

## Getting Started

```bash
npm install
npm run dev
```

Open the URL shown in the terminal (usually `http://localhost:5173`).

## Controls

| Key | Action |
|-----|--------|
| Arrow Left | Left flipper |
| Arrow Right | Right flipper |
| Space (hold) | Charge launcher |
| Space (release) | Launch ball |

## Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Start dev server |
| `npm run build` | Type-check and build for production |
| `npm test` | Run tests |
| `npm run format` | Format code with Prettier |

## Project Structure

```
src/
├── main.ts                  Entry point
├── constants.ts             Canvas, physics, color config
├── game/
│   ├── Game.ts              Game loop (fixed timestep + render)
│   └── InputManager.ts      Keyboard input
├── board/
│   ├── BoardGeometry.ts     Data-driven board layout
│   ├── Board.ts             Wall colliders
│   ├── Ball.ts              Dynamic ball
│   ├── Flipper.ts           Kinematic flippers (pivot + offset)
│   ├── flipperLogic.ts      Pure flipper angle logic
│   ├── Launcher.ts          Charge-based launcher
│   ├── launcherLogic.ts     Pure launcher state machine
│   └── Pin.ts               Bumper pins with hit glow
├── physics/
│   └── PhysicsWorld.ts      Rapier2D world wrapper
└── layers/
    ├── DeepSpaceLayer.ts    Starfield background
    ├── BoardLayer.ts        Game world container
    └── UILayer.ts           Hit counter
```

## Architecture

- **Fixed timestep** (120 Hz) for deterministic physics, variable-rate rendering
- **fixedUpdate/render split** — physics and state run at fixed rate, Pixi draws once per frame
- **Kinematic flippers** — body at pivot, collider offset, only rotation needed
- **Data-driven geometry** — board layout defined in BoardGeometry.ts
- **Pure logic modules** — flipperLogic.ts and launcherLogic.ts are testable without Pixi/Rapier

## Multiplayer (planned)

A Rust server will be authoritative for deep-space: linear ball movement at low update rate, ball ownership transfers between clients. The client remains authoritative for its own pinball simulation.
