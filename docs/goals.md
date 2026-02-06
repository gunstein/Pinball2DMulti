# Goals

Last updated: 2026-02-06

## Purpose

Pinball2DMulti is a "cozy" multiplayer 2D pinball game played in the browser. Each player has their own local pinball board with a transparent background. Behind the board is a shared deep-space (a unit sphere) that connects all players. When a ball escapes through the top slot, it enters deep-space and can be captured by any player's portal.

## Scope

- Local pinball board with flippers, launcher, bumpers, score counter
- Shared deep-space connecting all players via sphere model
- Multiplayer via WebSocket with authoritative server
- Bot players so there's always someone to play with
- Production deployment with containers and automatic HTTPS

## Non-goals

- Competitive ranking or matchmaking
- Persistent state or accounts
- Native mobile app (browser-only)

## Milestones (completed)

- Local pinball with Rapier2D + PixiJS physics
- Multiplayer with Rust WebSocket server and authoritative deep-space
- Sphere model with Fibonacci portal placement and great-circle ball movement
- Client-side interpolation for smooth 60fps rendering with 10Hz server updates
- Bot players with personalities (Eager, Relaxed, Chaotic)
- Activity-based bot control (bots freeze when no active players)
- Tested with 500+ concurrent clients (97% delivery rate)
- Container deployment with Traefik + Let's Encrypt

## Tech stack

- **Client:** TypeScript, PixiJS, Rapier2D (WASM), Vite, Vitest
- **Server:** Rust, Tokio, Axum (WebSocket)
- **Deployment:** Podman/Docker, Traefik, Nginx
