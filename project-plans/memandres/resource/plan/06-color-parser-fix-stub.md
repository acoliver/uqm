# Phase 06: Color Parser Fix — Stub

## Phase ID
`PLAN-20260224-RES-SWAP.P06`

## Prerequisites
- Required: Phase 05a (.rmp Parser Implementation Verification) completed
- Expected: Working `parse_propfile()` and `parse_type_path()` functions

## Requirements Implemented (Expanded)

### REQ-RES-066: rgb() Format
**Requirement text**: The system shall support `rgb(r, g, b)` format with
8-bit integer components and implicit alpha of 0xFF.

### REQ-RES-067: rgba() Format
**Requirement text**: The system shall support `rgba(r, g, b, a)` format.

### REQ-RES-068: rgb15() Format
**Requirement text**: The system shall support `rgb15(r, g, b)` format with
5-bit components converted via `(x << 3) | (x >> 2)`.

### REQ-RES-069: C Integer Formats
**Requirement text**: Accept decimal, hex (`0x`), and octal (`0`) formats.

### REQ-RES-070-072: Clamping and Error Handling
### REQ-RES-073-074: Color Serialization

## Implementation Tasks

### Files to modify
- `rust/src/resource/resource_type.rs`
  - Add `parse_c_color(descriptor: &str) -> Result<(u8, u8, u8, u8), ColorError>`
    - Parses `rgb()`, `rgba()`, `rgb15()` formats
    - Supports decimal, hex, octal component values
    - Clamping with warnings
    - Returns RGBA tuple
  - Add `serialize_color(r: u8, g: u8, b: u8, a: u8) -> String`
    - Formats as `rgb(0x%02x, ...)` or `rgba(0x%02x, ...)` depending on alpha
  - Stub both with `todo!()`
  - Mark existing `ColorResource::from_hex` as `#[deprecated]` (wrong format for .rmp)
  - marker: `@plan PLAN-20260224-RES-SWAP.P06`
  - marker: `@requirement REQ-RES-066-074`

### Pseudocode traceability
- Color parsing follows C `DescriptorToColor` in resinit.c
- Color serialization follows C `ColorToString` in resinit.c

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `parse_c_color()` function exists (stub)
- [ ] `serialize_color()` function exists (stub)
- [ ] Existing `ColorResource::from_hex` marked deprecated
- [ ] Plan markers present

## Semantic Verification Checklist
- [ ] Compilation succeeds
- [ ] Function signatures match needed behavior

## Success Criteria
- [ ] Stubs compile
- [ ] Existing tests still pass (deprecated method still available)

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/resource_type.rs`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P06.md`
