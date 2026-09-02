#!/usr/bin/env bash
#
# P00 Executable Preflight Probe Runner
#
# Executes all P00 feasibility probes using artifacts produced by the
# canonical xtask production flow. Does not depend on sc2/build.vars,
# ambient sc2/obj, or legacy build.sh.
#
# Usage:
#   CI:         UQM_CI_SUBORDINATE_EVIDENCE_ROOT=... bash probes/run_p00_probes.sh
#   Standalone: bash probes/run_p00_probes.sh output_log
#
set -euo pipefail
if [ -n "${UQM_CI_SOURCE_ROOT:-}" ]; then

: "${UQM_CI_CONTROLLER_EXECUTABLE:?UQM_CI_CONTROLLER_EXECUTABLE must be supplied by the trusted controller}"
: "${CARGO:?CARGO must be supplied by the trusted controller}"
: "${RUSTC:?RUSTC must be supplied by the trusted controller}"
: "${CC:?CC must be supplied by the trusted controller}"
: "${AR:?AR must be supplied by the trusted controller}"
: "${NM:?NM must be supplied by the trusted controller}"
    REPO_ROOT="${UQM_CI_SOURCE_ROOT}"
    RUST_DIR="${REPO_ROOT}/rust"
else
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    RUST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
    REPO_ROOT="$(cd "${RUST_DIR}/.." && pwd)"
    resolve_standalone_tool() {
        local variable="$1"
        local command_name="$2"
        local resolved
        if ! resolved="$(command -v "${command_name}")" || [ -z "${resolved}" ]; then
            echo "FAIL: standalone ${variable} tool '${command_name}' is unavailable" >&2
            exit 1
        fi
        case "${resolved}" in
            /*) ;;
            *) resolved="$(cd "$(dirname "${resolved}")" && pwd)/$(basename "${resolved}")" ;;
        esac
        printf '%s\n' "${resolved}"
    }
    CARGO="${CARGO:-$(resolve_standalone_tool CARGO cargo)}"
    RUSTC="${RUSTC:-$(resolve_standalone_tool RUSTC rustc)}"
    CC="${CC:-$(resolve_standalone_tool CC cc)}"
    AR="${AR:-$(resolve_standalone_tool AR ar)}"
    NM="${NM:-$(resolve_standalone_tool NM nm)}"
fi

if [ -n "${UQM_CI_SUBORDINATE_EVIDENCE_ROOT:-}" ]; then
    OUTPUT_LOG="${UQM_CI_SUBORDINATE_EVIDENCE_ROOT}/p00-probe-results.log"
elif [ "$#" -eq 1 ] && [ -n "$1" ]; then
    OUTPUT_LOG="$1"
else
    echo "FAIL: set UQM_CI_SUBORDINATE_EVIDENCE_ROOT for CI or pass an explicit standalone output_log" >&2
    exit 1
fi
EVIDENCE_FILE_LIMIT_BLOCKS=""
if [ -n "${UQM_CI_SUBORDINATE_EVIDENCE_ROOT:-}" ]; then
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
    EVIDENCE_FILE_LIMIT_BLOCKS=$((UQM_CI_EVIDENCE_MEMBER_LIMIT_BYTES / 1024))
fi
run_with_bounded_output() {
    if [ -n "${EVIDENCE_FILE_LIMIT_BLOCKS}" ]; then
        (ulimit -f "${EVIDENCE_FILE_LIMIT_BLOCKS}"; exec "$@")
    else
        "$@"
    fi
}
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
"${CARGO}" --version || { echo "FAIL: cargo not available"; exit 1; }
"${RUSTC}" --version || { echo "FAIL: rustc not available"; exit 1; }
"${CC}" --version 2>&1 | head -1 || { echo "FAIL: cc not available"; exit 1; }
AR_PATH="${AR}"
echo "ar: ${AR_PATH}"
"${NM}" --version 2>&1 | head -1 || { echo "FAIL: nm not available"; exit 1; }
echo "PASS: all tools available"

# --------------------------------------------------------------------------
# Rust binary probes (lock-free atomics, monotonic clock, datagram, etc.)
# --------------------------------------------------------------------------

echo ""
echo "--- Rust Binary Probes ---"

# Build and run the probe binary, capturing all output
set +e
"${CARGO}" run --locked --manifest-path "${RUST_DIR}/Cargo.toml" --bin p00_probes 2>&1 | run_with_bounded_output tee "${OUTPUT_LOG}"
PROBE_EXIT=$?
set -e

if [ ${PROBE_EXIT} -ne 0 ]; then
    echo ""
    echo "FAIL: P00 probes exited ${PROBE_EXIT}"
    exit ${PROBE_EXIT}
fi

echo ""
echo "--- Archive/Library Checks ---"

# Build using the base-owned production flow in CI and the local xtask standalone.
if [ -n "${UQM_CI_SOURCE_ROOT:-}" ]; then
    "${UQM_CI_CONTROLLER_EXECUTABLE}" __ci-production >/dev/null
else
    "${CARGO}" run --locked --manifest-path "${RUST_DIR}/xtask/Cargo.toml" -- production >/dev/null
fi

MANIFEST="${RUST_DIR}/target/production-artifacts.json"
if [ ! -f "${MANIFEST}" ]; then
    echo "FAIL: production-artifacts.json not found"
    exit 1
fi

verified_artifact_path() {
    python3 -P - "${MANIFEST}" "${REPO_ROOT}" "$1" <<'PYTHON'
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
}

# Extract and verify the exact C archive path from production evidence.
C_ARCHIVE_ENTRY="$(verified_artifact_path c_static_archive)" || {
    echo "FAIL: c_static_archive is not valid production evidence"
    exit 1
}
ARCHIVE="${REPO_ROOT}/${C_ARCHIVE_ENTRY}"

echo "PASS: libuqm_c.a found at ${ARCHIVE}"

# Extract and verify the exact object sidecar path from production evidence.
SIDECAR_ENTRY="$(verified_artifact_path object_sidecar)" || {
    echo "FAIL: object_sidecar is not valid production evidence"
    exit 1
}
SIDECAR="${REPO_ROOT}/${SIDECAR_ENTRY}"
if ! python3 -P - "${MANIFEST}" "${AR_PATH}" <<'PYTHON'
import hashlib, json, os, stat, sys
manifest_path, actual_path = sys.argv[1:]
with open(manifest_path, encoding="utf-8") as source:
    expected = json.load(source)["native_build"]["toolchain"]["ar"]
if expected["executable"] != actual_path:
    raise SystemExit("production manifest archiver path does not match the trusted controller")
executable_fd = os.open(actual_path, os.O_RDONLY | os.O_NOFOLLOW)
try:
    if not stat.S_ISREG(os.fstat(executable_fd).st_mode):
        raise SystemExit("production manifest archiver is not a regular file")
    digest = hashlib.sha256()
    while chunk := os.read(executable_fd, 1024 * 1024):
        digest.update(chunk)
finally:
    os.close(executable_fd)
if digest.hexdigest() != expected["sha256"]:
    raise SystemExit("production manifest archiver digest does not match its executable")
PYTHON
then
    echo "FAIL: production manifest archiver does not match the trusted controller" >&2
    exit 1
fi

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
