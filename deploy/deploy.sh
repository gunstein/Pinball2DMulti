#!/usr/bin/env bash
set -euo pipefail

SOURCE_DIR=~/source/Pinball2DMulti
PROXY_DIR=~/reverseproxy

SERVICES=(pinball_web pinball_server pinball_bevy_web)

echo "=== Pulling latest code ==="
cd "$SOURCE_DIR"
git pull --rebase

echo "=== Building containers ==="
cd "$PROXY_DIR"
podman-compose build "${SERVICES[@]}"

echo "=== Recreating containers ==="
podman-compose up -d --force-recreate "${SERVICES[@]}"

echo "=== Pruning old images ==="
podman image prune -f

echo "=== Done ==="
podman-compose ps
