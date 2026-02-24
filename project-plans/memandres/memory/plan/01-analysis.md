# Phase 01: Analysis

## Phase ID
`PLAN-20260224-MEM-SWAP.P01`

## Prerequisites
- Required: Phase 00a (Preflight Verification) completed
- All preflight checks passed

## Purpose
Produce domain and flow analysis artifacts for the memory allocator swap.

## Analysis Outputs

### Entity/State Analysis
- See `analysis/domain-model.md` for complete entity analysis of all 6 functions
- Key entities: `HMalloc`, `HFree`, `HCalloc`, `HRealloc`, `mem_init`, `mem_uninit`
- State transitions: None — these are stateless allocation wrappers (no global state beyond the heap)

### Edge/Error Handling Map

| Edge Case | C Behavior | Rust Behavior | Risk |
|---|---|---|---|
| OOM (size > 0) | Fatal log + abort/exit | Fatal log + abort | LOW — both crash |
| Zero-size alloc | Implementation-defined | Guaranteed non-null | LOW — Rust is safer |
| NULL free | No-op | No-op | NONE |
| Zero-size calloc | Latent UB | Safe | LOW — Rust fixes bug |
| Zero-size realloc | Implementation-defined | Free + 1-byte alloc | LOW — Rust is safer |
| Cross-alloc free | Safe (same heap) | Safe (same heap) | NONE |

### Integration Touchpoints

1. **`memlib.h`** — the ONLY include path for all 322+ call sites
2. **`config_unix.h`** — compile-time flag definition
3. **`Makeinfo`** — build system file exclusion
4. **`w_memlib.c`** — `#error` guard addition
5. **`memory.rs`** — log level semantic fix (minor)
6. **`logging.rs`** — `Fatal` alias addition (minor)

### Old Code to Replace/Remove

| File | Action |
|---|---|
| `w_memlib.c` | Excluded from build when `USE_RUST_MEM` defined. NOT deleted — kept for fallback. `#error` guard added. |
| `memlib.h` | Modified — `#ifdef` block added around declarations. Original declarations preserved in `#else` branch. |

### Requirements Mapped to Analysis

| Requirement | Analysis Artifact |
|---|---|
| REQ-MEM-001 | domain-model.md: Macro Interaction Safety section |
| REQ-MEM-002 | domain-model.md: w_memlib.c entity |
| REQ-MEM-003 | Makeinfo conditional pattern (verified in preflight) |
| REQ-MEM-004 | config_unix.h pattern (verified in preflight) |
| REQ-MEM-005 | domain-model.md: OOM Path Differences section |
| REQ-MEM-006 | domain-model.md: all entity tables |
| REQ-MEM-007 | domain-model.md: Header Macro Redirect + original path |

## Verification Commands

```bash
# Verify analysis artifacts exist
test -f project-plans/memandres/memory/analysis/domain-model.md && echo "OK"
test -f project-plans/memandres/memory/analysis/pseudocode/component-001.md && echo "OK"
```

## Structural Verification Checklist
- [ ] `analysis/domain-model.md` created with entity analysis
- [ ] All 6 functions analyzed (HMalloc, HFree, HCalloc, HRealloc, mem_init, mem_uninit)
- [ ] Zero-size handling documented
- [ ] OOM path differences documented
- [ ] Cross-allocation safety documented
- [ ] Integration touchpoints listed

## Semantic Verification Checklist (Mandatory)
- [ ] All requirements have corresponding analysis artifacts
- [ ] Edge cases are mapped with risk levels
- [ ] Old code replacement plan is explicit
- [ ] No missing integration points

## Success Criteria
- [ ] Domain model covers all 6 functions with C vs Rust comparison
- [ ] All edge cases documented with risk assessment
- [ ] Integration touchpoints identified

## Failure Recovery
- Rollback: N/A — analysis phase produces documentation only
- Blocking issues: any missing C/Rust source files would require plan revision

## Phase Completion Marker
Create: `project-plans/memandres/memory/.completed/P01.md`
