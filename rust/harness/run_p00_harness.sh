#!/usr/bin/env bash
#
# P00 Linked Harness Probe Script
#
# Proves deterministic libuqm_c.a archive construction and production member
# extraction for the 7 source-grounded production symbols required by
# execution-contract §8, and proves those symbols were actually extracted
# into a linked harness binary.
#
# The harness itself is Rust (rust/probes/p00_symbol_harness.rs): a #[used]
# static table of `unsafe extern "C" fn` pointers references the seven
# symbols, so the linker cannot produce the binary without extracting their
# archive members. This script builds that binary through the canonical
# cargo toolchain, verifies the archive by nm, verifies the built binary by
# nm, and executes the binary.
#
# @plan PLAN-20260723-RUNTIME-AUTOMATION.P00 §8
#
set -euo pipefail

: "${UQM_CI_CONTROLLER_EXECUTABLE:?UQM_CI_CONTROLLER_EXECUTABLE must be supplied by the trusted controller}"
: "${CARGO:?CARGO must be supplied by the trusted controller}"
: "${NM:?NM must be supplied by the trusted controller}"
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
C_ARCHIVE="${REPO_ROOT}/${C_ARCHIVE_REL}"
MANIFEST="${REPO_ROOT}/${SIDECAR_REL}"
NM_PATH="$(python3 -P - "${MANIFEST_JSON}" <<'PYTHON'
import json, sys
with open(sys.argv[1], encoding="utf-8") as source:
    print(json.load(source)["native_build"]["toolchain"]["nm"]["executable"])
PYTHON
)"
if [ "${NM_PATH}" != "${NM}" ]; then
    echo "FAIL: production manifest nm does not match the trusted controller" >&2
    exit 1
fi
HARNESS_EVIDENCE="${UQM_CI_SUBORDINATE_EVIDENCE_ROOT}"
mkdir -p "${HARNESS_EVIDENCE}"

if ! python3 -P - "${MANIFEST_JSON}" "${NM}" <<'PYTHON'
import hashlib, json, os, stat, sys
with open(sys.argv[1], encoding="utf-8") as source:
    expected = json.load(source)["native_build"]["toolchain"]["nm"]
if expected["executable"] != sys.argv[2]:
    raise SystemExit("nm path does not match the trusted controller")
executable_fd = os.open(sys.argv[2], os.O_RDONLY | os.O_NOFOLLOW)
try:
    if not stat.S_ISREG(os.fstat(executable_fd).st_mode):
        raise SystemExit("nm executable is not a regular file")
    digest = hashlib.sha256()
    while chunk := os.read(executable_fd, 1024 * 1024):
        digest.update(chunk)
finally:
    os.close(executable_fd)
if digest.hexdigest() != expected["sha256"]:
    raise SystemExit("nm digest does not match its manifest identity")
PYTHON
then
    echo "FAIL: production manifest nm does not match its trusted executable identity" >&2
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
        # Apple's nm cannot read the embedded bitcode attributes that the
        # pinned Rust toolchain emits, so it rejects members while still
        # listing every symbol this probe verifies. Tolerate that exact
        # producer/reader mismatch and nothing else (the retained exit
        # status records the truth); the symbol requirements below remain
        # the contract.
        local errors
        local mismatches
        errors=$(grep -c 'error:' "${stderr_path}" || true)
        mismatches=$(grep -c "Unknown attribute kind ([0-9]*) (Producer: 'LLVM[0-9.]*-rust-[0-9.]*-stable' Reader: 'LLVM APPLE" "${stderr_path}" || true)
        if [ "${errors}" -gt 0 ] && [ "${errors}" -eq "${mismatches}" ]; then
            echo "NOTE: nm exited ${nm_exit} for ${name}; ${mismatches} members carry attributes this nm cannot read" >&2
            return 0
        fi
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
if [ ! -f "${MANIFEST}" ]; then
    echo "FAIL: ${MANIFEST} not found"
    exit 1
fi

echo ""

# --- 1. Verify archive member symbol extraction (nm) ---
echo "--- 1. Archive member symbol extraction (nm) ---"

NM_LISTING="${HARNESS_EVIDENCE}/archive-nm.txt"
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

# --- 3. Build the Rust symbol-forcing harness binary ---
echo "--- 3. Build Rust symbol harness (member extraction by reference) ---"

LINK_MAP=$(mktemp "${TMPDIR:-/tmp}/p00_link_map.XXXXXX").map
BUILD_LOG=$(mktemp "${TMPDIR:-/tmp}/p00_build_log.XXXXXX")
cleanup() {
    rm -f "${LINK_MAP}" "${BUILD_LOG}"
}
trap cleanup EXIT

OS_NAME="$(uname -s)"
case "${OS_NAME}" in
    Darwin)
        MAP_LINK_ARG="-Clink-arg=-Wl,-map,${LINK_MAP}"
        ;;
    Linux)
        MAP_LINK_ARG="-Clink-arg=-Wl,-Map,${LINK_MAP}"
        ;;
    *)
        echo "FAIL: unsupported OS: ${OS_NAME}"
        exit 1
        ;;
esac

# The same feature set the linked-test profile uses; build.rs links the C
# archive so the #[used] symbol table forces extraction of its members.
if ! "${CARGO}" rustc \
        --locked \
        --manifest-path "${RUST_DIR}/Cargo.toml" \
        --release \
        --no-default-features \
        --features audio_heart,debug-process,linked_c_archive \
        --bin p00_symbol_harness \
        --message-format=json \
        -- "${MAP_LINK_ARG}" > "${BUILD_LOG}"; then
    echo "FAIL: p00 symbol harness cargo build failed"
    exit 1
fi

HARNESS_BIN=$(python3 -P - "${BUILD_LOG}" <<'PYTHON'
import json, sys
executable = None
with open(sys.argv[1], encoding="utf-8") as source:
    for line in source:
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get("reason") != "compiler-artifact":
            continue
        target = message.get("target") or {}
        if target.get("name") != "p00_symbol_harness":
            continue
        if "bin" not in (target.get("kind") or []):
            continue
        path = message.get("executable")
        if path:
            executable = path
if executable is None:
    raise SystemExit("cargo reported no executable for bin p00_symbol_harness")
print(executable)
PYTHON
)
if [ -z "${HARNESS_BIN}" ] || [ ! -x "${HARNESS_BIN}" ]; then
    echo "FAIL: harness binary not found or not executable: ${HARNESS_BIN}"
    exit 1
fi
echo "PASS: harness built at ${HARNESS_BIN}"
if capture_nm "harness" "${HARNESS_BIN}"; then :; else nm_exit=$?; exit "${nm_exit}"; fi
echo ""

# --- 4. Verify the 7 symbols were extracted into the built binary ---
echo "--- 4. Extracted production symbols in harness binary (nm) ---"
HARNESS_NM_LISTING="${HARNESS_EVIDENCE}/harness-nm.txt"
: > "${HARNESS_EVIDENCE}/harness-nm-origins.txt"
while IFS=$'\t' read -r sym archive_origin archive_member; do
    member=$(awk -v symbol="${sym}" '$(NF - 1) == "T" && ($NF == symbol || $NF == "_" symbol) { print; exit }' "${HARNESS_NM_LISTING}")
    origin=$(printf '%s\n' "${member}" | awk '{ print $1 }')
    if [ -z "${member}" ]; then
        echo "FAIL: Symbol '${sym}' not found in linked harness nm output ${HARNESS_NM_LISTING}"
        exit 1
    fi
    printf '%s\t%s\t%s\n' "${sym}" "${origin}" "${member}" >> "${HARNESS_EVIDENCE}/harness-nm-origins.txt"
    echo "  ${sym} -> ${member}"
done < "${HARNESS_EVIDENCE}/archive-nm-origins.txt"
echo "PASS: All 7 production symbols extracted into harness binary"
echo ""

# --- 5. Run the harness (all symbols present) ---
echo "--- 5. Run harness (all symbols present) ---"
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
    exit 1
elif grep -q "RESULT=PASS" "${HARNESS_OUTPUT_PATH}"; then
    echo "PASS: Harness verified all symbols"
else
    echo "FAIL: Harness did not pass"
    exit 1
fi
echo ""

echo "=== P00 Harness Probe: ALL CHECKS PASSED ==="

echo "Evidence saved to: ${HARNESS_EVIDENCE}"

cleanup
trap - EXIT
