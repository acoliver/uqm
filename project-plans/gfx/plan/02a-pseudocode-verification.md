# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P02a`

## Prerequisites
- Required: Phase P02 completed
- Expected artifacts: 7 pseudocode component files

## Verification Commands

```bash
# Verify all pseudocode files exist and are non-trivial
for f in project-plans/gfx/analysis/pseudocode/component-*.md; do
  lines=$(wc -l < "$f")
  echo "$f: $lines lines"
  if [ "$lines" -lt 20 ]; then echo "FAIL: too short"; fi
done

# Verify numbered format is used (should find lines starting with digits)
for f in project-plans/gfx/analysis/pseudocode/component-*.md; do
  count=$(grep -cE '^ *[0-9]+:' "$f")
  echo "$f: $count numbered lines"
  if [ "$count" -lt 5 ]; then echo "FAIL: insufficient numbered pseudocode"; fi
done
```

## Structural Verification Checklist
- [ ] All 7 component files exist
- [ ] Each uses numbered algorithmic format
- [ ] Each has validation points section
- [ ] Each has error handling section
- [ ] Each has ordering constraints section
- [ ] Each has integration boundaries section
- [ ] Each has side effects section

## Semantic Verification Checklist (Mandatory)
- [ ] Component-001 covers REQ-INIT-095 (already-initialized guard)
- [ ] Component-001 covers REQ-INIT-097 (partial init cleanup)
- [ ] Component-002 covers REQ-PRE-010 (blend mode) and REQ-PRE-020 (clear)
- [ ] Component-003 covers REQ-SCR-090 (screen 1 skip)
- [ ] Component-003 covers REQ-SCR-160 (negative rect check)
- [ ] Component-003 covers REQ-SCR-170 (pixel slice with pitch)
- [ ] Component-003 covers REQ-ERR-065 (no copy on update fail)
- [ ] Component-004 covers REQ-SCALE-060 (RGBX→RGBA)
- [ ] Component-004 covers REQ-SCALE-070 (RGBA→RGBX)
- [ ] Component-004 covers REQ-SCALE-050 (source rect scaling)
- [ ] Component-005 covers REQ-CLR-020/030 (blend mode before color)
- [ ] Component-006 covers REQ-POST-020 (no texture/upload in postprocess)

## Success Criteria
- [ ] All structural checks pass
- [ ] All semantic checks pass

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P02a.md`
