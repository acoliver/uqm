# Phase 11a: Race Batch 1 Verification

## Phase ID
`PLAN-20260314-SHIPS.P11a`

## Prerequisites
- Required: Phase 11 (Race Batch 1) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Per-Race Verification Matrix

| Race | Template Constants | Weapon | Special | AI | Collision |
|------|-------------------|--------|---------|----|-----------| 
| Arilou | [ ] | [ ] autoaim laser | [ ] teleport | [ ] | [ ] default |
| Human | [ ] | [ ] spread laser | [ ] nuke | [ ] | [ ] default |
| Spathi | [ ] | [ ] torpedo | [ ] BUTT missile | [ ] | [ ] default |
| Supox | [ ] | [ ] globule | [ ] lateral thrust | [ ] | [ ] default |
| Thraddash | [ ] | [ ] flame | [ ] afterburner trail | [ ] | [ ] default |
| Yehat | [ ] | [ ] twin pulse | [ ] shield | [ ] | [ ] shield override |
| Druuge | [ ] | [ ] mass driver + recoil | [ ] furnace | [ ] | [ ] default |
| Ilwrath | [ ] | [ ] hellfire | [ ] cloak | [ ] | [ ] default |

## Semantic Verification Checklist
- [ ] All 8 races return correct constants in descriptor_template()
- [ ] All 8 races have weapon behavior tests that verify observable outcomes
- [ ] All 8 races have special ability tests that verify observable outcomes
- [ ] All 8 races have AI tests
- [ ] Registry dispatch returns real implementations for all 8 races
- [ ] No StubShip for these 8 races

## Gate Decision
- [ ] PASS: proceed to Phase 12
- [ ] FAIL: return to Phase 11 and fix issues
