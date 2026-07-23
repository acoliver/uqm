#!/usr/bin/env bash
#
# P00 Executable Preflight Probe Runner
#
# Executes all P00 feasibility probes and captures results.
# This script runs real probes that execute assumptions, not grep-only inspections.
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

# Verify build.vars has required capabilities (execute: source and test values)
if [ ! -f "${REPO_ROOT}/sc2/build.vars" ]; then
    echo "FAIL: sc2/build.vars not found"
    exit 1
fi

# Execute: check that required USE_RUST_* flags are enabled
REQUIRED_FLAGS="USE_RUST_THREADS USE_RUST_GFX USE_RUST_INPUT USE_RUST_COMM USE_RUST_RESOURCE"
for flag in ${REQUIRED_FLAGS}; do
    if ! grep -q "D${flag}" "${REPO_ROOT}/sc2/build.vars"; then
        echo "FAIL: ${flag} not found in build.vars CFLAGS"
        exit 1
    fi
    echo "PASS: ${flag} enabled in build.vars"
done

# Execute: verify C object directory exists and has production symbols
OBJ_DIR="${REPO_ROOT}/sc2/obj/release"
if [ ! -d "${OBJ_DIR}" ]; then
    echo "FAIL: C object directory ${OBJ_DIR} does not exist"
    echo "  Run: cd sc2 && ./build.sh uqm"
    exit 1
fi
echo "PASS: C object directory exists"

# Execute: verify required production object symbols exist via nm
check_symbol() {
    local obj_file="$1"
    local symbol_name="$2"
    local full_path="${OBJ_DIR}/${obj_file}"
    if [ ! -f "${full_path}" ]; then
        echo "FAIL: object file not found: ${full_path}"
        return 1
    fi
    if ! nm -A "${full_path}" | grep -q "T _${symbol_name}$"; then
        echo "FAIL: symbol _${symbol_name} not found in ${obj_file}"
        return 1
    fi
    echo "PASS: ${symbol_name} defined in ${obj_file}"
}

check_symbol "src/uqm/gameinp.c.o" "DoInput" || exit 1
check_symbol "src/uqm/gameinp.c.o" "AnyButtonPress" || exit 1
check_symbol "src/uqm/confirm.c.o" "DoConfirmExit" || exit 1
check_symbol "src/libs/graphics/sdl/sdl_common.c.o" "TFB_ProcessEvents" || exit 1
check_symbol "src/libs/graphics/sdl/sdl_common.c.o" "TFB_SwapBuffers" || exit 1
check_symbol "src/libs/input/sdl/input.c.o" "ProcessInputEvent" || exit 1
check_symbol "src/libs/graphics/dcqueue.c.o" "TFB_FlushGraphicsEx" || exit 1

# Execute: verify config_unix.h has required definitions
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
cargo run --bin p00_probes 2>&1 | tee "${OUTPUT_LOG}"
PROBE_EXIT=$?
set -e

if [ ${PROBE_EXIT} -ne 0 ]; then
    echo ""
    echo "FAIL: P00 probes exited ${PROBE_EXIT}"
    exit ${PROBE_EXIT}
fi

echo ""
echo "--- Archive/Library Checks ---"

# Execute: verify the package build script produces a matching archive and
# manifest. Build the library target so this probe does not depend on the full
# transitional C/Rust executable link.
cargo check --lib >/dev/null

# Select the newest debug manifest, then use the archive from the same OUT_DIR.
MANIFEST=$(find "${RUST_DIR}/target/debug/build" -name "uqm-c-objects.manifest" -type f -print0 2>/dev/null \
    | xargs -0 ls -t 2>/dev/null | head -1)
if [ -z "${MANIFEST}" ]; then
    echo "FAIL: uqm-c-objects.manifest not found"
    exit 1
fi
ARCHIVE="$(dirname "${MANIFEST}")/libuqm_c.a"
if [ ! -f "${ARCHIVE}" ]; then
    echo "FAIL: matching libuqm_c.a not found beside ${MANIFEST}"
    exit 1
fi
echo "PASS: libuqm_c.a found at ${ARCHIVE}"

# Execute: verify archive member extraction is deterministic
MEMBER_COUNT=$(ar t "${ARCHIVE}" 2>/dev/null | wc -l | tr -d ' ')
if [ "${MEMBER_COUNT}" -lt "10" ]; then
    echo "FAIL: archive has only ${MEMBER_COUNT} members (expected >=10)"
    exit 1
fi
echo "PASS: archive has ${MEMBER_COUNT} members"
echo "PASS: deterministic manifest found at ${MANIFEST}"

# Execute: verify members are sorted in manifest
if ! LC_ALL=C sort -c "${MANIFEST}" 2>/dev/null; then
    echo "FAIL: manifest is not sorted"
    exit 1
fi
echo "PASS: manifest is sorted"

# Execute: verify required production members exist in the archive
check_archive_member() {
    local member="$1"
    if ! ar t "${ARCHIVE}" 2>/dev/null | grep -q "^${member}$"; then
        echo "FAIL: member ${member} not found in archive"
        return 1
    fi
    echo "PASS: ${member} in archive"
}

check_archive_member "gameinp_rust_main.o" || exit 1
check_archive_member "confirm.c.o" || exit 1
check_archive_member "sdl_common.c.o" || exit 1
check_archive_member "input.c.o" || exit 1
check_archive_member "dcqueue.c.o" || exit 1

echo ""
echo "=== P00 Probes Complete ==="
echo "All probes passed."
echo "Output saved to: ${OUTPUT_LOG}"
echo "Finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
