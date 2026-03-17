# Phase 04: Loader Consolidation — Stubs

## Phase ID
`PLAN-20260314-AUDIO-HEART.P04`

## Prerequisites
- Required: Phase P03a completed
- Expected files from previous phase: updated `rust/src/sound/control.rs`

## Requirements Implemented (Expanded)

### Canonical file-backed loader ownership boundary
**Requirement text**: The final audio-heart subsystem shall consolidate resource-loading logic so that there is a single canonical loading implementation per resource type, with all entry points routing through it.

Behavior contract:
- GIVEN: Real loading logic exists inline in `heart_ffi.rs` and stubs exist in `music.rs`/`sfx.rs`
- WHEN: Canonical loader functions are created
- THEN: Both FFI and internal paths route through the same implementation

### Music file loading
**Requirement text**: When a music file is loaded, the audio-heart subsystem shall open the requested content resource through UIO, create an appropriate decoder, and return an opaque music handle representing playable content.

### Sound bank loading
**Requirement text**: When a sound-bank file is loaded, the audio-heart subsystem shall parse the bank resource, load each referenced sound entry through UIO, decode the referenced audio into mixer-ready buffers, and return an opaque handle representing the bank.

Why it matters:
- Without canonical loaders, internal paths (fileinst.rs) return empty/broken results
- Resource system integration requires real loaders at all entry points
- The ownership boundary for shared loader logic affects compile/link safety and long-term maintainability

## Implementation Tasks

### Architecture Note (mandatory before coding)

Before creating stubs, compare at least these placement options for canonical file-backed loader logic:

1. **New `rust/src/sound/loading.rs` module**
   - Pros: isolates loader logic; explicit shared location for both FFI and internal callers
   - Risks to evaluate: duplicated `extern "C"` declarations, unconditional compilation in non-`audio_heart` builds, possible reverse dependency from core Rust logic back into C-facing bindings

2. **Expand `rust/src/sound/fileinst.rs`**
   - Pros: keeps file-loading ownership near existing file-instance routing and guard logic
   - Risks to evaluate: may over-concentrate FFI/file/decode responsibilities in one module

3. **Lower-level helper module (file/decode helper under sound/)**
   - Pros: narrows ownership to file read + decoder acquisition helpers reused by higher-level loaders
   - Risks to evaluate: may spread canonical-loader semantics across multiple modules unless responsibilities are kept crisp

**Decision output required:**
- chosen placement
- why it best satisfies the ownership boundary
- why the rejected options were not chosen
- whether non-`audio_heart` builds still compile/link cleanly with the chosen module inclusion strategy

### Stub creation

Create the chosen shared loader location with:
- canonical music loader stub: `pub fn load_music_canonical(filename: &str) -> AudioResult<MusicRef>`
- canonical bank loader stub: `pub fn load_sound_bank_canonical(filename: &str) -> AudioResult<SoundBank>`
- helper seams that P05/P06 will use:
  - UIO read seam (function or trait-backed helper)
  - decoder factory seam
  - mixer upload seam for non-streaming bank sample construction
- any required FFI declarations or wrappers, justified by the architecture note
- marker: `@plan PLAN-20260314-AUDIO-HEART.P04`

### Files to create/modify

#### Chosen loader module/file
- Add stub functions for canonical music/bank loading
- Add test-friendly seams for UIO read / decoder factory / mixer upload
- If direct `extern "C"` declarations are introduced here, document why they are valid for all relevant feature combinations

#### `rust/src/sound/mod.rs`
- Add the chosen module export if needed
- Ensure feature gating matches the architecture note's compile/link safety decision
- marker: `@plan PLAN-20260314-AUDIO-HEART.P04`

### Pseudocode traceability
- Stubs for PC-01 lines 01-18 and PC-02 lines 01-26
- Structural ownership groundwork for PC-03

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Verify non-audio_heart compilation assumptions captured in the architecture note
cargo check --workspace --all-targets 2>&1 | head -50
```

## Structural Verification Checklist
- [ ] Shared loader module/file created with stub functions
- [ ] Architecture note compares at least 3 placement options and records final choice
- [ ] UIO read / decoder factory / mixer upload seams exist at the stub boundary
- [ ] Project compiles successfully
- [ ] No panics at compile time (`todo!()` is runtime-only if still used in stubs)

## Semantic Verification Checklist
- [ ] Stub functions have correct signatures matching the canonical loader contract
- [ ] The chosen ownership boundary is explicitly justified
- [ ] Any FFI declarations introduced outside `heart_ffi.rs` are justified and compile-safe
- [ ] Non-`audio_heart` build/link impact considered and recorded
- [ ] No existing behavior is changed (stubs are not wired in yet)

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/
```

Note: `todo!()` in stub bodies is explicitly allowed in stub phases per plan template.

## Success Criteria
- [ ] Shared loader ownership boundary decided and documented
- [ ] Stub module/file exists with correct function signatures and seams
- [ ] Project compiles
- [ ] No existing tests broken

## Failure Recovery
- rollback: restore only files touched by the chosen placement

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P04.md`
