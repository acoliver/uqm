# Phase 05a: .rmp Parser Fix — Implementation Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P05a`

## Prerequisites
- Required: Phase 05 completed

## Verification Checklist

### Structural
- [ ] No `todo!()` in `propfile.rs` or `index.rs` implementation code
- [ ] No `TODO`/`FIXME`/`HACK` markers in implementation code

### Semantic
- [ ] All P04 parser tests pass
- [ ] Parsing `"GFXRES:base/comm/arilou/arilou.ani"` → correct type and path
- [ ] Parsing `"3DOVID:a:b:c:89"` → type=`"3DOVID"`, path=`"a:b:c:89"`
- [ ] Keys are case-sensitive: `"comm.Arilou"` != `"comm.arilou"`
- [ ] Inline comments stripped: `"key = value # comment"` → value=`"value"`
- [ ] Prefix applied: prefix `"config."` + key `"sfxvol"` → `"config.sfxvol"`

### Quality Gates
```bash
cargo fmt --all --check && echo "FMT OK"
cargo clippy --workspace --all-targets --all-features -- -D warnings && echo "CLIPPY OK"
cargo test --workspace --all-features && echo "TESTS OK"
grep -RIn "TODO\|FIXME\|HACK" rust/src/resource/propfile.rs rust/src/resource/index.rs | grep -v "test\|Test" && echo "CLEAN" || echo "MARKERS FOUND"
```

## Gate Decision
- [ ] PASS: proceed to P06
- [ ] FAIL: fix implementation
