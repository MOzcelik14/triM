#!/usr/bin/env bash
# Backwards-compatible entry point for the native Rust Debian package.
# The old Python 'Architecture: all' package has been retired.
set -Eeuo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
if [[ ! -x target/release/trim ]]; then
  echo "Building triM. native Rust binary..."
  cargo build --locked --release -p trim-app
fi
exec bash scripts/package-linux.sh deb
