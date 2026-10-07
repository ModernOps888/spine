#!/usr/bin/env bash
# SPINE Anti-Sycophancy Reality Gateway & HUD Stopper
echo "Stopping SPINE Gateway and HUD..."
pkill -f "spine" || true
pkill -f "vite.*3333" || true
echo "SPINE processes stopped."
