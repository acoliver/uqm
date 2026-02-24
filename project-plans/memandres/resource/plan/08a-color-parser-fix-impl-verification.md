# Phase 08a: Color Parser Fix — Implementation Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P08a`

## Prerequisites
- Required: Phase 08 completed

## Verification Checklist
- [ ] All P07 color tests pass
- [ ] No `todo!()` in resource_type.rs
- [ ] `parse_c_int` handles hex, decimal, octal correctly
- [ ] CC5TO8 formula: `(x << 3) | (x >> 2)` — verify rgb15(31,0,0) → (255,0,0,255)
- [ ] Serialization format is lowercase hex with `0x` prefix

### Quality Gates
```bash
cargo fmt --all --check && echo "FMT OK"
cargo clippy --workspace --all-targets --all-features -- -D warnings && echo "CLIPPY OK"
cargo test --workspace --all-features && echo "TESTS OK"
```

## Gate Decision
- [ ] PASS: proceed to P09
- [ ] FAIL: fix implementation
