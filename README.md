# Pinball2DMulti

A "cozy" 2D pinball in the browser with a transparent board and a shared "deep-space" behind it. When a ball escapes through the escape slot, it continues in deep-space and can return to your board (or enter another player's board in multiplayer).

## Status

- Local pinball works: Rapier2D + PixiJS (flippers, launcher, bumpers, score counter).
- **Multiplayer works:** Rust WebSocket server with authoritative deep-space.
  - Sphere model: balls move along great circles on a unit sphere
  - Portal hit via dot-product test, reroute failsafe for "cozy return"
  - Players get unique colors and portals via Fibonacci sphere placement
  - Client-side interpolation for smooth 60fps rendering with 10Hz server updates
- Tested with 500+ concurrent clients (97% delivery rate in release build).

## Tech stack

- **PixiJS** - rendering
- **Rapier2D** - WASM physics
- **TypeScript** + **Vite** - client
- **Rust** + **Tokio** + **Axum** - server
- **Vitest** - testing

## Run locally

### Server (Rust)

```bash
cd server
cargo run --release
```

Server listens on `ws://localhost:9001/ws`.

### Load testing

```bash
cd server
cargo run --release --bin loadtest -- --clients 200 --duration 30
```

Options:
- `--clients N` - Number of concurrent clients (default: 100)
- `--duration S` - Test duration in seconds (default: 30)
- `--escape-rate R` - Ball escapes per second per client (default: 0.5)
- `--url URL` - Server URL (default: ws://127.0.0.1:9001/ws)

### Client (TypeScript)

```bash
npm install
npm run dev
```

Open the URL shown by Vite (typically `http://localhost:5173`).

## Controls

| Key | Action |
|-----|--------|
| Left arrow | Left flipper |
| Right arrow | Right flipper |
| Space (hold) | Charge launcher |
| Space (release) | Launch ball |

## Scripts

### Client

| Command | Description |
|---------|-------------|
| `npm run dev` | Start dev server |
| `npm run build` | Typecheck + production build |
| `npm run preview` | Preview production build |
| `npm test` | Run tests (Vitest) |
| `npm run format` | Prettier |

### Server

| Command | Description |
|---------|-------------|
| `cargo run` | Run server (debug) |
| `cargo run --release` | Run server (release, recommended) |
| `cargo build --release` | Build release binary |
| `cargo test` | Run server tests |

## Code structure

### Client (TypeScript)

```
src/
├── main.ts
├── constants.ts
├── game/
│   ├── Game.ts              Fixed timestep + render
│   └── InputManager.ts      Keyboard input
├── board/
│   ├── BoardGeometry.ts     Data-driven brett (segmenter/definisjoner)
│   ├── Board.ts             Vegger/colliders
│   ├── Ball.ts              Rapier-ball + Pixi graphics
│   ├── Flipper.ts           Kinematisk flipper (pivot + collider offset)
│   ├── flipperLogic.ts      Ren flipperlogikk (testbar)
│   ├── Launcher.ts          Launcher visuals
│   ├── launcherLogic.ts     Ren launcher state machine (testbar)
│   └── Pin.ts               Bumper/pin med hit glow
├── physics/
│   └── PhysicsWorld.ts      Rapier wrapper + unit conversions
├── layers/
│   ├── BoardLayer.ts
│   ├── UILayer.ts
│   └── SphereDeepSpaceLayer.ts  Deep-space "neighborhood disk" view
└── shared/
    ├── ServerConnection.ts  WebSocket client med reconnect
    ├── SphereDeepSpace.ts   Sfaere-sim (ren logikk, brukes i mock mode)
    ├── sphere.ts            Fibonacci sphere + PortalPlacement
    ├── vec3.ts              3D vektor-matte
    ├── MockWorld.ts         Mock spillerliste/portaler
    └── types.ts             Delt kontrakt (Player, SpaceBall3D, config)
```

### Server (Rust)

```
server/src/
├── main.rs              Entry point + Axum setup
├── ws.rs                WebSocket handler per client
├── game_loop.rs         Tick loop + broadcast
├── state.rs             GameState wrapper
├── deep_space.rs        SphereDeepSpace (pure logic)
├── sphere.rs            Fibonacci sphere + PortalPlacement
├── vec3.rs              3D vector math
├── protocol.rs          JSON message types
└── config.rs            Server + deep-space config
```

## Architecture

- Pinball simulation is client-local and runs with fixed timestep (120 Hz).
- Deep-space is a pure logic module (`SphereDeepSpace`) without Pixi/Rapier dependencies.
- **Escape pipeline:**
  1. Ball escapes through escape slot -> snapshot (vx/vy)
  2. Snapshot mapped into deep-space (sphere)
  3. When ball "hits portal" -> capture event
  4. Capture by you -> ball injected back onto your board
- **Performance:** Pooled Graphics objects for deep-space rendering (no per-frame clear/redraw). Ball pool to avoid Rapier/Pixi allocation spikes. Zero-alloc tick loop in `SphereDeepSpace`.

## Testing

Pure logic is tested with Vitest (flipper/launcher/vec3/sphere/deep-space). Rapier integration tested in `tests/physics.test.ts`.

## Server architecture

- **Authoritative deep-space:** Server simulates all ball movement on the sphere
- **Client-authoritative pinball:** Rapier physics runs locally on each client
- **Protocol versioning:** Client and server check protocol version on connect
- **Reconnect with backoff:** Automatic reconnect on disconnect (500ms -> 5s)
- **Rate limiting:** Max 30 ball_escaped messages per second per client
- **Broadcast optimization:** Pre-serialized JSON, 10Hz updates, 4 decimal precision
