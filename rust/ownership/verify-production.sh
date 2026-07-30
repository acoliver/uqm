#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
RUST_ROOT="$ROOT/rust"
TARGET="$RUST_ROOT/target"
MANIFEST="$TARGET/production-artifacts.json"
REPORT="$TARGET/ownership-production-report.json"

cargo run --quiet --locked --manifest-path "$RUST_ROOT/xtask/Cargo.toml" -- verify

artifact_path() {
    jq -er --arg role "$1" \
        '([.artifacts[] | select(.role == $role)] | if length == 1 then .[0].path else error("role must occur exactly once: " + $role) end)' \
        "$MANIFEST"
}

RUST_ARCHIVE="$ROOT/$(artifact_path rust_static_archive)"
C_ARCHIVE="$ROOT/$(artifact_path c_static_archive)"
EXECUTABLE="$ROOT/$(artifact_path executable)"

cargo run --quiet --locked --manifest-path "$RUST_ROOT/ownership/Cargo.toml" -- \
    "$ROOT" artifacts "$RUST_ARCHIVE" "$C_ARCHIVE" "$EXECUTABLE" > "$REPORT"

printf 'strict production ownership verified: report=%s\n' "$REPORT"
