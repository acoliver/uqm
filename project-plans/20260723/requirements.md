# EARS Requirements: Rust Runtime Automation and Real-Game Proof

Plan ID: `PLAN-20260723-RUNTIME-AUTOMATION`
Normative architecture: [overview.md](overview.md)
Authoritative reducer/ABI/I/O/transport contract: [analysis/authoritative-execution-contract.md](analysis/authoritative-execution-contract.md). It controls wherever a shorter phase description is less specific.
Verification codes: `M` machine, `R` review. Artifact inspection is supplemental and never the sole acceptance method.

A **committed presentation** in this slice means that the real Rust backend's `Canvas::present()` call returned normally. It does not claim display acknowledgement. An **input tick** is the post-increment ordinal of one active automation service callback; the callback that reaches its inclusive maximum is counted but admits no scheduler work. A **terminal state** is the first-wins sticky automation result.

## Architecture (long-term; remain OPEN after this slice)

### REQ-ARCH-001 — Rust ownership
The target runtime shall own event polling, input/repeat, scheduling, drawing, presentation, automation, shutdown, and supervision in Rust with zero in-tree C implementation code. **Verify:** R.

### REQ-ARCH-002 — Common input source
Physical and automated input shall feed one typed Rust input-state machine rather than automation-only legacy-global mutation. **Verify:** R.

### REQ-ARCH-003 — Presented frame
The target Rust presenter shall emit an immutable typed `PresentedFrame` and monotonic sequence at its public commit boundary. **Verify:** R/M in the future architecture.

### REQ-ARCH-004 — Transitional seam removal
When Rust owns input and nested scheduling, the project shall remove the C `DoInput` automation callbacks, immediate setter, and test shims. **Verify:** R.

## Quality and baseline

### REQ-QUALITY-001 — Strict gates
The implementation shall pass `cargo check --workspace --all-features`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features`, without weakening lint configuration or adding blanket allows. **Verify:** M/R.

### REQ-QUALITY-002 — Baseline remediation and executable probes first
Before P01, P00 shall preserve the tree, remediate every fmt/strict-Clippy failure and the existing Cargo-test `-luqm_rust` linker failure forward, test behavior-affecting changes, and provide executable platform/SDL/datagram/file/process/atomic/binding/link probes. P00a shall require zero strict exits and successful minimal production-archive harness without waiver. **Verify:** M plus saved before/after/probe evidence.

### REQ-QUALITY-003 — Preserve user work
No phase shall reset, restore, checkout, replace wholesale, or globally reformat away current edits. Formatting changes shall be isolated from semantic changes in evidence. **Verify:** R/M diff comparison.

## Mode, build, and dependencies

### REQ-MODE-001 — Runtime opt-in
When both `--automation-script` and `--automation-output` are supplied, Rust shall validate/setup automation before `run_uqm`. **Verify:** M.

### REQ-MODE-002 — Complete pair
If exactly one automation option is supplied, startup shall fail before game initialization and artifact creation. **Verify:** M.

### REQ-MODE-003 — Inactive behavior
Without both options, direct automation callbacks shall execute only a saturating lock-free ABI-entry diagnostic increment and an acquire activation load, then return the neutral result. They shall not access TLS/depth, allocate, format/log, lock, call C/SDL/I/O, mutate input/activity, enter the active gate, schedule, or create artifacts. Existing non-automation behavior in full restart/graphics shells remains active. **Verify:** M allocator/lock/counter unit tests and real-child proof.

### REQ-BUILD-001 — Supported linked configuration
Active automation shall require the actual linked binary to provide `RUST_OWNS_MAIN`, `USE_RUST_THREADS`, `USE_RUST_GFX`, `USE_RUST_COMM`, and `USE_RUST_RESTART`; unsupported configurations shall fail before game init. **Verify:** M symbol/build metadata.

### REQ-BUILD-002 — Runtime feature, no fictional flag
The slice shall not add `RUST_AUTOMATION` or an automation Cargo feature. **Verify:** R/M search.

### REQ-DEP-001 — Existing libraries
Use existing `clap`, `image`, and error facilities; add only justified direct dependencies. **Verify:** R.

### REQ-DEP-002 — Typed JSON
Add `serde` derive and `serde_json` as direct dependencies; do not rely on transitives. **Verify:** M/R.

### REQ-DEP-003 — Synchronous execution
Scheduling, observations, capture, trace, and child supervision shall be synchronous; no async runtime is added. Scoped synchronous output-drain threads are permitted. **Verify:** R.

## Script

### REQ-SCRIPT-001 — Full pre-runtime validation
Read UTF-8 JSON and validate the complete document before game initialization or output side effects beyond an exclusively created run root. Errors include path and step index. **Verify:** M malformed/missing/non-UTF8 tests.

### REQ-SCRIPT-002 — Closed versioned root
Version 1 requires `version`, `name`, all three positive budgets, and nonempty `steps`; version must be 1 and unknown/duplicate fields are rejected. **Verify:** M tables.

### REQ-SCRIPT-003 — Closed typed actions
Only `wait_input_ticks`, `set_menu_key`, `tap_menu_key`, `capture`, `assert_activity`, `assert_main_menu_transition`, and final `finish` are accepted. **Verify:** M.

### REQ-SCRIPT-004 — Bounds, budget relationship, and ordering
Counts are representable/nonnegative, tap hold is positive, budgets are positive, activity values fit `u16`, `equals & !mask == 0`, and exactly one `finish` is last. Checked static lower bounds shall reject a script requiring N admitted callbacks when the corresponding inclusive maximum is less than N+1; dynamic waits/assertions remain bounded by declared budgets. **Verify:** M boundary/overflow tests.

### REQ-SCRIPT-005 — Typed keys and labels
Keys are the six menu variants mapped from `controls.h` (up/down/left/right/select/cancel); labels are nonempty and reject separators, `..`, and controls. **Verify:** M exhaustive mapping/table.

### REQ-SCRIPT-006 — No fake semantic assertion
Semantic actions are accepted only for an implemented typed observer. A capture/checkpoint cannot emit assertion pass. **Verify:** M schema and negative tests.

## Scheduler and watchdog

### REQ-SCHED-001 — Normative reducer
The scheduler shall implement the authoritative table and timelines in `analysis/authoritative-execution-contract.md` §2, advance actions only after a matching two-phase commit, arm capture once with a nonzero atomic generation, block on capture, reject stale/duplicate generations, and keep input durations independent of presentation count. **Verify:** M table-driven tests.

### REQ-SCHED-002 — Tap edge
A tap holds for exactly N admitted ordinary input updates, releases before the next admitted update, settles for exactly M subsequent admitted input callbacks, and preserves unowned controls. The callback that reaches an inclusive watchdog maximum performs the surrounding ordinary update only after terminal release and is not a held/admitted scheduler update. **Verify:** M timelines and linked boundary test.

### REQ-SCHED-003 — Arithmetic safety
Counters, durations, record sequences, state versions, and capture generations shall use checked arithmetic; overflow is a typed terminal failure, never wraparound or panic. **Verify:** M boundaries/property tests.

### REQ-DET-001 — Deterministic scope
For a supplied callback/event sequence, parsing, scheduler transitions, key ownership, counters, terminal priority, and names are deterministic. Game timing, RNG, audio, animations, and pixels are not claimed deterministic. **Verify:** M/R.

### REQ-WATCH-001 — Inclusive limits
On an Input/Present entry the applicable counter is checked-added and stored before comparison. Work is permitted only when post-increment `input_seen < max_input_ticks`, post-increment `present_seen < max_presentations`, and sampled `elapsed < timeout`; equality is terminal and admits no action work. A maximum `M` therefore admits at most `M-1` callbacks, and validation requires at least `N+1` for N required updates. **Verify:** M initial/one-below/equal/one-over timelines.

### REQ-WATCH-002 — Service points and priority
Every active input and committed-present callback samples monotonic time once, applies the authoritative reducer in §2 of the execution contract, and evaluates all budgets before action work. Applicable counter overflow is typed first; simultaneous limit priority is input, presentation, wall, then clock regression. Terminal callbacks do not increment again. **Verify:** M transition/property tests.

### REQ-WATCH-003 — Monotonic clock
Wall deadlines derive from `Instant`; a test clock that regresses shall produce a typed clock failure. **Verify:** M.

### REQ-WATCH-004 — Cooperative versus hard hang
Reports shall distinguish cooperative timeout from parent-observed hard hang. **Verify:** M real child and controlled fixture.
## Reusable I/O and state primitives

### REQ-IO-001 — Durable exclusive file primitive
A reusable synchronous writer shall implement temporary create-new in the destination directory, write/encode callback, buffered flush, file recovery, file sync, close, exclusive no-replace final-name publication, and supported directory-sync classification with fault injection at every step. It removes only its owned temporary file on failure and does not itself claim capture completion. **Verify:** M.

### REQ-IO-002 — Exclusive artifact primitive
Validated labels and monotonic sequence components shall produce root-confined exclusive paths and reject collision/traversal without overwriting. It does not itself claim a graphics artifact was produced. **Verify:** M.

### REQ-IO-003 — Identity manifest primitive
A reusable identity module shall compute SHA-256 file and sorted root-confined tree manifests, with stable ordering and mutation sensitivity. End-to-end identity integration remains a proof responsibility. **Verify:** M.

### REQ-STATE-001 — Sticky terminal reducer
A pure reducer shall make the first terminal outcome absorbing, retain later errors as secondary evidence, and emit release/abort/stop intents. It does not itself mutate C activity or unwind a lifecycle. **Verify:** M/property.

### REQ-STATE-002 — Synchronization failure model
The pure runtime model shall represent poison as terminal-only recovery, unusable lock as atomic fallback, and callback reentry as fail-closed without lock acquisition. Actual ABI containment remains an integration responsibility. **Verify:** M.

### REQ-STATE-003 — Finalization ownership model
The pure model shall permit exactly one state take/finalization and shall reject late writer access. Actual lifecycle teardown ordering remains P05-owned. **Verify:** M.

### REQ-STATE-004 — Nonoverlapping lock-phase model
Instrumentation shall accept graphics copy/release -> pure runtime reserve/commit lock/release -> separate ordered-file lock/release, and reject runtime/ordered-I/O overlap or either lock across C/SDL/graphics/logging/waits/observers. Actual graphics/input paths remain P06/P07-owned. **Verify:** M.


## Input, observation, and semantic proof

### REQ-INJECT-001 — Injection location
Every real `DoInput` iteration shall call service after both SDL pump opportunities and before its sole `UpdateInputState`. **Verify:** M linked production harness and real trace.

### REQ-INJECT-002 — Input tick
An active-gate service call checked-increments the input ordinal exactly once before the watchdog comparison, including the boundary callback that reaches the inclusive limit; sticky-terminal and inactive calls do not increment it. Separate atomics count ABI entries and active-gate entries. **Verify:** M.

### REQ-INJECT-003 — Bounds-checked production setter
`c_SetImmediateMenuKey` shall bounds-check before indexing, normalize nonzero to 1, and leave all state unchanged on invalid indices. **Verify:** M production-symbol linked harness; ASan where available is supplemental.

### REQ-INJECT-004 — Ownership overlay
Only owned menu slots are written, after physical pumps; all player and unowned menu state remains physical-path-owned. **Verify:** M sentinels/conflict test.

### REQ-INJECT-005 — Sole input update
Automation shall not call `c_UpdateInputState` or directly write current/pulsed state. Real `DoInput` calls `UpdateInputState` exactly once. **Verify:** M/R.

### REQ-INJECT-006 — Actual observations
Post-update observation shall read exact existing `c_GetCurrentMenuKey` and `c_GetPulsedMenuKey` with validated indices and trace intended/current/pulsed values. **Verify:** M linked and real proof.

### REQ-INJECT-007 — Observation stop propagation
The post-update observer shall return stop. `DoInput` combines service and observation stop and checks immediately after observation, before journal work, sounds, `inputCallback`, or `InputFunc`; if observation fails, all those counters remain zero for that iteration. **Verify:** M linked real-`DoInput` harness with forced getter/trace/panic failures.

### REQ-SEM-001 — Typed synchronous main-menu observer
At the actual Rust navigation commit, order draw -> `state.cur_state = new_item.as_u8()` -> `ops.sync_cur_state` -> typed `MainMenuTransition`. The observer returns `CallbackControl`; Stop propagates synchronously through `handle_navigate`, `do_restart_frame`, and the full panic-contained `rust_do_restart_frame` shell before sleep or later frame work. The complete `rust_start_game` shell is also panic-contained and maps terminal/panic to 0. **Verify:** M unit/linked + real binary.

### REQ-SEM-002 — Machine movement assertion
P08 shall require an observed `NewGame -> LoadGame` transition caused by its down tap; missing, duplicate, wrong-source, or wrong-target events fail. Screenshots are supplemental. **Verify:** M.

## Terminal, unwind, and lifecycle

### REQ-EXIT-001 — First-wins sticky terminal
The first terminal outcome is irreversible through finalization; later errors are recorded as secondary and cannot turn failure into success. **Verify:** M transition/property tests.

### REQ-EXIT-002 — Exact OR-only abort
Terminal processing reads activity via `mainloop::ffi`, writes `before | CHECK_ABORT` where `CHECK_ABORT == 0x4000`, and preserves every prior bit. **Verify:** M property test.

### REQ-EXIT-003 — Terminal callback behavior
Every later service/observation/safe-point callback releases owned keys, reasserts abort, and conservatively requests stop. **Verify:** M clear-between-calls test.

### REQ-EXIT-004 — Clearing-caller matrix
The implementation shall cover every current `&= ~CHECK_ABORT` site and relevant parent loop listed in overview §4, distinguish local abort semantics from automation terminal state, and include linked integration tests for restart/setup/battle/pick-melee plus source-regression coverage for FMV. **Verify:** M/R regenerated inventory.

### REQ-EXIT-005 — Non-DoInput loops
`DoConfirmExit`, `BackgroundInitKernel`, `MeleeGameOver`, `AnyButtonPress`, current `rust/src/comm/talk_segue.rs::do_talk_segue -> c_UpdateInputState`, and every newly inventoried non-`DoInput` input loop shall expose automation safe points before and post-update observation after each direct update, or a source-grounded outer guard that prevents reaching it. Terminal state exits without response sound/action, button wait, communication continuation, or continued play. **Verify:** M linked/shared-production-helper tests.

### REQ-EXIT-006 — Outer Rust guard
Rust lifecycle/game/restart orchestration shall check the sticky terminal at outer boundaries, reassert abort before/after nested calls, and prevent a clear site from creating another normal iteration. **Verify:** M integration tests.

### REQ-EXIT-007 — No alternate process exit
Callbacks shall not use `process::exit`, `ExitRequested`, `KEY_EXIT`, SDL quit, or global signals for cooperative termination. **Verify:** R/M.

### REQ-EXIT-008 — Existing cleanup then status
Cooperative outcomes unwind through game cleanup and `teardown_subsystems`; status is mapped only afterward, zero only for fully evidenced success. **Verify:** M ordered lifecycle tests and real proof.

### REQ-EXIT-009 — Post-teardown evidence
`run_end` is finalized and every automation output handle is closed before teardown. Only after teardown returns shall lifecycle code create_new/write/flush/sync/drop `teardown-complete.json` and attempt supported directory sync; it shall not keep a marker handle open across teardown. **Verify:** M forced ordering/failure and real child.

## FFI and synchronization

### REQ-FFI-001 — Complete ABI shells
Every C-facing automation path and the complete `rust_do_restart_frame`, `rust_start_game`, and `rust_gfx_postprocess` shells shall follow execution-contract §3: ABI counter, atomic inactive fast path, active counter, depth guard, outer `catch_unwind`, pure transition under mutex, unconditional unlock before external work, reservation/effect/ordered-publish/validated-commit, fallback, and conservative return. **Verify:** M forced-panic/order tests per export/path.

### REQ-FFI-002 — Lock-free fallback mirrors
Mutex poison shall recover only to terminal. Fixed-size lock-free atomics shall mirror first terminal/status, abort request, runtime phase, capture request generation, and per-key ownership/value so fallback can release/abort without the mutex. Normal scheduling never resumes; startup rejects active mode if required atomics are not lock-free. **Verify:** M poison/unusable-lock/release tests.

### REQ-FFI-003 — Reentrancy
Only after the active gate, a thread-local depth guard shall reject nested callbacks without blocking or locking, latch `ReentrantCallback` through mirrors, release mirrored keys, OR abort outside locks, and return the callback-specific conservative value. **Verify:** M callback-induced reentry per shell.

### REQ-FFI-004 — Lock ordering and pure transition
The runtime mutex may protect only a pure state transition/reservation or validated commit. It shall be unconditionally released before C, SDL, present/graphics acquisition, logging, condition waits, trace/file I/O, observer callbacks, or teardown. Ordered I/O uses a separate synchronization object and no runtime-lock nesting. **Verify:** M instrumented locks/R.

### REQ-FFI-005 — Two-phase commit and finalization race
Every side-effecting callback shall reserve a checked sequence/state version under lock, execute effects unlocked, publish/cancel in strict order, then re-lock to commit only a matching token/generation. Finalization clears activation/capture, drains active shells/reservations, takes ownership exactly once, and denies late access to dropped resources. **Verify:** M stale/duplicate/panic/concurrency tests.

## Presentation and capture

### REQ-PRESENT-001 — Present-call completion
Count and observe exactly once only after `Canvas::present()` returns normally. Since the API returns `()`, this means call completion, not display acknowledgement. **Verify:** M graphics boundary.

### REQ-PRESENT-002 — Excluded paths
No-redraw early return and `TFB_FlushGraphicsEx(TRUE)` shall not count or complete capture. **Verify:** M production-symbol linked present harness.

### REQ-SHOT-001 — Logical source
Capture logical main surface 0 at 320x240 after present-call completion; metadata states that window scaling, transition/fade/system-box overlays, and direct video may be absent. **Verify:** M/R.

### REQ-SHOT-002 — ABI-authoritative validated surface
Validate pointer, dimensions, positive pitch, checked size, pixels, BPP, and RGB masks through `sdl2::sys` ABI-authoritative SDL types or narrow C accessors compiled against the linked SDL headers. Capture shall not dereference the existing partial hand-written `SDL_Surface` format pointer as an assumed `SDL_PixelFormat`. **Verify:** M ABI/version, padded/mask/null/overflow tests.

### REQ-SHOT-003 — Real SDL locking
Evaluate `SDL_MUSTLOCK` through the linked macro accessor (or ABI-equivalent proven helper). When true, real `SDL_LockSurface` success is required before reading and real `SDL_UnlockSurface` occurs on every exit via a guard; lock failure is terminal/no-read. A production-linked real lock-required surface test and a linked forced-lock-failure test are mandatory; fake tests are supplemental. **Verify:** M.

### REQ-SHOT-004 — Generated capture completion and durability
Capture request uses a nonzero atomic generation and remains pending until a matching-generation owned snapshot completes temporary create-new encode, writer flush, file recovery/sync/close, exclusive final-name publication, supported directory-sync attempt, ordered trace publication, and validated reducer commit. Stale/duplicate generations and any failure cannot advance. **Verify:** M fault-injection each step/generation.

### REQ-SHOT-005 — Exclusive names
Sanitized label plus presentation sequence is created exclusively and never overwrites. **Verify:** M collision tests.

### REQ-SHOT-006 — Required failure
Copy/lock/encode/write/flush/sync/close/record failure latches terminal and cannot produce a capture success. **Verify:** M.

## Trace and identity

### REQ-TRACE-001 — Two-phase ordered event JSONL
Reserve checked sequence plus immutable payload under the pure runtime transition, unlock, execute effects, then synchronously publish or cancel through a dedicated ordered commit cursor before validated state commit. Every reservation has an RAII cancellation publication so gaps cannot deadlock. Emit independently parseable run-start, input, present, semantic, capture, error, run-end, and lifecycle records with monotonic elapsed observations. P05 integrates lifecycle, P06 input/menu, P07 present/capture, and P08 validates the full order. **Verify:** M.

### REQ-TRACE-002 — Real state only
No invented menu/game state or screenshot-as-pass field is permitted. **Verify:** M/R.

### REQ-TRACE-003 — Trace failure
A write/flush/finalization failure is terminal and cannot report success. **Verify:** M writer injection.

### REQ-TRACE-004 — Identity digest
Metadata/report shall include SHA-256 digests for executable, script, sorted content manifest, generated build configuration, and initial/final isolated config manifest. Paths alone are insufficient. **Verify:** M mutation tests.

### REQ-TRACE-005 — Terminal ordering
Exactly one attempted `run_end` precedes actual teardown; exactly one `teardown_complete` follows it. Missing/reversed/duplicate markers fail. **Verify:** M.

## Inactive transport

### REQ-TRANSPORT-001 — Authenticated main-thread normal-SDL transport
Only proof inactive-smoke binds an exclusive mode-0600 Unix datagram endpoint with random 256-bit nonce, typed command IDs, replay rejection, packet cap, and acknowledgements. Immediately before existing C `TFB_ProcessEvents` calls in `DoInput` and TaskSwitch/Sleep pump macros, the main thread nonblocking-pumps requests and only `SDL_PushEvent`s ABI-authoritative key events. The initialized child queries the first actual production-parsed `menu.down.N` `VCONTROL_KEY`; no parent default/independent parse is allowed. **Verify:** M real child.

### REQ-TRANSPORT-002 — Path-specific observable counters
A separate inactive counters file shall distinguish datagram accept/reject/ack, push/fail, C `SDL_PollEvent` before `ProcessInputEvent`, Rust VControl dispatch, ordinary post-`UpdateInputState` menu-down observation, SDL quit push/poll, per-shell ABI entry, active-gate entry, automation service transitions, and setter writes. Inactive acceptance requires active-gate/service/setter zero while ABI entry may be nonzero. **Verify:** M.

### REQ-TRANSPORT-003 — Observed cooperative quit and separate receipt
After `key_observed` acknowledgement, `quit_smoke` shall push genuine `SDL_QUIT`; stop is permitted only after C `TFB_ProcessEvents` polls it and the lifecycle observes `QuitPosted`. It then performs normal teardown and creates separate `inactive-teardown-complete.json` after counters/socket/acks close. The active teardown receipt must be absent. No handler-direct input/stop, custom mutating event, or global signal/name kill is allowed. **Verify:** M.

## Linked tests

### REQ-TEST-001 — Unit/property tests
All parser, reducer, watchdog, terminal, trace, identity, capture, lock, and runner boundaries shall have deterministic positive/error/boundary tests; scheduler/watchdog arbitrary callback sequences use `proptest` already present. **Verify:** M.

### REQ-TEST-002 — P00/P00a feasibility and production C input harness
Before P01, a minimal declared Cargo harness shall prove deterministic production archive construction, search/archive/external-library order, Rust export retention, rerun dependencies, and extraction of source-grounded `gameinp_rust_main.o::DoInput`/`AnyButtonPress` and `confirm.c.o::DoConfirmExit`, or shared production guard helpers called by every real site. P06 extends it for the setter/order matrix; shims shall not copy tested logic. **Verify:** M link map/`nm`, run, mutation.

### REQ-TEST-003 — Production swap/SDL harness
A separate declared Cargo harness shall link real `sdl_common.c.o::TFB_SwapBuffers`/`TFB_ProcessEvents` and `dcqueue.c.o::TFB_FlushGraphicsEx`, rely on `TFB_InitGraphics` for its single DCQ init, use preflight-proven dummy+hidden software SDL, and prove skip/no-redraw/present plus real lock-required/lock-failure paths. **Verify:** M link map/`nm`, run, mutation.

## Proof runner and real proof

### REQ-PROOF-001 — Real launch/drive/observe/verify
The runner shall launch the actual Rust-owned binary with real initialization/content/C game logic/Rust graphics, drive input, observe typed menu transition and presentation, and verify all evidence. **Verify:** M.

### REQ-PROOF-002 — `ChildSession` supervision
A `ChildSession` shall own Child/identity/pipes/bounded readers/socket/manifest. `try_wait Some` is stored as the one successful reap and is not followed by `wait`; otherwise failure performs cooperative stop, bounded poll, child-only kill if live, and `wait` retried on Interrupted until the one reap or hard failure. Only then close parent pipes, drain/join, clean socket, and orphan-check. Kill/reader/join errors never skip required reap; explicit `finish` is required and Drop only a nonpanicking backstop. **Verify:** M full fault/panic/hang table.

### REQ-PROOF-003 — Unique roots and orphan refusal
Each run exclusively creates a unique root/config/output/socket/manifest. Preflight refuses matching live manifest-owned children or a live process with matching executable identity; it never globally `pkill`s. Teardown performs owned-orphan checks. **Verify:** M.

### REQ-PROOF-004 — Machine main-menu proof
The real script captures before, taps down, requires typed `NewGame -> LoadGame`, captures after, requires `CHECK_ABORT` clear, and finishes. PNG visual review is supplemental. **Verify:** M.

### REQ-PROOF-005 — Cooperative timeout proof
A real script reaches a watchdog limit, reasserts abort through clear sites, emits failure/run_end/teardown_complete, exits nonzero before parent deadline, and leaves no child. **Verify:** M.

### REQ-PROOF-006 — Hard-hang proof
A controlled child that reaches no callback is child-killed/waited and classified hard hang, distinct from cooperative timeout. **Verify:** M.

### REQ-PROOF-007 — Report-after-teardown
The parent writes `proof-report.json` only after child wait, output drain, identity/trace/capture/transport validation, teardown marker, and orphan checks. **Verify:** M.

### REQ-PROOF-008 — No manual-only acceptance
Verifier shall fail if semantic observer, inactive transport/counters, teardown marker, identity digest, production-linked C harness, or autonomous proof command is absent. **Verify:** M gate mutation tests.

## Traceability/ownership

| Primary owner | Requirements |
|---|---|
| P00/P00a | QUALITY-001..003; existing Cargo-test linker blocker; actual strict-Clippy inventory; executable environment and TEST-002/003 minimal link feasibility probes |
| P01/P01a | BUILD-002, DEP-001..003, SCRIPT-001..006 |
| P02/P02a (pure) | SCHED-001..003, DET-001, WATCH-001..003, atomic capture-generation model, TEST-001 property subset |
| P03/P03a (pure I/O) | IO-001..003 and TRACE-001 ordered reservation/publication primitive only |
| P04/P04a (pure runtime) | STATE-001..004, WATCH-004 classification, ABI shell/mirror/fallback model only |
| P05/P05a | MODE-001..003, BUILD-001, EXIT-006/008/009 lifecycle, FFI-005 finalization, lifecycle trace integration, active teardown receipt |
| P06/P06a | INJECT-001..007, SEM-001, EXIT-001..007 input/unwind integration, FFI-001..005 input/menu shells, input/menu trace integration, TEST-002 extension |
| P07/P07a | PRESENT-001..002, SHOT-001..006, TRACE-001..003 present/capture integration, capture generation, FFI-001/004 graphics shell, TEST-003 extension |
| P08/P08a | SEM-002, TRANSPORT-001..003, PROOF-001..008, TRACE-004/005 and cross-callback order, inactive teardown receipt, watchdog/hang; ARCH review remains OPEN |
