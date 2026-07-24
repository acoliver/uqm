#!/usr/bin/env bash
#
# Hard-hang proof: Launch UQM with a very tight wallclock budget.
# The watchdog will fire WallTimeout, the coordinator will finalize,
# and the binary will exit with a non-zero status (terminal=WallTimeout).
#
# If the binary hangs (never exits), the alarm kills it, and we classify
# that as a hard hang (exit code = signal).
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/rust"
OUTPUT="${1:-/tmp/uqm-proof-hard-hang}"

rm -rf "${OUTPUT}"
mkdir -p "${OUTPUT}"

UQM_BIN="${RUST_DIR}/target/release/uqm"

if [ ! -x "${UQM_BIN}" ]; then
    echo "FAIL: binary not found at ${UQM_BIN}"
    exit 2
fi

# Kill any previous instance
pkill -x uqm 2>/dev/null || true
sleep 0.5

# Create the hard-hang script: tight wallclock budget, no finish step
# The watchdog will fire WallTimeout after 3 seconds.
cat > "${OUTPUT}/hard-hang.json" << 'EOF'
{
    "version": 1,
    "name": "hard-hang",
    "budgets": {
        "max_input_ticks": 200,
        "max_presentations": 200,
        "max_wallclock_seconds": 3
    },
    "steps": [
        {"action": "wait_input_ticks", "count": 100},
        {"action": "finish"}
    ]
}
EOF

echo "=== Hard-Hang Proof ==="
echo "Launching UQM with 3s wallclock budget and 100-tick wait..."
echo ""

# Run with a 30s outer alarm (safety net in case the binary truly hangs)
EXIT_CODE=0
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy perl -e 'alarm 30; exec @ARGV' \
    "${UQM_BIN}" \
    --automation-script="${OUTPUT}/hard-hang.json" \
    --automation-output="${OUTPUT}" \
    >"${OUTPUT}/stdout.log" 2>&1 || EXIT_CODE=$?

echo "Binary exit code: ${EXIT_CODE}"
echo ""

# Check if the watchdog fired and the coordinator finalized
if [ -f "${OUTPUT}/teardown-complete.json" ]; then
    echo "Teardown receipt found:"
    cat "${OUTPUT}/teardown-complete.json"
    echo ""
    TERMINAL=$(python3 -c "import json; print(json.load(open('${OUTPUT}/teardown-complete.json'))['terminal'])" 2>/dev/null || echo "unknown")
    STATUS=$(python3 -c "import json; print(json.load(open('${OUTPUT}/teardown-complete.json'))['status'])" 2>/dev/null || echo "unknown")
    echo ""
    echo "Terminal: ${TERMINAL}"
    echo "Status: ${STATUS}"
    
    if [ "${TERMINAL}" = "WallTimeout" ] && [ "${STATUS}" = "1" ]; then
        echo ""
        echo "PASS: Watchdog fired WallTimeout, binary exited cooperatively"
        echo "Trace records: $(wc -l < "${OUTPUT}/trace.jsonl" 2>/dev/null || echo 0)"
        exit 0
    elif [ "${TERMINAL}" = "Success" ]; then
        echo ""
        echo "PASS: Script completed before timeout (cooperative exit)"
        exit 0
    else
        echo ""
        echo "UNEXPECTED terminal: ${TERMINAL}"
        exit 1
    fi
else
    echo "FAIL: No teardown receipt — binary was killed by outer alarm (hard hang)"
    echo "stdout tail:"
    tail -5 "${OUTPUT}/stdout.log" 2>/dev/null || true
    exit 1
fi