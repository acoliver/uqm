# Requirements Traceability Appendix

Plan ID: PLAN-20260314-CAMPAIGN

## Purpose

This appendix maps the normative requirement families in `requirements.md` to implementation and verification phases in this plan, including verifier-facing inspection-surface, adjunct-sensitivity, and reporting obligations.

## Traceability Table

| Requirement family / obligation | Phase(s) |
|---|---|
| New-game and load-game entry behavior | P03.5, P09, P15, P16 |
| Campaign loop dispatch and terminal conditions | P09, P15, P16 |
| Deferred transition semantics / no save mutation | P09, P16 |
| Hyperspace/interplanetary/quasispace transitions | P10, P16 |
| Encounter handoff / post-encounter campaign effects | P11, P16 |
| Starbase flow / progression-point resume / mandatory-next-action rule | P07, P12, P16 |
| Initial event registration and 18-event catalog | P04, P16 |
| §8.6 row-specific normalization / checkpoint-bundle rules | P04, P05, P14, P16 |
| Campaign save serialization / summary normalization | P05, P06, P16 |
| Campaign load restoration / resume normalization | P03.5, P07, P16 |
| General load safe-failure contract | P03.5, P07, P16 |
| Adjunct-dependency rule for covered contexts | P03.5, P07, P14, P16 |
| Cross-boundary restore failure => safe campaign-load failure | P03.5, P07, P16 |
| Unknown event selector / malformed scheduled-event rejection | P05, P07, P14, P16 |
| Legacy save compatibility | P08, P16 |
| Legacy-starbase observational exception | P08, P14, P16 |
| End-state same-subsystem round-trip | P06, P07, P08, P16 |
| Conditional export applicability rule | P05, P14, P16 |
| Campaign Canonical Export Document schema / error result | P05, P14, P16 |
| Export-success vs overall covered-context distinction | P05, P14, P16 |
| Claim-family inspection-surface selection | P05, P14, P16 |
| No-mixing raw-save/export facts within one claim family | P05, P14, P16 |
| Verifier report minimum fields | P05, P14, P16 |
| §10.1 load/export outcome classification examples/rules | P05, P14, P16 |
| Cross-build/C-vs-Rust comparative evidence | P15, P16 |
| Mandatory `restart.c` start/load owner seam wiring | P01, P03.5, P15, P16 |
| Mandatory `starcon.c` campaign-loop owner seam wiring | P01, P03.5, P15, P16 |

## Mandatory Planning Gates

Before execution begins:
- P01/P01a must show that every later P15 seam has a validated inventory row.
- P03.5/P03.5a must settle accessor ownership for activity globals, queues, starbase markers, and rollback-safe mutation ordering.
- P05/P05a must freeze canonical export schema, malformed-save error/result shape, claim-family surface selection, and verifier reporting before P06/P07 harden save/load semantics.
- P15/P16 must prove the validated `restart.c` and `starcon.c` owner seams actually switch live runtime control under `USE_RUST_CAMPAIGN`.

## Notes

- This appendix is intentionally requirement-family oriented, not implementation-file oriented.
- Section-number precision beyond the family level remains in the individual phase files where the concrete work occurs.
