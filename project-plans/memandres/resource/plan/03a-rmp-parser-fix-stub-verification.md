# Phase 03a: .rmp Parser Fix — Stub Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P03a`

## Prerequisites
- Required: Phase 03 completed

## Verification Checklist

### Structural
- [ ] `parse_propfile()` function exists in `propfile.rs`
- [ ] `resource_type` field exists on `ResourceEntry`
- [ ] `parse_type_path()` helper exists in `index.rs`
- [ ] Case-sensitive key storage confirmed (no `.to_lowercase()` in index)
- [ ] Plan markers present in modified code

### Semantic
- [ ] `parse_propfile()` signature accepts `&str`, callback, optional prefix
- [ ] `parse_type_path()` signature accepts `&str` and returns `(Option<String>, &str)`
- [ ] Existing `PropertyFile` tests either pass or are documented as needing update

### Compilation
```bash
cargo fmt --all --check && echo "FMT OK"
cargo clippy --workspace --all-targets --all-features -- -D warnings && echo "CLIPPY OK"
cargo test --workspace --all-features 2>&1 | tail -20
```

## Gate Decision
- [ ] PASS: proceed to P04
- [ ] FAIL: fix stubs
