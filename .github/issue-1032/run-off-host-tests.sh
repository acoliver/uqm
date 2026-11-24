#!/usr/bin/env bash
# Use only on a disposable Linux checkout. Leaves the candidate patch applied.
set -euo pipefail
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
[[ $(uname -s) == Linux ]] || fail 'Linux required; no build was started'
[[ $# == 3 ]] || fail "usage: $0 /absolute/disposable-checkout FULL_SOURCE_SHA /absolute/checkout/ignored-receipts"
[[ $1 == /* && $3 == /* ]] || fail 'checkout and receipt paths must be absolute'
[[ $2 =~ ^[0-9a-f]{40}$ ]] || fail 'expected source SHA must be full, lowercase hex'
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
checkout=$(cd -- "$1" && pwd -P)
receipt=$(realpath -m -- "$3")
cd -- "$checkout"
[[ $(git rev-parse --show-toplevel) == "$checkout" ]] || fail 'path must be the disposable checkout root'
[[ $(git rev-parse HEAD) == "$2" ]] || fail 'HEAD differs from expected source SHA'
[[ -z $(git status --porcelain=v1 --untracked-files=all) ]] || fail 'checkout is dirty'
case "$receipt/" in "$checkout/"?*/) ;; *) fail 'receipts must be strictly under the checkout' ;; esac
git check-ignore -q -- "$receipt/" || fail 'receipt directory must be Git-ignored'
if [[ -e $receipt ]]; then
    [[ -d $receipt && -z $(find "$receipt" -mindepth 1 -print -quit) ]] || fail 'receipt directory must be empty'
fi
python3 "$here/verify-off-host.py" source "$checkout" "$here/patch-validation.json" pre
mkdir -p -- "$receipt"
export CARGO_TARGET_DIR="$receipt/target" CARGO_TERM_COLOR=never
printf 'source_sha=%s\ncheckout=%s\npatch=%s\ntarget=%s\n' "$2" "$checkout" "$here/heart-ffi-module-boundary.patch" "$CARGO_TARGET_DIR" > "$receipt/source.txt"
printf 'phase\texit\n' > "$receipt/exits.tsv"
trap 'rc=$?; printf "%s\n" "$rc" > "$receipt/runner.exit"' EXIT
run() {
    local phase=$1 rc=0
    shift
    printf '%q ' "$@" > "$receipt/$phase.command"
    printf '\n' >> "$receipt/$phase.command"
    "$@" > "$receipt/$phase.log" 2>&1 || rc=$?
    printf '%s\t%s\n' "$phase" "$rc" >> "$receipt/exits.tsv"
    printf '%s: exit %s (%s)\n' "$phase" "$rc" "$receipt/$phase.log"
    return "$rc"
}
run rustc rustc -Vv
run cargo-version cargo --version
grep -qx 'release: 1.98.0' "$receipt/rustc.log" || fail 'requires the inspected Rust 1.98.0 toolchain'
if run parent cargo test --manifest-path rust/Cargo.toml --locked -p uqm-engine --lib --no-default-features io:: -- --test-threads=1; then
    fail 'parent unexpectedly succeeded; reproduction incomplete'
fi
run parent-diagnostic python3 "$here/verify-off-host.py" parent "$receipt/parent.log"
run patch-check git apply --check --whitespace=error-all "$here/heart-ffi-module-boundary.patch"
run patch-apply git apply --whitespace=error-all "$here/heart-ffi-module-boundary.patch"
run postimage python3 "$here/verify-off-host.py" source "$checkout" "$here/patch-validation.json" post
run diff-check git diff --check
run diff git diff -- rust/crates/engine/src/sound/mod.rs
run engine-check cargo check --manifest-path rust/Cargo.toml --locked -p uqm-engine --lib --no-default-features
run io-tests cargo test --manifest-path rust/Cargo.toml --locked -p uqm-engine --lib --no-default-features io:: -- --test-threads=1
run io-count python3 "$here/verify-off-host.py" tests "$receipt/io-tests.log"
run sound-tests cargo test --manifest-path rust/Cargo.toml --locked -p uqm-engine --lib --no-default-features sound:: -- --test-threads=1
run sound-count python3 "$here/verify-off-host.py" tests "$receipt/sound-tests.log"
run audio-tests cargo test --manifest-path rust/Cargo.toml --locked -p uqm-engine --lib --no-default-features --features audio_heart -- --test-threads=1
run audio-count python3 "$here/verify-off-host.py" tests "$receipt/audio-tests.log"
printf 'Qualification passed; logs and exits: %s\n' "$receipt"
