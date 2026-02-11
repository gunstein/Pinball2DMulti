# client_bevy

Rust/Bevy client for Pinball2DMulti. Runs as a native desktop app or in the browser via WASM.

## Prerequisites

Start the game server first (in a separate terminal):

```bash
cd server
cargo run --release
```

## Run as native app

```bash
cd client_bevy
cargo run --release
```

Connects to `ws://127.0.0.1:9001/ws` by default. Override with:

```bash
PINBALL_WS_URL=ws://localhost:9001/ws cargo run --release
```

## Run in browser (WASM)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
cd client_bevy
trunk serve --release
```

Open `http://localhost:8080` in the browser. The WebSocket endpoint is auto-derived from the page host. For local trunk dev on port 8080, the client automatically connects to `ws://127.0.0.1:9001/ws`.

Note: `PINBALL_WS_URL` env var is not used in browser builds.

## Project structure

- `src/main.rs` — app bootstrap, plugin assembly, log config
- `src/constants.rs` — board dimensions, physics constants, color palette
- `src/coord.rs` — typed coordinate/velocity boundary conversions (`PxPos`, `WireVel`)
- `src/board/` — board geometry, flipper/launcher state machines
- `src/shared/` — server protocol, websocket connection (`cfg`-split native/WASM), vec3 math
- `src/game/core.rs` — camera, physics config, system-set ordering
- `src/game/walls.rs` — board walls, drain colliders
- `src/game/flippers.rs` — flipper entities, fixed-step movement
- `src/game/launcher.rs` — launcher charge/launch logic
- `src/game/ball.rs` — ball spawning, collision, escape, respawn
- `src/game/pins.rs` — bumper entities, hit timers, glow visuals
- `src/game/deep_space.rs` — deep-space sphere projection, stars, portal/ball dots
- `src/game/network.rs` — websocket event processing, activity heartbeat
- `src/game/input.rs` — keyboard input
- `src/game/hud/` — UI overlay (connection status, player list, hit counter)

## Notes

- Pinned to Bevy `0.17.3` because `bevy_rapier2d 0.32.x` targets that version.
- WASM build uses `data-wasm-opt="0"` in `index.html` to work around an externref table bug in wasm-opt.
- WASM target runs physics at 60 Hz (native: 120 Hz) and disables MSAA for better browser performance.
- Rapier contact tuning mirrors the TypeScript client to reduce stuck-ball cases:
  - lower friction for ball/flipper/walls
  - friction combine rule = `Min`
  - ball sleeping disabled (`Sleeping::disabled()`)
