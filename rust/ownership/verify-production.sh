#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
RUST_ROOT="$ROOT/rust"
TARGET="$RUST_ROOT/target"
MESSAGES="$TARGET/ownership-production-messages.jsonl"
PATHS="$TARGET/ownership-production-paths.txt"
REPORT="$TARGET/ownership-production-report.json"
REENTRY_MESSAGES="$TARGET/ownership-production-reentry.jsonl"

mkdir -p "$TARGET"
cargo run --quiet --manifest-path "$RUST_ROOT/ownership/Cargo.toml" -- "$ROOT" > "$TARGET/ownership-prelink-report.json"
cargo clean --manifest-path "$RUST_ROOT/Cargo.toml" -p uqm
cargo build --manifest-path "$RUST_ROOT/Cargo.toml" --release \
    --features audio_heart,linked_c_archive --bin uqm --message-format=json-render-diagnostics \
    > "$MESSAGES" 2> "$TARGET/strict-production-link.log"

python3 - "$MESSAGES" "$PATHS" <<'PY'
import json
import pathlib
import sys

messages = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines() if line]
out_dirs = [message["out_dir"] for message in messages if message.get("reason") == "build-script-executed" and "#uqm@" in message.get("package_id", "")]
executables = [message["executable"] for message in messages if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "uqm" and message.get("executable")]
rust_archives = [filename for message in messages if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "uqm_rust" for filename in message.get("filenames", []) if pathlib.Path(filename).name.startswith("libuqm_rust-") and filename.endswith(".a")]
if len(set(out_dirs)) != 1 or len(set(executables)) != 1 or len(set(rust_archives)) != 1:
    raise SystemExit(f"current Cargo invocation did not identify exact artifacts: out={out_dirs}, exe={executables}, rust={rust_archives}")
pathlib.Path(sys.argv[2]).write_text("\n".join((out_dirs[-1], rust_archives[-1], executables[-1])) + "\n")
PY

OUT_DIR=$(sed -n '1p' "$PATHS")
RUST_ARCHIVE=$(sed -n '2p' "$PATHS")
EXECUTABLE=$(sed -n '3p' "$PATHS")
C_ARCHIVE="$OUT_DIR/libuqm_c.a"
SIDECAR="$OUT_DIR/uqm-c-objects.manifest"
BUILD_REPORT="$OUT_DIR/provider-report.json"

for artifact in "$RUST_ARCHIVE" "$C_ARCHIVE" "$EXECUTABLE" "$SIDECAR" "$BUILD_REPORT"; do
    [ -f "$artifact" ] || { echo "missing exact current artifact: $artifact" >&2; exit 1; }
done

cargo run --quiet --manifest-path "$RUST_ROOT/ownership/Cargo.toml" -- \
    "$ROOT" artifacts "$RUST_ARCHIVE" "$C_ARCHIVE" "$EXECUTABLE" > "$REPORT"

FIRST=$(shasum -a 256 "$RUST_ARCHIVE" "$C_ARCHIVE" "$EXECUTABLE" "$BUILD_REPORT" "$REPORT")
cargo clean --manifest-path "$RUST_ROOT/Cargo.toml" -p uqm
cargo build --manifest-path "$RUST_ROOT/Cargo.toml" --release \
    --features audio_heart,linked_c_archive --bin uqm --message-format=json-render-diagnostics \
    > "$REENTRY_MESSAGES" 2>> "$TARGET/strict-production-link.log"
python3 - "$REENTRY_MESSAGES" "$PATHS" <<'PY'
import json
import pathlib
import sys

messages = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines() if line]
expected = pathlib.Path(sys.argv[2]).read_text().splitlines()
out_dirs = {message["out_dir"] for message in messages if message.get("reason") == "build-script-executed" and "#uqm@" in message.get("package_id", "")}
executables = {message["executable"] for message in messages if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "uqm" and message.get("executable")}
rust_archives = {filename for message in messages if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "uqm_rust" for filename in message.get("filenames", []) if pathlib.Path(filename).name.startswith("libuqm_rust-") and filename.endswith(".a")}
if out_dirs != {expected[0]} or rust_archives != {expected[1]} or executables != {expected[2]}:
    raise SystemExit("reentry Cargo messages do not identify the same exact artifacts")
PY
SECOND=$(shasum -a 256 "$RUST_ARCHIVE" "$C_ARCHIVE" "$EXECUTABLE" "$BUILD_REPORT" "$REPORT")
[ "$FIRST" = "$SECOND" ] || { echo "exact production artifacts changed across reentry" >&2; exit 1; }

printf 'strict production ownership verified: report=%s\n' "$REPORT"
