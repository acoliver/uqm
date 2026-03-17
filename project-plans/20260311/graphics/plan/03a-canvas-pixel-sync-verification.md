# Phase 03a: Canvas Pixel Sync Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P03a`

## Prerequisites
- Required: Phase P03 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- canvas
```

## Structural Verification Checklist
- [ ] `rust_canvas_from_surface` imports surface pixels
- [ ] `rust_canvas_flush` function exists and is exported via `#[no_mangle]`
- [ ] `rust_canvas_destroy` calls flush before drop
- [ ] Plan/requirement traceability markers present in modified code
- [ ] 5 new tests added and passing

## Semantic Verification Checklist (Mandatory)
- [ ] Import: RGBX surface pixel → RGBA canvas pixel conversion verified
- [ ] Export: RGBA canvas pixel → RGBX surface pixel conversion verified
- [ ] Roundtrip identity: import → no-op → export preserves pixel values
- [ ] Pitch handling: surfaces with pitch != width*4 work correctly
- [ ] Partial draws: unmodified pixels survive roundtrip
- [ ] Error paths: null surface, null pixels, zero dims return error

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/canvas_ffi.rs
# Must return no matches in implementation code (test comments are acceptable)
```

## Success Criteria
- [ ] All verification commands pass
- [ ] All semantic checks pass
- [ ] No deferred implementation patterns found
