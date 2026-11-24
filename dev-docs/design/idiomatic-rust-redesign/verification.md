# Persistence and verification need fewer authorities, not another layer of wrappers

## Scope and verdict

Source pin: [`678f8c11defe28a85cfccac29899410209e9963c`](https://github.com/acoliver/uqm/tree/678f8c11defe28a85cfccac29899410209e9963c). Every source reference below refers to that commit. Paths are repository-relative and line numbers are one-based.

The save codec is already a useful Rust boundary. The main remaining design work is in campaign installation and the verification system: loading mutates owners before all validation succeeds, automation repeats execution and evidence state, and several proof/reporting implementations make different promises about the same operation. Moving those responsibilities behind coherent owners would improve this code more than renaming C-derived records or splitting long files mechanically.

Priorities:

1. Stage and validate complete campaign restores before committing live state. Remove temporary live-state mutation from save capture where the format does not require it.
2. Give a run one execution state and one typed evidence vocabulary. Keep the existing reducer, production observations and ordered publication semantics.
3. Replace the outer native controller's optimistic cleanup reporting with observed process receipts. Remove unused proof models after moving their useful tests to real paths.
4. Give online and offline evidence validation the same bounded, descriptor-backed input interface. Avoid whole-bundle cloning and temporary materialization.
5. Repair source-path selection after the crate moves and distinguish measured trace sizing from enforced upper bounds.
6. Evolve parity by defining observable contracts explicitly. Retain exact wire, gameplay, timing, count, save and pixel checks where they prove behavior; stop treating internal object layout as behavior.

This is a source design audit, not a runtime qualification or an exploit assessment. No build, test, gameplay, native capture, package operation or repository program was run. Test descriptions mean inspected test source, not passing executions. The only intended new file is this report. The preexisting `.llxprt/LLXPRT.md` modification was left alone. HEAD was verified as the requested pin at the start.

All requested module families were walked. Persistence implementation coverage is deeper than the many unrelated species integration cases: those integration cases were grouped by shared harness and contract, with representative bodies inspected. The appendix lists the complete source inventory, including test-only files; inventory is not a claim that every assertion or platform branch received a line-by-line correctness review.

## 1. Persistence: preserve the file boundary and replace the live transaction boundary

### 1.1 There are three responsibilities, but more than three representations

The dependency direction is sound: `uqm-save` owns file data and codecs, engine save maps world owners, and application savegame owns mounted storage and mode/session integration. `rust/crates/save/src/lib.rs:1`, `rust/crates/engine/src/save/mod.rs:1-22`, `rust/src/savegame/mod.rs:1` and `rust/src/savegame/live.rs:1-12` establish those boundaries. Keep that direction.

The representations currently form this chain:

| Representation | Responsibility | Assessment |
|---|---|---|
| `uqm_save::SaveFile`, `Summary`, fixed-width section records | UMS file contents | Retain a distinct wire model. Fixed arrays, raw name bytes and numeric tags are justified here. |
| `RuntimeSections`, queue/group/state-file section aggregates | Transfer between decoded sections and owners | Useful if they are validation plans or scoped projections. Mere field-copy aggregates should not become independently mutable snapshots. |
| `World`, `StoryState`, clock/event owner, queues, state files, flagship | Running campaign | These should be the sole runtime authorities. The `GameRecord` story array is explicitly a shadow. |
| `SummaryDesc` with `SisState`, counts, arrays and NUL-padded name | Menu preview | A second nearly complete summary model. It still has C-style counts and sentinels despite no longer being an external ABI. |
| legacy `LegacySave` with modern `SaveFile`, image array and group reference | Pre-UMS import | A distinct import result is justified. Synthesizing modern bytes to decode them again is unnecessary. |
| automation campaign snapshots and save-provenance campaign JSON | Observations | Separate views are reasonable, but interpretation of star identity and saved/live state must be deliberate. |

Evidence: `rust/crates/save/src/types.rs:78-112,330-355`; `rust/crates/engine/src/save/summary_record.rs:5-61`; `rust/crates/engine/src/save/mod.rs:24-38,88-126`; `rust/src/savegame/legacy.rs:43-83`; `rust/src/automation/campaign_state.rs:21-72,300-386`; `rust/xtask/src/native_save.rs:300-459`.

A top-level Rust design would use one campaign owner, a validated restore plan, a wire DTO and a small menu view. It would not force runtime state into the wire DTO, nor retain a whole duplicate summary solely to preserve old callback signatures. `Point` in the codec and runtime geometry need not be unified: their meanings and dependency constraints differ even when their current scalar fields match.

### 1.2 Complete restore validation precedes neither complete mutation nor RNG effects

`restore_runtime` validates its event section locally, then replaces story, installs events and applies globals. `apply_with_files` invokes it before restoring queues, loaded groups and state files. Any later error follows earlier committed changes. The flagship summary is restored at the end. Local prevalidation is real, but does not make the whole load transactional.

Evidence: `rust/crates/engine/src/save/mod.rs:107-126,210-265`; queue restoration in `rust/crates/engine/src/save/queues.rs`; `rust/crates/engine/src/save/groups.rs`; `rust/crates/engine/src/save/statefiles.rs`; the existing pure state-file planner in `rust/crates/save/src/statefiles.rs`.

The queue path also rebuilds NPC/escort records through operations that consume captain-name RNG and preserve live fleet resource/template fields. Simply cloning the wire structure or rolling back a few globals would not solve this. Build a `RestorePlan` against borrowed catalog/templates, reserve required capacities, validate all domain constraints, and stage random-dependent choices with an explicitly committed RNG state. Commit owners only when the plan is complete. If allocation or owner installation remains fallible, that belongs before the irreversible commit as well.

Application completion currently resolves stars with the mode's coordinate lookup, whereas `runtime/star::resolve_star` additionally rejects disagreeing descriptor fields and noncatalog points. That stricter resolver is used by provenance extraction. These are two acceptance policies, not interchangeable helper functions. Preserve gameplay compatibility with one explicitly named resolver policy; provenance may require stronger evidence without silently redefining which saves gameplay can load.

Evidence: `rust/src/savegame/live/load.rs:12-25,44-58`; `rust/src/savegame/runtime/star.rs:23-27,115-197`; `rust/xtask/src/native_save.rs:300-459`.

### 1.3 Capture temporarily changes the campaign to express saved state

The live capture path saves the original IP point, prepares hyperspace or solar fields, calls group persistence, then delegates to `capture_prepared`. Engine `snapshot` captures summary/runtime, restores the live IP point and clears the orbit marker, and only then captures queues, groups and state files. Fallible operations exist on both sides of that cleanup.

Evidence: `rust/src/savegame/live/save.rs:14-29`; `rust/src/savegame/runtime/live.rs`; `rust/crates/engine/src/save/mod.rs:129-177`; `rust/tests/save_owned.rs:104-197`.

A Rust capture should derive saved location from an immutable session projection rather than overwrite the game to make the serializer see it. Group/scan persistence that truly belongs to save behavior should be an explicit preparation effect with its own tested commit semantics. Preserve successful post-save effects that gameplay depends on, including live orbit/location normalization, rather than assuming all mutation is accidental. Test early errors for exact world, RNG, clock and state-file consequences.

### 1.4 Slot APIs erase errors and retain historical failure behavior

The engine installs a function-pointer service with `CStr`, `u16`, mutable output summary, `bool` success and unreadable-slot `year_index == 0`. The app returns `u32` 1/0, then converts that back to `bool`. Its typed storage errors are logged and lost at this boundary.

Evidence: `rust/crates/engine/src/save/slots.rs:11-32`; `rust/src/savegame/live.rs:6-26`; `rust/src/savegame/live/load.rs:28-78`; `rust/src/savegame/live/save.rs:32-48`.

Use a validated `SlotId`, a raw-byte save name type, a typed `SlotPreview` state and `Result`. An ordinary installed service is sufficient; no service-container framework is needed. Menu rendering should decide how errors become user messages. A summary is a returned value, not an output parameter whose apparent validity can predate successful persistence.

Storage writes encode before opening, then open the final slot, write, finish and remove that final slot on failure. An old save is not protected against a failed overwrite. Modern read-open failures are also collapsed into missing/legacy detection, losing the original I/O cause. Full load probes a summary and reopens the file for its body. Summary failure triggers legacy fallback, body failure does not.

Evidence: `rust/src/savegame/slots.rs:43-87,101-129`; `rust/src/savegame/live/load.rs:34-57`; `rust/crates/save/src/error.rs:52-104`.

Atomic replacement is a deliberate failure-behavior improvement, not a byte-compatibility requirement. Add it through the Files port with tests for write, finish, replacement and cleanup faults, keeping the old slot until commit. Do not quietly change fallback precedence while doing it. Prefer one opened, identified stream for a full load; retain summary-only reads for the picker.

### 1.5 Legacy decoding belongs with the codec, not engine bit helpers

The legacy importer lives in the app, uses engine raw story-bit helpers, creates synthetic UMS chunks, and feeds those bytes into the modern decoder. Its `Reader` allocates a `Vec` even for a byte or a skipped region. This adds memory traffic and couples format migration to runtime state machinery.

Evidence: `rust/src/savegame/legacy.rs:2-5,7-54,57-83`; `rust/crates/save/src/lzh/mod.rs`; `rust/crates/save/src/lzh/decode.rs`; `rust/crates/save/src/lzh/encode.rs`; `rust/crates/save/src/lzh/huffman.rs`.

Move pre-UMS import and pure bit transposition to `uqm-save`, decode directly into its typed import result, and keep mounting/slot lookup in the app. Use stack arrays and bounded skip buffers. LZH is already `Read`/`Write` based and has byte fixtures covering dictionary wrap and adaptive reconstruction. Retain the algorithm's wire-specific dictionary, lookahead, tie ordering and EOF lookahead behavior. The shared `CodeDesc` combines encoder/decoder bookkeeping and optional tree state; splitting direction-specific state is reasonable if exact compressed bytes stay identical, but replacing the algorithm with a generic compressor is not.

The LZH decoder zero-extends `UnexpectedEof` during original-style lookahead; it is not evidence by itself of a parsing bug. Declared output length, aggregate save limits and encoder length overflow deserve explicit boundary tests. The encoder's wrapping stream-length bookkeeping should be a fallible representability check rather than silently wrap beyond the format's width. No oversized-stream failure was reproduced here.

### 1.6 Codec validation is good locally but incomplete as an aggregate contract

Modern decoding checks declared sizes before allocation/skip, validates section shapes, bounds per-chunk collections, and exposes typed external-data errors. Retain those safeguards. Repeated queue chunks append, while several singleton sections replace prior values. A per-chunk bound is not a whole-file resource budget. The codec also accepts publicly constructible values that the encoder sometimes rejects with assertions, such as invalid canned-node records.

Evidence: `rust/crates/save/src/decode.rs:1-15,36-125` and `read_chunks`; `rust/crates/save/src/types.rs:37-61,335-355`; `rust/crates/save/src/encode.rs`; `rust/crates/save/src/error.rs:19-43`.

Define whether a decode API accepts any physically available stream or a configured maximum save size, collection count and decompressed length. Preserve supported duplicate-chunk semantics instead of making a cleanup accidentally reject old files. Use validated construction or fallible encoding for externally sourced invalid model data. Internal impossible states should still fail visibly. Keep `SaveName`'s raw non-NUL bytes; replacing it with `String` would narrow the actual format contract.

## 2. Automation: retain pure controllers and remove parallel protocol knowledge

### 2.1 The reducer is useful; the run state around it needs one authority

The scheduler already uses a closed action enum, typed phases, checked counters, absorbing terminal state and effect plans. Campaign predicates distinguish unavailable values from zero. Scenario initialization has a consumed-once pending scene. Those are good Rust designs.

Evidence: `rust/src/automation/scheduler.rs:97-215`; `rust/src/automation/scheduler/assertions.rs`; `rust/src/automation/script.rs`; `rust/src/automation/scenario.rs:122-218`; `rust/src/automation/campaign_state.rs:300-386`.

The coordinator separately holds scheduler state, terminal class, finalization, readiness, counters, pending input, capture state and many domain trackers. `RuntimeModel` adds another mutable runtime state plus atomics and an active-shell lifecycle. Terminal state is mirrored into an atomic first-wins cell and synchronized back into coordinator state. The terminal helper reads the mirror, records an inner terminal, publishes it, and raises the gameplay abort bit. This is a maintained synchronization protocol, even though no race was reproduced.

Evidence: `rust/src/automation/coordinator.rs:155-460`; `rust/src/automation/runtime.rs:29-63,119-158,240-345`; `rust/src/automation/outcome.rs:26-181`; `rust/src/automation/coordinator/terminal.rs:1-20`.

Prefer one run-owned execution state on the admitted callback path, with narrow snapshots for observers and only the cross-thread mirrors that callbacks actually require. Represent the current action's active controller as an enum carrying its state, rather than keeping unrelated controller options indefinitely. Keep battle, navigation, mineral, turn, rebirth and ending controllers separate where they represent different domain algorithms. Do not replace all of them with a generic workflow engine.

The observer boundary matters: campaign values are observed before effects; injected player input is checked against production current/pulsed state; save capture is deferred until the borrowed game owner is returned. Preserve those orderings, not just eventual terminal success. `rust/src/automation/observer.rs:1-152`, `rust/src/automation/coordinator/save_capture.rs:1-115`, `rust/src/automation/coordinator.rs:2934-2961`.

### 2.2 Evidence is still a string protocol inside a mostly typed system

`TraceRecord` has a kind plus many optional payload fields and multiple schema versions. Producers repeatedly construct sparse records. Consumers recover domain facts from labels such as observed input, displacement, mineral collection and battle-frame completion. Some predicate modules share label helpers, which is better than independent ad hoc parsing, but the protocol still makes display formatting part of validation logic.

Evidence: `rust/src/automation/trace.rs:29-263`; `rust/src/automation/coordinator/tracing.rs:1-196`; `rust/src/automation/native_predicate.rs:82-133`; `rust/src/automation/campaign_state.rs:332-386`; `rust/src/automation/ship_resources.rs:46-124`; `rust/src/automation/pkunk_rebirth.rs:153-217`; `rust/src/automation/native_window.rs:2556-2660`.

Introduce a closed domain-event payload enum with per-event fields and constructors. Evaluate the action's predicate against that event, and derive the human-readable label from it. Keep the exact existing JSON/label formatter at the compatibility edge until a versioned migration proves equivalence. This avoids changing every recording merely to obtain better internal types.

Tracing also handles serialization failure inconsistently: several helpers commit only on successful `to_jsonl`, so a serialization failure becomes RAII cancellation, while presentation tracing explicitly calls the failure path. The types currently serialized may make that failure unlikely. A single fallible append API should nevertheless define the contract once and propagate failure into finalization.

The ordered commit implementation itself is worth retaining: reservation drop publishes cancellation, later submissions wait for gaps, and sink failure is latched. Sequence exhaustion should be an explicit terminal error, not reuse the last sequence through `checked_add(...).unwrap_or(sequence)`. Evidence: `rust/src/automation/trace.rs:313-420,457-486`.

### 2.3 There are unused specifications compiled beside production implementations

Searches across Rust source found `run_lifecycle`, `validate_proof_run`, `ArchitectureReview`, `ChildSessionModel`, and runtime transition-reservation helpers used by their model tests/reexports rather than the actual production execution path. `TransportState` is a pure authentication model exercised by model/integration tests, not evidence of an implemented socket transport. Preserve genuine production helpers that share these files.

The pure proof receipt accepts any present exit code, and its initial aggregate validity check makes several later diagnostic variants unreachable. That does not mean the real proof accepts nonzero exits. The actual gameplay validator explicitly requires `Some(0)`, no signal, reaping, drainage and cleanup. Similarly, generic `run_lifecycle` records nondurable trace state, while production `AutomationObserver::finalize` durably publishes trace and teardown and propagates evidence failures to main.

Evidence: `rust/src/automation/proof.rs:123-153,240-279`; `rust/src/automation/lifecycle.rs:166-230`; `rust/src/automation/runtime.rs:244-269`; `rust/src/automation/child_session.rs:545-723`; `rust/src/bin/uqm_gameplay_proof.rs:1560-1585`; `rust/src/automation/coordinator.rs:2511-2572`; `rust/src/main.rs:161-184`; tests/reexports in `rust/src/automation/mod.rs:119-189`.

Delete uncalled model-only public surfaces after moving useful invariants into tests of real production reducers, appenders and process supervisors. Do not delete the reference native trace oracle: it is explicitly test-only and checks a real optimized implementation against its prior semantics. That is a justified independent oracle, not a second production authority.

### 2.4 Scenario inventories are distributed and path selection is stale

Script discovery, the domain matrix, parity corpus, per-scenario counts, native smoke set, checkpoint planning and CI selection each carry part of scenario policy. The coordinator even enables native floors by the literal scenario name `linked-playable-v1`. These should be explicit scenario capabilities or admitted policy, not incidental names.

`automation/suite.rs` still maps `rust/src/battle`, `rust/src/comm`, `rust/src/planets`, `rust/src/starbase`, and `rust/src/save`/`load` paths after those families moved or became `savegame`. Its safe fallback runs the entire required suite, so this is a selection-efficiency and maintenance defect, not proof of skipped tests. The CI triple-recording selector also lists old hook, time, sound, collection and pacing paths. Full Linux parity is selected independently for nondraft PRs when the repository variable is on; stale paths chiefly affect extra cross-platform/jitter recording selection.

Evidence: `rust/src/automation/suite.rs:866-947`; `rust/src/automation/coordinator.rs:460`; `.github/scripts/parity-policy.cjs:1-26`; `.github/workflows/ci.yaml:175-228,272-378`; `rust/xtask/src/native_config.rs:1-159`; `rust/xtask/src/native_cli.rs:279-362`; `rust/xtask/parity-corpus.toml`; `rust/xtask/smoke.json`.

Use one validated scenario inventory for identities, tags, fixture status and required capabilities. Keep distinct views for gameplay coverage, deterministic parity and native smoke: their obligations differ. Generate or validate path-selection views against the current tree and fail when a named specific source path no longer exists. Unknown paths must keep their conservative behavior. Do not replace all checks with one metadata declaration that can claim its own success.

## 3. Proof binaries and xtask: consolidate mechanisms without merging evidence claims

### 3.1 The outer supervisor makes weaker claims than the inner one

Production `ChildSession` has an explicit process identity, anchored group ownership, bounded log readers, targeted signals, nested-registration handling, reaping, cleanup diagnostics and bounded reader joins. Native candidate launch additionally clears environment, confines filesystem/process operations, and uses `spawn_leaf` to strip controller protocol authority.

Evidence: `rust/src/automation/child_session.rs:2092-2421,2434-2721,2941-3022`; `rust/src/automation/native_runner.rs:1716-1797`; `rust/src/automation/native_isolation.rs:1-65`.

The native suite's outer controller instead goes through `native_gpu::run_scenario` to `xtask/process::run`. That helper ignores several kill/wait/probe errors, suppresses reader/join failures and unconditionally records `process_group_cleanup = "verified-empty"` and `pipe_cleanup = "complete"`. Its reader joins are unbounded even when a child timeout was supplied. A descendant retaining a pipe outside the group is a plausible blocking case, not a reproduced hang in this audit.

Evidence: `rust/xtask/src/lib.rs:351-534`; `rust/xtask/src/native_gpu.rs:114-200`; `rust/xtask/src/process.rs:85-169`; `rust/xtask/src/native_suite.rs:162-268`.

Use the observed-receipt supervisor for outer controller execution as well, with policy for controller registration versus candidate isolation. Do not infer cleanup from an exit status. Preserve primary failure and attach cleanup failures separately. The parity runner has another polling/kill implementation that streams logs to files and bounds reap; bring it under the same receipt semantics without forcing ordinary replay to retain native screenshot evidence. `rust/xtask/src/parity/process.rs:13-108`.

The gameplay proof binary and native acceptance controller should remain different entry points. Headless causal/gameplay proof does not prove compositor visibility. Native evidence does not prove deterministic equal pixels at equal simulation boundaries. Share mechanisms and typed evidence reading, not a generic `passed` boolean that hides which claim was proved.

### 3.2 Offline evidence reading reintroduces whole-bundle copies and weaker file access

The native boundary has component-by-component no-follow descriptor reads, bounded member/aggregate inventories and checked streamed hashing. Shared inputs are retained once, sealed and reverified through held descriptors. Keep those mechanisms.

Evidence: `rust/src/automation/native_window.rs:1241-1380,1989-2078`; `rust/src/automation/native_artifacts.rs:15-135,215-310`; `rust/xtask/src/native_inputs.rs:53-152`.

`xtask/evidence` instead inventories files into byte vectors with path-based reads, holds the entire bundle in a map and clones it from `files()`. Offline suite validation materializes scenarios/shared data into a temporary tree so validators can reopen them by path. Some suite metadata is also read separately rather than through the held snapshot.

Evidence: `rust/xtask/src/evidence.rs:12-142`; `rust/xtask/src/native_suite.rs:2344-2425,2465-2522` and `materialize_member`.

The memory cost is proportional to retained payload, potentially multiplied by clones and scenario materialization. No measured peak-memory figure is asserted. Static symlink rejection is not the same contract as descriptor-confined reads under mutation; this is a boundary inconsistency, not a demonstrated arbitrary-file-read exploit.

Make validators consume a bounded `EvidenceReader` with checked relative member identity and streaming reads. A directory-backed reader can serve online/offline validation; a small memory reader can serve unit tests. Return borrowed metadata iterators instead of cloning file bytes. Remove temporary materialization and ensure all members used in one verdict come from the same admitted reader. Keep immutable shared storage rather than copying the executable and content into every scenario.

Evidence writing also differs: durable automation artifacts use exclusive staging, sync and no-overwrite publication; xtask publisher replacement uses a fixed temporary extension and rename without the same durability contract. Assign explicit publication modes for immutable objects versus replaceable status pointers. One small implementation for each mode is enough.

### 3.3 Incremental trace parsing still rereads and compares the full prefix

The trace cursor caches parsed records and semantic/checkpoint scans. That prevents repeated JSON parsing and repeated semantic work. Its caller still reads the bounded full trace for each relevant publication, and `parse_through` compares the full prior byte prefix with `starts_with`. Thus the optimization does not make I/O or prefix comparison incremental.

Evidence: `rust/src/automation/native_runner.rs:704-805`; `rust/src/automation/native_trace_cursor.rs:88-209`; `rust/src/automation/native_window.rs:2556-2636`; `rust/src/automation/native_trace_oracle.rs:1-4`.

For many growing publications, total bytes reread can approach quadratic growth. Use a held descriptor, committed byte offsets and an explicit append/publication protocol. A descriptor alone does not prove immutability against rewriting existing bytes: preserve the current prefix-replacement detection or replace it with an equally strong chunk/prefix commitment. Reopen/rotation, truncation, prefix rewrite, partial last line, blank lines and claimed-count regression must remain tested. Keep the one-shot oracle as a differential test and add byte-read counters, not only parse/scan visit counters.

### 3.4 Admission calls measured rates upper bounds

`NativeTraceProfile::max_trace_bytes` assumes at most two presentations per input, one graphics change per twelve ticks and combat on at most one quarter of ticks. The comments identify measured rates, and the calculation is based on input ticks rather than independently configured presentation limits. Exhaustive action classification is useful, but a measured combat share or graphics-change interval is not itself an enforced invariant.

Evidence: `rust/src/automation/native_trace_bound.rs:1-85,88-156`; `rust/src/automation/native_contract.rs`; `rust/src/automation/native_trace.rs`.

This can make preflight's claim stronger than its proof and reject or exhaust evidence capacity for valid workloads. The journal's actual byte bound still prevents unbounded writes. Derive hard reservation from enforced input/presentation/record quotas and actual maximum encoded field widths, or label sizing as an estimate and make capacity admission explicitly policy-based. Do not silently drop native diagnostics or raise all limits until tests pass. Tests should drive all-combat traces, graphics changes every allowed presentation, widest identifiers/counters and independent presentation budgets.

### 3.5 Reporting has good provenance checks but should expose domain failures directly

Native acceptance validates UID/Aqua execution identity, exact command environment, inventory, linked-build receipt, provenance, window evidence, publications, terminal state and semantic trace. Suite journaling validates ordered scenario transitions, fixed selection and first-failure stability. Galleries retain missing/failed/not-run obligations and distinguish original OS screenshots from cropped client images and renderer readbacks. Retain those distinctions.

Evidence: `rust/src/automation/native_window.rs:3474-3505`; `rust/xtask/src/native_suite.rs:1857-2025,2420-2522`; `rust/xtask/src/native_gallery.rs:48-110,310-430`; `rust/xtask/src/native_frames.rs:199-315`.

Most orchestration errors are strings, while some domain errors are enums with Debug-only display and no source chain. Use typed errors where callers branch on identity, admission, timeout, storage, parse or semantic failure; attach source errors and operation/member context at I/O edges. Keep presentation formatting and CLI exit mapping at entry points. Avoid creating one enormous shared error enum that couples the save codec to native-window proof.

Save extraction binds output bytes, slot, producer script, seed, terminal and linked source commit, then derives campaign state from the output save itself. That is useful evidence. Starting-save admission currently checks a bounded regular file, not the sidecar provenance as an acceptance obligation. Decide and encode which scripts require a verified producer chain versus intentionally imported fixtures. Do not describe all admitted saves as provenance-verified merely because extraction can write a sidecar. Save and sidecar are separate writes; publish a complete pair through a manifest/commit point if consumers require both.

Evidence: `rust/xtask/src/native_save.rs:11-17,155-230,228-459,460-487`; `rust/xtask/src/native_suite.rs:24-51`.

## 4. Exact parity: define the contract before changing its representation

### 4.1 What is actually observable

| Contract | Retain exactly | Internal design freedom |
|---|---|---|
| Modern UMS, pre-UMS import, MLE, LZH | Tags, widths, endian, chunk order on write, supported optional/repeated chunk semantics, raw names, legacy transposition and fixture compressed bytes | Rust struct layout, field names, private aggregates, allocation strategy and parser organization |
| Saved campaign | Captured/restored location, story/events, queue order, resource preservation, captain RNG consumption, activity handoff, documented normalization | Owner identities, temporary section structs and public aliases |
| Input and scheduler | Admitted callbacks, hold/release/settle boundaries, current/pulsed observation, checked inclusive budgets and first-terminal precedence | How action state is stored and borrowed |
| Deterministic gameplay | Input I records, state S values/events, field sidecar, ordered spawn/death/hook/resource identities, RNG and transition outcomes | Physical pointers, allocation locations and runtime storage layout |
| Deterministic rendering | P pixel digests at the specified committed frame, plus relevant geometry/colors/contents | Renderer implementation and internal frame-handle layout |
| Scenario regression | Exact action-phase sequence/counters, terminal/completion/exit agreement, expected failures and known-bug classification | Lossless count storage encoding, report layout and scheduling concurrency |
| Runtime save qualification | All 74 capture payload/metadata pairs, first mode-entry admission and frozen-clock metadata | Capture harness implementation if it preserves the same boundary |
| Native proof | Real OS window/capture, exact executable/content/script identity, checkpoint-to-action-to-presentation correlation, cleanup and failed/not-run evidence | JSON implementation details under an explicitly versioned schema |
| Release | Versioned target assets, content discovery away from checkout, correct imports/architectures, checksums and no user-state effects during read-only smoke | Script language and staging organization |

Sources: `rust/crates/save/src/format.rs`; `rust/src/savegame/runtime/mod.rs:27-32`; `rust/tests/save_writer_fill.rs:9-27`; `rust/src/automation/watchdog.rs:163-253`; `rust/src/automation/parity.rs`; `rust/src/automation/parity/bindings.rs`; `rust/xtask/src/parity/compare.rs:3-164`; `rust/xtask/src/parity/counts.rs`; `rust/tests/save_runtime_capture.rs:13-123,339-465`; `rust/src/automation/native_window.rs:3474-3505`; `scripts/release/package.py:29-47,156-237`.

The repeated-first-element live save writer is explicitly tested historical behavior. The standalone codec preserves distinct element amounts. Keep this asymmetry at a named writer-compatibility boundary, not as an invariant of the runtime inventory. Fixing the gameplay behavior would be a separate, explained behavior change with its own golden delta.

Logical parity IDs already replace raw pointers with deterministic bindings. Preserve generation and stale-reference distinctions while removing the need to expose C-shaped records to the observer. Hooks can be stable logical hook identities; storage addresses are not a user contract. Do not replace an address with a name while accidentally collapsing two distinct generations or owner lifetimes.

### 4.2 Migration without deleting coverage

1. Inventory each currently serialized field as gameplay state, ordered event, external representation or implementation diagnostic. Record why it is checked. Do not presume every field in `parity_state` is disposable.
2. For a structural refactor, emit the existing exact format from the new typed observation. Require byte-identical traces, fields, counts and all save captures. This is the preferred route and needs no golden churn.
3. If an implementation field must disappear, first add the replacement behavior/lifetime assertion and a typed semantic projection. Run old and new projections over the same observations and prove correspondence. Preserve order, cardinality, binding-generation and failure evidence.
4. Version any changed trace/field schema. Keep frozen old recordings as decoder/projection fixtures and compare equivalent semantic fields across the transition. Do not keep two runtime serializers indefinitely after migration.
5. For intentional gameplay/pixel corrections, retain exact checks on unchanged domains, identify the precise changed field/frame/region and show repeat recording stability. An explained pixel change does not authorize unrelated S/I/count/save changes. Use the existing owner-authorized golden admission, not a relaxed comparator.
6. Retain Linux full checks, selected cross-platform/jitter checks, all scenario counts, all save pairs and species callback/opponent coverage. Preserve expected-failure and known-bug accounting. No new blanket ignore lists, timing tolerances or automatic golden acceptance.

Evidence: `rust/xtask/src/parity/manifest.rs:4-58`; `rust/xtask/src/parity/inventory.rs`; `rust/xtask/src/parity/coverage.rs:161-235`; `rust/xtask/src/parity/recording.rs`; `rust/xtask/src/parity/recording/replay.rs`; `rust/xtask/src/parity/recording/saves.rs`; `.github/workflows/ci.yaml:188-223,252-262,319-378`.

Native real-clock frame differences must not be normalized into headless parity. Native battle diagnostics, pixel changes and OS capture evidence answer different questions. `rust/xtask/src/native_battle_diff.rs:130-198` intentionally reports causal inputs/state before real-clock presentation diagnostics. Keep those categories visible.

## 5. Quality, coverage, integration support, release and CI

### Quality and coverage

The quality tool is not just a line-count script. It parses configured source across targets, tracks identity-scoped debt, crate/layer edges, raw/FFI patterns, complexity, unsafe confinement, reviewed macros and dependency/build-script restrictions. Its move ledger conserves workspace totals and rejects retained sources or reused credit. Preserve that enforcement, including zero first-party native-source files. Source analysis is not compiler name resolution, and a passing ratchet cannot prove good ownership architecture.

Evidence: `rust/xtask/src/quality/walk.rs:98-177`; `rust/xtask/src/quality/cfg.rs:15-62`; `rust/xtask/src/quality/layers.rs`; `rust/xtask/src/quality/unsafe_allowed.rs`; `rust/xtask/src/quality/deps.rs:65-201`; `rust/xtask/src/quality/metrics.rs:157-164`; `rust/xtask/src/quality/transfers.rs:8-68,192-218`; `rust/xtask/src/quality/baseline.rs:297-365`.

Coverage enforces a workspace line floor and new-production-file requirements, cross-checks LLVM JSON and LCOV, and uses Git-backed source proofs for declaration relocation. It distinguishes moved uncovered debt from new covered lines rather than fabricating coverage. Its debt allowlists and owner-threading token comparison are migration machinery. Retire a rule only when no live baseline entry depends on it and the replacement still rejects the same laundering cases. Do not drop the checks because the original epic is closed.

Evidence: `rust/xtask/src/coverage.rs:62-180` and `collect`; `rust/xtask/src/coverage_lcov.rs:11-113`; `rust/xtask/src/coverage_move_check.rs:182-272`; `rust/xtask/src/coverage_source.rs:91-156`; `rust/xtask/src/coverage_threading.rs:296-325`; `rust/xtask/src/coverage_debt.rs:79-169`.

Keep existing focused parser/conservation tests. Add enforcement mutation tests against the proposed new module layout: retained duplicate owner, renamed unsafe growth, hidden macro path, moved uncovered lines claimed as covered, missing executable-line evidence and a stale source-path rule must all fail. Separate measured-source collection, attribution and baseline policy; avoid another rewrite of the whole tool solely to reduce file count. No lint threshold or required check changes are authorized by this audit.

### Integration and test-support families

There are several useful layers: pure codec fixtures, explicit-owner campaign tests, reexecuted native UI tests, species/battle tests, production automation decisions, process failures and actual runtime save captures. Keep all of them. Codec roundtrip alone cannot catch a loader that restores the wrong owner; model-only transport tests cannot prove an actual IPC path; renderer readbacks cannot prove an OS window.

The engine display fixture builds small local resources, installs actual graphics/input contexts and tears them down with guards. Battle support uses a thread-local dynamically checked fixture and a serialized run guard, allowing scoped RefMut access without unsafe. That is acceptable test-only machinery, but it can hide reliance on ambient context. Prefer explicit fixture methods for new tests and migrate shared helpers together with their callers; do not demand production DI abstractions solely to simplify test setup.

Evidence: `rust/crates/engine/src/test_support/campaign_display.rs:1-129`; `rust/crates/engine/src/test_support/native_session.rs:1-176`; `rust/tests/support/battle_context.rs:15-101`; `rust/tests/support/ship_pool.rs:1-59`; `rust/crates/engine/src/sound/test_support.rs:13-65`; `rust/crates/modes/src/starbase/test_support.rs:24-135`.

`native_process::own_graphics_process` blocks on child output without a deadline or byte budget. Bound it and preserve exact-test-case admission. Restore process environment after fixtures that change SDL drivers, or keep the mutation wholly within a dedicated child. Test unwind/early resource-load failure as well as success. Sound lock tests use a measured grace window; keep synchronization intent and avoid asserting scheduler speed as domain behavior.

`schema_contract` usefully freezes serialized story bit positions, input binding numbers and species/module IDs. Assertions about primitive Rust scalar sizes add no application coverage, and internal enum/layout checks should be retained only when they map to external bytes or behavior. Replace obsolete layout assertions with behavior tests in the same change, not by simply deleting them. `rust/tests/schema_contract.rs:12-64`.

### Release and CI

The release route already stages away from checkout, verifies target-specific dependencies/architecture, unpacks each asset and checks version/help/content discovery and missing-content failure without user-state creation. It is proportionate to keep this in Python and the Homebrew publisher in shell. Neither language implies a C-shaped runtime architecture.

Evidence: `scripts/release/package.py:29-84,156-237`; `scripts/release/test_package.py:13-69`; `scripts/release/update_homebrew_tap.sh:1-53`; `.github/workflows/release.yml:38-57,95-139`.

Gaps to make explicit: extracted deb/rpm smoke is not package installation/uninstallation testing; ad-hoc macOS signing is not notarization; CLI smoke is not SDL gameplay startup; current package unit tests do not fault-inject every external packaging/signing/upload command. Add focused tests only for claimed release guarantees. Publishing should retain the exact validated asset identity through checksum aggregation and stable-tap update. Do not add a second Rust release framework merely for language uniformity.

CI has fmt, quality, clippy, release check/build, workspace tests, dependency checks, coverage, target builds, parity and failure scenarios. Several jobs are nondraft-only. Parity/failure jobs require repository variable `UQM_PARITY == 'on'`. This audit read the workflow, not the current remote variable or live run history. Never report parity as executed based on the YAML. Keep policy tests aligned with moved crates and test default/release/native feature combinations intentionally. Release/debug-process/native acceptance claims differ from normal shipping features and should remain visible.

Evidence: `.github/workflows/ci.yaml:23-68,85-168,174-228,272-425`; `rust/Cargo.toml`; `rust/xtask/src/clippy.rs`; `rust/xtask/src/deny.rs`; `rust/xtask/src/cli.rs:7-42`.

## 6. Vertical implementation units

Each unit includes its new path, every affected caller and deletion of the replaced implementation in the same PR. These are dependency-coherent units, not separate introduce/migrate/delete PRs. The tests below are proposed acceptance work, not checks run during this audit.

| Unit | Scope and implementation | Tests required | Delete with migration | Explicitly retain |
|---|---|---|---|---|
| V1. One persistence format boundary | Move legacy import/bit transposition into `uqm-save`; decode directly to typed records; separate codec representability from runtime validity; bound decode resources. | Existing UMS/MLE/LZH exact fixtures and roundtrips; malformed/truncated/repeated chunks; raw names; streaming I/O faults; decompression/aggregate/encoder-length boundaries. | App legacy parser/engine raw-bit dependency; synthetic modern-byte staging; per-byte/skip heap allocations. | UMS and legacy bytes, optional-chunk semantics, MLE's raw layout, LZH tie order and documented lookahead behavior. |
| V2. Planned campaign capture and restore | Engine `RestorePlan` validates all owners before commit; stage RNG; immutable session save projection; explicit successful capture effects; use named star-resolution policy. Migrate menu and save-capture callers. | Late queue/group/state-file failure leaves world, RNG and clock unchanged; successful RNG/order/resource parity; capture failures at every fallible phase; all 74 runtime saves; owner tests. | Sequential partial-install pipeline; temporary location mutation used only for serialization; duplicate star-acceptance logic where policy is the same. | Live activity until handoff, deferred ship reset, actual normalization effects and successful save bytes. |
| V3. Typed slot service and safe persistence | `SlotId`, raw save name, preview enum, typed results; Files-port replace operation; one opened full-load source; return preview after successful write. | Missing/permission/directory distinction; modern/legacy precedence; corrupt body after valid summary; all write/finish/replace/cleanup faults; picker and save-problem behavior. | u32-to-bool service roundtrip, year-zero internal absence sentinel, duplicate full summary representation where unnecessary, destructive overwrite path. | Filenames, fifty-slot UI, raw bytes, format precedence and user-visible successful save/load behavior. Failure-safety change is documented. |
| V4. Run-owned execution state | Consolidate coordinator/runtime phase and terminal ownership; current-action state enum; narrow cross-thread mirror; move scenario-name policy to admission. | Reducer transitions and invalid states; first terminal wins across callbacks; reentry/late callback; actual input observation; deferred capture; inactive path; shutdown failure and duplicate finalize. | Unused reserve/commit-transition model API, unused lock-order model, parallel fields made obsolete by the new owner. | Pure domain controllers, admission order, key release, watchdog precedence and exact scenario counts. |
| V5. Typed event and trace contract | Domain payload enum, validated constructors, single append/failure path; exact old formatter at boundary; shared typed projections for predicates/counts/proof. | Golden JSON bytes; schema/payload mismatch; all action predicates; canceled reservations/gaps/sink error; sequence overflow; partial input pairs; trace oracle differential cases. | Sparse record construction repetition, label reparsing as internal domain state, duplicate terminal/kind enums where they express the same versioned schema. | Exact legacy formatter until migration, ordered commit, typed checkpoint identity, native-only payload distinctions. |
| V6. One observed process receipt | Use production supervisor for outer native controller and parity tooling; bounded pipe/reap cleanup; truthful typed receipt; keep launch policies distinct. Remove model-only public proof/lifecycle/transport surfaces after preserving useful tests. | Real child exit0/nonzero/signal; timeout; ignored TERM; retained pipe; group survivors; observer failure; partial reader launch; interrupted supervisor; stale identity/bystander; cleanup failures. | Optimistic `xtask/process` cleanup strings/ignored-error path, duplicate parity supervision, uncalled `run_lifecycle`/proof receipt/architecture model and unused reexports. | Anchored group identity, candidate sandbox, nested controller registration, first failure plus cleanup diagnostics, production receipt writers/status helpers. |
| V7. One retained-evidence reader | Descriptor-confined bounded reader for suite/native/gameplay validators; streaming hashes and metadata iterators; no temp materialization; explicit immutable/status publication modes. | Transported bundles, path/symlink mutation, missing/extra/member hash drift, quota exhaustion, interrupted status publication, shared-object mutation, all tampered manifest/capture/trace fixtures. Add bounded-byte-read and peak-live-buffer assertions. | Whole-bundle byte cloning, temporary scenario copies, path-based rereads after snapshot admission, duplicated publisher mechanics with weaker guarantees. | Shared sealed inputs, independent manifest validation, original OS PNGs, failed/not-run evidence and exact selection journal. |
| V8. Admitted scenarios and trace quotas | Validated inventory views, current-tree path mappings, explicit native capabilities and derived/enforced record quotas; policy views consumed by xtask/CI/smoke. | Every current scenario classified; missing/duplicate/stale mapping; conservative unknown path; full-combat and dense-graphics workloads; independent presentation budget; preflight before build/spawn. | Stale source-prefix tables, name-triggered floors, unsupported hard-bound claims from sample averages, duplicate identity discovery. | Distinct gameplay/parity/native obligations, content pins, explicit fixtures, startup/idle/wall/input budgets and complete failure receipts. |
| V9. Behavior-oriented parity observations | Map owned domain observations into stable exact I/S/P/fields; replace implementation-dependent fields only with tested equivalent behavior/generation facts; version intentional format changes. | Full exact corpus, counts, save pairs and species coverage; repeated recordings; cross-platform/jitter as selected; owner/generation/stale-ID negative cases; old/new projection correspondence; precise pixel deltas for intended fixes. | C-shaped observation shims and aliases whose callers all migrated; old runtime serializer after a complete version transition. | Frozen old decoder fixtures, unchanged-domain exactness, no approved species absences, failure classifications and owner-authorized golden protection. |
| V10. Enforcement and fixtures follow the new owners | Preserve quality/coverage policy while measuring new identities; retire only unused migration entries; explicit fixture handles and bounded reexecution; current CI path policy; focused package failure tests. | Existing move/conservation/macro/cfg/dependency/coverage tests; negative mutation tests; fixture unwind/environment isolation; exact child test selection/deadline; unpacked asset identity/content smoke and release matrix tests. | Completed zero-use debt scaffolding only with proof of no live entries; redundant fixture APIs and obsolete layout assertions replaced in the same unit. | Thresholds, required checks, unsafe confinement, zero-native rule, Git-backed uncovered debt, Python/shell release route and target-specific external representations. |

Suggested ordering: V1 and V2 establish persistence responsibilities; V3 completes their menu/storage callers. V4 and V5 establish execution/evidence vocabulary. V6 and V7 can then remove duplicated verification mechanisms. V8 updates admission and policy with the shared types. V9 removes implementation-shaped observation debt under unchanged gates. V10 accompanies the relevant units rather than becoming a backlog of deferred caller migrations. Independent storage or supervisor safety work need not wait for cosmetic renaming.

## 7. Explicit retain list

Retain safe codec separation and explicit owner borrowing. Retain fixed-width wire structures, story bit positions, externally meaningful numeric IDs, raw save names, legacy import and exact compression fixtures. Retain RNG/order-sensitive queue reconstruction, saved/live location distinctions, callback admission timing and the live writer's isolated compatibility behavior until intentionally changed.

Retain the pure scheduler, pure watchdog ordering, domain-specific controllers, unavailable-versus-zero predicates, supplied observer ownership and deferred save capture. Retain first-terminal semantics, actual production input evidence, RAII publication cancellation and durable finalization.

Retain independent native/headless proof claims, targeted process identity, sandboxed candidate authority, original OS screenshots, checkpoint correlations, streaming/native inventory bounds, sealed shared inputs, complete failure/not-run rows and the test-only trace oracle.

Retain exact parity, counts, all save capture pairs, species engagement/callback checks, golden protection, quality/layer/dependency/unsafe ratchets, nonfabricated coverage attribution and package smoke outside the checkout. Retain SDL/platform/external file representations where they have a real boundary. None of these requires retaining C-shaped application callbacks, duplicate runtime records or unused model APIs.

## 8. Verification limits and next evidence

No behavior was reproduced, no tests were executed, no CI state was queried and no screenshots were viewed. The report establishes source-level control flow, duplicate policy and architectural scope. It does not establish observed data loss, a concurrency race, a security exploit, a leaked child, measured memory usage or an actual trace-budget exhaustion.

Before implementation, capture the existing focused regression results on the pinned source, then follow each unit's acceptance tests. Preserve the current recording set and failure fixtures while migrating. The first proposed failing regression should target late load validation with already changed story/clock state; the second should target outer-supervisor receipt claims under injected cleanup/read failure. Those exercise the strongest source-grounded correctness gaps without starting with broad gameplay reruns.

The file-family appendix below is generated from the pinned Git tree. It is a coverage inventory, not an additional generated source or enforcement change.
