#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${RUST_DIR}/.." && pwd)"
EVIDENCE_ROOT="${RUST_DIR}/target/issue23-acceptance"
MANIFEST="${RUST_DIR}/target/production-artifacts.json"
PROOF_BIN="${RUST_DIR}/target/debug/uqm-gameplay-proof"

rm -rf "${EVIDENCE_ROOT}"
mkdir -p "${EVIDENCE_ROOT}"
preserve_harness_evidence() {
    for source in /tmp/p00-harness-evidence /tmp/p00-menu-binding-evidence; do
        if [ -d "${source}" ]; then
            cp -R "${source}" "${EVIDENCE_ROOT}/"
        fi
    done
}
trap preserve_harness_evidence EXIT HUP INT TERM

cargo run --locked --manifest-path "${RUST_DIR}/xtask/Cargo.toml" -- test
cargo run --locked --manifest-path "${RUST_DIR}/xtask/Cargo.toml" -- probe
cargo run --locked --manifest-path "${RUST_DIR}/xtask/Cargo.toml" -- production
cargo run --locked --manifest-path "${RUST_DIR}/xtask/Cargo.toml" -- harness
cargo build --locked --manifest-path "${RUST_DIR}/Cargo.toml" --bin uqm-gameplay-proof

run_proof() {
    local name="$1"
    local script="$2"
    local output="${EVIDENCE_ROOT}/${name}"
    "${PROOF_BIN}" run "${REPO_ROOT}" "${MANIFEST}" "${script}" "${output}"
    "${PROOF_BIN}" validate "${output}/lcar-v1.json"
}

run_proof menu "${SCRIPT_DIR}/main-menu-v1.json"
run_proof communication-held-seek "${SCRIPT_DIR}/real-sol-probe-held-seek-completion.json"
run_proof communication-replay "${SCRIPT_DIR}/real-sol-probe-completion.json"
run_proof planetside "${SCRIPT_DIR}/real-sol-rust-planetside-smoke.json"
run_proof battle-first "${SCRIPT_DIR}/battle-v1.json"
run_proof battle-second "${SCRIPT_DIR}/battle-v1.json"
"${PROOF_BIN}" compare-battle \
    "${EVIDENCE_ROOT}/battle-first/lcar-v1.json" \
    "${EVIDENCE_ROOT}/battle-second/lcar-v1.json"
"${PROOF_BIN}" validate-negative-fixtures
