# Phase 20a: Init, Index, and UIO Wrappers — Implementation Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P20a`

## Prerequisites
- Required: Phase 20 completed

## Verification Checklist
- [ ] All P19 tests pass
- [ ] No `todo!()`/placeholder markers in ffi_bridge.rs
- [ ] All 38 extern "C" functions have implementations
- [ ] `_cur_resfile_name` is `#[no_mangle]` accessible from C
- [ ] Sentinel handling correct for directory paths

### Quality Gates
```bash
cargo fmt --all --check && echo "FMT OK"
cargo clippy --workspace --all-targets --all-features -- -D warnings && echo "CLIPPY OK"
cargo test --workspace --all-features && echo "TESTS OK"
```

### Symbol Verification
```bash
# Verify all required symbols are exported
cd rust && cargo build --release 2>/dev/null
nm -g target/release/libuqm_rust.a 2>/dev/null | grep -E "T _?(InitResourceSystem|UninitResourceSystem|LoadResourceIndex|SaveResourceIndex|InstallResTypeVectors|res_GetResource|res_DetachResource|res_FreeResource|res_Remove|res_HasKey|res_GetString|res_GetInteger|res_GetBoolean|res_GetColor|res_PutString|res_PutInteger|res_PutBoolean|res_PutColor|res_IsString|res_IsInteger|res_IsBoolean|res_IsColor|res_GetIntResource|res_GetBooleanResource|res_GetResourceType|CountResourceTypes|res_OpenResFile|res_CloseResFile|LoadResourceFromPath|ReadResFile|WriteResFile|GetResFileChar|PutResFileChar|PutResFileNewline|SeekResFile|TellResFile|LengthResFile|DeleteResFile|GetResourceData|FreeResourceData)" | wc -l
# Expected: 38 (all symbols)
```

## Gate Decision
- [ ] PASS: proceed to P21
- [ ] FAIL: fix implementation
