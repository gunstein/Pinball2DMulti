# Pinball2DMulti

A "cozy" multiplayer 2D pinball game in the browser. Each player has their own pinball board with a shared deep-space behind it connecting all players.

See [workflow.md](workflow.md) for how documentation and tasks are organized.

See [docs/](docs/) for architecture, goals, and decisions.

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
| `npm run format` | Prettier |

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

Uses Podman/Docker + Traefik with automatic HTTPS. Config lives in [deploy/](deploy/).

```
Internet -> Traefik (80/443) -> pinball-server (9001, WebSocket)
                              -> pinball-web (80, Nginx static)
```

### Prerequisites

- Linux server with public IP
- Domain name pointing to your server
- Podman and podman-compose installed
- Ports 80 and 443 open

### Initial setup

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

### Updating

```bash
cd deploy && git pull --rebase && podman-compose build && podman-compose down && podman-compose up -d && podman image prune -f
```

Or use the deploy script: `./deploy/deploy.sh`

### Useful commands

All commands assume you are in `deploy/`.

| Command | Description |
|---------|-------------|
| `podman-compose ps` | Show running containers |
| `podman-compose logs -f` | Follow all logs |
| `podman-compose logs -f pinball-server` | Follow server logs only |
| `podman-compose down` | Stop all services |
| `podman-compose build --no-cache` | Full rebuild |
| `podman image prune` | Remove dangling build images |

### Integration with existing Traefik

If you already have Traefik running with file-based config, add to your existing `docker-compose.yml`:

```yaml
  pinball_web:
    build:
      context: "/path/to/Pinball2DMulti/client"
      dockerfile: "Containerfile"
    image: "pinball_web:local"
    restart: always
    expose: ["80"]

  pinball_server:
    build:
      context: "/path/to/Pinball2DMulti"
      dockerfile: "server/Containerfile"
    image: "pinball_server:local"
    restart: always
    expose: ["9001"]
```

Then create a Traefik dynamic config routing `/ws` to `pinball_server:9001` and everything else to `pinball_web:80`.

### Troubleshooting

- **Certificate not issued:** Check ports 80/443 are open, DNS resolves, check `podman-compose logs traefik`
- **WebSocket fails:** Check browser dev tools Network/WS tab, check `podman-compose logs pinball-server`
- **Podman socket denied:** `systemctl --user status podman.socket`
