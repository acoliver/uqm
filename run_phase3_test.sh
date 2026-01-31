#!/bin/bash
# Phase 3 Test Runner - Start game and trigger file/clock operations

cd /Users/acoliver/projects/uqm/sc2

# Set content directory explicitly
export UQM_CONTENT_DIR="/Users/acoliver/projects/uqm/sc2/content"

# Run the game in background
./uqm-debug --content "$UQM_CONTENT_DIR" 2>&1 &
UQM_PID=$!

# Wait a bit for initialization
sleep 3

# The game should be running; kill it after capturing logs
kill -9 $UQM_PID 2>/dev/null || true

# Show the log
echo "=== rust-bridge.log contents ==="
cat rust-bridge.log

echo ""
echo "=== Checking for markers ==="
for marker in "RUST_BRIDGE_PHASE0_OK" "RUST_FILE_EXISTS_CALLED" "RUST_COPY_FILE_CALLED" "RUST_CLOCK_INIT" "RUST_CLOCK_TICK"; do
    if rg -q "$marker" rust-bridge.log; then
        echo "[OK] Found: $marker"
    else
        echo " Missing: $marker"
    fi
done
