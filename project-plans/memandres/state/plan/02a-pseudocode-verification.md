# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P02a`

## Prerequisites
- Required: Phase P02 completed
- Expected files: `analysis/pseudocode/component-001.md`

## Structural Verification
- [ ] `analysis/pseudocode/component-001.md` exists and has numbered lines
- [ ] All sections present: A (redirects), B (seek fix), C (deadlock fix), D (return semantics)
- [ ] Redirect pseudocode covers all 7 functions: Open, Close, Read, Write, Seek, Length, Delete

## Semantic Verification
- [ ] OpenStateFile redirect: validates index, calls rust_open_state_file, returns pointer
- [ ] CloseStateFile redirect: computes index from pointer arithmetic, calls rust_close
- [ ] ReadStateFile redirect: computes index, passes buffer/size/count through
- [ ] WriteStateFile redirect: computes index, passes buffer/size/count through
- [ ] SeekStateFile redirect: computes index, casts offset to i64, passes whence
- [ ] LengthStateFile redirect: computes index, casts result to DWORD
- [ ] DeleteStateFile redirect: validates index, calls rust_delete (already index-based)
- [ ] Seek fix: new_pos computed per whence, negative clamp to 0, NO upper clamp
- [ ] Read: checks `self.ptr >= self.data.len()` (physical size)
- [ ] Write: grows to `max(required_end, data.len() * 3/2)`, updates `used` on advance
- [ ] Length: returns `self.used` (not data.len())
- [ ] Open: allocates `vec![0; size_hint]`, sets `used = 0` for write mode
- [ ] Copy: single lock, snapshot source, then mutate

## Traceability Check
- [ ] Implementation phases (P03–P08) can reference specific line ranges
- [ ] Each requirement maps to at least one pseudocode section

## Gate Decision
- [ ] PASS: proceed to P03
- [ ] FAIL: revise pseudocode
