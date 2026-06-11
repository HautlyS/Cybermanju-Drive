#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "=== Cybermanju Drive — Development Mode ==="
echo "Starting Vite dev server + Tauri..."
echo ""

npm run tauri:dev