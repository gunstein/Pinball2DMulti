# client_bevy

Native-first Rust/Bevy client for Pinball2DMulti, structured for future WASM support.

## Goals

- Translate the TypeScript client architecture into Rust modules.
- Match the existing server protocol (`/ws`) and gameplay semantics.
- Keep visuals and board geometry aligned with the original client.

## Run

```bash
cd client_bevy
cargo run --release
```

By default it connects to `ws://127.0.0.1:9001/ws`.

Override with:

```bash
PINBALL_WS_URL=ws://localhost:9001/ws cargo run --release
```

## Current structure

- `src/main.rs`: app bootstrap + explicit feature-plugin assembly (Pinball2D style)
- `src/game/core.rs`: core resources/messages/system-set ordering
- `src/game/walls.rs`: board walls + drain colliders + wall visuals
- `src/game/flippers.rs`: flipper entities, fixed-step movement, flipper visuals
- `src/game/launcher.rs`: launcher charge state, launch logic, charge bar visual
- `src/game/ball.rs`: ball spawning, collision/escape/respawn logic, ball visuals
- `src/game/pins.rs`: bumper entities, hit timers, pin visuals
- `src/game/deep_space.rs`: deep-space projection and overlay visuals
- `src/game/network.rs`: websocket event intake + protocol state + activity heartbeat
- `src/game/hud/`: UI components and systems (spawn/update/interaction split)
- `src/game/input.rs`: keyboard input resource and mapping
- `src/board/`: translated board geometry and state machines (flipper/launcher)
- `src/shared/`: server protocol, websocket connection, deep-space interpolation, vec3 math

## Bevy Version Note

- Latest stable Bevy is `0.18.0`.
- Current `bevy_rapier2d` release line (`0.32.x`) targets Bevy `0.17.3`.
- This client is currently pinned to Bevy `0.17.3` to keep Rapier-based physics working.

## WASM preparation

- Network layer is isolated in `src/shared/connection.rs` with `cfg` boundaries.
- Native websocket implementation is in place (`tokio-tungstenite`).
- WASM websocket transport is now implemented via `web-sys::WebSocket`.
- Browser default WS URL is derived from `window.location` (`ws(s)://<host>/ws`).

## WASM run (next)

Minimal web bootstrap is added in `client_bevy/index.html`.

Recommended flow:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
cd client_bevy
trunk serve
```

Notes:

- In browser builds `PINBALL_WS_URL` env var is not used.
- WebSocket endpoint is auto-derived from the current page host.
