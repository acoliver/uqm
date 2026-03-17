# Phase 08a: res_OpenResFile Sentinel — Verification

## Phase ID
`PLAN-20260314-RESOURCE.P08a`

## Prerequisites
- Phase 08 complete
- Phase 0.5 recorded one concrete repository path, fixture, or directory-backed resource case for sentinel validation

## Structural Verification Checklist
- [ ] `uio_stat` declared in extern block using the preflight-confirmed ABI
- [ ] `res_OpenResFile` has directory detection before `uio_fopen`
- [ ] `STREAM_SENTINEL` returned for directories
- [ ] `LoadResourceFromPath` explicitly rejects `STREAM_SENTINEL`
- [ ] Tests added and compile
- [ ] This phase records the exact concrete sentinel-validation path/fixture identified in Phase 0.5

## Semantic Verification Checklist
- [ ] `test_length_res_file_returns_1_for_sentinel` — PASSES
- [ ] `test_stream_sentinel_constant_is_all_bits_set` — PASSES
- [ ] `test_load_resource_from_path_rejects_stream_sentinel_without_callback` — PASSES
- [ ] `test_load_resource_from_path_zero_length_closes_and_skips_callback` — PASSES
- [ ] Existing file I/O tests still pass

## Integration Verification
- [ ] Full engine build succeeds with `USE_RUST_RESOURCE` using the authoritative preflight-recorded build command
- [ ] Boot to main menu — resource loading works
- [ ] Execute the exact concrete loose-file speech or directory-backed resource path/fixture recorded in Phase 0.5 and verify it exercises the sentinel branch successfully
- [ ] Record the exact path/fixture, command/harness, and factual result in the phase completion marker

## Success Criteria
- [ ] Sentinel-producing and sentinel-rejecting behaviors are both verified
- [ ] Verification uses a concrete recorded path/fixture, not an execution-time placeholder
- [ ] Integration verification passes

## Failure Recovery
- rollback steps: `git checkout -- project-plans/20260311/resource/plan/08a-res-openresfile-sentinel-verification.md`
- blocking issues to resolve before next phase: missing proof that `LoadResourceFromPath` blocks sentinel callbacks, or no concrete sentinel-validation path/fixture recorded

## Gate Decision
- [ ] Phase 08 complete and verified
- [ ] Proceed to Phase 09
