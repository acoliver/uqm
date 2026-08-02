#!/usr/bin/env bash
#
# P00 Linked Harness Probe Script
#
# Proves deterministic libuqm_c.a archive construction, force-load ordering,
# production member extraction for the 7 source-grounded
# production symbols required by execution-contract §8.
#
# This script compiles and links a standalone C harness against the production
# C archive produced by the canonical xtask production flow.
#
# @plan PLAN-20260723-RUNTIME-AUTOMATION.P00 §8
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${RUST_DIR}/.." && pwd)"

cd "${RUST_DIR}"

echo "=== P00 Linked Harness Probe ==="
echo ""

# Build once when needed, then reject stale evidence instead of rebuilding it.
MANIFEST_JSON="${RUST_DIR}/target/production-artifacts.json"
if [ -f "${MANIFEST_JSON}" ]; then
    cargo run --locked --manifest-path "${RUST_DIR}/xtask/Cargo.toml" -- verify >/dev/null
else
    cargo run --locked --manifest-path "${RUST_DIR}/xtask/Cargo.toml" -- production >/dev/null
fi

# Extract exact artifact paths from production evidence
extract_artifact_path() {
    local role="$1"
    python3 - "${MANIFEST_JSON}" "${role}" <<'PYTHON' || {
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    manifest = json.load(source)
for artifact in manifest.get("artifacts", []):
    if artifact.get("role") == sys.argv[2]:
        print(artifact["path"])
        sys.exit(0)
sys.exit(1)
PYTHON
        echo "FAIL: ${role} not found in production manifest" >&2
        exit 1
    }
}

C_ARCHIVE_REL="$(extract_artifact_path c_static_archive)"
SIDECAR_REL="$(extract_artifact_path object_sidecar)"
RUST_ARCHIVE_REL="$(extract_artifact_path rust_static_archive)"
C_ARCHIVE="${REPO_ROOT}/${C_ARCHIVE_REL}"
MANIFEST="${REPO_ROOT}/${SIDECAR_REL}"
RUST_ARCHIVE="${REPO_ROOT}/${RUST_ARCHIVE_REL}"
CC_PATH="$(python3 - "${MANIFEST_JSON}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    print(json.load(source)["native_build"]["toolchain"]["cc"]["executable"])
PYTHON
)"
NM_PATH="$(python3 - "${MANIFEST_JSON}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    print(json.load(source)["native_build"]["toolchain"]["nm"]["executable"])
PYTHON
)"
PKG_CONFIG_PATH="$(python3 - "${MANIFEST_JSON}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    print(json.load(source)["native_build"]["toolchain"]["pkg_config"]["executable"])
PYTHON
)"
OUT_DIR="$(dirname "${C_ARCHIVE}")"
HARNESS_ARCHIVE="${OUT_DIR}/libp00_harness_shim.a"

if [ ! -f "${C_ARCHIVE}" ]; then
    echo "FAIL: ${C_ARCHIVE} not found"
    exit 1
fi
if [ ! -f "${HARNESS_ARCHIVE}" ]; then
    echo "FAIL: ${HARNESS_ARCHIVE} not found"
    exit 1
fi
if [ ! -f "${MANIFEST}" ]; then
    echo "FAIL: ${MANIFEST} not found"
    exit 1
fi

echo "OUT_DIR: ${OUT_DIR}"

echo ""

# --- 1. Verify archive member symbol extraction (nm) ---
echo "--- 1. Archive member symbol extraction (nm) ---"

for sym in DoInput AnyButtonPress DoConfirmExit TFB_ProcessEvents TFB_SwapBuffers ProcessInputEvent TFB_FlushGraphicsEx; do
    member=$("${NM_PATH}" -A "${C_ARCHIVE}" 2>/dev/null | awk -v symbol="${sym}" '$(NF - 1) == "T" && ($NF == symbol || $NF == "_" symbol) { print; exit }')
    if [ -z "${member}" ]; then
        echo "FAIL: Symbol '${sym}' not found in ${C_ARCHIVE}"
        exit 1
    fi
    echo "  ${sym} -> $(echo "${member}" | cut -d: -f2-)"
done
echo "PASS: All 7 production symbols found in C archive"
echo ""

# --- 2. Verify deterministic manifest ---
echo "--- 2. Deterministic object manifest ---"
MANIFEST_LINES=$(wc -l < "${MANIFEST}")
echo "  Manifest entries: ${MANIFEST_LINES}"
if LC_ALL=C sort -c "${MANIFEST}" 2>/dev/null; then
    echo "PASS: Manifest is sorted (deterministic)"
else
    echo "FAIL: Manifest is not sorted"
    exit 1
fi
echo ""

# --- 3. Compile and link the harness with force-load ordering ---
echo "--- 3. Compile and link harness (force-load order per §8) ---"

HARNESS_MAIN=$(mktemp -t p00_harness_main).c
LINK_MAP=$(mktemp -t p00_link_map).map
HARNESS_BIN=$(mktemp -t p00_harness_bin)
cleanup() {
    rm -f "${HARNESS_MAIN}" "${HARNESS_BIN}" "${LINK_MAP}"
}
trap cleanup EXIT

cat > "${HARNESS_MAIN}" << 'HARNESS_EOF'
#include <stdio.h>
extern int p00_harness_verify_symbols(void);

int main(void) {
    int count = p00_harness_verify_symbols();
    printf("harness_symbol_count=%d\n", count);
    if (count < 0) {
        printf("RESULT=FAIL\n");
        return 1;
    }
    printf("RESULT=PASS\n");
    return 0;
}
HARNESS_EOF

# Use the exact Rust static archive from the production manifest.
if [ ! -f "${RUST_ARCHIVE}" ]; then
    echo "FAIL: ${RUST_ARCHIVE} not found after production build"
    rm -f "${HARNESS_MAIN}" "${HARNESS_BIN}" "${LINK_MAP}"
    exit 1
fi

# Discover prerequisite flags with the exact pkg-config from production evidence.
PKG_CFLAGS=$("${PKG_CONFIG_PATH}" --cflags sdl2 libpng liblzma)
PKG_LIBS=$("${PKG_CONFIG_PATH}" --libs sdl2 libpng liblzma)

# Platform-specific strict linking. Keep the command in an if condition so
# set -e cannot bypass the explicit diagnostic and retained evidence path.
OS_NAME="$(uname -s)"
if [ "${OS_NAME}" = "Darwin" ]; then
    if ! "${CC_PATH}" ${PKG_CFLAGS} "${HARNESS_MAIN}" \
        -L"${OUT_DIR}" \
        -Wl,-force_load,"${HARNESS_ARCHIVE}" \
        "${C_ARCHIVE}" \
        "${RUST_ARCHIVE}" \
        ${PKG_LIBS} -lz -lm -lbz2 -lobjc \
        -framework Cocoa -framework CoreAudio -framework AudioToolbox -framework CoreFoundation \
        -Wl,-map,"${LINK_MAP}" \
        -o "${HARNESS_BIN}" 2>&1; then
        echo "FAIL: Darwin harness link failed"
        exit 1
    fi
elif [ "${OS_NAME}" = "Linux" ]; then
    if ! "${CC_PATH}" ${PKG_CFLAGS} "${HARNESS_MAIN}" \
        -L"${OUT_DIR}" \
        -Wl,--whole-archive "${HARNESS_ARCHIVE}" -Wl,--no-whole-archive \
        "${C_ARCHIVE}" \
        "${RUST_ARCHIVE}" \
        ${PKG_LIBS} -lz -lm -lbz2 -lasound \
        -Wl,-Map,"${LINK_MAP}" \
        -o "${HARNESS_BIN}" 2>&1; then
        echo "FAIL: Linux harness link failed"
        exit 1
    fi
else
    echo "FAIL: unsupported OS: ${OS_NAME}"
    exit 1
fi
echo "PASS: Harness linked successfully"
echo ""

# --- 4. Run the harness (no mutation — all symbols present) ---
echo "--- 4. Run harness (all symbols present) ---"
HARNESS_OUTPUT=$("${HARNESS_BIN}" 2>&1)
echo "${HARNESS_OUTPUT}"
if echo "${HARNESS_OUTPUT}" | grep -q "RESULT=PASS"; then
    echo "PASS: Harness verified all symbols"
else
    echo "FAIL: Harness did not pass"
    rm -f "${HARNESS_MAIN}" "${HARNESS_BIN}" "${LINK_MAP}"
    exit 1
fi
echo ""

echo "=== P00 Harness Probe: ALL CHECKS PASSED ==="

# Save outputs for P00a evidence
HARNESS_EVIDENCE="/tmp/p00-harness-evidence"
mkdir -p "${HARNESS_EVIDENCE}"
cp "${LINK_MAP}" "${HARNESS_EVIDENCE}/link-map.txt"
"${NM_PATH}" -A "${C_ARCHIVE}" > "${HARNESS_EVIDENCE}/archive-nm.txt" 2>/dev/null
"${NM_PATH}" "${HARNESS_BIN}" > "${HARNESS_EVIDENCE}/harness-nm.txt" 2>/dev/null
cp "${MANIFEST}" "${HARNESS_EVIDENCE}/object-manifest.txt"
echo "Evidence saved to: ${HARNESS_EVIDENCE}"

cleanup
trap - EXIT
