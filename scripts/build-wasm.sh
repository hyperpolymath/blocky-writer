#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." >/dev/null 2>&1 && pwd)"
CRATE_DIR="$REPO_ROOT/rust/pdftool_core"

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "wasm-pack not found. Install with: cargo install wasm-pack" >&2
  exit 1
fi

cd "$CRATE_DIR"
wasm-pack build --target web --out-dir pkg
