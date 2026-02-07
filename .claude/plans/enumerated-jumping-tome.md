# Plan: Info icon with version info and GitHub link

## Overview
Add a discreet info icon (bottom-left corner of the PixiJS canvas) that opens a small overlay showing client build time, server version, and a GitHub link.

## Approach: DOM overlay (not PixiJS)
The info panel contains text, a clickable link, and needs click handling — all things that are simpler and more accessible in DOM/CSS than PixiJS. The icon itself will be a small DOM element positioned over the canvas.

## Changes

### 1. Server: add `server_version` to WelcomeMsg
**File:** `server/src/protocol.rs`

Add field to `WelcomeMsg`:
```rust
pub server_version: String,
```

**File:** `server/src/game_loop.rs`

Set the field when constructing WelcomeMsg using `env!("CARGO_PKG_VERSION")` (resolved at compile time from Cargo.toml).

**File:** `server/tests/integration.rs`

Update any test that constructs `WelcomeMsg` directly to include the new field.

### 2. Client: receive and store server version
**File:** `client/src/shared/ServerConnection.ts`

- Add `serverVersion: string` to `WelcomeMsg` interface
- Store it on the `ServerConnection` instance
- Expose via getter `getServerVersion(): string`
- Pass through `onWelcome` callback (add parameter)

### 3. Client: create info icon + panel in DOM
**File:** `client/src/main.ts`

After creating the Game instance, create the info UI:
- A small `ⓘ` button (fixed position, bottom-left, semi-transparent, ~24x24px)
- A hidden info panel that toggles on click
- Panel content:
  - **Client:** formatted build time (already have `__BUILD_TIME__`)
  - **Server:** version from welcome message (e.g. "v0.1.0")
  - **GitHub:** link to `https://github.com/gunstein/Pinball2DMulti`
- Click outside panel or click icon again → close
- Style: dark semi-transparent background matching the game aesthetic (`#050510` bg, `#4da6a6` accent matching `UI_COLOR`)

All DOM elements created programmatically in JS (no changes to `index.html`). Styles set inline to keep it self-contained.

### 4. Wire up: Game passes server version to main.ts
**File:** `client/src/game/Game.ts`

Expose `getServerVersion()` that delegates to `ServerConnection.getServerVersion()`.

Or simpler: `main.ts` holds a reference to a version display updater, and Game calls it when welcome arrives.

**Chosen approach:** Game emits the server version via a callback that main.ts uses to update the panel text. This keeps DOM out of Game.ts.

## Files modified
1. `server/src/protocol.rs` — add `server_version` to WelcomeMsg
2. `server/src/game_loop.rs` — set server_version from `env!("CARGO_PKG_VERSION")`
3. `server/tests/integration.rs` — update WelcomeMsg constructions in tests
4. `client/src/shared/ServerConnection.ts` — parse + store + expose server version
5. `client/src/game/Game.ts` — expose callback/getter for server version
6. `client/src/main.ts` — create DOM info icon + panel, wire up version data

## Visual design
- Icon: small `ⓘ` character, 24x24px, bottom-left corner, semi-transparent white
- Panel: dark rounded box (~200px wide), appears above the icon on click
- Monospace font matching the game UI
- Subtle opacity/transition

## Verification
- `cd server && cargo test` — all tests pass with new WelcomeMsg field
- `cd client && npx vitest run` — client tests pass
- Visual: icon visible in bottom-left, click opens panel with correct info, link works, click again closes
