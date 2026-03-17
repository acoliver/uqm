# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-UIO.P02a`

## Prerequisites
- Required: Phase 02 completed
- Pseudocode document exists at `project-plans/20260311/uio/plan/02-pseudocode.md`

## Structural Verification Checklist
- [ ] All 7 pseudocode components exist and are numbered
- [ ] Every pseudocode function has validation points
- [ ] Error handling is explicit in every function
- [ ] Ordering constraints are documented (mount precedence, dedup order)
- [ ] Integration boundaries are marked (FFI, archive registry, mount registry)
- [ ] Side effects are documented (errno setting, status updates)
- [ ] ABI-sensitive allocation strategy is explicit for `uio_DirList`
- [ ] Lifecycle and concurrency rules are explicit, not implied

## Semantic Verification Checklist

### Component 001: Stream State Machine
- [ ] Lines 01-04: `uio_feof` reflects actual `stream.status` field, not hardcoded
- [ ] Lines 05-08: `uio_ferror` reflects actual `stream.status` field, not hardcoded
- [ ] Lines 09-12: `uio_clearerr` resets status to STATUS_OK
- [ ] Lines 13-21: `uio_fseek` clears EOF and error on success
- [ ] Lines 22-36: `uio_fclose` frees buffer before deallocating stream
- [ ] Lines 37-43: `uio_fflush` rejects NULL with errno = EINVAL
- [ ] Lines 44-51: `uio_fwrite` sets operation and status correctly
- [ ] Lines 52-65: `uio_fputc`/`uio_fputs` set operation and status

### Component 002: errno and FFI failure containment
- [ ] Lines 66-69: Platform-appropriate errno setter identified
- [ ] Lines 70-87: exported functions are wrapped in panic containment with API-level sentinels
- [ ] Lines 88-94: pattern applies to ALL error paths and unsupported stubs, not just some

### Component 003: Mount ordering, lifecycle, and concurrency baseline
- [ ] Lines 95-104: mount metadata includes explicit placement/insertion state
- [ ] Lines 105-122: all four placement modes (TOP/BOTTOM/ABOVE/BELOW) are handled
- [ ] Lines 105-122: validation of `relative` parameter matches spec (null for TOP/BOTTOM, non-null and active for ABOVE/BELOW)
- [ ] Lines 123-141: resolution encodes the provisional rule from requirements (placement first, then longer prefix, then recency)
- [ ] Lines 142-158: unmount/repository-close lifecycle behavior is explicit
- [ ] Lines 159-165: concurrency expectations are explicit for reader/mutator serialization and same-handle synchronization

### Component 004: Archive support and live-handle safety floors
- [ ] Lines 166-193: archive index built from ZIP central directory
- [ ] Lines 194-198: lookup normalizes path before searching
- [ ] Lines 199-203: archive open creates independently live handle state
- [ ] Lines 204-221: archive handle supports read, seek, fstat
- [ ] Lines 222-227: stream API audit covers all archive-backed stream operations, not just open/read/seek
- [ ] Post-unmount safety floor is addressed for already-open archive handles/streams

### Component 005: Directory enumeration and ABI-safe DirList allocation
- [ ] Lines 228-243: merges across STDIO and ZIP mounts in precedence order
- [ ] Lines 228-243: first-seen deduplication by name
- [ ] Lines 237-241: `.rmp` ordering rule is scoped correctly to the provisional acceptance case
- [ ] Lines 244-254: all 5 match types implemented (literal, prefix, suffix, substring, regex)
- [ ] Lines 255-269: public `uio_DirList` layout remains C-compatible and bookkeeping stays private

### Component 006: FileBlock and stdio access cleanup behavior
- [ ] Lines 276-284: unsupported or partial-failure paths return clean failure sentinels and clean up allocations
- [ ] Lines 285-295: `uio_getFileLocation` has explicit archive-backed success behavior
- [ ] Lines 296-306: temp resources are cleaned up on any stdio-access failure path

### Component 007: Transplant semantics and post-unmount safety floors
- [ ] Lines 307-317: transplant creates a distinct mount record, not aliasing the original mount handle identity
- [ ] Lines 313-315: archive-backed transplant references shared backing safely without reusing original mount identity
- [ ] Lines 318-323: live directory/file/stream behavior after unmount and shutdown misuse preserves the no-crash/no-UB floor

## Requirement Traceability Check
- [ ] Every REQ-UIO-* engine-critical requirement has at least one pseudocode function reference
- [ ] Compatibility-complete families missing from current implementation (`CONC`, `FFI`, `LIFE`, `BOUND`, `INT`, `ERR-007/012`) are represented explicitly
- [ ] Implementation phases can reference specific pseudocode components/ranges
- [ ] No pseudocode function exists without a corresponding requirement or gap

## Gate Decision
- [ ] PASS: pseudocode complete and traceable, proceed to Phase 03
- [ ] FAIL: coverage gaps found — update pseudocode before proceeding
