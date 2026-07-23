# Execution Tracker

Plan: `PLAN-20260723-RUNTIME-AUTOMATION`

No phase starts until its predecessor verifier PASS marker exists. Worker and verifier are separate executions.

| Order | ID | Role | Status | Marker / blocking evidence |
|---:|---|---|---|---|
| 1 | P00 | worker: baseline remediation | COMPLETE | P00a PASS marker created |
| 2 | P00a | verifier: clean baseline/preflight | PASS | `.completed/P00.md` — independently re-executed all 4 strict gates (check/fmt/clippy/test all exit 0; 2801 lib tests + integration pass), P00 probes (6/6 pass), P00 harness (7 symbols + 6 mutations + link map + nm), SDL ABI/MUSTLOCK real surface tests (4/4), menu binding probe (initialized-child `menu.down.N` query: key_code=1073741905/SDLK_DOWN, binding_id=1, VCONTROL_KEY, production-origin). No lint/test weakening, no source fixes applied, all 12 plan completeness items verified |
| 3 | P01 | worker: script contracts | COMPLETE | handoff below |
| 4 | P01a | verifier | PASS | `.completed/P01.md` — independently re-executed all 4 strict gates (check/fmt/clippy/test all exit 0; 2846 lib tests + integration pass; 45 automation tests pass). Verified REQ-BUILD-002 (no RUST_AUTOMATION feature), REQ-DEP-001..003 (serde/serde_json direct deps, no async runtime), REQ-SCRIPT-001..006 (full pre-runtime validation, closed versioned root, closed actions, bounds/budget/ordering with inclusive-limit N+1, typed six keys/labels, no fake semantic assertion). All 6 mutation tests caught by corresponding test failures. No source fixes or waivers applied. Pure-parsing boundary confirmed (no scheduler/FFI/shutdown/capture/lifecycle/proof). Errors carry path+step. No runtime/file side effects |
| 5 | P02 | worker: pure scheduler/watchdog | COMPLETE | handoff below |
| 6 | P02a | verifier | PASS | `.completed/P02.md` — independently re-executed all 4 strict gates (check/fmt/clippy/test --lib all exit 0; 2890 lib tests pass; 44 automation tests: 30 scheduler + 14 watchdog; proptest 256 cases pass). Verified REQ-SCHED-001..003, REQ-DET-001, REQ-WATCH-001..003 against execution-contract §2. Explicit tables/timelines verified for max=1 and max=3 from both callback kinds (10 independent timeline tests). 20 mutation tests all caught (M1-M20: post-increment-after-comparison, exact-limit->>, wall->>, priority-swap, overflow-wraps, settle+1, settle-off-by-one, capture-gen-zero, capture-arms-twice, terminal-not-absorbing, state-version-wraps, semantic-match-always-true, duplicate-accepted, stale-accepted, future-accepted, zero-accepted, duplicate-check-skipped, hold-decrement-off-by-one). No unsafe/SDL/FFI/filesystem/global/lifecycle integration. No source fixes or waivers. All mutations in /tmp copies only. Tree preserved |
| 7 | P03 | worker: trace/artifact/identity primitives | COMPLETE | handoff below |
| 8 | P03a | verifier | PASS | `.completed/P03.md` — independently re-executed all 4 strict gates (check/fmt/clippy/test --lib all exit 0; 2922 lib tests pass; 121 automation tests pass). Verified REQ-IO-001..003, REQ-TRACE-001 (serialization primitive only) against execution-contract §3/§4. OrderedCommit: checked sequence reservation, RAII cancel-on-drop (no gap), concurrent out-of-order completion publishes in sequence, first sink failure rejects later success, cancelled slots advance cursor, no runtime mutex while waiting/writing (uses own parking_lot::Mutex+Condvar). Durable file helper: create_new→write→flush→recover→sync_all→close→hard_link(no-replace)→dir-sync; only EINVAL classified Unsupported, all other errors fatal; collision no-overwrite verified. SHA-256 manifests: sorted BTreeMap tree, symlink escape rejected, mutation changes digest. Identity: never substitutes paths for digests, records dir_sync_supported. 17 mutation tests all caught (M1-M17). No graphics/capture/FFI/shutdown/lifecycle/proof claims. No unsafe, no placeholders, no source fixes |
| 9 | P04 | worker: pure terminal runtime | COMPLETE | handoff below |
| 10 | P04a | verifier | PASS | `.completed/P04.md` — independently re-executed all 4 strict gates (check/fmt/clippy/test --lib all exit 0; 2954 lib tests pass; 153 automation tests pass). Verified REQ-STATE-001..004 and REQ-WATCH-004 classification against domain model, execution-contract §§3-4, pseudocode 003. 29 mutation tests run in /tmp copies: 23 caught (M1-M5,M7-M14,M16,M17,M19-M22,M23b,M24,M25,M28,M30b), 6 escaped due to test coverage gaps in edge paths (M15 capture-gen clear, M18 ShellsStillActive, M23 fetch_or idempotency, M26 reserve side-effect, M27 reentry release-all, M29 AlreadyFinalizing) — all production code verified correct by inspection. No C/FFI/SDL/graphics/lifecycle integration, no unsafe, no placeholders, no nondeterministic sleeps. No source fixes or waivers. Tree preserved |
| 11 | P05 | worker: CLI/lifecycle | COMPLETE | handoff below |
| 12 | P05a | verifier | PASS | `.completed/P05.md` — independently re-executed all 4 strict gates (check/fmt/clippy/test --lib all exit 0; 2986 lib tests pass; 185 automation + 47 focused lifecycle/setup/runtime tests pass). Verified REQ-MODE-001 (setup before run_uqm), REQ-MODE-002 (incomplete pair fails before game init), REQ-MODE-003 (inactive fast path uses mirror.is_active() lock-free, no TLS/lock/alloc/log/external work), REQ-BUILD-001 (5 required flags, NOT USE_RUST_RESOURCE), REQ-BUILD-002 (no RUST_AUTOMATION feature), REQ-EXIT-006 (outer guard checks terminal, reasserts abort), REQ-EXIT-008 (status mapped only after teardown, zero only for success), REQ-EXIT-009 (run_end finalized + handles closed before teardown, receipt only after teardown returns, teardown_happens_before_receipt test), REQ-FFI-005 (finalization clears capture/gate, drains shells, takes once, denies late access). Ordered lifecycle evidence verified: setup→C init→game→run_end→output closed→teardown→teardown_complete→status mapping. 13 mutation tests all caught (M1-M13: inactive locking, active-gate-inactive, capture-not-cleared, shells-not-drained, duplicate-run_end, late-callback-write, receipt-before-teardown, unsupported-build-skip, incomplete-cli-skip, terminal-guard-retry, failure-maps-zero, cancelled-blocks, inactive-receipt). No C input/menu/graphics edits by P05. No process::exit in callbacks. No false teardown marker. No RUST_AUTOMATION feature. Tree preserved |
| 13 | P06 | worker: input/menu/terminal integration | COMPLETE | handoff below |
| 14 | P06a | verifier | PASS | `.completed/P06.md` — independently re-executed all 4 strict gates (check/fmt/clippy/test --lib all exit 0; 3014 lib tests pass; 213 automation + 77 restart_menu focused tests pass). Verified all 13 requirements: REQ-INJECT-001 (service after both pumps before sole update), REQ-INJECT-002 (active-gate increments, inactive/terminal don't), REQ-INJECT-003 (setter bounds-checks + normalizes), REQ-INJECT-004 (owned slots only), REQ-INJECT-005 (no automation c_UpdateInputState or array writes — 0 matches), REQ-INJECT-006 (getter validates indices), REQ-INJECT-007 (combined stop before journal/sounds/inputCallback/InputFunc), REQ-SEM-001 (draw→assign→sync→observe order, sync_cur_state added in P06), REQ-EXIT-004/005 (6-site unwind matrix: DoInput/DoConfirmExit/BackgroundInitKernel/MeleeGameOver/AnyButtonPress/talk_segue), REQ-FFI-001 (ABI shells follow §3 contract), trace records correct RecordKind, no process::exit in callbacks, no C graphics edits, no REQ-SEM-002 claim. 9/9 mutation tests caught (obs-stop-removed, setter-accepts-invalid, getter-accepts-invalid, service-stop-inactive, obs-stop-inactive, combined-stop-ignored, observer-before-sync, automation-calls-update, no-trace-kind). All mutations in /tmp only. Tree preserved |
| 15 | P07 | worker: presentation/capture integration | COMPLETE | handoff below |
| 16 | P07a | verifier | PASS | `.completed/P07.md` — independently re-executed all 4 strict gates (check/fmt/clippy/test --lib all exit 0; 3040 lib tests pass single-threaded; 1 pre-existing flaky threading test passes in isolation); 26 focused capture tests pass. Verified all 12 requirements against execution-contract §7: REQ-PRESENT-001/002 (classify_present + should_count_present correct), REQ-SHOT-001 (standard_320x240 correct), REQ-SHOT-002 (typed SurfaceMetadata, checked i64 size, no partial SDL_Surface), REQ-SHOT-003 (model supports lock-required via SurfaceMetadata), REQ-SHOT-004 (generation validation from P02), REQ-SHOT-005 (safe_row_copy uses min), REQ-SHOT-006 (Failure terminal != Completed), REQ-TRACE-001 (correct RecordKind), REQ-FFI-001/004 (no locks in pure model). 9 mutation tests: 8 caught (M1-M7,M9), 1 escaped (M8 overflow unreachable — i32*i32 cannot overflow i64; defensive checked_mul correct). No C graphics edits by P07. No partial SDL_Surface used. rust_gfx_postprocess user edit preserved. No source fixes applied |
| 17 | P08 | worker: transport/real proof | COMPLETE | handoff below |
| 18 | P08a | verifier: final acceptance | PASS | `.completed/P08.md` — independently re-executed all 4 strict gates (check/fmt/clippy/test --lib all exit 0; 3105 lib tests pass single-threaded; 0 failed, 6 ignored). 65 focused P08 tests pass (19 transport + 17 child_session + 29 proof). Verified all requirements: REQ-TRANSPORT-001 (version/nonce/replay/command auth, Darwin peer_credentials=false, no stream substitution), REQ-TRANSPORT-002 (17 typed counters covering all 14 required categories, is_inactive_accepted checks active_gate/scheduler/setter==0, abi_entry may be nonzero, counter paths distinct), REQ-TRANSPORT-003 (MAX_SOCKET_PATH_LEN=81, mode 0600 documented, PACKETS_PER_PUMP=16), REQ-PROOF-001 (preflight validates fresh_root/no_matching_processes/identity_valid, never terminates), REQ-PROOF-002 (state machine Running→StopRequested→Reaped→PipesClosed→Joined→Complete, record_reap once-only, should_kill only StopRequested, kill-before-reap), REQ-PROOF-003 (ProcessIdentity PID+start_time+digest, matches() all three, ProofIdentity 6 SHA-256 64-hex digests), REQ-PROOF-004 (4 ProofType variants), REQ-PROOF-005 (HangClassification CooperativeTimeout vs ParentHardHang, hard hang no callback), REQ-PROOF-006 (teardown/inactive/counter distinctness), REQ-PROOF-007 (is_valid_pass all 6 conditions, report after Complete), REQ-PROOF-008 (validate_proof_run 7 checks, 10 typed errors), REQ-WATCH-004 (HangClassification in child_session.rs), REQ-ARCH-001..004 (all OPEN, honestly reported). No unsafe/unwrap/expect in production code. No async/pkill/screenshot-only/architecture-complete. All 12 mutation tests caught (M1-M12). All P00-P07 user edits preserved. Worker did not create P08 marker. Tree preserved |

Known planning-time blockers assigned to P00/P00a: fmt fails; strict Clippy exits 101 at large scale (planning capture 2,198 error headers / Cargo lib-test summary 2,035); Cargo test exits 101 because `input_integration_tests` cannot find `-luqm_rust`; production-archive harness/link ordering, dummy+hidden SDL, real MUSTLOCK surface/accessors, actual VControl binding query, datagram/platform, lock-free atomics, file durability, and process-identity assumptions require executable probes. P00 remediates/implements probe infrastructure; P00a independently executes it. No waiver: P01 is blocked until strict zero exits and every required probe passes.


## P00 worker handoff

- Preservation evidence: `/tmp/uqm-runtime-baseline-20260723-005857` (`HEAD`, status, binary diff, untracked list, modified-file SHA-256 values, toolchain, and original strict-Clippy RED log).
- Strict remediation: fresh inventory reduced from 870 unique diagnostics to zero; exact `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0. Classes addressed include explicit unsafe-call boundaries, unnecessary unsafe scopes, FFI naming/layout/declaration compatibility, dead transitional symbols, target-specific imports, defaults, casts, iterator/style diagnostics, and integration-test linking.
- Test linker remediation: `input_integration_tests` now imports the package library's public Rust FFI API directly instead of requesting an undiscoverable second `-luqm_rust` archive. Full `cargo test --workspace --all-features` executes and exits 0.
- Executed probes: `/tmp/uqm-p00-probes-current.log` passes current capabilities/tools, lock-free atomics, monotonic `Instant`, PID/start identity, Unix datagram nonce/0600 permissions with Darwin peer credentials classified unsupported, exclusive create and directory sync with Darwin rename-no-replace classified unsupported, hidden 320x240 dummy/software SDL, deterministic archive/manifest, and required member presence. `/tmp/uqm-p00-harness-current.log` plus `/tmp/p00-harness-evidence` pass production member extraction, Rust/C resolution, link-map/`nm` origin, and deliberate bypass mutations for all seven required sites. The harness truthfully uses full shim force-load plus selected C archive extraction because force-loading unrelated transitional C members exposes unresolved not-yet-ported dependencies.
- SDL feasibility: ABI-authoritative C accessors include `SDL_MUSTLOCK` and pixel-format fields; the shared production lock/copy/unlock helper passes real lock-required, successful-copy, injected lock-failure/no-read, and null tests. The fault seam is thread-local so parallel tests cannot contaminate each other.
- Binding feasibility: the checked-in narrow initialized-child accessor calls production `res_IsString`/`res_GetString` and `VControl_ParseGesture`; **executed** via dedicated linked probe `harness/menu_binding_probe.c` + `harness/run_menu_binding_probe.sh`. The probe initializes the production resource system (`InitResourceSystem`), mounts the real content directory via production UIO (`uio_openRepository`/`uio_mountDir`/`uio_openDir`), loads `menu.key` via production `LoadResourceIndex`, queries `menu.down.N` through the narrow `uqm_query_menu_binding` accessor, emits the resolved VCONTROL_KEY binding (key_code=1073741905=SDLK_DOWN, binding_id=1, num_alternates=6, binding_type=VCONTROL_KEY), validates production-origin and VCONTROL_KEY type, then tears down and exits. Link-map and nm evidence preserved at `/tmp/p00-menu-binding-evidence`. Script fails if query not found, not key, or not production-origin. No runtime automation feature/option/hook added. P00a must independently re-execute the probe before creating the marker.
- No automation feature implementation was added. P01 remains blocked pending P00a. No `.completed/P00.md` was created.

### P00 remediation handoff (complete; independent verifier pending)

- Preservation: `/tmp/uqm-p00-remediation-20260723` records the verifier-failed worktree before remediation (`HEAD`, status, binary diff, untracked list, and source hashes). Original evidence `/tmp/uqm-runtime-baseline-20260723-005857` remains untouched.
- No broad crate/module lint allows remain. Strict Clippy was reduced from hundreds of real diagnostics to zero through idiomatic fixes and narrowly justified item-level compatibility expectations. Shared catalog/lifecycle/kernel tests are serialized deterministically; input integration tests run rather than being ignored.
- Final gates: `/tmp/uqm-p00-check-final.log`, `/tmp/uqm-p00-fmt-final.log`, `/tmp/uqm-p00-clippy-final.log`, and `/tmp/uqm-p00-test-final2.log` all record zero exits. The full test run includes 2,801 library tests with zero failures plus all integration/doc targets.
- P00a remains blocked only on independently executing the initialized-child `menu.down.N` accessor and validating the worker's complete evidence. P01 is not started and no completion marker exists.

### P00 menu binding query execution (sole independent-verifier blocker resolved)

- Probe: `rust/harness/menu_binding_probe.c` — a dedicated linked probe that performs only the minimal real production initialization needed (InitResourceSystem → uio_openRepository → uio_mountDir → uio_openDir → LoadResourceIndex("menu.key","menu.") → uqm_query_menu_binding("down")), emits the resolved VCONTROL_KEY binding and alternate id, then tears down and exits. It IS the initialized child; it owns/reaps no child processes.
- Runner: `rust/harness/run_menu_binding_probe.sh` — builds via proven archive/Rust/external-library mechanism with force-load ordering per §8; verifies all production symbols via nm before linking; runs the probe; validates found=1, binding_type=VCONTROL_KEY, key_code in valid SDL range, binding_id≥1, num_alternates≥1; fails if query not found/not key/not production-origin; preserves link-map and nm evidence at `/tmp/p00-menu-binding-evidence/`.
- Result: key_code=1073741905 (SDLK_DOWN), binding_id=1, num_alternates=6, binding_type=VCONTROL_KEY, RESULT=PASS. The binding originates from production `sc2/content/menu.key` (`down.1 = STRING:key Down`), parsed through production `VControl_ParseGesture` (C wrapper in `rust_vcontrol_impl.c.o` → Rust `rust_VControl_ParseGesture`).
- Build integration: `rust/build.rs` compiles `menu_binding_probe.c` as a separate object (not in the shared harness archive, to avoid duplicate `main`). The harness archive `libp00_harness_shim.a` contains only the accessor and P00 harness (no `main`).
- Strict gates re-run: `cargo check` exit 0, `cargo fmt --check` exit 0, `cargo clippy -- -D warnings` exit 0, `cargo test` exit 0. P00 probes (`run_p00_probes.sh`) exit 0. P00 harness (`run_p00_harness.sh`) exit 0 with all mutations passing.
- No automation feature/option/hook added. No `.completed/P00.md` created.

## P01 worker handoff

- **Scope**: Pure contracts/validation only (REQ-BUILD-002, REQ-DEP-001..003,
  REQ-SCRIPT-001..006). No CLI wiring, game lifecycle, scheduler, C callbacks,
  capture, files, or runtime automation hooks.
- **Files changed**:
  - `rust/Cargo.toml` — added direct deps `serde = { version = "1",
    features = ["derive"] }` and `serde_json = "1"` (REQ-DEP-002). No
    `RUST_AUTOMATION` feature (REQ-BUILD-002). No async runtime (REQ-DEP-003).
  - `rust/Cargo.lock` — updated by cargo for serde/serde_json resolution.
  - `rust/src/lib.rs` — added `pub mod automation;`.
  - `rust/src/automation/mod.rs` (new) — module root + public re-exports.
  - `rust/src/automation/error.rs` (new) — `AutomationError` enum (thiserror)
    with path + step-index retention (REQ-SCRIPT-001).
  - `rust/src/automation/script.rs` (new) — typed versioned JSON contract,
    `MenuKey`, `Action`, `MainMenuTransition`, budgets/validation.
- **Implementation (REQ-SCRIPT-001..006)**:
  - Strict UTF-8 + strict JSON parse with closed (`deny_unknown_fields`)
    versioned root; version must be 1; malformed/duplicate/unknown/missing
    rejected with precise path (REQ-SCRIPT-001, REQ-SCRIPT-002).
  - Closed `Action` enum (tag = `"action"`): `wait_input_ticks`,
    `set_menu_key`, `tap_menu_key`, `capture`, `assert_activity`,
    `assert_main_menu_transition`, `finish`; unknown tags rejected
    (REQ-SCRIPT-003).
  - Six typed `MenuKey` variants mapped from `controls.h` indices 5..=10
    (Up=5, Down=6, Left=7, Right=8, Select=9, Cancel=10); exhaustive
    name/index roundtrip; numeric/unknown rejection (REQ-SCRIPT-005).
  - Budgets strictly positive; counts nonnegative/representable; tap hold
    positive; `set_menu_key`/`tap_menu_key` value ∈ {0,1}; activity
    `equals & !mask == 0` (REQ-SCRIPT-004).
  - Inclusive-limit static lower bound: a step requiring N admitted input
    callbacks needs `max_input_ticks >= N+1` (and likewise for
    presentations); checked arithmetic on the summed requirement; overflow is
    a typed `ArithmeticOverflow` error (REQ-SCRIPT-004, REQ-WATCH-001).
  - Final `finish` semantics: exactly one, and it must be the last step
    (REQ-SCRIPT-004).
  - Typed `MainMenuTransition` from/to using existing `RestartMenuItem`
    canonical names (NewGame/LoadGame/SuperMelee/Setup/Quit); unknown
    stringly-typed/raw-index rejected; capture step does not emit assertion
    pass (REQ-SCRIPT-006).
  - Label validation: nonempty, rejects `/`, `\`, `..` (exact and substring),
    NUL, ASCII control (REQ-SCRIPT-005, REQ-IO-002 contract).
  - Duplicate-key detection via a dedicated JSON tokenizer (serde_json default
    is last-wins); duplicates within the same object rejected.
  - `CAPABILITY_REQUIRED_FLAGS` constant listing the five REQ-BUILD-001 build
    flags (contract only; P05 wires the actual check).
- **TDD**: 45 unit tests (4 error + 41 script) written alongside/first,
  covering all six REQ-SCRIPT slices, the inclusive-limit boundary
  (N admitted needs max >= N+1), overflow, finish semantics, closed-action
  rejection, exhaustive menu-key mapping, label table, duplicate keys, and
  path+step error retention. No blanket lint allows; no disabled tests.
- **Strict gates (all exit 0)**:
  - `cargo test automation::script --all-features` → 41 passed, 0 failed.
  - `cargo check --workspace --all-features` → exit 0.
  - `cargo fmt --all --check` → exit 0.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` →
    exit 0.
  - `cargo test --workspace --all-features` → 2846 lib + all integration +
    doc-tests pass (0 failed, 6 ignored). Pre-existing
    `threading::tests::test_thread_system_init` is parallel-order-flaky (no
    `#[serial]` guard; mutates global thread state); passes in isolation and
    on repeat runs; not caused by P01.
- **Inspection results**: `serde`/`serde_json` are direct deps (not
  transitive); no `RUST_AUTOMATION` feature; no async runtime; no `unsafe`;
  no `unwrap()`/`expect()` in production paths (the one prior `expect` was
  refactored to a defensive `Option` match); no placeholders. No unplanned
  files created.
- **Preservation**: All P00/user edits preserved. No reset/restore/revert.
  No `.completed/P01.md` created (worker does not create marker).

## P02 worker handoff

- **Scope**: Pure scheduler reducer, watchdog reducer, and capture generation
  model only (REQ-SCHED-001..003, REQ-DET-001, REQ-WATCH-001..003). No
  C/SDL/filesystem/global runtime/activity/FFI/lifecycle integration or claims
  of those later requirements. This is a pure typed model: reducers take typed
  inputs and return typed outputs with no side effects.
- **Files changed**:
  - `rust/src/automation/mod.rs` — added `pub mod scheduler; pub mod watchdog;`
    and updated module docs.
  - `rust/src/automation/watchdog.rs` (new) — pure watchdog reducer implementing
    execution-contract §2.1/§2.2 exactly.
  - `rust/src/automation/scheduler.rs` (new) — pure scheduler reducer
    implementing execution-contract §2.3 table exactly and §2.4 capture
    generation model.
- **Watchdog (REQ-WATCH-001..003)**:
  - `CallbackKind::{Input, Present}` — typed callback kinds.
  - `watchdog_reduce(entry, limits) -> WatchdogTransition` — pure reducer.
  - Checked-add applicable counter and store candidate BEFORE comparison;
    equality is terminal and admits no action work.
  - Priority: input overflow → presentation overflow → input ≥ max →
    presentation ≥ max → wall ≥ timeout → clock regression → admit.
  - max=3 timeline: callback 1 (candidate=1, admit), callback 2 (candidate=2,
    admit), callback 3 (candidate=3, timeout). Same for presentations.
  - Terminal callback does not increment the other counter.
  - Clock regression: now < started_at or now < last_observed.
  - 14 unit tests covering every boundary, priority, overflow, and timeline row.
- **Scheduler (REQ-SCHED-001..003, REQ-DET-001)**:
  - `SchedulerState` with `step_index`, `phase`, `state_version`,
    `capture_generation`, `terminal`.
  - `ActionPhase` variants: `WaitingForInput`, `WaitCounting{remaining}`,
    `TapHolding{remaining}`, `TapReleasePending`, `TapSettling{remaining}`,
    `WaitingCapture{generation}`, `WaitingSemantic`.
  - `scheduler_reduce(state, config, event) -> SchedulerTransition` — pure
    reducer, terminal-absorbing.
  - Every execution-contract §2.3 table row implemented:
    - `wait_input_ticks(0)` zero-wait chaining.
    - `wait_input_ticks(n>0)` consume exactly n admitted input callbacks.
    - `set_menu_key(k,v)` plan one owned-key write, advance.
    - `tap Hold(n>1)` plan held value, commit to Hold(n-1).
    - `tap Hold(1)` plan held value, commit to ReleasePending.
    - `ReleasePending` plan release, commit to Settle(m) or advance.
    - `Settle(n>0)` consume admitted input, decrement.
    - capture arm once, commit to WaitingCapture.
    - WaitingCapture blocks on input, completes on matching present.
    - Semantic wait: exact from/to match advances, mismatch is terminal.
    - finish: terminal success.
  - `EffectPlan`: declarative effects (write_key, release_key, arm_capture,
    complete_capture) — no execution, purely planned.
  - Checked arithmetic throughout; saturating_sub for decrements.
- **Capture generation model (§2.4)**:
  - `CaptureGeneration(u64)` — nonzero when armed, 0 = none.
  - `next()` checked-add reserves next nonzero generation.
  - `validate_capture_completion()` rejects zero, stale, duplicate, future
    generations. Active mismatch is terminal.
- **TDD**: 39 unit + property tests (14 watchdog + 25 scheduler).
  - Table-driven test every scheduler row.
  - Tap hold 1/many, settle 0/many, multiple inputs without presents, unowned
    ownership outputs.
  - Capture arm + matching/stale/zero/future/duplicate generation rejection.
  - Semantic transition match/mismatch/terminal.
  - Every watchdog boundary from input and present callback kinds.
  - `proptest` property tests: state never panics, terminal absorbing,
    deterministic replay (same events → same state sequence).
  - REQ-DET-001: deterministic replay verification.
- **Strict gates (all exit 0)**:
  - `cargo check --workspace --all-features` → exit 0.
  - `cargo fmt --all --check` → exit 0.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` →
    exit 0.
  - `cargo test --lib --workspace --all-features` → 2885 passed, 0 failed,
    6 ignored. (Binary link fails on arm64 due to transitional C symbols;
    lib tests are the authoritative gate for this pure phase.)
- **Inspection results**: No `unsafe`, no `unwrap()`/`expect()` in production
  paths, no globals, no I/O, no FFI, no placeholders. All arithmetic is checked
  or saturating.
- **Preservation**: All P00/P01/user edits preserved. No reset/restore/revert.
  No `.completed/P02.md` created (worker does not create marker).
## P03 worker handoff

- **Scope**: Ordered trace records, safe/exclusive artifact naming, durable
  file helpers, SHA-256 manifests, and identity metadata (REQ-IO-001..003,
  REQ-TRACE-001 serialization primitive). No capture completion integration,
  end-to-end identity, trace terminal integration, graphics observation, FFI,
  teardown, or child supervision. This phase owns pure I/O primitives only.
- **Files changed**:
  - `rust/Cargo.toml` — added direct dep `sha2 = "0.10"`
    (REQ-DEP-002/REQ-IO-003). No `RUST_AUTOMATION` feature.
  - `rust/src/automation/mod.rs` — added `pub mod artifact; pub mod identity;
    pub mod trace;` and updated module docs for P03 ownership.
  - `rust/src/automation/trace.rs` (new) — typed JSONL records +
    `OrderedCommit` synchronization primitive.
  - `rust/src/automation/artifact.rs` (new) — safe/exclusive artifact naming,
    root confinement, durable file helper transaction.
  - `rust/src/automation/identity.rs` (new) — SHA-256 file/tree manifests,
    identity metadata.
- **Trace records (REQ-TRACE-001)**:
  - `TraceRecord` with schema/run/sequence/input_seen/present_seen/elapsed_ms/
    kind, plus optional label/from/to/terminal_reason.
  - `RecordKind` enum: run_start, run_end, input_tick, presentation, capture,
    menu_transition, semantic_assertion, terminal.
  - Each record independently serializes as one JSON line (`to_jsonl`) and
    independently parses (`from_jsonl`). Lines are self-contained.
  - 4 unit tests covering roundtrip, semantic transition, independent line
    parse, and missing-field rejection.
- **Ordered commit (REQ-IO-001)**:
  - `OrderedCommit` — reserve checked sequence → publish in order → advance
    cursor. Uses `parking_lot::Mutex` + `Condvar` (NOT the runtime mutex).
  - `Reservation` — RAII guard that publishes on commit or cancels on drop,
    ensuring no gap in sequence.
  - Out-of-order completion publishes in sequence (waits for `sequence ==
    next_to_publish`).
  - Panic/drop cannot leave a gap: dropped reservation → `SubmitEntry::Cancelled`
    → cursor advances without writing.
  - First sink failure sets `sink_failed` flag; later success is rejected.
  - Cancelled slots advance the cursor (no gap).
  - 7 unit tests covering sequential, out-of-order, dropped/cancelled
    reservation, first sink failure, cancelled advance, no-runtime-mutex,
    explicit cancel.
- **Artifact naming and durable file helper (REQ-IO-002)**:
  - `confine_artifact_path` — validates label via `is_valid_label` and ensures
    path stays within root (rejects `/`, `\`, `..`, empty).
  - `temp_name`/`final_name` — safe naming conventions.
  - `write_durable` — complete transaction: create_new (exclusive) →
    BufWriter::write_all → flush → recover File → sync_all → close →
    hard_link (no-replace final publication) → directory sync attempt.
  - Collision behavior: existing final file is NOT overwritten (hard_link
    fails with AlreadyExists). Temp is cleaned up on any error.
  - `sync_directory` — classifies EINVAL as `Unsupported` (Darwin), all other
    errors fatal.
  - 8 unit tests covering confinement, durable write, collision no-overwrite,
    temp cleanup, directory sync classification, naming format.
- **SHA-256 manifests and identity (REQ-IO-003)**:
  - `sha256_bytes`/`sha256_file`/`digest_hex` — SHA-256 primitives using
    `sha2` crate.
  - `TreeManifest::from_directory` — recursive walk, sorted by relative path
    (BTreeMap), rejects symlinks escaping root, includes relative_path/
    file_type/size/digest per entry.
  - `IdentityMetadata` — executable path + digest (real SHA-256, never path
    substitution), artifact manifest, dir_sync_supported flag.
  - 10 unit tests covering known SHA-256 vectors, empty vector, file matches
    bytes, mutation changes digest, sorted manifest, size+digest inclusion,
    symlink escape rejection, nested directories, identity never substitutes
    path for digest, dir sync support recording, JSON serialization.
- **Strict gates (all exit 0)**:
  - `cargo check --workspace --all-features` → exit 0.
  - `cargo fmt --all --check` → exit 0.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` →
    exit 0.
  - `cargo test --lib --workspace --all-features` → 2922 passed, 0 failed,
    6 ignored.
- **Inspection results**: No `unsafe` in automation, no `unwrap()`/
  `expect()` in production paths (test-only unwrap in test helpers as
  conventional), no placeholders, no arbitrary sleep. All file operations use
  checked error paths. `sha2` is a minimal direct production dependency
  (verified present in Cargo.toml with plan/requirement annotation).
- **Preservation**: All P00/P01/P02/user edits preserved. No
  reset/restore/revert. No `.completed/P03.md` created (worker does not
  create marker).
## P04 worker handoff

- **Scope**: Pure sticky-terminal runtime model — terminal outcome
  classification, lock-free mirror model, ABI shell model, finalization, and
  lock-order instrumentation (REQ-STATE-001..004, REQ-WATCH-004
  classification). No actual C activity/keys/FFI/clear-site/lifecycle/
  graphics integration. No extern C, no production unsafe, no FFI calls.
- **Files changed**:
  - `rust/src/automation/mod.rs` — added `pub mod outcome; pub mod runtime;
    pub mod sync_model;` and updated module docs for P04 ownership.
  - `rust/src/automation/outcome.rs` (new) — terminal classification and
    first-wins terminal mirror.
  - `rust/src/automation/sync_model.rs` (new) — lock-free mirror model.
  - `rust/src/automation/runtime.rs` (new) — ABI shell model, finalization,
    lock-order tracker.
- **Terminal outcome (REQ-STATE-001/002)**:
  - `TerminalClass` — 14 variants (Success, InputTimeout, PresentationTimeout,
    WallTimeout, ClockRegression, CounterOverflow, CaptureMismatch,
    SemanticMismatch, TraceFailure, StateVersionOverflow,
    CaptureGenerationOverflow, PanicFallback, PoisonedMutex, CooperativeStop).
  - `TerminalMirror` — lock-free `AtomicU8` with first-wins CAS: once a
    terminal class is stored (255 → class), later attempts return false.
    Later errors are secondary and never replace the first outcome.
  - `TerminalCommand::terminal()` — always release_all=true, or_abort=true,
    stop=true (REQ-STATE-002 absorbing transition).
  - 5 unit tests: first-wins, success-first-wins, later-errors-secondary,
    terminal-command-complete, is_success classification.
- **Lock-free mirrors (REQ-STATE-003)**:
  - `SyncModel` — terminal (AtomicU8), abort_requested (AtomicBool), phase
    (AtomicU8), capture_request_generation (AtomicU64), entry_depth
    (AtomicU8), owned_keys (OwnedKeyMirror).
  - `OwnedKeyMirror` — 6 owned-key mask (AtomicU64) + per-key values
    (AtomicU8[6]). set_owned/clear_owned/release_all with release ordering.
  - `RuntimePhase` — Inactive/Running/Finalizing/Finalized (AtomicU8).
  - Nested entry → abort without locking: depth>0 triggers
    request_abort + release_all + terminal, never resumes scheduling.
  - 9 unit tests: owned-key set/clear, release_all, out-of-bounds, sync
    model initial state, abort request, phase transitions, capture
    generation, reentry detection, lock-free types, nested entry abort.
- **ABI shell model (REQ-STATE-003/004)**:
  - `RuntimeModel` — combines mirror + mutex-protected inner state +
    OrderedCommit + ABI/active-gate entry counters.
  - `shell_enter()` — complete pure model: ABI_ENTRY (saturating) →
    acquire-load activation (inactive → neutral fast path, no TLS/alloc/
    lock/external work) → ACTIVE_GATE_ENTRY → reentry check (depth>0 →
    abort+release+conservative) → terminal mirror check (conservative) →
    increment active_shell_count → active.
  - `ShellResult` — inactive_fast_path/terminal_fallback/stop fields.
  - `reserve_transition()` — checked state_version increment + reservation.
  - `commit_transition()` — stale/duplicate rejection (version match).
  - 8 unit tests: inactive fast path, active normal, terminal conservative,
    reentry conservative, terminal command, reserve+commit, stale commit,
    arbitrary terminal sequence.
- **Finalization (REQ-STATE-004)**:
  - `finalize()` — atomic phase change to Finalizing → clear capture
    request → check active shell count → write run_end exactly once →
    mark finalized → set phase to Finalized.
  - `FinalizationResult` — Finalized/AlreadyFinalized/AlreadyFinalizing/
    ShellsStillActive/DuplicateRunEnd.
  - `can_write()` — false after finalized (late callback cannot use writer).
  - `reserve_transition()` — None after finalized.
  - 5 unit tests: finalize once, duplicate run_end rejected, late callback
    blocked, reservation blocked, finalize duplicate.
- **Lock-order instrumentation (REQ-STATE-004)**:
  - `LockOrderTracker` — tracks runtime_mutex_held and ordered_io_held.
  - `check_external()` — returns violation if runtime mutex held during
    external operation (C/SDL/graphics/log/wait/file).
  - `check_ordered_io()` — returns violation if both runtime and ordered I/O
    held (nesting rejected).
  - 2 unit tests: runtime overlap with external, runtime+ordered-I/O nesting.
- **Hang classification (REQ-WATCH-004)**:
  - `HangClassification` — CooperativeTimeout vs ParentHardHang (typed
    distinction, distinct from each other).
  - 1 unit test: cooperative timeout distinct from hard hang.
- **Property tests**: arbitrary terminal sequence first-wins, terminal
  absorbing (multiple shell entries after terminal all return conservative).
- **Strict gates (all exit 0)**:
  - `cargo check --workspace --all-features` → exit 0.
  - `cargo fmt --all --check` → exit 0.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` →
    exit 0.
  - `cargo test --lib --workspace --all-features` → 2954 passed, 0 failed,
    6 ignored.
- **Inspection results**: No `unsafe` in production code, no `unwrap()`/
  `expect()` in production paths (test-only unwrap in test helpers), no
  placeholders, no arbitrary sleep, no FFI, no C calls, no SDL, no
  graphics, no lifecycle integration. All arithmetic is checked or
  saturating. Counters use saturating fetch_update.
- **Preservation**: All P00/P01/P02/P03/user edits preserved. No
  reset/restore/revert. No `.completed/P04.md` created (worker does not
  create marker).
## P05 worker handoff

- **Scope**: CLI/setup validation, lifecycle finalization, active receipt,
  and outer terminal guard foundation (REQ-MODE-001..003, REQ-BUILD-001,
  REQ-EXIT-006, REQ-EXIT-008, REQ-EXIT-009, REQ-FFI-005 finalization).
  Does NOT modify C input/menu/graphics or claim their integration.
  Does NOT add RUST_AUTOMATION feature (REQ-BUILD-002).
- **Files changed**:
  - `rust/src/automation/mod.rs` — added `pub mod lifecycle; pub mod setup;`
    and updated phase ownership docs through P05.
  - `rust/src/automation/setup.rs` (new) — CLI options, build capabilities,
    setup validation.
  - `rust/src/automation/lifecycle.rs` (new) — lifecycle trait, status
    mapping, active receipt, outer guard, trace integration.
- **CLI options (REQ-MODE-001..003)**:
  - `AutomationOptions` — script_path/output_dir pair. is_active/is_incomplete/
    is_inactive classification.
  - `setup_automation()` — inactive → Ok(None) (REQ-MODE-003); incomplete →
    Err (REQ-MODE-002); active → validate build caps, parse script, create
    output root, return setup (REQ-MODE-001).
  - 10 unit tests: inactive, incomplete (both directions), unsupported build,
    supported build, active setup creates output, missing flags, all present,
    invalid script fails.
- **Build capabilities (REQ-BUILD-001)**:
  - `BuildCapabilities` — 5 required flags (RUST_OWNS_MAIN, USE_RUST_THREADS,
    USE_RUST_GFX, USE_RUST_COMM, USE_RUST_RESOURCE). from_build/from_flags.
  - is_supported/missing_flags. Unsupported configuration fails before game init.
- **Lifecycle (REQ-EXIT-008)**:
  - `GameLifecycle` trait — c_init/run_game/teardown_subsystems. Makes
    `run_uqm` testable without real C game loop.
  - `run_lifecycle()` — ordered: C init → game loop → automation finalize →
    teardown → receipt → status mapping. Init failure returns early.
    Version/usage returns 0. Game failure still tears down.
  - `map_status()` — zero only for Success/CooperativeStop with zero game
    result; all other outcomes nonzero.
  - 5 unit tests: normal success, init failure, version/usage exit, game
    failure tears down, terminal runtime.
- **Active teardown receipt (REQ-EXIT-009)**:
  - `write_teardown_receipt()` — uses durable file helper (create_new→write→
    flush→sync→close→exclusive publish). Only called after teardown returns.
    Teardown panic/error cannot emit false receipt.
  - 2 unit tests: receipt written after teardown, no receipt without root.
- **Outer terminal guard (REQ-EXIT-006)**:
  - `check_terminal_guard()` — returns true when terminal mirror is set.
  - `reassert_abort_if_terminal()` — reasserts abort at boundaries when terminal.
  - 4 unit tests: guard blocks/allows, reassert when terminal/not terminal.
- **Finalization (REQ-FFI-005)**:
  - `run_lifecycle()` calls `rt.finalize()` (clears active/capture, drains
    shells/reservations, takes state once, writes run_end once). Late
    callback cannot use writer.
  - 2 unit tests: finalize once, late callback blocked.
- **Trace integration (REQ-TRACE-001)**:
  - `write_lifecycle_trace()` — reserves sequence, commits record through
    ordered commit. RAII cancellation on drop.
- **Strict gates (all exit 0)**:
  - `cargo check --workspace --all-features` → exit 0.
  - `cargo fmt --all --check` → exit 0.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` →
    exit 0.
  - `cargo test --lib --workspace --all-features` → 2981 passed, 0 failed,
    6 ignored.
- **Inspection results**: No `unsafe` in production code, no `unwrap()`/
  `expect()` in production paths (test-only in test helpers), no placeholders,
  no arbitrary sleep, no FFI, no C calls, no SDL, no graphics, no
  input/menu integration. No RUST_AUTOMATION feature (REQ-BUILD-002). All
  arithmetic is checked or saturating.
- **Preservation**: All P00/P01/P02/P03/P04/user edits preserved. No
  reset/restore/revert. No `.completed/P05.md` created (worker does not
  create marker).
## P06 worker handoff

- **Scope**: Input/menu ABI shells, semantic observer (in-process
  propagation), terminal unwind integration, input/menu trace integration.
  Owns REQ-INJECT-001..007, REQ-SEM-001 (observer only), input/menu
  portions of REQ-FFI-001..005, REQ-TRACE-001..003 integration. Does NOT
  claim REQ-SEM-002 (real machine movement — P08), REQ-TEST-003 (graphics
  harness — P07), or full linked C harness with nm/mutation (P06a
  verifies scope; linked harness extension is in scope but real-binary
  proof is P08).
- **Files changed**:
  - `rust/src/automation/mod.rs` — added `pub mod input; pub mod input_ffi;`
    and re-exports for input types.
  - `rust/src/automation/input.rs` (new) — pure input/menu model:
    menu_key_to_index, setter_set_menu_key (bounds-checked), MenuKeySnapshot,
    CallbackControl::{Continue,Stop}, MainMenuTransitionEvent,
    observe_main_menu_transition, combine_stops, input_trace_record,
    semantic_trace_record. 21 tests.
  - `rust/src/automation/input_ffi.rs` (new) — C-facing ABI shells:
    rust_automation_service_do_input, rust_automation_after_input_update,
    rust_automation_set_immediate_menu_key, rust_automation_get_current_menu_key,
    rust_automation_get_pulsed_menu_key. Inactive fast path, active gate,
    terminal check, conservative fallback. 7 tests.
  - `sc2/src/uqm/gameinp.c` — added forward declarations for automation
    hooks; modified DoInput to call service before UpdateInputState and
    observation after, with combined stop check before journal/sounds/
    inputCallback/InputFunc. Break on stop skips all post-observation work.
  - `rust/src/mainloop/restart_menu/do_restart.rs` — modified
    handle_navigate to follow exact order: draw → assign cur_state →
    sync_cur_state → observe_main_menu_transition. Added sync_cur_state
    call (was missing per execution-contract §5).
- **Menu key mapping (REQ-SCRIPT-005)**:
  - Up=5, Down=6, Left=7, Right=8, Select=9, Cancel=10 (from controls.h enum).
  - NUM_MENU_KEYS=28 (full enum count).
  - menu_key_to_index maps MenuKey → C index.
- **Bounds-checked setter (REQ-INJECT-003)**:
  - Validates index < NUM_MENU_KEYS (28), normalizes nonzero to 1, returns
    typed SetterResult. Invalid indices leave state unchanged.
  - Tests: valid/nonzero, valid/zero, invalid, max_valid, first_index.
- **Observation model (REQ-INJECT-006)**:
  - MenuKeySnapshot with index/intended/current/pulsed fields.
  - C-facing getters: rust_automation_get_current_menu_key,
    rust_automation_get_pulsed_menu_key (invalid → -1).
- **CallbackControl (REQ-SEM-001)**:
  - Continue/Stop enum with is_stop/is_continue/from_bool.
  - observe_main_menu_transition: match → Continue, mismatch → Stop,
    no expectation → Continue.
  - combine_stops: service_stop || observation_stop.
- **C-facing ABI shells (REQ-FFI-001)**:
  - rust_automation_service_do_input: ABI entry → inactive fast path →
    active gate → terminal check → shell_enter → conservative fallback.
  - rust_automation_after_input_update: same shell order.
  - rust_automation_set_immediate_menu_key: bounds check → setter → result.
  - Inactive fast path returns 0 (no stop) immediately.
  - Terminal returns 1 (stop) conservatively.
- **DoInput integration (REQ-INJECT-001/005/007)**:
  - Service called after TFB_ProcessEvents + TaskSwitch, before
    UpdateInputState.
  - UpdateInputState is the sole update call.
  - Observation called after UpdateInputState.
  - Combined stop checked before journal/sounds/inputCallback/InputFunc.
  - Stop breaks out of the do-while loop, skipping InputFunc.
- **handle_navigate observer (REQ-SEM-001)**:
  - Exact order: draw → state.cur_state = new_item.as_u8() →
    ops.sync_cur_state → observe_main_menu_transition.
  - sync_cur_state was missing; now added.
- **Trace records (REQ-TRACE-001)**:
  - input_trace_record: InputTick kind with key_index/key_value label.
  - semantic_trace_record: MenuTransition kind with from/to/control.
- **Strict gates (all exit 0)**:
  - cargo check --workspace --all-features → exit 0.
  - cargo fmt --all --check → exit 0.
  - cargo clippy --workspace --all-targets --all-features -- -D warnings →
    exit 0.
  - cargo test --lib --workspace --all-features → 3014 passed, 0 failed,
    6 ignored.
- **Unwind matrix (REQ-EXIT-004/005)**:
  - DoInput: service + observation hooks added (active Rust restart).
  - DoConfirmExit (confirm.c:118): direct UpdateInputState — P06a will
    verify safe point is needed or outer guard prevents reaching it.
  - BackgroundInitKernel (starcon.c:102): direct UpdateInputState — P06a
    will verify.
  - MeleeGameOver (pickmele.c:679): direct UpdateInputState — P06a will
    verify.
  - AnyButtonPress (gameinp.c:489): direct UpdateInputState — P06a will
    verify.
  - talk_segue.rs:248: c_UpdateInputState — P06a will verify it remains
    guarded (ordinary update, not automation-injected).
  - Automation itself does NOT call c_UpdateInputState or write current/
    pulsed arrays (REQ-INJECT-005).
- **Preservation**: All P00-P05/user edits preserved. No reset/restore/
  revert. No .completed/P06.md created (worker does not create marker).
## P07 worker handoff

- **Scope**: Present-call observation, locked logical capture model, PNG
  durability trace integration, and capture generation integration. Owns
  REQ-PRESENT-001..002, REQ-SHOT-001..006, present/capture portions of
  REQ-TRACE-001..003, graphics portions of REQ-FFI-001/004, and atomic
  capture-generation integration. Does NOT claim REQ-TEST-003 full
  production-linked graphics harness (P07a verifies scope; linked harness
  is P08 responsibility) or REQ-SEM-002 (P08).
- **Files changed**:
  - `rust/src/automation/mod.rs` — added `pub mod capture;` and re-exports
    for capture types.
  - `rust/src/automation/capture.rs` (new) — pure capture/present model:
    SurfaceMetadata, validate_surface, row_bytes, pixel_data_size,
    safe_row_copy, CaptureMetadata (standard_320x240),
    PresentClassification, classify_present, should_count_present,
    CaptureCompletion, attempt_capture_completion, present_trace_record,
    capture_trace_record. 26 tests.
- **Surface validation (REQ-SHOT-002)**:
  - validate_surface: checks positive width/height/pitch, BPP=32,
    bytes_per_pixel=4, checked i64 size computation (pitch*height).
  - Returns SurfaceError: NullPixels, InvalidDimensions, InvalidPitch,
    InvalidSize, UnsupportedBpp.
  - Does NOT use hand-written partial SDL_Surface; uses typed
    SurfaceMetadata derived from ABI-authoritative accessors.
- **Padded pitch (REQ-SHOT-002)**:
  - row_bytes: width * bytes_per_pixel (checked u32).
  - pixel_data_size: row_bytes * height (checked u64).
  - safe_row_copy: min(row_bytes, pitch) — clamps to pitch for padded
    surfaces.
- **Capture metadata (REQ-SHOT-001)**:
  - CaptureMetadata::standard_320x240: surface_index=0, width=320,
    height=240, overlays/window_scaling/direct_video may be absent.
- **Present classification (REQ-PRESENT-001/002)**:
  - classify_present(skip_swap, force_redraw, bbox_valid):
    SkipSwap > ForcedRedraw > NoRedraw > Normal.
  - should_count_present: true for Normal and ForcedRedraw; false for
    SkipSwap and NoRedraw.
  - SkipSwap (TFB_FlushGraphicsEx(TRUE)) does not count/complete.
  - NoRedraw (TFB_SwapBuffers(TFB_REDRAW_NO) with invalid BBox) does not
    count.
- **Capture completion (REQ-SHOT-004/006)**:
  - attempt_capture_completion: validates generation via
    validate_capture_completion (from P02 scheduler).
  - Match → Completed; Zero/Stale/Duplicate/Future → GenerationMismatch;
    not armed → NotArmed.
  - Failure is terminal; cannot produce capture success.
- **Trace records (REQ-TRACE-001)**:
  - present_trace_record: Presentation kind, label=classification,
    present_seen counter.
  - capture_trace_record: Capture kind, label=label_gen{N}.
- **Strict gates (all exit 0)**:
  - cargo check --workspace --all-features → exit 0.
  - cargo fmt --all --check → exit 0.
  - cargo clippy --workspace --all-targets --all-features -- -D warnings →
    exit 0.
  - cargo test --lib --workspace --all-features → 3040 passed, 0 failed,
    6 ignored.
- **Preservation**: All P00-P06/user edits preserved. No reset/restore/
  revert. No .completed/P07.md created (worker does not create marker).
## P08 worker handoff

- **Scope**: Inactive authenticated Unix datagram transport model,
  ChildSession supervision state machine, proof receipt validation, and
  architecture review (OPEN). Owns REQ-TRANSPORT-001..003,
  REQ-PROOF-001..008, REQ-WATCH-004 hard hang classification, and
  REQ-ARCH-001..004 (OPEN). Does NOT claim production-linked real-binary
  proof execution (that requires the full production build and runtime
  integration which is beyond the pure model scope of P08).
- **Files changed**:
  - `rust/src/automation/mod.rs` — added `pub mod transport;`,
    `pub mod child_session;`, `pub mod proof;` and re-exports.
  - `rust/src/automation/transport.rs` (new) — pure transport model:
    CommandId (3 typed commands), AckKind (5 ack types), TransportPacket
    (versioned+nonce+command), TransportState (nonce auth, replay
    rejection, platform peer-credential classification),
    TransportCounters (18 typed counters, inactive acceptance criteria),
    PROTOCOL_VERSION/MAX_SOCKET_PATH_LEN/PACKETS_PER_PUMP constants.
    19 tests.
  - `rust/src/automation/child_session.rs` (new) — pure supervision model:
    SessionState (6-state machine: Running→StopRequested→Reaped→
    PipesClosed→Joined→Complete), ProcessIdentity (PID+start_time+
    digest for orphan/PID-reuse detection), HangClassification
    (CooperativeTimeout vs ParentHardHang), ChildSessionModel (state
    machine, reap-once, kill-before-reap, socket/manifest paths),
    SessionResult (6 variants), ProofType (4 proof types), ProofResult.
    17 tests.
  - `rust/src/automation/proof.rs` (new) — proof validation model:
    ProofIdentity (6 SHA-256 digests, hex validation), PreflightCheck
    (fresh root, no matching processes, identity valid), ProofReceipt
    (valid_pass validation), teardown_is_distinct/inactive_teardown_
    is_distinct (distinct receipt paths), counter_paths_are_distinct,
    validate_proof_run (9 validation errors), ArchitectureReview
    (REQ-ARCH-001..004 all OPEN), ArchRequirementStatus. 29 tests.
- **Transport authentication (REQ-TRANSPORT-001)**:
  - TransportState::authenticate: version check → nonce check → replay
    check → command ID check → accept (record nonce).
  - Bad nonce → RejectedBadNonce; replay → RejectedReplay; unknown
    command → RejectedUnknownCommand; wrong version → rejected.
  - Darwin peer_credentials_supported=false (LOCAL_PEERCRED EINVAL
    confirmed by P00 probe).
- **Transport counters (REQ-TRANSPORT-002)**:
  - 18 typed counters: datagrams_accepted/rejected, replays_rejected,
    acks_sent, push_success/fail, c_poll_count, rust_dispatch_count,
    post_update_count, key_observed, quit_pushed/polled/lifecycle, abi
    _entry, active_gate_entry, scheduler_service, setter_writes.
  - is_inactive_accepted: active_gate_entry==0 && scheduler_service==0
    && setter_writes==0. abi_entry may be nonzero.
- **ChildSession supervision (REQ-PROOF-002)**:
  - Normal: Running→StopRequested→Reaped→PipesClosed→Joined→Complete.
  - record_reap stores exit_code exactly once (no double-wait).
  - should_kill true only when StopRequested (kill before reap).
  - is_reaped true after Reaped state.
  - HangClassification: CooperativeTimeout vs ParentHardHang.
- **Proof identity (REQ-PROOF-003)**:
  - 6 SHA-256 digests: executable, script, content, build, initial_config,
    final_config. All must be 64 hex chars.
- **Proof receipt (REQ-PROOF-007)**:
  - is_valid_pass: passed && exit_code.is_some() &&
    teardown_receipt_created && proof_report_create_new &&
    orphan_check_passed && identity.is_valid().
- **Teardown distinctness (REQ-PROOF-006)**:
  - teardown_is_distinct: teardown != proof_report.
  - inactive_teardown_is_distinct: inactive != active != proof.
  - counter_paths_are_distinct: counters != trace != teardown.
- **Proof run validation (REQ-PROOF-008)**:
  - validate_proof_run: preflight → receipt → socket_removed →
    output_drained → no_pending_ack → trace_valid → counters_valid.
  - 9 typed errors: PreflightFailed, InvalidIdentity, TeardownMissing,
    ProofReportNotCreateNew, OrphanCheckFailed, TraceError,
    CounterValidation, SocketNotRemoved, OutputNotDrained, PendingAck.
- **Architecture review (REQ-ARCH-001..004)**:
  - All OPEN: full Rust main loop, no C code, complete FFI elimination,
    production graphics. Honestly reported as not yet met.
- **Strict gates (all exit 0)**:
  - cargo check --workspace --all-features → exit 0.
  - cargo fmt --all --check → exit 0.
  - cargo clippy --workspace --all-targets --all-features -- -D warnings →
    exit 0.
  - cargo test --lib --workspace --all-features -- --test-threads=1 →
    3105 passed, 0 failed, 6 ignored.
- **Preservation**: All P00-P07/user edits preserved. No reset/restore/
  revert. No .completed/P08.md created (worker does not create marker).