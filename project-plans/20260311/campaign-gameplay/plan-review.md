# CAMPAIGN-GAMEPLAY Plan Review

Reviewed inputs:
- All files under `project-plans/20260311/campaign-gameplay/plan/`
- `project-plans/20260311/campaign-gameplay/requirements.md`
- `project-plans/20260311/campaign-gameplay/specification.md`

Review focus:
- gap identification
- phase ordering
- REQ coverage
- concrete paths
- template compliance
- verification adequacy
- missing phases

## Overall assessment

The plan is strong overall: it is detailed, mostly traceable to the requirements/specification, and unusually careful about verifier-facing obligations. I found **2 SUBSTANTIVE** issues and **2 PEDANTIC** issues.

---

## Findings

### 1. SUBSTANTIVE — No implementation phase actually ports the campaign loop owner in `starcon.c` / start-flow owner in `restart.c`

**Why this matters**

The plan correctly says the subsystem is currently C-owned and repeatedly warns that replacement seams must be validated before guard/wrap work. But after that caution, no concrete implementation phase is dedicated to actually replacing or wrapping the top-level owner seams that make the Rust campaign loop run.

P09 creates `rust/src/campaign/loop_dispatch.rs` with `campaign_run()`, `start_game()`, and related logic. P15 adds generic FFI and build-toggle work. But the plan never contains a phase whose implementation tasks explicitly wire `starcon.c` and `restart.c` to call the Rust loop/start-flow at runtime. Instead, P15 says validated C files “may include portions of `restart.c`, `starcon.c`, ... Final file list and exact functions are determined by the seam inventory, not assumed here.”

That is too vague for a subsystem whose core functionality is the top-level campaign loop and start/load entry. Without an explicit phase task to bridge or replace the actual owner call paths in `starcon.c` and `restart.c`, the plan can complete all Rust modules and still fail to ever execute them in-game.

**Evidence**

- `00-overview.md` identifies the current owners as `starcon.c` and `restart.c`.
- `09-deferred-transitions-dispatch.md` implements Rust loop logic but does not name the concrete C owner seam that will invoke it.
- `15-c-side-bridge-build-toggle.md` leaves the critical C file list open-ended instead of making campaign-loop/start-flow wiring mandatory.
- The requirements make top-level start flow and campaign loop behavior mandatory, not optional.

**Required fix**

Add an explicit implementation obligation, not just a verification note, for the validated `starcon.c` and `restart.c` runtime ownership seams:
- which exact function/call sites are replaced or wrapped,
- which Rust export(s) they call,
- when those guards are introduced,
- and how `USE_RUST_CAMPAIGN` switches the live runtime to the Rust loop/start flow.

This can be a dedicated bridge/wiring subphase or a strengthened P15, but it must make the top-level owner seams mandatory rather than merely possible.

---

### 2. SUBSTANTIVE — Phase ordering is backwards for export/report design versus save/load implementation that depends on it

**Why this matters**

The plan recognizes verifier-surface and claim-family reporting obligations as important, but it delays concrete export/report implementation until P14, after save/load, legacy support, dispatch, transitions, encounter, starbase, and hyper menu are already implemented.

That ordering is risky because the save/load phases already depend on context-indexed adjunct classification, covered-context normalization, and persistence-boundary evidence strategy. Those concepts are partially introduced earlier, but the actual canonical export/report surface and claim-family selection logic are deferred until near the end.

For this subsystem, export/report design is not just a final reporting feature; it defines the verifier-facing comparison objects for many covered contexts and malformed-save cases. If the export-side normalized types, classification rules, and verifier-report schema are finalized only in P14, earlier phases can easily serialize/restore state in ways that are hard to inspect or impossible to classify correctly without rework.

**Evidence**

- `05-save-summary-export-types.md` adds some export types, but the actual export contract, surface selection, outcome classification, and verifier report helpers are deferred to `14-canonical-export-document.md`.
- `07-load-deserialization-validation.md` already depends on context-indexed adjunct classification and persistence-boundary evidence obligations.
- `requirements.md` and `specification.md` make inspection-surface selection, no-mixing, malformed-save export behavior, and verifier reporting normative parts of conformance, not optional post-processing.

**Required fix**

Move the full verifier-surface contract earlier, or split P14 into:
- an earlier design/implementation phase that finalizes canonical export schema, claim-family surface selection, report entry schema, and outcome classification before P06/P07; and
- a later execution phase that fills in remaining derivation logic once save/load code exists.

At minimum, the plan needs an earlier mandatory phase that freezes the export/report contract before serialization/deserialization and validation semantics harden.

---

### 3. PEDANTIC — The plan reads “spec/requirements” but the actual authoritative files are at the campaign-gameplay root

**Why this matters**

The request said to read `spec/requirements`, but in the repo the normative files are:
- `project-plans/20260311/campaign-gameplay/requirements.md`
- `project-plans/20260311/campaign-gameplay/specification.md`

The plan itself also references `../requirements.md` and `../specification.md`, not a `spec/requirements/` directory. This is not a failure of the plan, but the path language is inconsistent with the actual tree.

**Evidence**

- No `project-plans/20260311/campaign-gameplay/spec/` directory exists.
- The plan and normative files live at the campaign-gameplay root.

**Suggested fix**

Normalize path references in review/process docs to the actual file locations to avoid audit confusion.

---

### 4. PEDANTIC — Template compliance is slightly inconsistent around phase progression text and naming

**Why this matters**

The plan is mostly consistent, but there are small template inconsistencies:
- `03a-types-domain-model-verification.md` says PASS proceeds to Phase 04, while `03b-c-state-accessor-bridge.md` sits between P03a and P04.
- `03ba-c-state-accessor-bridge-verification.md` also says PASS proceeds to Phase 04.
- The “03b/03ba” filenames map to “P03.5/P03.5a”, which is workable but slightly irregular.

These do not meaningfully block execution because the overview and phase prerequisites make the intended order clear.

**Suggested fix**

Make the phase progression text and file naming fully consistent with the numeric phase model:
- P03a should proceed to P03.5,
- P03.5a should proceed to P04,
- and file naming can either stay as-is with an explicit note or be aligned more directly.

---

## Coverage summary by requested review category

### Gap identification
- Strong overall.
- Main miss: the plan identifies top-level ownership risk but does not convert `starcon.c` / `restart.c` runtime wiring into an explicit mandatory implementation gap.

### Phase ordering
- Generally coherent bottom-up structure.
- Main issue: verifier/export/report contract lands too late relative to save/load implementation.

### REQ coverage
- Broad and thoughtful.
- Most requirement families are covered, including nuanced verifier-facing ones.
- No obvious missing major requirement family besides the runtime-owner wiring gap described in Finding 1.

### Concrete paths
- Generally good and concrete.
- Normative file path references inside the plan are concrete.
- External request wording around `spec/requirements` does not match the repo layout, but that is not a plan failure.

### Template compliance
- Mostly compliant.
- Minor inconsistency in phase progression text around P03a/P03.5.

### Verification adequacy
- Strong overall and unusually strict.
- The persistence-boundary evidence requirements are good.
- The main adequacy risk is that export/report verification is designed too late relative to implementation phases that should already be constrained by it.

### Missing phases
- No obviously missing broad functional phase like “events” or “save/load.”
- But there is effectively a missing **runtime ownership/wiring phase** for the actual `starcon.c` / `restart.c` live call path.

---

## Final verdict

**Needs revision before execution** due to the two SUBSTANTIVE issues above.

Priority order:
1. Add explicit mandatory runtime-owner wiring for `starcon.c` and `restart.c` so the Rust campaign loop/start flow actually become the live execution path.
2. Move or split export/report contract work earlier so save/load and validation phases are built against a finalized verifier-facing model instead of retrofitted later.
