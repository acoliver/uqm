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

: "${UQM_CI_CONTROLLER_EXECUTABLE:?UQM_CI_CONTROLLER_EXECUTABLE must be supplied by the trusted controller}"
: "${CARGO:?CARGO must be supplied by the trusted controller}"
: "${CC:?CC must be supplied by the trusted controller}"
: "${NM:?NM must be supplied by the trusted controller}"
: "${PKG_CONFIG:?PKG_CONFIG must be supplied by the trusted controller}"
case "${UQM_CI_EVIDENCE_MEMBER_LIMIT_BYTES:-}" in
    ''|*[!0-9]*)
        echo "FAIL: UQM_CI_EVIDENCE_MEMBER_LIMIT_BYTES must be an authority-provided positive integer" >&2
        exit 1
        ;;
esac
if [ "${UQM_CI_EVIDENCE_MEMBER_LIMIT_BYTES}" -lt 1024 ]; then
    echo "FAIL: evidence member limit is smaller than one Bash file-limit block" >&2
    exit 1
fi
if [ -z "${UQM_CI_SUBORDINATE_EVIDENCE_ROOT:-}" ]; then
    echo "FAIL: UQM_CI_SUBORDINATE_EVIDENCE_ROOT must be authority-provided" >&2
    exit 1
fi
EVIDENCE_FILE_LIMIT_BLOCKS=$((UQM_CI_EVIDENCE_MEMBER_LIMIT_BYTES / 1024))
run_with_bounded_output() {
    (ulimit -f "${EVIDENCE_FILE_LIMIT_BLOCKS}"; exec "$@")
}

if [ -n "${UQM_CI_SOURCE_ROOT:-}" ]; then
    REPO_ROOT="${UQM_CI_SOURCE_ROOT}"
    RUST_DIR="${REPO_ROOT}/rust"
else
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    RUST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
    REPO_ROOT="$(cd "${RUST_DIR}/.." && pwd)"
fi

cd "${RUST_DIR}"

echo "=== P00 Linked Harness Probe ==="
echo ""

# Build once when needed. A production manifest intentionally lacks the
# two-build determinism proof required by `xtask verify`; strict artifact and
# provider validation below still consumes its canonical recorded paths/tools.
MANIFEST_JSON="${RUST_DIR}/target/production-artifacts.json"
if [ ! -f "${MANIFEST_JSON}" ]; then
    "${UQM_CI_CONTROLLER_EXECUTABLE}" __ci-production >/dev/null
fi

# Extract and verify exact artifact paths from production evidence.
extract_artifact_path() {
    local role="$1"
    python3 -P - "${MANIFEST_JSON}" "${REPO_ROOT}" "${role}" <<'PYTHON' || {
import hashlib, json, os, stat, sys
manifest_path, root, role = sys.argv[1:]
with open(manifest_path, encoding="utf-8") as source:
    manifest = json.load(source)
artifact = next((item for item in manifest.get("artifacts", []) if item.get("role") == role), None)
if artifact is None:
    raise SystemExit(f"missing artifact role {role}")
path = artifact.get("path")
if not isinstance(path, str) or not path.startswith("rust/target/"):
    raise SystemExit(f"artifact {role} is outside rust/target")
components = path.split("/")
if any(component in ("", ".", "..") for component in components):
    raise SystemExit(f"artifact {role} has a non-normal path")
fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY)
try:
    for component in components[:-1]:
        next_fd = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
        os.close(fd)
        fd = next_fd
    file_fd = os.open(components[-1], os.O_RDONLY | os.O_NOFOLLOW, dir_fd=fd)
    try:
        metadata = os.fstat(file_fd)
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size != artifact.get("byte_length"):
            raise SystemExit(f"artifact {role} contradicts its declared type or length")
        digest = hashlib.sha256()
        while chunk := os.read(file_fd, 1024 * 1024):
            digest.update(chunk)
        if digest.hexdigest() != artifact.get("sha256"):
            raise SystemExit(f"artifact {role} contradicts its declared digest")
    finally:
        os.close(file_fd)
finally:
    os.close(fd)
print(path)
PYTHON
        echo "FAIL: ${role} is not valid production evidence" >&2
        exit 1
    }
}

C_ARCHIVE_REL="$(extract_artifact_path c_static_archive)"
SIDECAR_REL="$(extract_artifact_path object_sidecar)"
RUST_ARCHIVE_REL="$(extract_artifact_path rust_static_archive)"
C_ARCHIVE="${REPO_ROOT}/${C_ARCHIVE_REL}"
MANIFEST="${REPO_ROOT}/${SIDECAR_REL}"
RUST_ARCHIVE="${REPO_ROOT}/${RUST_ARCHIVE_REL}"
CC_PATH="$(python3 -P - "${MANIFEST_JSON}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    print(json.load(source)["native_build"]["toolchain"]["cc"]["executable"])
PYTHON
)"
NM_PATH="$(python3 -P - "${MANIFEST_JSON}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    print(json.load(source)["native_build"]["toolchain"]["nm"]["executable"])
PYTHON
)"
PKG_CONFIG_PATH="$(python3 -P - "${MANIFEST_JSON}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    print(json.load(source)["native_build"]["toolchain"]["pkg_config"]["executable"])
PYTHON
)"
if [ "${CC_PATH}" != "${CC}" ] || [ "${NM_PATH}" != "${NM}" ] || [ "${PKG_CONFIG_PATH}" != "${PKG_CONFIG}" ]; then
    echo "FAIL: production manifest tools do not match the trusted controller" >&2
    exit 1
fi
OUT_DIR="$(dirname "${C_ARCHIVE}")"
HARNESS_ARCHIVE="${OUT_DIR}/libp00_harness_shim.a"
HARNESS_EVIDENCE="${UQM_CI_SUBORDINATE_EVIDENCE_ROOT}"
mkdir -p "${HARNESS_EVIDENCE}"

if ! python3 -P - "${MANIFEST_JSON}" "${CC}" "${NM}" "${PKG_CONFIG}" <<'PYTHON'
import hashlib, json, os, stat, sys
with open(sys.argv[1], encoding="utf-8") as source:
    tools = json.load(source)["native_build"]["toolchain"]
for name, actual_path in zip(("cc", "nm", "pkg_config"), sys.argv[2:]):
    expected = tools[name]
    if expected["executable"] != actual_path:
        raise SystemExit(f"{name} path does not match the trusted controller")
    executable_fd = os.open(actual_path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        if not stat.S_ISREG(os.fstat(executable_fd).st_mode):
            raise SystemExit(f"{name} executable is not a regular file")
        digest = hashlib.sha256()
        while chunk := os.read(executable_fd, 1024 * 1024):
            digest.update(chunk)
    finally:
        os.close(executable_fd)
    if digest.hexdigest() != expected["sha256"]:
        raise SystemExit(f"{name} digest does not match its manifest identity")
PYTHON
then
    echo "FAIL: production manifest tools do not match trusted executable identities" >&2
    exit 1
fi

capture_nm() {
    local name="$1"
    shift
    local stdout_path="${HARNESS_EVIDENCE}/${name}-nm.txt"
    local stderr_path="${HARNESS_EVIDENCE}/${name}-nm.stderr.txt"
    local exit_path="${HARNESS_EVIDENCE}/${name}-nm.exit.txt"
    local nm_exit

    if run_with_bounded_output "${NM_PATH}" "$@" > "${stdout_path}" 2> "${stderr_path}"; then
        nm_exit=0
    else
        nm_exit=$?
    fi
    printf '%s\n' "${nm_exit}" > "${exit_path}"

    if [ "${nm_exit}" -ne 0 ]; then
        echo "FAIL: nm exited ${nm_exit} for ${name}: ${NM_PATH} $*" >&2
        cat "${stdout_path}"
        cat "${stderr_path}" >&2
        return "${nm_exit}"
    fi
}

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

NM_LISTING="${HARNESS_EVIDENCE}/archive-nm.txt"
HARNESS_MAIN=""
LINK_MAP=""
HARNESS_BIN=""
cleanup() {
    rm -f "${HARNESS_MAIN}" "${HARNESS_BIN}" "${LINK_MAP}"
}
trap cleanup EXIT
if capture_nm "archive" -A "${C_ARCHIVE}"; then :; else nm_exit=$?; exit "${nm_exit}"; fi
: > "${HARNESS_EVIDENCE}/archive-nm-origins.txt"
for sym in DoInput AnyButtonPress DoConfirmExit TFB_ProcessEvents TFB_SwapBuffers ProcessInputEvent TFB_FlushGraphicsEx; do
    member=$(awk -v symbol="${sym}" '$(NF - 1) == "T" && ($NF == symbol || $NF == "_" symbol) { print; exit }' "${NM_LISTING}")
    origin=$(printf '%s\n' "${member}" | awk '{ print $1 }')
    if [ -z "${member}" ]; then
        echo "FAIL: Symbol '${sym}' not found in retained nm output ${NM_LISTING}"
        exit 1
    fi
    printf '%s\t%s\t%s\n' "${sym}" "${origin}" "${member}" >> "${HARNESS_EVIDENCE}/archive-nm-origins.txt"
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

HARNESS_MAIN=$(mktemp "${TMPDIR:-/tmp}/p00_harness_main.XXXXXX").c
LINK_MAP=$(mktemp "${TMPDIR:-/tmp}/p00_link_map.XXXXXX").map
HARNESS_BIN=$(mktemp "${TMPDIR:-/tmp}/p00_harness_bin.XXXXXX")

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
PKG_CFLAGS_RAW="$("${PKG_CONFIG_PATH}" --cflags sdl2 libpng liblzma)"
PKG_LIBS_RAW="$("${PKG_CONFIG_PATH}" --libs sdl2 libpng liblzma)"
parse_pkg_config_args() {
    local raw="$1"
    local output="$2"
    python3 -P - "${raw}" > "${output}" <<'PYTHON'
import shlex, sys
for value in shlex.split(sys.argv[1]):
    if "\n" in value or "\r" in value:
        raise SystemExit("pkg-config emitted a flag containing a line break")
    print(value)
PYTHON
}
PKG_CFLAGS_FILE="${HARNESS_EVIDENCE}/pkg-config-cflags.txt"
PKG_LIBS_FILE="${HARNESS_EVIDENCE}/pkg-config-libs.txt"
parse_pkg_config_args "${PKG_CFLAGS_RAW}" "${PKG_CFLAGS_FILE}"
parse_pkg_config_args "${PKG_LIBS_RAW}" "${PKG_LIBS_FILE}"
PKG_CFLAGS=(__uqm_empty_array_sentinel__)
while IFS= read -r flag; do PKG_CFLAGS+=("${flag}"); done < "${PKG_CFLAGS_FILE}"
PKG_LIBS=(__uqm_empty_array_sentinel__)
while IFS= read -r flag; do PKG_LIBS+=("${flag}"); done < "${PKG_LIBS_FILE}"

# Platform-specific strict linking. Keep the command in an if condition so
# set -e cannot bypass the explicit diagnostic and retained evidence path.
OS_NAME="$(uname -s)"
if [ "${OS_NAME}" = "Darwin" ]; then
    if ! "${CC_PATH}" "${PKG_CFLAGS[@]:1}" "${HARNESS_MAIN}" \
        -L"${OUT_DIR}" \
        -Wl,-force_load,"${HARNESS_ARCHIVE}" \
        "${C_ARCHIVE}" \
        "${RUST_ARCHIVE}" \
        "${PKG_LIBS[@]:1}" -lz -lm -lbz2 -lobjc \
        -framework Cocoa -framework CoreAudio -framework AudioToolbox -framework CoreFoundation \
        -Wl,-map,"${LINK_MAP}" \
        -o "${HARNESS_BIN}" 2>&1; then
        echo "FAIL: Darwin harness link failed"
        exit 1
    fi
elif [ "${OS_NAME}" = "Linux" ]; then
    if ! "${CC_PATH}" "${PKG_CFLAGS[@]:1}" "${HARNESS_MAIN}" \
        -L"${OUT_DIR}" \
        -Wl,--gc-sections \
        -Wl,--whole-archive "${HARNESS_ARCHIVE}" -Wl,--no-whole-archive \
        -Wl,--start-group "${C_ARCHIVE}" "${RUST_ARCHIVE}" -Wl,--end-group \
        "${PKG_LIBS[@]:1}" -lz -lm -lbz2 -lasound \
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
if capture_nm "harness" "${HARNESS_BIN}"; then :; else nm_exit=$?; exit "${nm_exit}"; fi
echo ""

# --- 4. Run the harness (no mutation — all symbols present) ---
echo "--- 4. Run harness (all symbols present) ---"
mkdir -p "${HARNESS_EVIDENCE}"
HARNESS_OUTPUT_PATH="${HARNESS_EVIDENCE}/harness-output.txt"
if run_with_bounded_output "${HARNESS_BIN}" > "${HARNESS_OUTPUT_PATH}" 2>&1; then
    HARNESS_EXIT=0
else
    HARNESS_EXIT=$?
fi
cat "${HARNESS_OUTPUT_PATH}"

printf '%s\n' "${HARNESS_EXIT}" > "${HARNESS_EVIDENCE}/harness-exit.txt"
if ! run_with_bounded_output cp "${LINK_MAP}" "${HARNESS_EVIDENCE}/link-map.txt"; then
    echo "FAIL: cannot retain bounded harness link map" >&2
    exit 1
fi
if ! run_with_bounded_output cp "${MANIFEST}" "${HARNESS_EVIDENCE}/object-manifest.txt"; then
    echo "FAIL: cannot retain bounded harness object manifest" >&2
    exit 1
fi

if [ "${HARNESS_EXIT}" -ne 0 ]; then
    echo "FAIL: Harness exited ${HARNESS_EXIT}"
    rm -f "${HARNESS_MAIN}" "${HARNESS_BIN}" "${LINK_MAP}"
    exit 1
elif grep -q "RESULT=PASS" "${HARNESS_OUTPUT_PATH}"; then
    echo "PASS: Harness verified all symbols"
else
    echo "FAIL: Harness did not pass"
    rm -f "${HARNESS_MAIN}" "${HARNESS_BIN}" "${LINK_MAP}"
    exit 1
fi
echo ""

echo "=== P00 Harness Probe: ALL CHECKS PASSED ==="

echo "Evidence saved to: ${HARNESS_EVIDENCE}"

cleanup
trap - EXIT
