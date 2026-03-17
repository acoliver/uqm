# Phase 07a: SaveResourceIndex Filtering — Verification

## Phase ID
`PLAN-20260314-RESOURCE.P07a`

## Prerequisites
- Phase 07 complete

## Structural Verification Checklist
- [ ] `SaveResourceIndex` uses `continue` for entries without `to_string_fun`
- [ ] No fallback format path remains
- [ ] Real-path `SaveResourceIndex` test coverage is present and compiles

## Semantic Verification Checklist
- [ ] `test_save_resource_index_skips_entries_without_to_string_real_path` — PASSES
- [ ] `test_save_resource_index_skips_unknownres_entries_real_path` — PASSES
- [ ] `test_save_emits_all_value_types_with_to_string` — PASSES
- [ ] `test_save_respects_root_filter_and_strip_root_real_path` — PASSES

## Regression Check
- [ ] All existing tests pass
- [ ] `cargo clippy` clean
- [ ] Config save/load round-trip still functional in engine

## Success Criteria
- [ ] Real-path `SaveResourceIndex` behavior verified
- [ ] Regression checks pass

## Failure Recovery
- rollback steps: `git checkout -- project-plans/20260311/resource/plan/07a-save-index-filtering-verification.md`
- blocking issues to resolve before next phase: missing real-path write verification for `SaveResourceIndex`

## Gate Decision
- [ ] Phase 07 complete and verified
- [ ] Proceed to Phase 08
