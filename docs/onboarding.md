# Developer Onboarding

Last updated: 2026-02-11

This document gives a new developer everything they need to understand the project, what has been built, what works, and where to focus next.

## What is Pinball2DMulti?

A multiplayer 2D pinball game in the browser. Each player has their own local pinball board with a transparent background. Behind the board is a shared "deep space" — a unit sphere that connects all players. When a ball escapes through the top slot, it enters deep space and travels along a great circle until another player's portal captures it. Bots keep the game alive when few players are online.

Live at: https://pinball.vatnar.no

## Architecture at a glance

```
Browser (per player)                    Server (Rust)
+--------------------------+            +---------------------------+
| TS client (production)   |            | Axum WebSocket (port 9001)|
|   PixiJS + Rapier2D      |  WebSocket | 60 Hz game loop           |
|   120 Hz local physics   | <--------> | SphereDeepSpace (auth.)   |
|   Deep-space rendering   |            | BotManager (3 bots)       |
+--------------------------+            | Player/portal management  |
                                        +---------------------------+
| Bevy client (in dev)     |      |
|   bevy_rapier2d + lyon   | <----+
|   Native + WASM          |
+--------------------------+
```

Key split: **local physics** (flippers, balls, bumpers) runs entirely on the client at 120 Hz. **Deep space** (ball movement on sphere, captures, rerouting) runs on the server at 60 Hz, broadcast at 10 Hz. Clients interpolate between server snapshots.

### Runtime event flow (input -> physics -> network -> render)

```
Input (keyboard/touch)
  -> Client fixed step (120 Hz):
       flippers/launcher update
       ball physics step (Rapier)
       collision handling (drain/pin/escape sensor)
  -> Escape event? send `ball_escaped(vx, vy)` to server
  -> Server tick (60 Hz):
       deep-space sphere simulation
       capture checks against portals
       emits `transfer_in` to target client
       emits `space_state` snapshots (10 Hz)
  -> Client:
       receives snapshots/events
       interpolates deep-space state
       renders board + deep-space + HUD
```

## Repo structure

```
Pinball2DMulti/
  client/          TypeScript client (production, feature-complete)
  client_bevy/     Rust/Bevy client (in development)
  server/          Rust game server
  shared/          pinball-shared crate (types shared between server + client_bevy)
  deploy/          Standalone Docker Compose example
  docs/            Architecture docs (design.md, goals.md, decisions.md)
```

## Getting started

### Newcomer path (recommended order)

If this is your first time in the repo, follow this exact order and avoid jumping between codebases:

1. Run `server` + `client` (TypeScript) locally and verify gameplay.
2. Read `client/src/main.ts` and `client/src/game/Game.ts` end-to-end.
3. Read `client/src/board/BoardGeometry.ts`, then `Ball.ts`, `Flipper.ts`, `Launcher.ts`.
4. Read `client/src/shared/ServerConnection.ts` and `DeepSpaceClient.ts`.
5. Read `server/src/protocol.rs`, `server/src/game_loop.rs`, and `server/src/deep_space.rs`.
6. Move to `client_bevy/` only after the TS + server flow is clear.

Why this order: it keeps your mental stack small by learning one runtime model first (TS + Pixi + Rapier) before adding Bevy ECS.

### Prerequisites

- Rust (stable, currently 1.89)
- Node.js 20+
- For Bevy WASM: `rustup target add wasm32-unknown-unknown && cargo install trunk`

### Run locally

Terminal 1 — server:
```bash
cd server && cargo run --release
```

Terminal 2 — TS client:
```bash
cd client && npm install && npm run dev
```

Terminal 3 (optional) — Bevy client:
```bash
cd client_bevy && cargo run --release     # native
# or: trunk serve --release               # WASM at localhost:8080
```

### Run tests

```bash
cd client && npm test              # 130 tests, ~600ms
cd server && cargo test            # 89 unit + 15 integration tests
cargo test -p pinball-shared       # 47 tests (also generates TS types)
cargo test -p pinball-client-bevy  # 51 tests
```

### Dev commands (quick reference)

| Goal | Command |
|------|---------|
| Run server | `cd server && cargo run --release` |
| Run TS client | `cd client && npm run dev` |
| Run Bevy native | `cd client_bevy && cargo run --release` |
| Run Bevy WASM | `cd client_bevy && trunk serve --release` |
| TS tests | `cd client && npm test` |
| Server tests | `cd server && cargo test` |
| Bevy tests | `cargo test -p pinball-client-bevy` |
| Shared types/tests | `cargo test -p pinball-shared` |

## The three codebases

### 1. Server (Rust) — stable, production-ready

The server is mature and battle-tested at 500+ concurrent clients.

**Key modules:**
- `game_loop.rs` — Single-threaded 60 Hz loop using `tokio::select!`. All mutable state in one place, no locks.
- `deep_space.rs` — Authoritative sphere simulation. Balls move on great circles (Rodrigues rotation), captured at portals via dot-product test.
- `bot.rs` — 3 bot personalities (Eager/Relaxed/Chaotic). Freeze when no real player active for 30s.
- `ws.rs` — Per-client handler with rate limiting (30 escapes/sec, 10 pauses/sec, 1 activity/sec).
- `state.rs` — GameState hub: players, deep-space, bots, portal placement.

**Performance patterns:**
- Pre-serialized JSON broadcasts (serialize once, O(1) clone via `Utf8Bytes`)
- Zero-alloc hot path (`rotate_normalize_in_place`)
- 4-decimal precision rounding (halves JSON payload size)

### 2. TypeScript client — production, feature-complete

This is the primary client. It works well, looks smooth, and is deployed in production.

**Tech:** PixiJS v8, Rapier2D WASM, Vite, Vitest

**Features (all working):**
- 2D board: flippers (tapered convex hull), launcher (charge + release), bumpers with hit glow
- Deep-space rendering: azimuthal equidistant projection, 150 twinkling stars, comet tails on balls, glow effects
- Multiplayer: WebSocket with exponential backoff reconnect, client-side interpolation
- HUD: connection dot, player list with colors, hit counter, info panel with versions
- Bot mode: screensaver AI (auto-flipper/launcher), wake lock
- Mobile: touch zones (left/right flipper, launcher area), viewport handling
- Auto-reload: polls `/version.json` every 60s, reloads on new deploy
- Ball pooling: zero-alloc rendering with pre-drawn Graphics objects

**Architecture:**
- Three-layer rendering: SphereDeepSpaceLayer (back) -> BoardLayer (middle) -> UILayer (front)
- `DeepSpaceClient` abstracts server vs. offline mock mode
- Pure logic modules (`flipperLogic.ts`, `launcherLogic.ts`, `SphereDeepSpace.ts`) are fully testable
- 130 tests across 11 files

### 3. Bevy client — in development, needs work

A Rust/Bevy alternative client. Runs native (desktop) and in browser (WASM via Trunk). Core gameplay works but lacks polish compared to the TS client.

**Tech:** Bevy 0.17.3, bevy_rapier2d 0.32, bevy_prototype_lyon (vector shapes)

**What works:**
- Board rendering: walls, tapered flippers, bumpers with hit glow, launcher with charge bar
- Ball physics: spawn, launch (with stacked ball boost), drain, escape, respawn
- Deep-space sphere: stereographic projection, 150 twinkling stars, portal/ball dots, self marker
- Network: WebSocket (native via tokio-tungstenite, WASM via web-sys/gloo), reconnect with backoff
- HUD: connection status, player list, hit counter, info panel, bot button (UI only)
- Shared types: imports from pinball-shared crate (same protocol types as server)
- 51 tests

**What's missing or incomplete (priority order):**

| Gap | Impact | Effort | Notes |
|-----|--------|--------|-------|
| Touch input | Blocks mobile use | Medium | TS has left/right/launch zones via `InputManager.ts` |
| Bot AI | No screensaver mode | Low | UI toggle exists, needs `ClientBot` equivalent (~100 lines) |
| Comet tails | Less dynamic deep-space | Medium | TS draws 4-segment trails behind balls |
| Auto-reload (WASM) | Manual refresh after deploy | Low | Poll `/version.json`, compare build time |
| Wake lock | Screen dims during bot | Low | `Navigator.wakeLock` API, only matters once bot works |

**WASM-specific notes:**
- Physics at 60 Hz (vs 120 Hz native) for performance
- MSAA disabled (lyon tessellation is the bottleneck)
- `data-wasm-opt="0"` in Trunk.toml works around externref table bug in wasm-opt 125
- Shape mutation avoidance (manual dirty tracking) to prevent unnecessary re-tessellation

## Shared crate (pinball-shared)

The `shared/` crate contains types used by both server and client_bevy:

- `protocol.rs` — `ServerMsg`, `ClientMsg`, `WelcomeMsg`, `BallWire`, `PlayerWire`, etc.
- `config.rs` — `DeepSpaceConfig` (portal radius, reroute timing, omega ranges)
- `vec3.rs` — 3D vector math (dot, cross, rotate, slerp, tangent basis)

The crate also uses `ts-rs` to auto-generate TypeScript interfaces. Running `cargo test -p pinball-shared` generates `.ts` files into `client/src/shared/generated/`. The TS client imports from there instead of maintaining manual type copies.

**If you change a wire type in `shared/src/protocol.rs`:**
1. Run `cargo test -p pinball-shared` to regenerate TS types
2. Check `client/src/shared/generated/` for updated files
3. Run `cd client && npx tsc --noEmit` to verify TS still compiles

## Key concepts

### Glossary (quick reference)

- **PPM**: Pixels-per-meter scale used at physics boundaries.
- **Fixed timestep**: deterministic simulation step (`120 Hz` for board physics on clients).
- **Escape slot**: top-board sensor that transfers a ball from board to deep space.
- **TransferIn**: server -> client message that injects a captured deep-space ball into a board.
- **Self-owned ball**: local player's ball; this is the one recolored when self player state updates.
- **Capture**: deep-space ball enters a portal cone (`dot >= cos(portal_alpha)`).
- **Reroute**: server-side redirect when a ball has not been captured for a while.
- **Interpolation**: client-side smoothing between deep-space snapshots from server.

### Deep space model

Balls on a unit sphere. Each player has a portal (a point on the sphere). Balls travel along great circles defined by position + rotation axis + angular velocity (omega).

- **Capture:** `dot(ball.pos, portal.pos) >= cos(portal_alpha)` where portal_alpha ~ 8.6 degrees
- **Reroute:** After 12s without capture, ball redirects toward a random portal (smooth slerp over 4s)
- **Min age:** Ball must exist 15s before it can be captured (prevents instant ping-pong)
- **Portal placement:** 2048 Fibonacci-distributed cells on sphere, one per player

### Wire protocol

JSON over WebSocket. Server -> client: `welcome`, `players_state` (2 Hz), `space_state` (10 Hz), `transfer_in`. Client -> server: `ball_escaped`, `set_paused`, `activity`.

All types defined in `shared/src/protocol.rs` with `#[serde(tag = "type")]` for discriminated unions.

### Activity heartbeat

Clients send `activity` when the player uses flippers or launcher. Server tracks last activity per player. Bots freeze when no real player has been active for 30s. This prevents idle server load.

### Coordinate systems

- **TS client:** Origin top-left, Y-down (PixiJS convention). Physics in meters (Rapier), rendering in pixels.
- **Bevy client:** Origin center, Y-up (Bevy convention). `px_to_world()` / `world_to_px_*()` convert.
- **Server:** No visual coordinates. Deep-space uses unit vectors on sphere.
- **Wire protocol:** Uses TS convention (Y-down) for escape/capture velocities. Bevy client converts at the boundary.

### Common pitfalls

- Mixing Y-axis conventions: TS/wire treat positive Y as down; Bevy world treats positive Y as up.
- Mixing pixel units and meter units: in clients, render-space uses pixels; wire velocities are meters/s.
- Confusing fixed-step simulation with render update: gameplay logic should live in fixed-step paths.
- Assuming all balls should recolor on player update: only self-owned board balls should track self color.
- Debugging from the wrong side first: if board physics looks wrong, start client-side; if capture/reroute is wrong, start server-side.

## Test strategy

Tests focus on pure logic and avoid rendering dependencies:

- **TS client:** Vitest with mocked PixiJS/Rapier. Each board entity, physics module, and sphere math has dedicated tests.
- **Bevy client:** Bevy's `App::update()` pattern. Spawn minimal entities, run systems, assert on component values.
- **Server:** Standard `#[test]` with custom configs (short timers for fast tests). Integration tests for bot exchanges and capture flow.
- **Shared:** Round-trip serialization tests for all protocol types.

## Deployment

Production runs on server "bee2" with Traefik v3.3 as reverse proxy.

```bash
# From the server:
cd ~/source/Pinball2DMulti && git pull --rebase
cd ~/reverseproxy
podman-compose build pinball_web pinball_server
podman-compose up -d --force-recreate pinball_web pinball_server
podman image prune -f
```

Domain: `pinball.vatnar.no`. Only the TS client is deployed in production. The Bevy client is not containerized yet.

**Important:** Do NOT use `deploy/compose.yml` on the production server — it creates its own Traefik instance that conflicts with the existing one.

## Where to contribute

### Bevy client improvements (most impactful)

The Bevy client has solid foundations but needs polish to match the TS client experience:

1. **Touch input** — Add pointer/touch event handling with left/right/launch zones. Look at `client/src/game/InputManager.ts` for the zone logic. In Bevy, use `TouchInput` events or `Pointer` events.

2. **Bot AI** — Port `client/src/game/ClientBot.ts` to Bevy. It's ~100 lines of pure logic: detect balls in flipper zone, alternate flippers with cooldown, auto-charge launcher. Wire it into the existing `HudUiState.bot_enabled` flag.

3. **Comet tails** — The TS client draws 4-segment trails behind deep-space balls using Rodrigues rotation backwards along the axis. In Bevy, this could be additional circle entities per ball or a custom mesh.

4. **Auto-reload for WASM** — Poll `/version.json` from JavaScript, reload page if build time changed. Could use `web-sys` to set an interval.

### General improvements

- **Sound effects** — Neither client has audio. Bumper hits, flipper clacks, and launcher spring would add a lot.
- **Visual juice** — Screen shake on bumper hit, particle effects on capture/escape.
- **Containerize Bevy WASM** — Add a Containerfile for the Trunk WASM build so it can be deployed alongside the TS client.

## Reading guide

For a deeper dive into the code, see `docs/design.md` which has a suggested reading order for both the TS client and server source files.
