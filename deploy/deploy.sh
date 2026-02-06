#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
git pull --rebase
podman-compose build
podman-compose down
podman-compose up -d
podman image prune -f
