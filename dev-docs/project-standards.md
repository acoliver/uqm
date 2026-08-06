# UQM Rust Port Standards

Mandatory development standards for this repository.

Audience: human contributors, LLM contributors, reviewers.

These standards are normative. "Must" and "must not" are requirements.

---

## 1) What this project is

This repository is porting The Ur-Quan Masters from C to Rust, subsystem by
subsystem. Two facts follow from that and shape everything below.

**The end state is zero first-party native code.** No C, C++, Objective-C,
assembly, implementation-bearing headers, generated native source, helper or
test native source, or internal migration ABI. Retained C is transitional and
is being deleted, not maintained. Adding C to solve a problem moves the project
backwards; port the subsystem instead.

**External libraries are the exception, and they are governed.** Only entries in
`rust/ownership/external-native-allowlist.json` may be linked. Each carries a
licence, provenance, security owner, update policy and the targets it applies
to. Vendoring or patching any of them is forbidden, because a patched copy is
first-party native code again. `build.rs` derives what it links from that file
and fails on an entry that does not declare its policy.

---

## 2) Core engineering principles

1. Fail fast. Prefer an invariant that reports a violation over a fallback that
   hides it.
2. Keep runtime behaviour deterministic and reproducible, because the evidence
   contract depends on replay.
3. Prefer strong typing and explicit domain modelling over shortcuts.
4. Optimise for maintainability over cleverness.
5. Treat lint, tests and docs as part of the change, not optional polish.

### Fail fast, specifically

Defensive layers that hedge against possible upstream bugs are an antipattern
here: they make the code hard to reason about and hide real defects. Do not add
guards, fallbacks or swallowed errors to work around a suspected bug. Find the
bug.

The exception is genuinely external input whose shape this program cannot
control: third-party data formats, network I/O, operating-system variance and
untrusted content parsing. Defensive handling is correct there.

---

## 3) Ownership discipline

Exactly one implementation owns each subsystem. A domain that has been cut over
to Rust must not retain a native fallback, a dormant parallel Rust path, a
feature-disabled replacement or a duplicate authority.

The same rule applies to configuration. Which transitional paths are active is
decided in one place; a second place that can also decide is a duplicate
authority even when the two currently agree.

When a subsystem is cut over, the superseded provider, bridge, object and build
entry, generated binding and ownership flag are removed in the same change.

---

## 4) Rust language standards

### Safety and correctness

`unsafe` is unavoidable at the C boundary and is therefore permitted only there.
Every `unsafe` block must be confined to an FFI or adapter module, must be as
small as the operation requires, and must carry a comment stating what makes it
sound. Do not spread `unsafe` into logic modules; wrap the boundary and keep the
rest safe.

Production paths must not rely on panic-driven control flow. Use `Result` and
`Option` with typed error propagation.

### Strong typing

- Do not use weak or stringly-typed values where an enum or struct fits.
- Public module APIs must have explicit types and stated contracts.
- Struct layouts mirrored from C must state which C type they mirror, and any
  offset assumption must be justified against the C definition rather than
  guessed. A wrong offset reads unrelated memory and can appear to work.

### Error handling

- No `unwrap` or `expect` in production paths. Tests may use them.
- Recoverable errors are handled explicitly.
- Unrecoverable ones return context-rich typed errors.
- Poisoned-lock `unwrap` is acceptable where continuing would use corrupt state:
  that is failing fast, not sloppiness.

### Documentation

- Public items in core modules carry doc comments.
- Comments explain why, not what.
- Update documentation when a contract changes.

---

## 5) Lint, complexity and security gates

These gates are enforced in CI by `.github/workflows/rust-quality.yaml`. They
may be tightened. They must not be weakened.

| Gate | Command |
|---|---|
| Formatting | `cargo fmt --all --check` |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` |
| Complexity | `lizard -C 40 -w` over tracked source, excluding generated build output |
| Advisories | `cargo audit --deny warnings` |

Notes that matter:

- Clippy runs over **all targets**. A lint failure in test or binary code counts.
- Complexity is measured over tracked production and test source. Generated
  build output is excluded because vendored code must not decide whether this
  gate passes; no first-party source may be excluded.
- `cargo audit` without `--deny warnings` exits zero on unsound, unmaintained
  and yanked crates. The flag is what makes the gate a gate.

### Explicit prohibitions

- Do not disable lints globally or add file- or module-scope `allow` to silence
  debt.
- Do not raise a complexity or size threshold to make code fit. Split the code.
- Do not add suppression directives to obtain a green result.
- Do not exclude first-party source from any gate.

If a local `allow` is genuinely unavoidable: scope it to the smallest item,
explain why in the code, and reference a tracking issue.

---

## 6) Testing standards

Every meaningful change includes or updates tests that verify behaviour.

**A test must be able to fail.** Before relying on a new test, confirm it fails
against the defect it describes. A test that passes both before and after a fix
proves nothing and is worse than none, because it looks like coverage.

Layers:

- Unit tests for pure logic.
- Property or fixture tests where input space is large.
- Production-linked integration tests across FFI boundaries, covering
  initialisation, normal behaviour, errors at external boundaries, restart and
  reentry, and teardown.

Tests must verify externally meaningful behaviour, must be deterministic, and
must not depend on execution order or on a shared temporary path that another
test could occupy.

---

## 7) Proof and evidence standards

Compilation and unit tests are not evidence that the game works.

- Gameplay behaviour is proven by the automation harness against the real built
  executable, with semantic assertions correlated to presented frames.
- A screenshot must come from the actual presented window when the claim is
  about what a player sees. The harness `capture` action reads the game's
  internal draw surface, which cannot show a present or swap defect, so it
  cannot support that claim on its own.
- Evidence tooling must refuse to emit a result it cannot stand behind. A
  capture step that cannot capture must report a failure rather than save
  something misleading.
- Prose stating that something was tested is not evidence.

---

## 8) LLM contributor rules

All rules above apply, plus:

1. Do not silently reduce strictness to make a build pass.
2. Do not report success from an exit code alone; inspect the artefact that the
   claim rests on.
3. Do not narrow an accepted scope without amending the issue first. Discovery
   is not permission to downscope.
4. Do not add TODO-only stubs or speculative abstractions in production paths.
5. State what was verified and what was not. An honest gap is acceptable; an
   unstated one is not.

---

## 9) Review and merge bar

A change is mergeable only when all of the following hold:

1. Behaviour matches the accepted scope.
2. Exactly one implementation owns each affected subsystem.
3. Every gate in section 5 passes without suppression or threshold change.
4. Tests cover the change, and any regression test has been shown to fail
   without the fix.
5. Documentation is updated where a contract changed.
6. Evidence meets section 7 for any behavioural claim.

---

## 10) Non-negotiable summary

- Do not add first-party native code.
- Do not link an external library that is not in the allowlist, and do not
  vendor or patch one that is.
- Do not disable or weaken a lint, complexity, security or visual gate.
- Do not use `unwrap` or `expect` in production paths.
- Do not use `unsafe` outside an FFI or adapter boundary.
- Do not leave a fallback, dormant path or duplicate authority behind a cutover.
- Do not present internal-buffer captures as proof of what a player sees.
