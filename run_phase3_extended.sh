#!/bin/bash
# Phase 3 Extended Test - Run game longer to trigger clock operations

cd /Users/acoliver/projects/uqm/sc2

# Clean log
> rust-bridge.log

# Set content directory explicitly
export UQM_CONTENT_DIR="/Users/acoliver/projects/uqm/sc2/content"

# Run the game in background
echo "Starting UQM game (will auto-quit after 10 seconds)..."
./uqm-debug --content "$UQM_CONTENT_DIR" 2>&1 &
UQM_PID=$!

# Wait for game to initialize
sleep 10

# Kill the game
kill -9 $UQM_PID 2>/dev/null || true

# Show the log
echo ""
echo "=== rust-bridge.log contents ==="
cat rust-bridge.log

echo ""
echo "=== Verification Results ==="
rg -n "RUST_BRIDGE_PHASE0_OK|RUST_FILE_EXISTS_CALLED|RUST_COPY_FILE_CALLED|RUST_CLOCK_INIT|RUST_CLOCK_TICK" rust-bridge.log || echo "No markers found"

echo ""
echo "=== Detailed marker check ==="
for marker in "RUST_BRIDGE_PHASE0_OK" "RUST_FILE_EXISTS_CALLED" "RUST_COPY_FILE_CALLED" "RUST_CLOCK_INIT" "RUST_CLOCK_TICK"; do
    count=$(rg -c "$marker" rust-bridge.log 2>/dev/null || echo "0")
    if [ "$count" -gt 0 ]; then
        echo "[OK] Found ($count occurrences): $marker"
    else
        echo "[FAIL] Missing: $marker"
    fi
done

echo ""
echo "=== Symbol verification ==="
echo "File operations symbols:"
nm /Users/acoliver/projects/uqm/sc2/uqm-debug | rg -i "fileExists|copyFile" | head -5

echo ""
echo "Clock operations symbols:"
nm /Users/acoliver/projects/uqm/sc2/uqm-debug | rg -i "InitGameClock|GameClockTick" | head -5

echo ""
echo "=== Call site verification ==="
otool -tV /Users/acoliver/projects/uqm/sc2/uqm-debug | rg "bl.*_fileExists|bl.*_InitGameClock" | head -10
