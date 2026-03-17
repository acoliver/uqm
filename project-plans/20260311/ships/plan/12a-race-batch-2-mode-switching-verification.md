# Phase 12a: Race Batch 2 Verification

## Phase ID
`PLAN-20260314-SHIPS.P12a`

## Prerequisites
- Required: Phase 12 (Race Batch 2) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Per-Race Verification Matrix

| Race | Template | Weapon | Special | Mode Switch | Private Data | AI | Edge Cases |
|------|----------|--------|---------|-------------|--------------|----|-----------| 
| Androsynth | [ ] | [ ] bubbles | [ ] blazer toggle | [ ] char mutate | [ ] custom data | [ ] | [ ] frame swap |
| Mmrnmhrm | [ ] | [ ] laser/missile | [ ] transform | [ ] full swap | [ ] form state | [ ] | [ ] char swap |
| Orz | [ ] | [ ] turret | [ ] marines | N/A | [ ] marine tracking | [ ] | [ ] boarding |
| Pkunk | [ ] | [ ] spread | [ ] insult | N/A | [ ] resurrect state | [ ] | [ ] resurrection |
| Shofixti | [ ] | [ ] dart | [ ] glory device | N/A | N/A | [ ] | [ ] self-damage |
| Syreen | [ ] | [ ] beam | [ ] siren song | N/A | N/A | [ ] | [ ] crew steal |
| Utwig | [ ] | [ ] bolt | [ ] absorb shield | N/A | N/A | [ ] | [ ] damage→energy |
| Vux | [ ] | [ ] laser | [ ] limpet | N/A | [ ] limpet state | [ ] | [ ] warp-in |

## Semantic Verification Checklist
- [ ] All 8 races have correct descriptor constants
- [ ] Mode-switching races correctly mutate and restore characteristics
- [ ] Private data is properly allocated and freed
- [ ] Edge cases are tested and pass
- [ ] 16 total races (Batch 1 + 2) compile and work

## Gate Decision
- [ ] PASS: proceed to Phase 13
- [ ] FAIL: return to Phase 12 and fix issues
