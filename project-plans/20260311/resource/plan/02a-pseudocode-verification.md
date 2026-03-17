# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-RESOURCE.P02a`

## Prerequisites
- Phase 02 pseudocode document complete

## Structural Verification Checklist
- [ ] All pseudocode sections (PC-1 through PC-9) have numbered lines
- [ ] Each PC section maps to a specific GAP
- [ ] Pseudocode includes error paths (null checks, missing handlers, warnings)
- [ ] Integration boundaries marked (C callback invocations)
- [ ] No pseudocode contradicts the spec
- [ ] Unverified ABI details are explicitly labeled as contingent examples, not final signatures

## Semantic Verification Checklist

### PC-1/PC-2 (UNKNOWNRES)
- [ ] UNKNOWNRES loadFun stores descriptor as str_ptr (spec §5.1)
- [ ] UNKNOWNRES has no freeFun → is_value_type = true
- [ ] When type not found, handler_key = "UNKNOWNRES", is_value_type = true
- [ ] Value-type loadFun is called with handler_key (not original type_name) to find the right handler

### PC-3 (get_resource value types)
- [ ] Value types (no freeFun) skip lazy-load entirely
- [ ] Value types increment refcount
- [ ] STRING/UNKNOWNRES return str_ptr as pointer
- [ ] INT32/BOOLEAN/COLOR return num as pointer
- [ ] Heap types retain existing lazy-load behavior

### PC-4 (res_GetString)
- [ ] Returns "" for null key
- [ ] Returns "" for missing key
- [ ] Returns "" for non-STRING type entry
- [ ] Returns "" for STRING with null str_ptr
- [ ] Returns str_ptr only when all conditions pass

### PC-5 (UninitResourceSystem)
- [ ] Iterates ALL entries before dropping
- [ ] Only calls freeFun for entries with non-null ptr AND handler has free_fun
- [ ] Does not crash if handler lookup fails for an entry
- [ ] After cleanup, state is set to None

### PC-6 (SaveResourceIndex)
- [ ] Entries without toString are SKIPPED (no fallback format)
- [ ] Root filtering still applies
- [ ] Entries WITH toString are serialized normally

### PC-7 (res_OpenResFile)
- [ ] Calls `uio_stat` before `uio_fopen`
- [ ] Returns `STREAM_SENTINEL` for directories
- [ ] Returns `uio_fopen` result for regular files
- [ ] Handles `uio_stat` failure gracefully (falls through to `uio_fopen`)
- [ ] Uses preflight-confirmed ABI artifacts instead of assuming a signature prematurely

### PC-8 (LoadResourceFromPath)
- [ ] Rejects null open result before callback invocation
- [ ] Rejects `STREAM_SENTINEL` before callback invocation
- [ ] Checks `length == 0` after a valid stream open
- [ ] Warns and closes file on zero length
- [ ] Returns null on sentinel/zero length
- [ ] Does not leak file handle

## Cross-Reference Check
- [ ] PC line numbers are unique within each section
- [ ] Implementation phases reference specific PC line ranges
- [ ] No pseudocode step contradicts another

## Success Criteria
- [ ] Pseudocode is complete and internally consistent
- [ ] Contingent ABI assumptions are called out explicitly
- [ ] Semantic checks pass

## Failure Recovery
- rollback steps: `git checkout -- project-plans/20260311/resource/plan/02a-pseudocode-verification.md`
- blocking issues to resolve before next phase: missing preflight artifact for `uio_stat` ABI or unresolved sentinel-handling path

## Gate Decision
- [ ] Pseudocode is complete and verified
- [ ] Proceed to Phase 03
