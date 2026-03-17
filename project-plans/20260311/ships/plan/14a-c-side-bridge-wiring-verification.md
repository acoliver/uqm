# Phase 14a: C-Side Bridge Wiring Verification

## Phase ID
`PLAN-20260314-SHIPS.P14a`

## Prerequisites
- Required: Phase 14 (C-Side Bridge Wiring) completed

## Verification Commands

```bash
# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C — both paths
# Test 1: USE_RUST_SHIPS=0 (C path)
# Test 2: USE_RUST_SHIPS=1 (Rust path)
```

## Structural Verification Checklist
- [ ] `USE_RUST_SHIPS` toggle exists in build.config
- [ ] `USE_RUST_SHIPS` defined in config_unix.h
- [ ] All C files properly guarded
- [ ] FFI exports compile and link
- [ ] No symbol conflicts

## Semantic Verification Checklist
- [ ] C-only build still works (guards preserve old path)
- [ ] Rust-enabled build compiles and links
- [ ] FFI functions are callable from C
- [ ] Element callbacks registered through Rust work in C battle loop
- [ ] Master ship list loads correctly through Rust path
- [ ] Ship spawn works through Rust path

## Gate Decision
- [ ] PASS: proceed to Phase 15
- [ ] FAIL: return to Phase 14 and fix issues
