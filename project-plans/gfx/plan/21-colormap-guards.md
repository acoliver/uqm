# Phase 21: Colormap FFI Bridge

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P21`

## Prerequisites
- Required: Phase P20a (DCQ Implementation Verification) completed
- Expected: Canvas FFI bridge fully implemented (P17)
- Expected: DCQ FFI bridge fully implemented (P20)
- Expected: Rust `cmap.rs` has colormap/fade support

## Requirements Implemented (Expanded)

### REQ-CMAP-010: Colormap FFI Exports
**Requirement text**: The Rust GFX backend shall export colormap management
functions via `#[no_mangle]` FFI, replacing the C `cmap.c` implementation.

Behavior contract:
- GIVEN: C code needs to set/get colormaps for fade effects
- WHEN: `rust_cmap_set(index, colormap_data)` is called
- THEN: The colormap is stored in Rust-managed state

### REQ-CMAP-020: Fade Functions
**Requirement text**: The Rust backend shall export fade-in/fade-out
functions that compute frame-by-frame fade amounts.

Behavior contract:
- GIVEN: A fade-to-black is initiated
- WHEN: `rust_cmap_fade_step(fade_state)` is called each frame
- THEN: Returns the current fade_amount (0–511 per REQ-CLR-070)

### REQ-CMAP-030: Palette Operations
**Requirement text**: The Rust backend shall export palette set/get
functions for indexed color operations.

Behavior contract:
- GIVEN: C code sets a palette via `rust_cmap_set_palette`
- WHEN: The palette is applied
- THEN: Subsequent color lookups use the new palette

## Implementation Tasks

### Colormap FFI exports (~8 functions)

| C Function (`cmap.c`) | Rust FFI Export | Purpose |
|---|---|---|
| `SetColorMap` | `rust_cmap_set` | Set active colormap |
| `GetColorMapAddress` | `rust_cmap_get` | Get colormap data |
| `XFormColorMap_step` | `rust_cmap_xform_step` | Color transform step |
| `FadeScreen` | `rust_cmap_fade_screen` | Initiate fade |
| `GetFadeAmount` | `rust_cmap_get_fade_amount` | Query current fade level |
| `TFB_SetColorMap` | `rust_cmap_tfb_set` | Low-level colormap set |
| `TFB_ColorMapFromIndex` | `rust_cmap_from_index` | Index → colormap |
| `init_colormap` / `uninit_colormap` | `rust_cmap_init` / `rust_cmap_uninit` | Lifecycle |

### Files to create
- `rust/src/graphics/cmap_ffi.rs` — Colormap FFI exports
  - ~8 `#[no_mangle]` functions
  - `catch_unwind` on all exports
  - Wire to `cmap.rs` API
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P21`
  - marker: `@requirement REQ-CMAP-010..030, REQ-FFI-030`

### Files to modify
- `rust/src/graphics/mod.rs` — Add `pub mod cmap_ffi;`
- `sc2/src/libs/graphics/sdl/rust_gfx.h` — Add `rust_cmap_*` declarations

**Note**: C file guards are NOT part of this phase. Guards are added in
P22 (Level 0) and P23 (Level 1-2). This phase focuses exclusively on
the colormap FFI bridge implementation.

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify colormap exports
grep -c '#\[no_mangle\]' rust/src/graphics/cmap_ffi.rs
# Expected: >= 8

# Verify exports are linkable
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep rust_cmap_ | wc -l
# Expected: >= 8

# Verify catch_unwind on all exports
grep -c 'catch_unwind' rust/src/graphics/cmap_ffi.rs
# Expected: >= 8
```

## Structural Verification Checklist
- [ ] `cmap_ffi.rs` created with ~8 `#[no_mangle]` exports
- [ ] Each export has `catch_unwind` wrapper
- [ ] `mod.rs` updated with `pub mod cmap_ffi`
- [ ] `rust_gfx.h` updated with colormap declarations
- [ ] Tests still pass

## Semantic Verification Checklist (Mandatory)
- [ ] Colormap FFI functions match C function signatures
- [ ] Fade step produces correct fade_amount values
- [ ] Colormap set/get round-trip preserves data
- [ ] All parameter types are C-compatible

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "todo!\|TODO\|FIXME\|HACK\|placeholder" rust/src/graphics/cmap_ffi.rs && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] Colormap FFI exports compile and link
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass
- [ ] All ~8 exports present and linkable

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/cmap_ffi.rs rust/src/graphics/mod.rs`
- blocking issues: `cmap.rs` API incompatible with C colormap signatures

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P21.md`

Contents:
- phase ID: P21
- timestamp
- files created: `rust/src/graphics/cmap_ffi.rs`
- files modified: `mod.rs`, `rust_gfx.h`
- colormap exports: count
- verification: cargo suite output
