# Phase 0.5: Preflight Verification

Plan ID: `PLAN-20260707-MAINLOOP.P0.5`

## Purpose

Verify every assumption the later phases rely on **before** writing any
production code. If any check fails, revise the plan rather than
implementing against a wrong assumption. This is the gate that prevents
the RaceDesc class of bug (assumed ABI ≠ real ABI) from recurring.

---

## 1. Toolchain Verification

Run each command; record output in `.completed/P0.5.md`.

```bash
cargo --version
rustc --version
cargo clippy --version
```

Expected: a stable Rust toolchain (edition 2021 per `rust/Cargo.toml`).

Coverage gate is **not** enforced by this plan (no `llvm-cov` in CI),
so it is **not** required here. If a future revision adds a coverage
gate, add:

```bash
cargo llvm-cov --version
```

- [ ] `cargo --version` succeeds
- [ ] `rustc --version` is 2021-edition-compatible
- [ ] `cargo clippy --version` succeeds

---

## 2. Dependency Verification

Confirm the crates this plan will use are already declared in
`rust/Cargo.toml` (we must not silently add new dependencies — verify
first, then add explicitly in the relevant phase if missing).

Crates the plan will rely on:

| Crate | Used for | Already in Cargo.toml? | Verification command |
|-------|----------|------------------------|----------------------|
| `libc` | `c_int`, `c_char`, `c_uchar` raw types | yes | `grep -n '^libc' rust/Cargo.toml` |
| `thiserror` | `MainLoopError` enum derive | yes | `grep -n 'thiserror' rust/Cargo.toml` |
| `serial_test` (dev) | serial boundary tests | yes | `grep -n 'serial_test' rust/Cargo.toml` |

- [ ] `libc` present
- [ ] `thiserror` present
- [ ] `serial_test` present (dev-dependency)

Feature flag to add in P03 (verified here that it does **not** already
exist with conflicting semantics):

```bash
grep -n 'c_integration' rust/Cargo.toml
```

- [ ] `c_integration` feature does **not** yet exist (will be added in P03)

---

## 3. Type / Interface Existence Verification

These are the exact symbols the plan assumes exist. Verify **each one**
exists with the assumed signature. A mismatch here is a blocking issue.

### 3.1 C symbols called from Rust (extern "C" targets)

| Symbol | C source location | Signature assumption | Verify |
|--------|-------------------|----------------------|--------|
| `TFB_PreInit` | `sc2/src/uqm.c:348` | `void TFB_PreInit(void)` | `grep -rn 'TFB_PreInit' sc2/src/` |
| `mem_init` | `sc2/src/uqm.c:349` | `void mem_init(void)` | `grep -rn 'mem_init' sc2/src/` |
| `InitThreadSystem` | `sc2/src/uqm.c:350` | `void InitThreadSystem(void)` | `grep -rn 'InitThreadSystem' sc2/src/` |
| `log_initThreads` | `sc2/src/uqm.c:351` | `void log_initThreads(void)` | `grep -rn 'log_initThreads' sc2/src/` |
| `initIO` | `sc2/src/uqm.c:352` | `void initIO(void)` | `grep -rn 'initIO\b' sc2/src/` |
| `InitTimeSystem` | `sc2/src/uqm.c:414` | `void InitTimeSystem(void)` | `grep -rn 'InitTimeSystem' sc2/src/` |
| `InitTaskSystem` | `sc2/src/uqm.c:415` | `void InitTaskSystem(void)` | `grep -rn 'InitTaskSystem' sc2/src/` |
| `Alarm_init` | `sc2/src/uqm.c:417` | `void Alarm_init(void)` | `grep -rn 'Alarm_init' sc2/src/` |
| `Callback_init` | `sc2/src/uqm.c:418` | `void Callback_init(void)` | `grep -rn 'Callback_init' sc2/src/` |
| `TFB_InitGraphics` | `sc2/src/uqm.c:434` | `void TFB_InitGraphics(int,int,const char*,int,int)` | `grep -rn 'TFB_InitGraphics' sc2/src/` |
| `InitColorMaps` | `sc2/src/uqm.c:441` | `void InitColorMaps(void)` | `grep -rn 'InitColorMaps' sc2/src/` |
| `init_communication` | `sc2/src/uqm.c:442` | `void init_communication(void)` | `grep -rn 'init_communication' sc2/src/` |
| `TFB_SetInputVectors` | `sc2/src/uqm.c:450` | variadic; see source | `grep -rn 'TFB_SetInputVectors' sc2/src/` |
| `TFB_InitInput` | `sc2/src/uqm.c:452` | `void TFB_InitInput(int,int)` | `grep -rn 'TFB_InitInput' sc2/src/` |
| `LoadKernel` | `sc2/src/uqm/starcon.c:178` | `BOOLEAN LoadKernel(int argc, char *argv[])` | `grep -rn 'LoadKernel' sc2/src/` |
| `StartGame` | `sc2/src/uqm/` | `BOOLEAN StartGame(void)` | `grep -rn 'StartGame' sc2/src/` |
| `VisitStarBase` | `sc2/src/uqm/` | `void VisitStarBase(void)` | `grep -rn 'VisitStarBase' sc2/src/` |
| `RaceCommunication` | `sc2/src/uqm/` | `void RaceCommunication(void)` | `grep -rn 'RaceCommunication' sc2/src/` |
| `ExploreSolarSys` | `sc2/src/uqm/` | `void ExploreSolarSys(void)` | `grep -rn 'ExploreSolarSys' sc2/src/` |
| `Battle` | `sc2/src/uqm/` | `void Battle(void(*)())` | `grep -rn '^Battle\b\|void Battle' sc2/src/` |
| `UninitGameKernel` | `sc2/src/uqm/starcon.c:283` | `void UninitGameKernel(void)` | `grep -rn 'UninitGameKernel' sc2/src/` |
| `FreeMasterShipList` | `sc2/src/uqm/starcon.c:284` | `void FreeMasterShipList(void)` | `grep -rn 'FreeMasterShipList' sc2/src/` |
| `FreeKernel` | `sc2/src/uqm/starcon.c:285` | `void FreeKernel(void)` | `grep -rn 'FreeKernel' sc2/src/` |
| `get_current_activity` | `sc2/src/uqm/rust_bridge_macros.c:132` | `UWORD get_current_activity(void)` | **already exists** — verified |

### 3.2 C symbols to be **created** by this plan (verified absent now)

```bash
grep -rn 'set_current_activity\|rust_dispatch_activity\|rust_game_loop' sc2/src/ rust/src/
```

- [ ] `set_current_activity` absent (P03/P07 will add it)
- [ ] `rust_dispatch_activity` absent (P07 will add it)
- [ ] `rust_game_loop` absent (P06 will add it)

### 3.3 C-side game-state accessor (existing macro)

The plan reads `CHMMR_BOMB_STATE` via the existing `GET_GAME_STATE` macro
path. Verify the existing Rust FFI exposes a keyed accessor:

```bash
grep -n 'rust_get_game_state\|rust_set_game_state' rust/src/state/ffi.rs
```

- [ ] `rust_get_game_state(*const c_char) -> c_uchar` exists
- [ ] `rust_set_game_state(*const c_char, c_uchar)` exists

These let Rust read `CHMMR_BOMB_STATE` by string key, satisfying
REQ-ML-010 without sharing struct offsets.

### 3.4 Activity enum values (C reference)

Confirm the enum/flag values the Rust types must mirror. Source:
`sc2/src/uqm/globdata.h:893-918` (already read for spec §3.1).

- [ ] `SUPER_MELEE = 0`, `IN_LAST_BATTLE = 1`, `IN_ENCOUNTER = 2`,
      `IN_HYPERSPACE = 3`, `IN_INTERPLANETARY = 4`, `WON_LAST_BATTLE = 5`,
      `IN_QUASISPACE = 6`, `IN_PLANET_ORBIT = 7`, `IN_STARBASE = 8`
- [ ] Flag bit positions: `CHECK_PAUSE=1<<0`, `IN_BATTLE=1<<1`,
      `START_ENCOUNTER=1<<2`, `START_INTERPLANETARY=1<<3`,
      `CHECK_LOAD=1<<4`, `CHECK_RESTART=1<<5`, `CHECK_ABORT=1<<6`
      (high byte; `MAKE_WORD(0, bit)`)

Verify:

```bash
sed -n '893,918p' sc2/src/uqm/globdata.h
```

- [ ] Values match spec §3.1 exactly

---

## 4. Call-Path Feasibility

Verify each planned integration call path is real and reachable.

### 4.1 C `Starcon2Main` → Rust game loop (P06/P07)

The plan modifies `sc2/src/uqm/starcon.c` `Starcon2Main` to delegate to `rust_game_loop()`. C `main()` in `uqm.c` is unchanged.
Verify the current structure:

```bash
sed -n '233,240p' sc2/src/uqm.c   # main() signature
```

- [ ] `main(int argc, char *argv[])` exists at uqm.c:233

### 4.2 C `Starcon2Main` (P05/P06/P07 reference)

The Rust state machine reimplements the body. Verify the reference:

```bash
sed -n '155,290p' sc2/src/uqm/starcon.c
```

- [ ] `Starcon2Main` exists; the encounter/interplanetary/battle dispatch
      is at lines 210–290 (reference for P05 pseudocode).

### 4.3 Existing Rust → C bridge pattern (P03 follows it)

Verify the established pattern this plan must follow:

```bash
sed -n '128,135p' sc2/src/uqm/rust_bridge_macros.c
```

- [ ] `get_current_activity` already follows the
      `extern "C" fn in Rust` ↔ `real function symbol in C` pattern.
      P03 will mirror this for `set_current_activity`.

### 4.4 build.rs compilation pattern (P03+ extends it)

```bash
sed -n '1,30p' rust/build.rs
```

- [ ] `cc::Build::new().file(...).compile(...)` pattern is established.
      P03+ will add `rust_test_bridge.c` and (P07) `rust_bridge_mainloop.c`.

---

## 5. Test Infrastructure Verification

### 5.1 Existing test harness

```bash
grep -rln '#\[cfg(test)\]' rust/src/game_init/ rust/src/state/
```

- [ ] Existing modules have `#[cfg(test)] mod tests` blocks — the pattern
      this plan will follow.

### 5.2 Serial-test dependency (boundary tests are global-state)

Boundary tests touch C globals (`CurrentActivity`, `CHMMR_BOMB_STATE`),
so they **must** be serial. Verify the dependency exists (checked in §2):
- [ ] `serial_test = "3.0"` present.

### 5.3 `c_integration` feature flag

Full C initialization requires SDL and content packs, which CI may lack.
The plan gates heavy integration tests behind a `c_integration` Cargo
feature (spec §8). Verify it does not yet exist (checked in §2):
- [ ] `c_integration` absent — will be added in P03.

### 5.4 Test bridge C file (to be created in P03)

Verify the planned path is free:

```bash
ls rust/src/mainloop/rust_test_bridge.c 2>/dev/null || echo "OK: does not exist yet"
```

- [ ] Path free.

---

## 6. Blocking Issues

List anything that failed above. **If non-empty, stop and revise the plan
before P01.**

- (none expected — populate from actual run)

---

## 7. Gate Decision

- [ ] **PASS** — all checks green; proceed to P01 (Analysis).
- [ ] **FAIL** — list blocking issues in §6; revise specification or
      phase list before continuing.

---

## 8. Completion Artifact

On PASS, create `project-plans/20260707/mainloop/.completed/P0.5.md`
with:

- phase ID `PLAN-20260707-MAINLOOP.P0.5`
- timestamp
- toolchain versions recorded
- the filled-in checklists above
- explicit PASS decision

This artifact is the prerequisite for P01.
