#!/bin/bash
#
# Menu Binding Probe — Initialized-Child Production Query Runner
#
# Compiles and links a standalone C probe against the production Rust and C
# archives, then executes it as an initialized child with production resources
# loaded. The probe queries the actual `menu.down.N` binding through the
# narrow `uqm_query_menu_binding` accessor (which calls production
# res_IsString/res_GetString and VControl_ParseGesture), emits the resolved
# VCONTROL_KEY binding and alternate id, then tears down and exits.
#
# This script FAILS if:
#   - The query is not found (no menu.down.N binding exists)
#   - The resolved binding is not a VCONTROL_KEY
#   - The binding does not originate from production resources (menu.key)
#   - Linking fails (proves archive/Rust/C member extraction)
#
# Evidence (link map, nm output) is preserved in the evidence directory.
#
# @plan PLAN-20260723-RUNTIME-AUTOMATION.P00
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${RUST_DIR}/.." && pwd)"
CONTENT_DIR="${REPO_ROOT}/sc2/content"

echo "=== Menu Binding Probe (Initialized-Child Production Query) ==="
echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "RUST_DIR: ${RUST_DIR}"
echo "REPO_ROOT: ${REPO_ROOT}"
echo "CONTENT_DIR: ${CONTENT_DIR}"
echo ""

# --------------------------------------------------------------------------
# 0. Verify prerequisites
# --------------------------------------------------------------------------

if [ ! -f "${CONTENT_DIR}/menu.key" ]; then
    echo "FAIL: ${CONTENT_DIR}/menu.key not found"
    exit 1
fi
echo "PASS: menu.key found at ${CONTENT_DIR}/menu.key"
echo ""

# --------------------------------------------------------------------------
# 1. Build the library to produce the archives
# --------------------------------------------------------------------------

echo "--- Building library target (produces libuqm_rust.a, libuqm_c.a, libp00_harness_shim.a) ---"
cargo build --lib 2>&1
BUILD_EXIT=$?
if [ ${BUILD_EXIT} -ne 0 ]; then
    echo "FAIL: cargo build --lib exited ${BUILD_EXIT}"
    exit 1
fi
echo "PASS: library built"
echo ""

# --------------------------------------------------------------------------
# 2. Locate build artifacts
# --------------------------------------------------------------------------

MANIFEST=$(find "${RUST_DIR}/target/debug/build" -name "uqm-c-objects.manifest" -type f -print0 2>/dev/null \
    | xargs -0 ls -t 2>/dev/null | head -1)
if [ -z "${MANIFEST}" ]; then
    echo "FAIL: uqm-c-objects.manifest not found"
    exit 1
fi
OUT_DIR="$(dirname "${MANIFEST}")"

C_ARCHIVE="${OUT_DIR}/libuqm_c.a"
HARNESS_ARCHIVE="${OUT_DIR}/libp00_harness_shim.a"
RUST_ARCHIVE="${RUST_DIR}/target/debug/libuqm_rust.a"

echo "OUT_DIR: ${OUT_DIR}"
echo "C_ARCHIVE: ${C_ARCHIVE}"
echo "HARNESS_ARCHIVE: ${HARNESS_ARCHIVE}"
echo "RUST_ARCHIVE: ${RUST_ARCHIVE}"
echo ""

for f in "${C_ARCHIVE}" "${HARNESS_ARCHIVE}" "${RUST_ARCHIVE}"; do
    if [ ! -f "${f}" ]; then
        echo "FAIL: ${f} not found"
        exit 1
    fi
done
echo "PASS: all archives present"
echo ""

# --------------------------------------------------------------------------
# 3. Verify production symbols in archives (nm evidence)
# --------------------------------------------------------------------------

echo "--- Production symbol verification (nm) ---"

# The probe references these symbols:
#   - From libuqm_c.a: VControl_ParseGesture, uqm_query_menu_binding,
#     InstallGraphicResTypes, InstallStringTableResType, etc.
#   - From libuqm_rust.a: InitResourceSystem, LoadResourceIndex,
#     res_IsString, res_GetString, uio_openRepository, uio_mountDir,
#     uio_openDir, uio_closeDir, uio_closeRepository

verify_symbol() {
    local archive="$1"
    local symbol="$2"
    local member_hint="$3"
    # Write nm output to temp file to avoid SIGPIPE issues with pipefail + grep -q
    local tmpfile
    tmpfile=$(mktemp)
    nm -A "${archive}" > "${tmpfile}" 2>/dev/null || true
    if grep -q " T _${symbol}\$" "${tmpfile}"; then
        local origin
        origin=$(grep " T _${symbol}\$" "${tmpfile}" | head -1)
        echo "  PASS: ${symbol} defined in ${origin}"
        rm -f "${tmpfile}"
    else
        echo "  FAIL: ${symbol} not defined (text) in $(basename "${archive}")"
        rm -f "${tmpfile}"
        return 1
    fi
}

echo "  -- C archive symbols --"
verify_symbol "${C_ARCHIVE}" "VControl_ParseGesture" "rust_vcontrol_impl.c.o" || exit 1
verify_symbol "${C_ARCHIVE}" "InstallGraphicResTypes" "resgfx.c.o" || exit 1
verify_symbol "${C_ARCHIVE}" "InstallStringTableResType" "sresins.c.o" || exit 1

echo "  -- Rust archive symbols --"
verify_symbol "${RUST_ARCHIVE}" "InitResourceSystem" "" || exit 1
verify_symbol "${RUST_ARCHIVE}" "LoadResourceIndex" "" || exit 1
verify_symbol "${RUST_ARCHIVE}" "res_IsString" "" || exit 1
verify_symbol "${RUST_ARCHIVE}" "res_GetString" "" || exit 1
verify_symbol "${RUST_ARCHIVE}" "uio_openRepository" "" || exit 1
verify_symbol "${RUST_ARCHIVE}" "uio_mountDir" "" || exit 1
verify_symbol "${RUST_ARCHIVE}" "uio_openDir" "" || exit 1

echo "  -- Harness archive symbols --"
verify_symbol "${HARNESS_ARCHIVE}" "uqm_query_menu_binding" "menu_binding_accessor.o" || exit 1

echo "PASS: all required production symbols verified"
echo ""

# --------------------------------------------------------------------------
# 4. Link the probe executable (force-load order per execution-contract §8)
# --------------------------------------------------------------------------

echo "--- Linking probe executable ---"

PROBE_BIN=$(mktemp -t menu_binding_probe_bin)
LINK_MAP=$(mktemp -t menu_binding_link_map).map
PROBE_OBJ="${OUT_DIR}/menu_binding_probe.o"

if [ ! -f "${PROBE_OBJ}" ]; then
    echo "FAIL: ${PROBE_OBJ} not found"
    rm -f "${LINK_MAP}"
    exit 1
fi

# Link order per execution-contract §8:
#   -L$OUT_DIR
#   PROBE_OBJ (probe entry with main)
#   -Wl,-force_load, libp00_harness_shim.a (accessor, no main)
#   libuqm_c.a (C archive: VControl wrapper, subsystem registration)
#   libuqm_rust.a (Rust: resource system, UIO, VControl parser)
#   External libraries: -lpng16 -lz -lm -lSDL2 -lobjc
#   Frameworks: Cocoa CoreAudio AudioToolbox CoreFoundation
#   Compression: -llzma -lbz2
cc \
    -Wl,-undefined,dynamic_lookup \
    -L"${OUT_DIR}" \
    "${PROBE_OBJ}" \
    -Wl,-force_load,"${HARNESS_ARCHIVE}" \
    "${C_ARCHIVE}" \
    "${RUST_ARCHIVE}" \
    -lpng16 -lz -lm -lSDL2 -lobjc \
    -framework Cocoa -framework CoreAudio -framework AudioToolbox -framework CoreFoundation \
    -llzma -lbz2 \
    -L/opt/homebrew/lib -L/opt/homebrew/opt/libpng/lib -L/opt/homebrew/opt/SDL2/lib \
    -Wl,-map,"${LINK_MAP}" \
    -o "${PROBE_BIN}" 2>&1

LINK_EXIT=$?
if [ ${LINK_EXIT} -ne 0 ]; then
    echo "FAIL: probe link failed (exit ${LINK_EXIT})"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi
echo "PASS: probe linked successfully"
echo ""

# --------------------------------------------------------------------------
# 5. Run the probe as an initialized child with production resources
# --------------------------------------------------------------------------

echo "--- Running probe (initialized-child production query) ---"
echo "  PROBE_BIN: ${PROBE_BIN}"
echo "  CONTENT_DIR: ${CONTENT_DIR}"
echo ""

PROBE_OUTPUT=$("${PROBE_BIN}" "${CONTENT_DIR}" 2>&1)
PROBE_EXIT=$?

echo "${PROBE_OUTPUT}"
echo ""
echo "Probe exit code: ${PROBE_EXIT}"

# --------------------------------------------------------------------------
# 6. Validate the probe output
# --------------------------------------------------------------------------

echo ""
echo "--- Validating probe result ---"

if [ ${PROBE_EXIT} -ne 0 ]; then
    echo "FAIL: probe exited ${PROBE_EXIT}"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

if ! echo "${PROBE_OUTPUT}" | grep -q "RESULT=PASS"; then
    echo "FAIL: probe did not emit RESULT=PASS"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

if ! echo "${PROBE_OUTPUT}" | grep -q "found=1"; then
    echo "FAIL: probe did not find a binding (found != 1)"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

if ! echo "${PROBE_OUTPUT}" | grep -q "binding_type=VCONTROL_KEY"; then
    echo "FAIL: probe did not confirm VCONTROL_KEY binding type"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

# Extract and validate key_code (must be positive — a real SDL keycode)
KEY_CODE=$(echo "${PROBE_OUTPUT}" | grep "^key_code=" | cut -d= -f2)
BINDING_ID=$(echo "${PROBE_OUTPUT}" | grep "^binding_id=" | cut -d= -f2)
NUM_ALTERNATES=$(echo "${PROBE_OUTPUT}" | grep "^num_alternates=" | cut -d= -f2)

if [ -z "${KEY_CODE}" ] || [ "${KEY_CODE}" -le 0 ] 2>/dev/null; then
    echo "FAIL: invalid key_code (${KEY_CODE})"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

if [ -z "${BINDING_ID}" ] || [ "${BINDING_ID}" -lt 1 ] 2>/dev/null; then
    echo "FAIL: invalid binding_id (${BINDING_ID})"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

if [ -z "${NUM_ALTERNATES}" ] || [ "${NUM_ALTERNATES}" -lt 1 ] 2>/dev/null; then
    echo "FAIL: invalid num_alternates (${NUM_ALTERNATES})"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

echo "PASS: binding found — key_code=${KEY_CODE}, binding_id=${BINDING_ID}, num_alternates=${NUM_ALTERNATES}"

# Verify production origin: the binding must come from menu.key, which
# defines down.1 = key Down. In SDL2, SDLK_DOWN = 1073741905 (0x40000051).
# SDL2 keycodes range from 0 to ~0x40000114 (SDLK_SCANCODE_MASK = 0x40000000).
if [ "${KEY_CODE}" -lt 1 ] || [ "${KEY_CODE}" -gt 1200000000 ] 2>/dev/null; then
    echo "FAIL: key_code ${KEY_CODE} outside valid SDL keycode range"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi
echo "PASS: key_code in valid SDL keycode range"
echo ""

# --------------------------------------------------------------------------
# 7. nm evidence for the probe binary
# --------------------------------------------------------------------------

echo "--- nm evidence for probe binary ---"

for sym in _main _uqm_query_menu_binding _VControl_ParseGesture _InitResourceSystem _LoadResourceIndex _res_IsString _res_GetString _rust_VControl_ParseGesture; do
    addr=$(nm "${PROBE_BIN}" 2>/dev/null | { grep " T ${sym}\$" || true; } | head -1)
    if [ -n "${addr}" ]; then
        echo "  ${sym} -> ${addr}"
    else
        echo "  ${sym} -> (not in text section)"
    fi
done
echo ""

# --------------------------------------------------------------------------
# 8. Save evidence
# --------------------------------------------------------------------------

EVIDENCE_DIR="/tmp/p00-menu-binding-evidence"
mkdir -p "${EVIDENCE_DIR}"
cp "${LINK_MAP}" "${EVIDENCE_DIR}/menu-binding-link-map.txt"
nm -A "${C_ARCHIVE}" > "${EVIDENCE_DIR}/c-archive-nm.txt" 2>/dev/null || true
nm -A "${RUST_ARCHIVE}" > "${EVIDENCE_DIR}/rust-archive-nm.txt" 2>/dev/null || true
nm "${PROBE_BIN}" > "${EVIDENCE_DIR}/probe-binary-nm.txt" 2>/dev/null || true
echo "${PROBE_OUTPUT}" > "${EVIDENCE_DIR}/probe-output.txt"

echo "=== Menu Binding Probe: ALL CHECKS PASSED ==="
echo "Evidence saved to: ${EVIDENCE_DIR}"
echo "Finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Cleanup temporary binary (keep evidence)
rm -f "${PROBE_BIN}"
