# Phase 08: Color Parser Fix — Implementation

## Phase ID
`PLAN-20260224-RES-SWAP.P08`

## Prerequisites
- Required: Phase 07a (Color TDD Verification) completed
- Expected: All color tests exist and fail (RED)

## Requirements Implemented (Expanded)

### REQ-RES-066: rgb(r, g, b) — 8-bit, alpha=0xFF
### REQ-RES-067: rgba(r, g, b, a) — 8-bit all channels
### REQ-RES-068: rgb15(r, g, b) — 5-bit, CC5TO8 conversion
### REQ-RES-069: C integer formats (decimal, 0x hex, 0 octal)
### REQ-RES-070: Clamp negative to 0 with warning
### REQ-RES-071: Clamp overflow to max with warning
### REQ-RES-072: Unrecognized format → error, 0x00000000
### REQ-RES-073: Serialize opaque as `rgb(0x%02x, 0x%02x, 0x%02x)`
### REQ-RES-074: Serialize transparent as `rgba(0x%02x, 0x%02x, 0x%02x, 0x%02x)`

## Implementation Tasks

### Files to modify
- `rust/src/resource/resource_type.rs`
  - Implement `parse_c_color()`:
    - Try matching `rgb(...)`, `rgba(...)`, `rgb15(...)` patterns
    - Parse components using C-compatible integer parsing (support 0x, 0 prefix)
    - Clamp with warnings
    - CC5TO8: `((x as u8) << 3) | ((x as u8) >> 2)`
    - Return (r, g, b, a) tuple
  - Implement `serialize_color()`:
    - If a == 0xff: `format!("rgb(0x{:02x}, 0x{:02x}, 0x{:02x})", r, g, b)`
    - Else: `format!("rgba(0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x})", r, g, b, a)`
  - Add `parse_c_int(s: &str) -> Option<i32>` helper for C-format integer parsing
    - `0x...` → hex
    - `0...` → octal (if starts with 0 and has more digits)
    - Otherwise → decimal
  - Replace `todo!()` stubs with real implementations
  - marker: `@plan PLAN-20260224-RES-SWAP.P08`
  - marker: `@requirement REQ-RES-066-074`

### Key implementation detail: C `sscanf %i` semantics
The C code uses `sscanf(descriptor, "rgb ( %i , %i , %i )", ...)` where `%i`
accepts decimal, hex (0x), and octal (0) prefixes. The Rust implementation
must match this behavior exactly.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `parse_c_color()` fully implemented
- [ ] `serialize_color()` fully implemented
- [ ] `parse_c_int()` helper implemented
- [ ] No `todo!()` markers remain
- [ ] Plan markers present

## Semantic Verification Checklist
- [ ] All P07 color tests pass (GREEN)
- [ ] rgb() with hex values works: `rgb(0x1a, 0x00, 0x1a)`
- [ ] rgba() with all channels works
- [ ] rgb15() with CC5TO8 conversion works
- [ ] Clamping works for negative and overflow
- [ ] Serialization format matches C exactly (lowercase hex, 0x prefix, 2-digit pad)
- [ ] Roundtrip works (parse → serialize → same string)

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/resource_type.rs
# Expected: 0 matches
```

## Success Criteria
- [ ] All P07 tests pass
- [ ] No deferred implementation markers
- [ ] Lint/format/test gates pass

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/resource_type.rs`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P08.md`
