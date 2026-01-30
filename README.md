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
cp .env.example .env
```

Edit `.env` with your values:

```bash
# Podman socket path
# For rootful podman:
PODMAN_SOCKET=/run/podman/podman.sock
# For rootless podman (replace 1000 with your UID):
# PODMAN_SOCKET=/run/user/1000/podman/podman.sock

# Your email for Let's Encrypt certificate notifications
LE_EMAIL=your-email@example.com

# Your domain name
PINBALL_HOST=cozypinball.yourdomain.com
```

3. **Prepare Let's Encrypt storage:**

```bash
mkdir -p letsencrypt
touch letsencrypt/acme.json
chmod 600 letsencrypt/acme.json
```

4. **Enable and start Podman socket** (if using rootful):

```bash
sudo systemctl enable --now podman.socket
```

For rootless:

```bash
systemctl --user enable --now podman.socket
```

5. **Build and start the services:**

```bash
podman-compose build
podman-compose up -d
```

6. **Verify everything is running:**

```bash
podman-compose ps
podman-compose logs -f
```

Visit `https://cozypinball.yourdomain.com` - it should load the game and connect to the WebSocket server automatically.

### Updating the deployment

When you want to deploy new changes:

```bash
cd pinball
./deploy.sh
```

Or manually:

```bash
git pull --rebase
podman-compose build
podman-compose up -d
```

### Useful commands

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
- Check Traefik logs: `podman-compose logs traefik`

**WebSocket connection fails:**
- Open browser dev tools → Network → WS tab
- Ensure the client is connecting to `wss://yourdomain.com/ws`
- Check server logs: `podman-compose logs pinball-server`

**Podman socket permission denied:**
- Verify socket path in `.env` matches your setup
- For rootless: ensure `podman.socket` is running: `systemctl --user status podman.socket`
- Check socket exists: `ls -la /run/podman/podman.sock` or `ls -la /run/user/$(id -u)/podman/podman.sock`

**Container build fails:**
- Ensure you have enough disk space
- Try with `--no-cache`: `podman-compose build --no-cache`

### Local development with production-like setup

To test the container setup locally:

```bash
# Set PINBALL_HOST to localhost in .env
PINBALL_HOST=localhost

# Comment out the HTTPS redirect in compose.yml or use HTTP only
podman-compose up -d
```

Then access `http://localhost` (note: WebSocket will use `ws://` not `wss://`).

### Security notes

- The `.env` file contains sensitive data and is excluded from git
- Let's Encrypt certificates are stored in `letsencrypt/` (also excluded from git)
- The server rate-limits clients to prevent abuse
- Consider adding firewall rules to only allow 80/443 from the internet
