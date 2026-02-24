# Phase 08a: C Header Redirect — Implementation Verification

## Phase ID
`PLAN-20260224-MEM-SWAP.P08a`

## Prerequisites
- Required: Phase 08 completed
- `USE_RUST_MEM` enabled in `config_unix.h`

## Verification Checks

### Build
- [ ] Full clean build succeeds with `USE_RUST_MEM` defined
- [ ] No linker errors
- [ ] No new compiler warnings
- [ ] `w_memlib.c` is NOT compiled (excluded by Makeinfo)

### Binary Verification
- [ ] Produced binary exists
- [ ] Binary launches without immediate crash

### Rust Side
- [ ] `cargo test --workspace --all-features` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes

### Symbol Verification
- [ ] `nm` on the binary or library shows `rust_hmalloc`, `rust_hfree`, etc. as defined symbols
- [ ] `HMalloc` does NOT appear as a defined symbol (it's now a macro, not a function)

## Verification Commands

```bash
# Flag is active
grep '^#define USE_RUST_MEM' sc2/config_unix.h

# Build
cd sc2 && ./build.sh uqm

# Symbol check (on the Rust library)
nm rust/target/release/libuqm_rust.a 2>/dev/null | grep -E 'rust_hmalloc|rust_hfree|rust_hcalloc|rust_hrealloc' | head -10

# Rust checks
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Gate Decision
- [ ] PASS: proceed to Phase 09
- [ ] FAIL: fix build/linker issues, or rollback to Phase 06
