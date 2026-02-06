# Decisions

Last updated: 2026-02-06

## Client-local physics, server-authoritative deep-space

Local physics avoids network latency for flipper/ball interaction. Only deep-space (the shared sphere) needs server authority since it connects all players.

## Pure logic modules separate from rendering

`flipperLogic.ts`, `launcherLogic.ts`, `SphereDeepSpace.ts`, `vec3.ts` have no PixiJS or Rapier dependencies. This makes them testable with simple unit tests.

## Config from server

Client receives `DeepSpaceConfig` in the welcome message. This ensures server and client always agree on physics parameters (portal radius, reroute timing, etc.).

## Single source of truth for board geometry

Board shape defined once in `BoardGeometry.ts`. Input zones in `BoardMetrics.ts` are derived from it automatically.

## Fibonacci sphere for portal placement

2048 Fibonacci-distributed points on a unit sphere give even coverage. Players are assigned cells via `PortalPlacement.allocate()`.

## Bots as first-class players

Bots use the same player infrastructure as real players. They just have an AI tick function instead of WebSocket input.

## Activity-based bot control

Bots freeze (no ball production, no timer countdown) when no real player has been active within 30 seconds. Activity is tracked via client heartbeat messages sent when the player uses flippers or launcher.

## Pre-serialized JSON broadcasts

Server serializes state updates once to `Utf8Bytes`, then clones O(1) to all subscribers. This avoids per-client serialization overhead at 500+ clients.

## 4-decimal precision for wire format

Rounding to 4 decimals reduces JSON payload size by ~50% with no visible quality loss for unit vectors and angular velocities.
