# Design

Last updated: 2026-02-06

## Reading guide

Suggested order for getting familiar with the codebase. Start with one side (client or server), then the other — they meet at the protocol layer.

### Client (TypeScript)

1. **`docs/design.md`** (this file) — read the architecture overview and escape pipeline first.
2. **`client/src/main.ts`** — entry point. Shows how PixiJS and Rapier are initialized, and how Game is created.
3. **`client/src/game/Game.ts`** — the orchestrator. Follow the `update()` method to see the frame loop: physics step → ball escape check → collision handling → deep-space tick → render.
4. **`client/src/board/BoardGeometry.ts`** — data-driven board layout. All positions, walls, bumpers, and flippers are defined here.
5. **`client/src/board/Flipper.ts`** + **`flipperLogic.ts`** — flipper rendering/physics and the pure state machine (good example of the testable-logic pattern used throughout).
6. **`client/src/board/Ball.ts`**, **`Launcher.ts`**, **`Pin.ts`** — the other board entities. Short files.
7. **`client/src/shared/ServerConnection.ts`** — WebSocket client. Look at the wire types at the top (the protocol contract), then `handleMessage()` for how server messages are dispatched.
8. **`client/src/shared/DeepSpaceClient.ts`** — abstraction over server vs. offline mock mode. Game.ts only talks to this.
9. **`client/src/shared/SphereDeepSpace.ts`** + **`vec3.ts`** — pure sphere math. Balls move on great circles, captured at portals. This is the core "deep space" concept.
10. **`client/src/layers/SphereDeepSpaceLayer.ts`** — 3D-to-2D projection for the star field and deep-space ball visualization.

### Server (Rust)

1. **`server/src/lib.rs`** — module overview and architecture doc comment. Read this first.
2. **`server/src/main.rs`** — Axum setup. Short — just wires routes and spawns the game loop.
3. **`server/src/protocol.rs`** — all message types. Compare with the wire types in `ServerConnection.ts` — they must match.
4. **`server/src/game_loop.rs`** — the 60 Hz tick loop. Follow the `select!` branches: player commands, tick logic, broadcast.
5. **`server/src/state.rs`** — GameState. Owns players, deep-space simulation, and bot manager.
6. **`server/src/deep_space.rs`** — authoritative sphere simulation. `tick()` moves balls and checks captures. This is the server-side equivalent of `SphereDeepSpace.ts`.
7. **`server/src/ws.rs`** — per-client WebSocket handler. Rate limiting, validation, message dispatch.
8. **`server/src/bot.rs`** — bot AI. Each bot has a personality that controls timing and velocity.
9. **`server/src/vec3.rs`**, **`sphere.rs`** — shared math (mirrored on client side).
10. **`server/src/config.rs`** — all tunable parameters with validation.

### Connecting the dots

After reading one side, the key insight is: client and server share the same sphere model but only the server is authoritative. The client interpolates between server snapshots for smooth rendering. The protocol is the contract between the two — `protocol.rs` and the wire types in `ServerConnection.ts` must stay in sync.

## Architecture overview

```
Browser (per player)                    Server (Rust)
+--------------------------+            +---------------------------+
| PixiJS rendering         |            | Axum WebSocket endpoint   |
| Rapier2D local physics   |  WebSocket | Game loop (60 Hz tick)    |
| InputManager             | <--------> | SphereDeepSpace (auth.)   |
| SphereDeepSpaceLayer     |            | BotManager                |
| DeepSpaceClient          |            | Player/portal management  |
+--------------------------+            +---------------------------+
```

- **Client-local physics:** Each player's pinball board runs Rapier2D locally at 120 Hz. No server involvement for flipper/ball physics.
- **Server-authoritative deep-space:** The server owns the sphere simulation (60 Hz tick, 10 Hz broadcast). Clients interpolate between snapshots.

## Escape pipeline

1. Ball exits through escape slot -> `Game.ts` captures snapshot (vx, vy)
2. Client sends `ball_escaped {vx, vy}` to server
3. Server maps 2D velocity to 3D great-circle motion on unit sphere
4. Ball moves along great circle, checked against portals via dot-product
5. Portal hit -> server sends `transfer_in {vx, vy, owner_id, color}` to target player
6. Client spawns ball at board entry point (top center) with capture velocity

## Sphere model

- Unit sphere with 2048 Fibonacci-distributed cells
- One portal per player, allocated via `PortalPlacement`
- Balls move along great circles defined by position + axis + omega
- Capture test: `dot(ball.pos, portal.pos) >= cos(portal_alpha)`
- Minimum capture age: 15s (ball must travel before it can be captured)
- Reroute failsafe: if no hit after 12s, ball is redirected toward a random portal

## Bot system

- 3 bot players by default (configurable via `BOT_COUNT` env)
- Personalities: Eager (fast), Relaxed (slow), Chaotic (unpredictable)
- Bots freeze when no real player has been active for 30 seconds
- Activity tracked via client heartbeat -> server `last_activity` timestamp

## Code structure

### Client (TypeScript)

```
client/src/
  main.ts                         Entry point (PixiJS + Rapier init)
  constants.ts                    Game constants
  game/
    Game.ts                       Orchestrator (120 Hz fixed timestep)
    InputManager.ts               Keyboard + touch input, activity tracking
  board/
    BoardGeometry.ts              Data-driven board definition
    BoardMetrics.ts               Derived input zones
    Board.ts                      Wall/collider management
    Ball.ts                       Rapier body + PixiJS graphics
    Flipper.ts                    Kinematic flipper
    flipperLogic.ts               Pure flipper state machine (testable)
    Launcher.ts                   Launcher visuals
    launcherLogic.ts              Pure launcher state machine (testable)
    Pin.ts                        Bumper with hit glow
  physics/
    PhysicsWorld.ts               Rapier wrapper + unit conversions
  layers/
    BoardLayer.ts                 Local board rendering
    UILayer.ts                    Score, players, connection state
    SphereDeepSpaceLayer.ts       Deep-space visualization
  shared/
    ServerConnection.ts           WebSocket client with reconnect + interpolation
    DeepSpaceClient.ts            Server/mock abstraction
    SphereDeepSpace.ts            Pure sphere simulation (zero-alloc tick)
    sphere.ts                     Fibonacci sphere + portal placement
    vec3.ts                       3D vector math
    MockWorld.ts                  Offline mock mode
    types.ts                      Shared type contracts
```

### Server (Rust)

```
server/src/
  lib.rs                          Library root (re-exports all modules)
  main.rs                         Entry point (Axum on 0.0.0.0:9001)
  game_loop.rs                    60 Hz tick, command handling, broadcast
  state.rs                        GameState (players, balls, bots, activity)
  deep_space.rs                   Sphere simulation (authoritative)
  bot.rs                          Bot AI with personalities
  ws.rs                           WebSocket handler (rate limiting, validation)
  protocol.rs                     JSON message types (camelCase wire format)
  config.rs                       Server + deep-space configuration
  player.rs                       Player struct + color generation
  sphere.rs                       Fibonacci sphere + portal placement
  vec3.rs                         3D vector math
  bin/
    loadtest.rs                   Load testing client
```

## Network protocol

**Server -> Client:** `welcome`, `players_state` (2 Hz), `space_state` (10 Hz), `transfer_in`

**Client -> Server:** `ball_escaped`, `set_paused`, `activity`

Optimization: 4-decimal precision rounding, pre-serialized JSON (`Utf8Bytes`), rate limiting (30 ball_escaped/sec, 10 set_paused/sec, 1 activity/sec).

## Versioning

- **Server version:** Set in `server/Cargo.toml` (`version = "x.y.z"`). Compiled into the binary via `env!("CARGO_PKG_VERSION")` and sent to the client in the `welcome` message.
- **Client build time:** A Unix timestamp baked in at build time by Vite (`__BUILD_TIME__`). Also written to `dist/version.json` so running clients can detect new deploys and auto-reload.
- **Info icon:** The client shows a small info button (bottom-left) displaying server version, client build time, and a link to the GitHub repo.

To bump the server version, edit `server/Cargo.toml` and redeploy.

## Performance

- Client: 120 Hz physics, 60 FPS rendering
- Server: 60 Hz tick, 10 Hz broadcast
- Ball pool + graphics pool to avoid allocation spikes
- Zero-alloc tick loop in SphereDeepSpace
- Tested: 500+ concurrent clients, 1000 connection limit
