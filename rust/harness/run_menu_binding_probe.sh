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

# Build once when needed. The production manifest is canonical evidence but
# intentionally lacks the two-build proof required by `xtask verify`.
MANIFEST_JSON="${RUST_DIR}/target/production-artifacts.json"
if [ ! -f "${MANIFEST_JSON}" ]; then
    "${UQM_CI_CONTROLLER_EXECUTABLE}" __ci-production >/dev/null
fi

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

C_ARCHIVE="${REPO_ROOT}/$(extract_artifact_path c_static_archive)"
RUST_ARCHIVE="${REPO_ROOT}/$(extract_artifact_path rust_static_archive)"
OUT_DIR="$(dirname "${C_ARCHIVE}")"
HARNESS_ARCHIVE="${OUT_DIR}/libp00_harness_shim.a"
CC_PATH="$(python3 -P -c 'import json,sys; print(json.load(open(sys.argv[1]))["native_build"]["toolchain"]["cc"]["executable"])' "${MANIFEST_JSON}")"
NM_PATH="$(python3 -P -c 'import json,sys; print(json.load(open(sys.argv[1]))["native_build"]["toolchain"]["nm"]["executable"])' "${MANIFEST_JSON}")"
PKG_CONFIG_TOOL="$(python3 -P -c 'import json,sys; print(json.load(open(sys.argv[1]))["native_build"]["toolchain"]["pkg_config"]["executable"])' "${MANIFEST_JSON}")"
if [ "${CC_PATH}" != "${CC}" ] || [ "${NM_PATH}" != "${NM}" ] || [ "${PKG_CONFIG_TOOL}" != "${PKG_CONFIG}" ]; then
    echo "FAIL: production manifest tools do not match the trusted controller" >&2
    exit 1
fi
EVIDENCE_DIR="${UQM_CI_SUBORDINATE_EVIDENCE_ROOT}"
mkdir -p "${EVIDENCE_DIR}"

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
    local stdout_path="${EVIDENCE_DIR}/${name}-nm.txt"
    local stderr_path="${EVIDENCE_DIR}/${name}-nm.stderr.txt"
    local exit_path="${EVIDENCE_DIR}/${name}-nm.exit.txt"
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

PRODUCTION_PACKAGES=(sdl2 libpng liblzma)
if [ "$(uname -s)" = "Darwin" ]; then
    PRODUCTION_PACKAGES+=(bzip2)
fi
if ! PKG_CFLAGS_RAW="$("${PKG_CONFIG_TOOL}" --cflags "${PRODUCTION_PACKAGES[@]}")"; then
    echo "FAIL: pkg-config --cflags failed for packages: ${PRODUCTION_PACKAGES[*]}" >&2
    exit 1
fi
if ! PKG_LIBS_RAW="$("${PKG_CONFIG_TOOL}" --libs "${PRODUCTION_PACKAGES[@]}")"; then
    echo "FAIL: pkg-config --libs failed for packages: ${PRODUCTION_PACKAGES[*]}" >&2
    exit 1
fi
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
PKG_CFLAGS_FILE="${EVIDENCE_DIR}/pkg-config-cflags.txt"
PKG_LIBS_FILE="${EVIDENCE_DIR}/pkg-config-libs.txt"
parse_pkg_config_args "${PKG_CFLAGS_RAW}" "${PKG_CFLAGS_FILE}"
parse_pkg_config_args "${PKG_LIBS_RAW}" "${PKG_LIBS_FILE}"
PKG_CFLAGS=(__uqm_empty_array_sentinel__)
while IFS= read -r flag; do PKG_CFLAGS+=("${flag}"); done < "${PKG_CFLAGS_FILE}"
PKG_LIBS=(__uqm_empty_array_sentinel__)
while IFS= read -r flag; do PKG_LIBS+=("${flag}"); done < "${PKG_LIBS_FILE}"

for file in "${C_ARCHIVE}" "${HARNESS_ARCHIVE}" "${RUST_ARCHIVE}"; do
    if [ ! -f "${file}" ]; then
        echo "FAIL: ${file} not found"
        exit 1
    fi
done
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
    local listing="$1"
    local symbol="$2"
    local member_hint="$3"
    local origins="$4"
    local origin

    origin=$(awk -v symbol="${symbol}" '$(NF - 1) == "T" && ($NF == symbol || $NF == "_" symbol) { print; exit }' "${listing}")
    if [ -z "${origin}" ]; then
        printf '%s\t%s\t%s\n' "${symbol}" "${member_hint}" "" >> "${origins}"
        echo "  FAIL: ${symbol} not defined (text) in retained nm output ${listing}"
        return 1
    fi
    printf '%s\t%s\t%s\n' "${symbol}" "${member_hint}" "${origin}" >> "${origins}"
    if [ -n "${member_hint}" ] && [[ "${origin}" != *"${member_hint}"* ]]; then
        echo "  FAIL: ${symbol} came from unexpected archive member; member_hint=${member_hint}; selected_origin=${origin}"
        return 1
    fi
    echo "  PASS: ${symbol} defined in ${origin}"
}

if capture_nm "c-archive" -A "${C_ARCHIVE}"; then :; else nm_exit=$?; exit "${nm_exit}"; fi
if capture_nm "rust-archive" -A "${RUST_ARCHIVE}"; then :; else nm_exit=$?; exit "${nm_exit}"; fi
if capture_nm "harness-archive" -A "${HARNESS_ARCHIVE}"; then :; else nm_exit=$?; exit "${nm_exit}"; fi
: > "${EVIDENCE_DIR}/c-archive-nm-origins.txt"
: > "${EVIDENCE_DIR}/rust-archive-nm-origins.txt"
: > "${EVIDENCE_DIR}/harness-archive-nm-origins.txt"

echo "  -- C archive symbols --"
verify_symbol "${EVIDENCE_DIR}/c-archive-nm.txt" "VControl_ParseGesture" "rust_vcontrol_impl.c.o" "${EVIDENCE_DIR}/c-archive-nm-origins.txt" || exit 1
verify_symbol "${EVIDENCE_DIR}/c-archive-nm.txt" "InstallGraphicResTypes" "resgfx.c.o" "${EVIDENCE_DIR}/c-archive-nm-origins.txt" || exit 1
verify_symbol "${EVIDENCE_DIR}/c-archive-nm.txt" "InstallStringTableResType" "sresins.c.o" "${EVIDENCE_DIR}/c-archive-nm-origins.txt" || exit 1

echo "  -- Rust archive symbols --"
verify_symbol "${EVIDENCE_DIR}/rust-archive-nm.txt" "InitResourceSystem" "" "${EVIDENCE_DIR}/rust-archive-nm-origins.txt" || exit 1
verify_symbol "${EVIDENCE_DIR}/rust-archive-nm.txt" "LoadResourceIndex" "" "${EVIDENCE_DIR}/rust-archive-nm-origins.txt" || exit 1
verify_symbol "${EVIDENCE_DIR}/rust-archive-nm.txt" "res_IsString" "" "${EVIDENCE_DIR}/rust-archive-nm-origins.txt" || exit 1
verify_symbol "${EVIDENCE_DIR}/rust-archive-nm.txt" "res_GetString" "" "${EVIDENCE_DIR}/rust-archive-nm-origins.txt" || exit 1
verify_symbol "${EVIDENCE_DIR}/rust-archive-nm.txt" "uio_openRepository" "" "${EVIDENCE_DIR}/rust-archive-nm-origins.txt" || exit 1
verify_symbol "${EVIDENCE_DIR}/rust-archive-nm.txt" "uio_mountDir" "" "${EVIDENCE_DIR}/rust-archive-nm-origins.txt" || exit 1
verify_symbol "${EVIDENCE_DIR}/rust-archive-nm.txt" "uio_openDir" "" "${EVIDENCE_DIR}/rust-archive-nm-origins.txt" || exit 1

echo "  -- Harness archive symbols --"
verify_symbol "${EVIDENCE_DIR}/harness-archive-nm.txt" "uqm_query_menu_binding" "menu_binding_accessor.o" "${EVIDENCE_DIR}/harness-archive-nm-origins.txt" || exit 1

echo "PASS: all required production symbols verified"
echo ""

# --------------------------------------------------------------------------
# 4. Link the probe executable (force-load order per execution-contract §8)
# --------------------------------------------------------------------------

echo "--- Linking probe executable ---"

PROBE_BIN=$(mktemp "${TMPDIR:-/tmp}/menu_binding_probe_bin.XXXXXX")
LINK_MAP=$(mktemp "${TMPDIR:-/tmp}/menu_binding_link_map.XXXXXX").map
cleanup() {
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
}
trap cleanup EXIT
PROBE_OBJ="${OUT_DIR}/menu_binding_probe.o"

if [ ! -f "${PROBE_OBJ}" ]; then
    echo "FAIL: ${PROBE_OBJ} not found"
    rm -f "${LINK_MAP}"
    exit 1
fi

# Link order per execution-contract §8, with the exact package flags obtained
# from the manifest-recorded pkg-config executable. No undefined-symbol or
# dynamic-lookup escape hatch is permitted.
OS_NAME="$(uname -s)"
if [ "${OS_NAME}" = "Darwin" ]; then
    if ! "${CC_PATH}" "${PKG_CFLAGS[@]:1}" \
        -L"${OUT_DIR}" \
        "${PROBE_OBJ}" \
        -Wl,-force_load,"${HARNESS_ARCHIVE}" \
        "${C_ARCHIVE}" \
        "${RUST_ARCHIVE}" \
        "${PKG_LIBS[@]:1}" -lz -lm -lobjc \
        -framework Cocoa -framework CoreAudio -framework AudioToolbox -framework CoreFoundation \
        -Wl,-map,"${LINK_MAP}" \
        -o "${PROBE_BIN}" 2>&1; then
        echo "FAIL: Darwin menu binding probe link failed"
        exit 1
    fi
elif [ "${OS_NAME}" = "Linux" ]; then
    if ! "${CC_PATH}" "${PKG_CFLAGS[@]:1}" \
        -L"${OUT_DIR}" \
        "${PROBE_OBJ}" \
        -Wl,--gc-sections \
        -Wl,--whole-archive "${HARNESS_ARCHIVE}" -Wl,--no-whole-archive \
        -Wl,--start-group "${C_ARCHIVE}" "${RUST_ARCHIVE}" -Wl,--end-group \
        "${PKG_LIBS[@]:1}" -lbz2 -lz -lm -lasound \
        -Wl,-Map,"${LINK_MAP}" \
        -o "${PROBE_BIN}" 2>&1; then
        echo "FAIL: Linux menu binding probe link failed"
        exit 1
    fi
else
    echo "FAIL: unsupported OS: ${OS_NAME}"
    exit 1
fi
echo "PASS: probe linked successfully"
if capture_nm "probe-binary" "${PROBE_BIN}"; then :; else nm_exit=$?; exit "${nm_exit}"; fi
echo ""

# --------------------------------------------------------------------------
# 5. Run the probe as an initialized child with production resources
# --------------------------------------------------------------------------

echo "--- Running probe (initialized-child production query) ---"
echo "  PROBE_BIN: ${PROBE_BIN}"
echo "  CONTENT_DIR: ${CONTENT_DIR}"
echo ""

save_evidence() {
    mkdir -p "${EVIDENCE_DIR}"
    printf '%s\n' "${PROBE_EXIT}" > "${EVIDENCE_DIR}/probe-exit.txt"
    if ! run_with_bounded_output cp "${LINK_MAP}" "${EVIDENCE_DIR}/menu-binding-link-map.txt"; then
        echo "FAIL: cannot retain bounded menu-binding link map" >&2
        return 1
    fi
}

PROBE_OUTPUT_PATH="${EVIDENCE_DIR}/probe-output.txt"
if run_with_bounded_output "${PROBE_BIN}" "${CONTENT_DIR}" > "${PROBE_OUTPUT_PATH}" 2>&1; then
    PROBE_EXIT=0
else
    PROBE_EXIT=$?
fi
save_evidence

cat "${PROBE_OUTPUT_PATH}"
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

if ! grep -q "RESULT=PASS" "${PROBE_OUTPUT_PATH}"; then
    echo "FAIL: probe did not emit RESULT=PASS"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

if ! grep -q "found=1" "${PROBE_OUTPUT_PATH}"; then
    echo "FAIL: probe did not find a binding (found != 1)"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

if ! grep -q "binding_type=VCONTROL_KEY" "${PROBE_OUTPUT_PATH}"; then
    echo "FAIL: probe did not confirm VCONTROL_KEY binding type"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi

# Extract and validate key_code (must be positive — a real SDL keycode)
KEY_CODE=$(awk -F= '/^key_code=/ { print $2; exit }' "${PROBE_OUTPUT_PATH}")
BINDING_ID=$(awk -F= '/^binding_id=/ { print $2; exit }' "${PROBE_OUTPUT_PATH}")
NUM_ALTERNATES=$(awk -F= '/^num_alternates=/ { print $2; exit }' "${PROBE_OUTPUT_PATH}")

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

# Verify production origin: menu.key defines down.1 = key Down, and SDL2
# assigns SDLK_DOWN the exact value 1073741905 (0x40000051).
if [ "${KEY_CODE}" != "1073741905" ]; then
    echo "FAIL: menu.down.1 key_code ${KEY_CODE} does not equal SDLK_DOWN (1073741905)"
    rm -f "${PROBE_BIN}" "${LINK_MAP}"
    exit 1
fi
echo "PASS: menu.down.1 is bound to SDLK_DOWN (1073741905)"
echo ""

# --------------------------------------------------------------------------
# 7. nm evidence for the probe binary
# --------------------------------------------------------------------------

echo "--- nm evidence for probe binary ---"

for sym in _main _uqm_query_menu_binding _VControl_ParseGesture _InitResourceSystem _LoadResourceIndex _res_IsString _res_GetString _rust_VControl_ParseGesture; do
    addr=$(awk -v symbol="${sym}" '$(NF - 1) == "T" && $NF == symbol { print; exit }' "${EVIDENCE_DIR}/probe-binary-nm.txt")
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

save_evidence

echo "=== Menu Binding Probe: ALL CHECKS PASSED ==="
echo "Evidence saved to: ${EVIDENCE_DIR}"
echo "Finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Cleanup temporary files (keep evidence)
cleanup
trap - EXIT
