# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P02a`

## Prerequisites
- Required: Phase 02 completed

## Verification Checklist

### Structural
- [ ] `component-001.md` has numbered pseudocode for property file parser
- [ ] `component-002.md` has numbered pseudocode for config API (process_resource_desc + Put functions + SaveResourceIndex)
- [ ] `component-003.md` has numbered pseudocode for resource dispatch (InstallResTypeVectors + GetResource + FreeResource + DetachResource + Remove + LoadResourceFromPath)
- [ ] All pseudocode has validation point annotations
- [ ] All pseudocode has error handling

### Semantic
- [ ] Property parser matches C `PropFile_from_string` — verify against propfile.c line by line
- [ ] TYPE:path split uses first `:` only — handles `3DOVID:a:b:c:89` correctly
- [ ] Auto-creation in Put functions matches C roundabout pattern (create then update)
- [ ] SaveResourceIndex skips entries without toString
- [ ] SaveResourceIndex correctly strips root prefix
- [ ] res_GetResource increments refcount ONLY on success
- [ ] res_FreeResource calls freeFun ONLY when refcount reaches 0
- [ ] res_DetachResource returns NULL when refcount > 1
- [ ] res_Remove calls freeFun before dropping entry
- [ ] LoadResourceFromPath sets/clears _cur_resfile_name

## Gate Decision
- [ ] PASS: proceed to P03
- [ ] FAIL: revise pseudocode
