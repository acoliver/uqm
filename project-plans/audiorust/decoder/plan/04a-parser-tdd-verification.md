# Phase 04a: Parser TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P04a`

## Prerequisites
- Required: Phase 04 completed
- Expected files: `rust/src/sound/aiff.rs` with test module

## Verification Checklist

### Structural
- [ ] `#[cfg(test)] mod tests` block exists in `aiff.rs`
- [ ] Tests compile: `cargo test --lib --all-features -- aiff --no-run` succeeds
- [ ] At least 20 test functions defined
- [ ] `build_aiff_file()` helper exists

### Semantic
- [ ] Valid AIFF tests cover: mono8, mono16, stereo8, stereo16
- [ ] Valid AIFC test covers SDX2 compression detection
- [ ] Error tests cover all validation paths: FP-2, FP-3, FP-9, SV-2, SV-3, SV-4, SV-5, SV-6, CH-2, CH-4, CH-5, CH-6
- [ ] f80 tests cover at least 6 known sample rates
- [ ] Edge case tests: odd chunk padding, unknown chunk skip, duplicate COMM
- [ ] Tests check specific `DecodeError` variants, not just `is_err()`
- [ ] No test passes with a `todo!()` or trivial stub implementation

### Quality
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] Test names are descriptive and follow `test_<behavior>` pattern

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile-only check
cargo test --lib --all-features -- aiff --no-run

# Count test functions
grep -c "#\[test\]" src/sound/aiff.rs

# Format check
cargo fmt --all --check

# Lint check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Gate Decision
- [ ] PASS: proceed to Phase 05
- [ ] FAIL: return to Phase 04 and add missing tests
