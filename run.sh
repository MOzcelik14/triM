#!/usr/bin/env bash
# triM. — Application Launcher
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
cd "$DIR"

# Native Rust Edition
if [ -x "$DIR/target/release/trim" ]; then
    exec "$DIR/target/release/trim" "$@"
elif [ -x "$DIR/target/debug/trim" ]; then
    exec "$DIR/target/debug/trim" "$@"
else
    echo "Building and launching native Rust triM..."
    cargo build --release -p trim-app
    exec "$DIR/target/release/trim" "$@"
fi
