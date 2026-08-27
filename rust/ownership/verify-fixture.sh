#!/bin/sh
set -eu

: "${UQM_CI_SOURCE_ROOT:?UQM_CI_SOURCE_ROOT must be supplied by the trusted controller}"
: "${UQM_CI_CONTROLLER_EXECUTABLE:?UQM_CI_CONTROLLER_EXECUTABLE must name the staged base controller}"
ROOT="$UQM_CI_SOURCE_ROOT"
FIXTURES="$ROOT/rust/ownership/fixtures"
OUT="$ROOT/rust/target/ownership-strict-link-fixture"

verify_hash() {
    expected=$1
    path=$2
    actual=$(shasum -a 256 "$path" | awk '{print $1}')
    [ "$actual" = "$expected" ] || { echo "fixture provenance mismatch: $path" >&2; exit 1; }
}

verify_hash f99bf7105764ab7dbff93550b9b0efaa1e361ae4943110d53decd72492f3361f "$FIXTURES/queue-provider.rs"
verify_hash 76062c0fc7495bed6411058f7d75d4db1fb6c67a143043050bab611c90c1fd9c "$FIXTURES/queue-consumer.rs"

rm -rf "$OUT"
mkdir -p "$OUT"
rustc --edition 2021 --crate-name fixture_rust --crate-type staticlib \
    "$FIXTURES/queue-provider.rs" -o "$OUT/libfixture_rust.a"
rustc --edition 2021 "$FIXTURES/queue-consumer.rs" \
    -L native="$OUT" -l static=fixture_rust -o "$OUT/uqm-fixture"
printf '' > "$OUT/empty-provider.rs"
rustc --edition 2021 --crate-name fixture_c --crate-type staticlib \
    "$OUT/empty-provider.rs" -o "$OUT/libfixture_c.a"

"$UQM_CI_CONTROLLER_EXECUTABLE" __ci-ownership-fixture
