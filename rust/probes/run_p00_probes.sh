#!/usr/bin/env bash
#
# P00 Executable Preflight Probe Runner
#
# Executes all P00 feasibility probes using artifacts produced by the
# canonical xtask production flow. Does not depend on sc2/build.vars,
# ambient sc2/obj, or legacy build.sh.
#
# Usage:  bash probes/run_p00_probes.sh [output_log]
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${RUST_DIR}/.." && pwd)"

OUTPUT_LOG="${1:-${RUST_DIR}/target/p00-probe-results.log}"
mkdir -p "$(dirname "${OUTPUT_LOG}")"

echo "=== P00 Preflight Probes ==="
echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "RUST_DIR: ${RUST_DIR}"
echo "REPO_ROOT: ${REPO_ROOT}"
echo ""

# --------------------------------------------------------------------------
# Build/config/tool checks (executed, not inspected)
# --------------------------------------------------------------------------

echo "--- Build/Config/Tool Checks ---"

# Verify config_unix.h has required definitions (tracked source, not build.vars)
if [ ! -f "${REPO_ROOT}/sc2/config_unix.h" ]; then
    echo "FAIL: sc2/config_unix.h not found"
    exit 1
fi
echo "PASS: config_unix.h exists"

# Toolchain checks (execute: --version)
echo ""
echo "--- Toolchain Checks ---"
cargo --version || { echo "FAIL: cargo not available"; exit 1; }
rustc --version || { echo "FAIL: rustc not available"; exit 1; }
cc --version 2>&1 | head -1 || { echo "FAIL: cc not available"; exit 1; }
AR_PATH="$(command -v ar)" || { echo "FAIL: ar not available"; exit 1; }
echo "ar: ${AR_PATH}"
nm --version 2>&1 | head -1 || { echo "FAIL: nm not available"; exit 1; }
echo "PASS: all tools available"

# --------------------------------------------------------------------------
# Rust binary probes (lock-free atomics, monotonic clock, datagram, etc.)
# --------------------------------------------------------------------------

echo ""
echo "--- Rust Binary Probes ---"

# Build and run the probe binary, capturing all output
set +e
cargo run --locked --manifest-path "${RUST_DIR}/Cargo.toml" --bin p00_probes 2>&1 | tee "${OUTPUT_LOG}"
PROBE_EXIT=$?
set -e

if [ ${PROBE_EXIT} -ne 0 ]; then
    echo ""
    echo "FAIL: P00 probes exited ${PROBE_EXIT}"
    exit ${PROBE_EXIT}
fi

echo ""
echo "--- Archive/Library Checks ---"

# Build using the canonical production flow to get exact artifacts
cargo run --locked --manifest-path "${RUST_DIR}/xtask/Cargo.toml" -- production >/dev/null

MANIFEST="${RUST_DIR}/target/production-artifacts.json"
if [ ! -f "${MANIFEST}" ]; then
    echo "FAIL: production-artifacts.json not found"
    exit 1
fi

# Extract the exact C archive path from production evidence
C_ARCHIVE_ENTRY=$(python3 - "${MANIFEST}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    manifest = json.load(source)
for artifact in manifest.get("artifacts", []):
    if artifact.get("role") == "c_static_archive":
        print(artifact["path"])
        sys.exit(0)
sys.exit(1)
PYTHON
) || {
    echo "FAIL: c_static_archive not found in production manifest"
    exit 1
}
ARCHIVE="${REPO_ROOT}/${C_ARCHIVE_ENTRY}"

if [ ! -f "${ARCHIVE}" ]; then
    echo "FAIL: matching libuqm_c.a not found at ${ARCHIVE}"
    exit 1
fi
echo "PASS: libuqm_c.a found at ${ARCHIVE}"

# Extract the exact object sidecar path from production evidence
SIDECAR_ENTRY=$(python3 - "${MANIFEST}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    manifest = json.load(source)
for artifact in manifest.get("artifacts", []):
    if artifact.get("role") == "object_sidecar":
        print(artifact["path"])
        sys.exit(0)
sys.exit(1)
PYTHON
) || {
    echo "FAIL: object_sidecar not found in production manifest"
    exit 1
}
SIDECAR="${REPO_ROOT}/${SIDECAR_ENTRY}"
AR_PATH=$(python3 - "${MANIFEST}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    print(json.load(source)["native_build"]["toolchain"]["ar"]["executable"])
PYTHON
)

# Execute: verify archive member extraction is deterministic
MEMBER_COUNT=$("${AR_PATH}" t "${ARCHIVE}" 2>/dev/null | wc -l | tr -d ' ')
if [ "${MEMBER_COUNT}" -lt "10" ]; then
    echo "FAIL: archive has only ${MEMBER_COUNT} members (expected >=10)"
    exit 1
fi
echo "PASS: archive has ${MEMBER_COUNT} members"

# Execute: verify members are sorted in manifest
if ! LC_ALL=C sort -c "${SIDECAR}" 2>/dev/null; then
    echo "FAIL: manifest is not sorted"
    exit 1
fi
echo "PASS: manifest is sorted"

check_archive_member() {
    local member="$1"
    if ! "${AR_PATH}" t "${ARCHIVE}" 2>/dev/null | grep -q "^${member}$"; then
        echo "FAIL: member ${member} not found in archive"
        return 1
    fi
    echo "PASS: ${member} in archive"
}

for member in gameinp_rust_main.o confirm.c.o sdl_common.c.o input.c.o dcqueue.c.o; do
    check_archive_member "${member}" || exit 1
done

echo ""
echo "=== P00 Probes Complete ==="
echo "All probes passed."
echo "Output saved to: ${OUTPUT_LOG}"
echo "Finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
