# Pinball2DMulti

A "cozy" 2D pinball in the browser with a transparent board and a shared "deep-space" behind it. When a ball escapes through the escape slot, it continues in deep-space and can return to your board (or enter another player's board in multiplayer).

## Status

- Local pinball works: Rapier2D + PixiJS (flippers, launcher, bumpers, score counter).
- **Multiplayer works:** Rust WebSocket server with authoritative deep-space.
  - Sphere model: balls move along great circles on a unit sphere
  - Portal hit via dot-product test, reroute failsafe for "cozy return"
  - Players get unique colors and portals via Fibonacci sphere placement
  - Client-side interpolation for smooth 60fps rendering with 10Hz server updates
- **Bot players:** Server spawns AI bots so there's always someone to play with
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

#### Bot configuration

The server spawns 3 bot players by default. Configure with environment variable:

```bash
# Run with 5 bots
BOT_COUNT=5 cargo run --release

# Run without bots
BOT_COUNT=0 cargo run --release
```

Bots have different personalities:
- **Eager** - Returns balls quickly (0.3-0.8s delay)
- **Relaxed** - Takes their time (1.5-4.0s delay)
- **Chaotic** - Unpredictable timing and direction

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
cd client
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

### Client (run in `client/`)

| Command | Description |
|---------|-------------|
| `npm run dev` | Start dev server |
| `npm run build` | Typecheck + production build |
| `npm run preview` | Preview production build |
| `npm test` | Run tests (Vitest) |
| `npm run test:quiet` | Run tests with minimal console output |
| `npm run test:verbose` | Run tests with verbose output (includes console logs) |
| `npm run format` | Prettier |

### Server

| Command | Description |
|---------|-------------|
| `cargo run` | Run server (debug) |
| `cargo run --release` | Run server (release, recommended) |
| `cargo build --release` | Build release binary |
| `cargo test` | Run server tests |
| `cargo test -- --nocapture` | Run server tests with logs |

## Code structure

### Client (TypeScript)

```
client/
├── src/
│   ├── main.ts
│   ├── constants.ts
│   ├── game/
│   │   ├── Game.ts              Fixed timestep + render + game orchestration
│   │   └── InputManager.ts      Keyboard/touch input
│   ├── board/
│   │   ├── BoardGeometry.ts     Data-driven board (segments/definitions)
│   │   ├── BoardMetrics.ts      Input zones derived from geometry
│   │   ├── Board.ts             Walls/colliders
│   │   ├── Ball.ts              Rapier-ball + Pixi graphics
│   │   ├── Flipper.ts           Kinematic flipper (pivot + collider offset)
│   │   ├── flipperLogic.ts      Pure flipper logic (testable)
│   │   ├── Launcher.ts          Launcher visuals
│   │   ├── launcherLogic.ts     Pure launcher state machine (testable)
│   │   └── Pin.ts               Bumper/pin with hit glow
│   ├── physics/
│   │   └── PhysicsWorld.ts      Rapier wrapper + unit conversions
│   ├── layers/
│   │   ├── BoardLayer.ts
│   │   ├── UILayer.ts
│   │   └── SphereDeepSpaceLayer.ts  Deep-space "neighborhood disk" view
│   └── shared/
│       ├── ServerConnection.ts  WebSocket client with reconnect
│       ├── SphereDeepSpace.ts   Sphere simulation (pure logic)
│       ├── sphere.ts            Fibonacci sphere + PortalPlacement
│       ├── vec3.ts              3D vector math
│       ├── MockWorld.ts         Mock player list/portals
│       └── types.ts             Shared contract (Player, SpaceBall3D, config)
├── tests/
├── index.html
├── package.json
└── vite.config.ts
```

### Server (Rust)

```
server/src/
├── main.rs              Entry point + Axum setup
├── ws.rs                WebSocket handler per client
├── game_loop.rs         Tick loop + broadcast
├── state.rs             GameState wrapper
├── bot.rs               Bot players (AI that plays automatically)
├── deep_space.rs        SphereDeepSpace (pure logic)
├── sphere.rs            Fibonacci sphere + PortalPlacement
├── vec3.rs              3D vector math
├── protocol.rs          JSON message types
└── config.rs            Server + deep-space config
```

## Onboarding guide

### Quick overview (30 seconds)

Each player has their own local pinball board (input + physics runs locally). Behind the board is a shared **deep-space** (a sphere) that connects all players. When a ball escapes through the top slot, it enters deep-space and can be captured by any player's portal.

### Understanding the flow

1. **Game.ts** - Start here. Orchestrates everything: setup, game loop, input → physics → escape/capture.
2. **SphereDeepSpace.ts** - Pure simulation logic for balls moving on the sphere.
3. **ServerConnection.ts** - WebSocket client with reconnect and client-side interpolation.
4. **PhysicsWorld.ts** - Thin Rapier wrapper.

### Where to find things

| Task | File(s) |
|------|---------|
| Change board geometry | `client/src/board/BoardGeometry.ts` |
| Change touch/input zones | `client/src/board/BoardMetrics.ts` (auto-derived from geometry) |
| Ball spawn/escape/capture | `client/src/game/Game.ts` |
| Deep-space simulation | `client/src/shared/SphereDeepSpace.ts` |
| Server protocol | `client/src/shared/ServerConnection.ts` + `server/src/protocol.rs` |
| Reconnect/network | `client/src/shared/ServerConnection.ts` |
| Rendering layers | `client/src/layers/*.ts` |

### Key design decisions

- **Pure logic is testable**: `flipperLogic.ts`, `launcherLogic.ts`, `SphereDeepSpace.ts` have no Pixi/Rapier dependencies.
- **Config from server**: Client receives `DeepSpaceConfig` in welcome message, ensuring server/client consistency.
- **Single source of truth**: Board geometry defined once in `BoardGeometry.ts`, derived values in `BoardMetrics.ts`.

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

Client pure logic is tested with Vitest (flipper/launcher/vec3/sphere/deep-space) in `client/tests`. Rapier integration is tested in `client/tests/physics.test.ts`.

## Server architecture

- **Authoritative deep-space:** Server simulates all ball movement on the sphere
- **Client-authoritative pinball:** Rapier physics runs locally on each client
- **Protocol versioning:** Client and server check protocol version on connect
- **Reconnect with backoff:** Automatic reconnect on disconnect (1s initial, up to 30s max)
- **Rate limiting:** Max 30 ball_escaped messages per second per client
- **Broadcast optimization:** Pre-serialized JSON, 10Hz updates, 4 decimal precision

## Production deployment

The application can be deployed using containers with Podman/Docker and Traefik as reverse proxy with automatic HTTPS.

### Architecture overview

```
Internet
    │
    ▼
┌─────────────────────────────────────────────┐
│  Traefik (reverse proxy)                    │
│  - Port 80/443                              │
│  - Let's Encrypt certificates               │
│  - Routes /ws to pinball-server             │
│  - Routes everything else to pinball-web    │
└─────────────────────────────────────────────┘
    │                           │
    ▼                           ▼
┌─────────────────┐    ┌─────────────────┐
│ pinball-server  │    │ pinball-web     │
│ (Rust, port     │    │ (nginx, port 80)│
│  9001 internal) │    │ serves static   │
│ WebSocket /ws   │    │ files from Vite │
└─────────────────┘    └─────────────────┘
```

### Prerequisites

- Linux server with public IP
- Domain name pointing to your server (e.g., `cozypinball.yourdomain.com`)
- Podman and podman-compose installed
- Ports 80 and 443 open/forwarded

### Initial setup

1. **Clone the repository on your server:**

```bash
git clone https://github.com/youruser/Pinball2DMulti.git pinball
cd pinball
```

2. **Create environment file:**

```bash
cp deploy/.env.example deploy/.env
```

Edit `deploy/.env` with your values:

```bash
# Your email for Let's Encrypt certificate notifications
LE_EMAIL=your-email@example.com

# Your domain name
PINBALL_HOST=cozypinball.yourdomain.com
```

3. **Prepare Let's Encrypt storage:**

```bash
mkdir -p deploy/letsencrypt
touch deploy/letsencrypt/acme.json
chmod 600 deploy/letsencrypt/acme.json
```

4. **Enable and start Podman socket (rootless):**

```bash
systemctl --user enable --now podman.socket
```

5. **Build and start the services:**

```bash
cd deploy
podman-compose build
podman-compose up -d
```

6. **Verify everything is running:**

```bash
cd deploy
podman-compose ps
podman-compose logs -f
```

Visit `https://cozypinball.yourdomain.com` - it should load the game and connect to the WebSocket server automatically.

### Updating the deployment

When you want to deploy new changes:

```bash
cd pinball
./deploy/deploy.sh
```

Or manually:

```bash
git pull --rebase
cd deploy
podman-compose build
podman-compose up -d
```

### Useful commands

All `podman-compose` commands below assume you are in `deploy/` (where `compose.yml` lives).

| Command | Description |
|---------|-------------|
| `podman-compose ps` | Show running containers |
| `podman-compose logs -f` | Follow all logs |
| `podman-compose logs -f pinball-server` | Follow server logs only |
| `podman-compose down` | Stop all services |
| `podman-compose up -d` | Start all services |
| `podman-compose build --no-cache` | Full rebuild (if caching issues) |

### Troubleshooting

**Certificate not issued:**
- Ensure ports 80 and 443 are accessible from the internet
- Check that DNS is pointing to your server: `dig +short cozypinball.yourdomain.com`
- Check Traefik logs: `podman-compose logs traefik` (from `deploy/`)

**WebSocket connection fails:**
- Open browser dev tools → Network → WS tab
- Ensure the client is connecting to `wss://yourdomain.com/ws`
- Check server logs: `podman-compose logs pinball-server` (from `deploy/`)

**Podman socket permission denied:**
- Ensure `podman.socket` is running: `systemctl --user status podman.socket`
- Check socket exists: `ls -la /run/user/$(id -u)/podman/podman.sock`

**Container build fails:**
- Ensure you have enough disk space
- Try with `--no-cache`: `podman-compose build --no-cache`

### Integration with existing Traefik setup

If you already have Traefik running with file-based configuration (no Docker/Podman provider), you can add pinball to your existing setup.

**1. Add services to your existing docker-compose.yml:**

```yaml
  pinball_web:
    container_name: "pinball_web"
    build:
      context: "/home/youruser/source/Pinball2DMulti/client"
      dockerfile: "Containerfile"
    image: "pinball_web:local"
    restart: always
    expose:
      - "80"

  pinball_server:
    container_name: "pinball_server"
    build:
      context: "/home/youruser/source/Pinball2DMulti"
      dockerfile: "server/Containerfile"
    image: "pinball_server:local"
    restart: always
    expose:
      - "9001"
```

> **Important:** The `context` for `pinball_web` must point to the `client/` subdirectory
> (where `package.json` lives), not the repo root. The `context` for `pinball_server`
> must point to the repo root (the server Containerfile needs access to `server/` and `shared/`).

**2. Create Traefik dynamic config `traefik-config/pinball.yml`:**

```yaml
http:
  routers:
    pinball-ws:
      rule: "Host(`pinball.yourdomain.com`) && PathPrefix(`/ws`)"
      entryPoints:
        - websecure
      tls:
        certResolver: myresolver
      service: pinball-ws-svc
      priority: 100

    pinball-web:
      rule: "Host(`pinball.yourdomain.com`)"
      entryPoints:
        - websecure
      tls:
        certResolver: myresolver
      service: pinball-web-svc
      priority: 1

  services:
    pinball-web-svc:
      loadBalancer:
        servers:
          - url: "http://pinball_web:80"

    pinball-ws-svc:
      loadBalancer:
        servers:
          - url: "http://pinball_server:9001"
```

**3. Build and deploy:**

```bash
# Pull latest code
cd ~/source/Pinball2DMulti && git pull

# Rebuild and restart containers
cd ~/reverseproxy && \
podman-compose down && \
podman rm -f pinball_web pinball_server 2>/dev/null || true && \
podman-compose build --no-cache pinball_web pinball_server && \
podman-compose up -d && \
podman ps --format "table {{.Names}}\t{{.Image}}\t{{.Status}}"
```

> **Note:** `--no-cache` ensures fresh builds. Without it, Podman may reuse
> cached layers and serve stale code. Always stop and remove containers before
> rebuilding to avoid "name already in use" errors.

Traefik will auto-reload the dynamic config if you have `--providers.file.watch=true`.

### Local development with production-like setup

To test the container setup locally:

```bash
# Set PINBALL_HOST to localhost in deploy/.env
PINBALL_HOST=localhost

# Comment out the HTTPS redirect in deploy/compose.yml or use HTTP only
cd deploy
podman-compose up -d
```

Then access `http://localhost` (note: WebSocket will use `ws://` not `wss://`).

### Security notes

- The `.env` file contains sensitive data and is excluded from git
- Let's Encrypt certificates are stored in `deploy/letsencrypt/` (also excluded from git)
- The server rate-limits clients to prevent abuse
- Consider adding firewall rules to only allow 80/443 from the internet
