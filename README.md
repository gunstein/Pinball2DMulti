# Pinball2DMulti

A "cozy" multiplayer 2D pinball game in the browser. Each player has their own pinball board with a shared deep-space behind it connecting all players.

See [workflow.md](workflow.md) for how documentation and tasks are organized.

See [docs/](docs/) for architecture, goals, and decisions.

## Setup

After cloning, enable the pre-commit hook that auto-formats code:

```bash
git config core.hooksPath .githooks
```

This runs `cargo fmt` on Rust files and `prettier` on TypeScript/JS files whenever you commit.

## Run locally

### Server (Rust)

```bash
cd server
cargo run --release
```

Server listens on `ws://localhost:9001/ws`.

Bot configuration:
```bash
BOT_COUNT=5 cargo run --release   # 5 bots (default 3)
BOT_COUNT=0 cargo run --release   # no bots
```

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

Touch: tap left/right side for flippers, bottom-right for launcher.

## Scripts

### Client (`cd client`)

| Command | Description |
|---------|-------------|
| `npm run dev` | Start dev server |
| `npm run build` | Typecheck + production build |
| `npm test` | Run tests |
| `npm run test:verbose` | Run tests with verbose output |
| `npm run typecheck` | Type-check without building |
| `npm run format` | Prettier |
| `npm run preview` | Preview production build locally |

### Server (`cd server`)

| Command | Description |
|---------|-------------|
| `cargo run --release` | Run server |
| `cargo test` | Run tests |

### Load testing

```bash
cd server
cargo run --release --bin loadtest -- --clients 200 --duration 30
```

## Production deployment

The repo provides Containerfiles for server and client. Deployment depends on your infrastructure.

```
Internet -> Traefik (HTTPS) -> pinball-server (9001, WebSocket at /ws)
                             -> pinball-web (80, Nginx static)
```

Note: the standalone `deploy/compose.yml` maps Traefik to host ports 8080/8443 instead of 80/443. Adjust or use a reverse proxy in front if you need standard ports.

### What this repo provides

- `server/Containerfile` — multi-stage Rust build, produces a minimal server image
- `client/Containerfile` — multi-stage Node build, produces an Nginx static image
- `deploy/compose.yml` — standalone example with its own Traefik (for fresh setups)

### Typical setup: existing Traefik + compose

If you already have a Traefik reverse proxy managing multiple services, add the pinball services to your existing `docker-compose.yml`:

```yaml
  pinball_web:
    container_name: pinball_web
    build:
      context: /path/to/Pinball2DMulti/client
      dockerfile: Containerfile
    image: pinball_web:local
    restart: unless-stopped
    expose: ["80"]
    networks: [web]

  pinball_server:
    container_name: pinball_server
    build:
      context: /path/to/Pinball2DMulti
      dockerfile: server/Containerfile
    image: pinball_server:local
    restart: unless-stopped
    expose: ["9001"]
    networks: [web]
```

Then add Traefik dynamic config to route `/ws` to `pinball_server:9001` and everything else to `pinball_web:80`.

### Updating after code changes

From the server, in your compose directory (not `deploy/`):

```bash
# 1. Pull latest code
cd /path/to/Pinball2DMulti && git pull --rebase

# 2. Rebuild and restart (from your compose directory)
cd /path/to/your/compose
podman-compose build pinball_web pinball_server
podman-compose up -d --force-recreate pinball_web pinball_server
podman image prune -f
```

**Important:** Only rebuild and recreate the pinball services — not your entire stack. Using `--force-recreate` ensures the new images are actually used. `podman image prune -f` removes the large intermediate build images (~1-2 GB each).

### Standalone setup

If you don't have an existing Traefik, use the included `deploy/compose.yml`:

```bash
# 1. Create environment file
cp deploy/.env.example deploy/.env
# Edit deploy/.env with your LE_EMAIL and PINBALL_HOST

# 2. Prepare Let's Encrypt storage
mkdir -p deploy/letsencrypt
touch deploy/letsencrypt/acme.json
chmod 600 deploy/letsencrypt/acme.json

# 3. Enable Podman socket (rootless)
systemctl --user enable --now podman.socket

# 4. Build and start
cd deploy
podman-compose build
podman-compose up -d
```

To update:
```bash
cd /path/to/Pinball2DMulti && git pull --rebase
cd deploy && podman-compose build && podman-compose down && podman-compose up -d && podman image prune -f
```

**Note:** The standalone setup uses `down`/`up` which restarts all services including Traefik. The `.env` file must exist with valid `PINBALL_HOST` and `LE_EMAIL` values — without it, Traefik labels will be empty and routing will fail.

### Useful commands

| Command | Description |
|---------|-------------|
| `podman-compose ps` | Show running containers |
| `podman-compose logs -f pinball-server` | Follow server logs |
| `podman-compose logs -f pinball-web` | Follow web logs |
| `podman-compose build --no-cache <service>` | Full rebuild |
| `podman image prune -f` | Remove dangling build images |

### Troubleshooting

- **WebSocket fails:** Check browser dev tools Network/WS tab, check `podman-compose logs pinball-server`
- **Empty `Host()` in Traefik logs:** `.env` file missing or `PINBALL_HOST` not set (standalone setup only)
- **Old code still running after deploy:** Container not recreated — use `--force-recreate` or `down`/`up`
- **Large dangling images after build:** Run `podman image prune -f` to clean multi-stage build layers
