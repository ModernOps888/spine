#!/usr/bin/env bash
# SPINE Anti-Sycophancy Reality Gateway & HUD Launcher
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "=========================================================="
echo " 🛡️ Starting SPINE Anti-Sycophancy Reality Gateway & HUD"
echo "=========================================================="

# Build or run backend
if [ ! -f "$DIR/bin/spine" ]; then
    echo "Building SPINE backend..."
    cargo build --release --manifest-path "$DIR/backend/Cargo.toml"
    mkdir -p "$DIR/bin"
    cp "$DIR/backend/target/release/spine-gateway" "$DIR/bin/spine"
fi

# Start Gateway
"$DIR/bin/spine" &
GATEWAY_PID=$!
echo "SPINE Gateway running on :8080 (PID: $GATEWAY_PID)"

# Start Frontend HUD
if [ -d "$DIR/frontend" ]; then
    cd "$DIR/frontend"
    npm run dev -- --port 3333 &
    HUD_PID=$!
    echo "SPINE HUD running on :3333 (PID: $HUD_PID)"
fi

wait
