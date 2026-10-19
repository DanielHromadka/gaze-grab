#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
case "${1:-}" in ''|--user) ;; *) echo 'Použití: ./scripts/install-gaze-grab.sh [--user]' >&2; exit 2 ;; esac
./scripts/build-gaze-grab.sh
./scripts/install-bundle.sh "$PWD/dist/Gaze Grab.app" cz.attosoft.gaze-grab "${1:-}"
