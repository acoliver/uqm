# Clean-checkout CI gates

## Native UI suite is a local gate

Hosted GitHub runners do not run the native-window UI suite. The 34-scenario suite takes
about 4500 s on arm64, the Intel runner renders at about 14 fps, and the evidence exceeds
upload limits. Hosted CI keeps the code-quality gates (format, check, clippy, tests,
ownership-link, harness, complexity, security, coverage, cache, workflow, mutations).
The native suite is a local gate run on a real Mac window with
`cargo run --locked --manifest-path rust/xtask/Cargo.toml -- native run|validate|report|gallery|replay`
(see `dev-docs/rust/autoplay-runner.md`). Its authority data stays in `rust/ci/gates.json`
under `native_acceptance` because the local CLI reads the scenario list and deadlines from it.

Issue #27/S4 owns CI composition. It consumes S1 provider and strict-link checks,
S2 matrix, build, determinism, verification, and package commands, and S3 fixtures,
probes, harnesses, gameplay proof, and teardown commands. Those domains retain the
semantics of their commands.

## Command authority

`rust/ci/gates.json` is the checked-in command authority. It declares gate order,
exact process command vectors, builtin gates, feature profiles, thresholds, cache
requirements, mutation targets, bootstrap profile, and the exact four supported
S2 tuple and runner mappings. `rust/build/supported-matrix.json` is a compatibility
input. `ci plan` rejects it unless its rows derive exactly the authority tuple set.

The required gate order is format, check, Clippy, tests, ownership/link,
probes/harnesses, complexity, security, coverage, package, bootstrap proof,
workflow, and mutations. Complexity is limited to 40. Security fetches the exact
RustSec advisory database revision declared by the authority, verifies that checkout,
and runs `cargo audit --db target/ci-advisory-db --no-fetch --deny warnings`.
Successful security evidence retains a deterministic pack of the verified database.
Offline replay checks the authority-pinned file count and SHA-256 without Git or
network access. Because the database revision is pinned, the pinned `cargo-audit`
must be able to parse it: that revision carries advisories scored with CVSS 4.0,
which releases before 0.22 reject outright while loading, failing the gate before
it examines this tree at all. Moving either pin means checking it against the
other.

Fresh native runs require clean tracked and untracked source admitted by S2.
Workflow controllers bind the candidate checkout through `UQM_CI_SOURCE_ROOT`,
the separately admitted exact policy bytes through `UQM_CI_AUTHORITY_PATH`, and
the requested suite through the trusted event selection. Native compilation uses
S4's distinct build UID; gameplay and observation retain the controller's Aqua
session. The containment probe requires protected-file writes and controller
signaling to be denied before candidate Cargo executes.

`native-suite-validate` and workflow `native validate` apply trusted source,
policy and shared-build correlation to successful evidence. Local `native
validate`, report, gallery and retained replay inspect historical inputs without
requiring the current checkout to be clean. Historical consistency does not
establish acceptance for the current workflow event.

Suite finalization retains a bounded intent tied to the preceding immutable
journal event. An interrupted publication recovers into a failed diagnostic
bundle without starting another child. Success also requires completed, drained,
verified-empty controller supervision. Plan, gates and the required
aggregate consume the base-owned `workflow_transport.py` implementation for descriptor traversal,
content-addressed indexes and upload receipts. Bootstrap failure indexes remain
available before that helper is installed.

Coverage measures the pure profile, which is the configuration the unit suite is
written for, and must reach 70 percent. The linked profile drives the C game
through FFI and is covered by the linked provider fixture and native acceptance
instead; running the whole unit suite against it exercises real C statics with no
game state. The exact pure feature profile is `audio_heart,debug-process`; the
exact linked profile adds `linked_c_archive`.

Workflow job limits, authority-fetch transport limits, and native-content attempt,
read, and backoff limits also come from `rust/ci/gates.json`. Jobs that depend on a
successful plan consume those values through plan outputs. The plan job must fetch
authority before checkout, so GitHub cannot evaluate that authority for its own
timeout or the fetch that obtains it. The `required-gates` job must also start when
plan outputs are absent or malformed. Its five-minute timeout and the plan bootstrap
transport bounds therefore remain trusted workflow literals. `ci workflow-check`
compares each literal with authority and rejects any drift.

Pull requests use `pull_request_target`, so GitHub executes the workflow body from
the base revision. The pre-checkout plan step fetches `rust/ci/gates.json` from both
the exact pull-request head and workflow revision. Bootstrap uses base policy only.
The workflow builds the supervisor and xtask from that workflow revision, then runs
`ci admit-policy BASE CANDIDATE OUTPUT` with the base-owned executable before
candidate policy can become the execution authority. It checks out the pull-request
head with persisted credentials disabled. The base-owned xtask evaluates the
pull-request working tree. Probe and harness shell programs that decide gate results
are compiled into that base-owned xtask. The controller stages those retained bytes in a runner-owned temporary root outside
the dedicated identity's writable evidence tree. It revalidates each selected
controller and script by no-follow type, owner, exact mode, name, and digest
immediately before launch, and supplies the exact-head checkout only as the scripts'
source root. A pull request can therefore change the source under test without
replacing the scripts that judge it. Its plan is still validated as
untrusted structured input by a separate shell step in the base-owned workflow,
which requires the complete tuple array to equal these four objects before it writes
the matrix output:
`macos/aarch64/macos-aarch64/macos-15/arm64`,
`macos/x86_64/macos-x86_64/macos-15-intel/x86_64`,
`linux/aarch64/linux-aarch64/ubuntu-24.04-arm/aarch64`, and
`linux/x86_64/linux-x86_64/ubuntu-24.04/x86_64`. It also requires the emitted
authority contract to equal the retained, base-admitted authority snapshot. This
literal tuple set prevents pull-request code from selecting another `runs-on` label.
Matrix values enter Bash only through quoted step environment variables.

Gate-tool versions and installation identities come from the base-admitted authority.
The workflow checks the installed Rust compiler's full release commit. It downloads the
`cargo-audit` and `cargo-llvm-cov` crate archives under authority-owned bounds and verifies
each authority SHA-256 before extraction. Extraction rejects links, special files, path
traversal, and resource-budget violations. Each verified crate must contain its published
`Cargo.lock`; Cargo fetches that locked dependency set without running build scripts, then
installs from the verified source with networking disabled. Actionlint installation uses
an isolated Go module cache, the public Go checksum
database, and an exact authority-pinned module sum. Lizard installation uses an
authority-generated requirements file with exact versions and SHA-256 hashes for the
Lizard wheel and its complete dependency set; pip runs with `--require-hashes`.
These base-authorized tool installations precede dedicated-identity provisioning. The
base controller validates the complete tool-install shell body against its trusted
SHA-256, so required checks cannot be preserved only as unreachable text while a weaker
command executes. The installations consume the base-owned authority and verified
distribution metadata, not pull-request source; pull-request-controlled commands run
only after containment is active.

On Linux and macOS, the gates job creates an otherwise-unused local identity before it
runs merge-deciding gate subprocesses. The base-owned controller starts ordinary and
non-GUI subprocesses under that identity and, before returning, kills and checks for
every process with its real or effective UID. This includes a process that calls
`setsid`, double-forks, and loses every observable ancestor before the controller's
first process snapshot. A base-owned containment check exercises that escape pattern
before authoritative gates run, and successful transport retains
`containment-check.result.json`. An always-running, marker-gated cleanup removes the
identity after killing and checking its remaining processes. The outer workflow
supervisor's receipt describes only its own process-tree boundary; dedicated-UID
containment is established by the separate containment-check receipt. After untrusted gates return, a base-owned supervised check revalidates the exact
source SHA, tracked and untracked state, and authority bytes, retaining
`source-revalidation.result.json`.

Candidate-policy admission preserves every base field except two bounded changes.
A candidate may lower any numeric field whose name starts with `maximum_` (for
example `zero_native_delta.maximum_transitional_native_inputs`) and never raise
one, so a porting PR can tighten the ceiling it improved. A candidate may also
append scenario entries to `native_acceptance.scenario_scripts`; existing entries
cannot be removed, changed (including their deadlines) or reordered. The
authority carries no content fingerprints for scenario scripts, start saves or
the Cargo manifest and lockfile; `--locked` builds and `cargo audit` cover
dependency drift, and start-save provenance JSON is documentation only. Commands,
tools, tuples, content, acceptance predicates, runtime limits, permissions and
transport limits cannot change through this rule. Unknown fields are rejected.
On push events other than to `main`, the workflow fetches `origin/main` and measures
the delta against `git merge-base origin/main HEAD`, so main's own changes are not
blamed on a branch that merged main. Pushes to `main` keep the previous-push base
with a `HEAD^` fallback.
The plan and each gate job independently admit policy; rejection does not replace
the installed policy. Supervised admission logs retain the decision and both hashes.
Source revalidation compares the checkout with the admitted candidate bytes.

Issue #210's controller-policy migration and Issue #34's runner are delivered
together in PR #209, as authorized. This implementation does not make a candidate
controller trusted in its own `pull_request_target` run. The currently installed
base still performs its old byte comparison and lacks the suite consumer and
admission command. Candidate-controller tests against the retained base policy
are migration evidence, not evidence that the new controller is installed on main.
Hosted validation must identify the actual workflow/controller revision and must
not relabel a push result as merge-context acceptance. No branch-protection,
permission, secret or checkout-reference bypass is part of this migration.
`Required S4 gates` remains the merge-context name; push runs retain the
`Required S4 gates (non-merge)` name.

### Outer child deadline for long scenarios

The native runtime contract carries one outer child deadline,
`native_acceptance.runtime_contract.outer_child_timeout_ms` (300000 ms). A scenario
entry may carry its own `outer_child_timeout_ms` in `scenario_scripts`, and the
only value accepted is 900000 ms (15 minutes). Approval, verbatim from the
repository owner: "yeah you can switch them to 15m", given for the proposal that
the longer scenarios get a longer outer timeout while the short ones remain at
300 s.

Scope of the rule:

- A script whose declared `max_wallclock_seconds` is above 300 must have a
  scenario entry with the 900000 ms deadline. A script at or below 300 must not.
  The wall budget is read from the script during admission, so an entry that
  disagrees with the script's own budget is refused before any build or launch.
  Scripts are listed in `gates.json` with their deadlines and are not
  fingerprinted. All 14 scripts above 300 s carry the deadline and the other 22
  keep the default.
- A wall budget is a failure ceiling, not a promise that every run takes that
  long. `real-sol-luna-moonbase-departure`
  and `real-sol-starbase-alliance` declare 900 s, carry the same 900000 ms deadline, and
  are ended by the outer deadline if a run does not finish first. The approval
  does not cover raising the value further.
- Every other runtime value is shared and unchanged: capture, observer and
  acknowledgement timeouts, kill grace, stream and capture budgets, the 1 MiB
  manifest limit and the inventory limits. The acknowledgement window stays at
  95000 ms, below both outer deadlines.
- The primary `native_acceptance.script` cannot carry the override because the
  single-script gate uses the shared contract.
- The controller hands each scenario child the contract of its own entry, the child
  supervisor uses that value as the `ChildSession` timeout, and the proof,
  failure-manifest, suite-success and replay consumers compare the recorded
  contract with the one the authority declares for that script.
- A retained bundle recorded before this change holds the 300000 ms contract for
  its long scripts. The current authority does not declare that contract for
  them, so such a bundle is checked by the source revision that produced it. It
  is not reinterpreted under the new deadlines. This includes the earlier revision
  a35d65d64, which covered 11 of the 14 scripts.

The sections below on autoplay selection and the `native-acceptance` step describe the local
native suite (`native run`, `native-test`); hosted CI no longer runs them (see the
local gate section). Autoplay selection is bound to a typed `uqm-s4-selection-v1` document. The base-owned
plan controller records event, exact source/controller/base revisions, merge base,
raw NUL-delimited changed paths, policy digest and the derived ordered scenario set.
Only `pull_request_target` may use changed-path selection. Empty, unmapped or
non-UTF-8 paths select the original complete 34-scenario suite. Push, schedule and
manual runs always select that complete suite. A Git inspection failure is an error,
not permission to execute a smaller suite. The existing workflow now declares a
daily schedule and manual trigger without adding another gate or tuple matrix.

Gate entry independently recomputes selection from the workflow event and exact
Git revisions, then requires equality with the plan output. It retains the binding
inside `source-preflight.json` before untrusted execution and supplies the resulting
script names explicitly to the native child. Ambient requested names cannot narrow
that set. Successful S4 validation requires the exact ordered suite, all completed
native proofs, shared descriptor/build identity, linked source and policy equality,
and unchanged script/content/runtime checks. Flat successful native bundles are no
longer accepted; failure transport retains its existing diagnostic layouts.
Offline replay checks the retained event-derived selection without repository state.
To bind downloaded evidence to an independently known workflow, provide
`UQM_CI_EVENT_NAME`, `UQM_CI_EXPECTED_SHA`, `UQM_CI_CONTROLLER_SHA` and, for PRs,
`UQM_CI_EVENT_BASE_SHA` to `ci validate-evidence`. Without these external inputs,
offline validation establishes retained consistency, not GitHub event authenticity.

Run focused authority checks from the repository root:

```sh
cargo fmt --manifest-path rust/xtask/Cargo.toml --all
cargo check --locked --manifest-path rust/xtask/Cargo.toml --all-targets
cargo test --locked --manifest-path rust/xtask/Cargo.toml
cargo clippy --locked --manifest-path rust/xtask/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- ci plan
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- ci workflow-check
```

Required execution builds xtask outside every path inspected by the empty-cache
preflight, then invokes that binary directly:

```sh
bootstrap_root="${RUNNER_TEMP:?}/uqm-s4-bootstrap"
trusted_xtask="${RUNNER_TEMP}/s4-gates-controller"
chmod 0750 "${RUNNER_TEMP}"
mkdir -p "$bootstrap_root"
chmod 0750 "$bootstrap_root"
install -m 0440 rust/ci/gates.json "$bootstrap_root/gates.json"
CARGO_HOME="$bootstrap_root/cargo-home" \
CARGO_TARGET_DIR="$bootstrap_root/target" \
cargo build --locked --manifest-path rust/xtask/Cargo.toml
install -m 0550 "$bootstrap_root/target/debug/uqm-xtask" "$trusted_xtask"

UQM_CI_SOURCE_ROOT="$PWD" \
UQM_CI_AUTHORITY_PATH="$bootstrap_root/gates.json" \
UQM_CI_TRUSTED_STAGING_ROOT="${RUNNER_TEMP}" \
UQM_CI_EXPECTED_SHA=<full-lowercase-40-hex-head> \
UQM_CI_BASE_SHA=<full-lowercase-40-hex-comparison-base> \
UQM_CI_EXPECTED_TUPLE=<macos-aarch64|macos-x86_64|linux-aarch64|linux-x86_64> \
UQM_CI_CACHE_MODE=isolated-empty \
UQM_CI_EVIDENCE_ROOT="${RUNNER_TEMP}/s4-command-evidence/bundle" \
"$trusted_xtask" ci run all
```

The runner requires a clean worktree at `UQM_CI_EXPECTED_SHA`. Local acceptance
must use a detached temporary worktree. Do not remove or hide user files to make a
development checkout appear clean.

## Required cache state

Required mode sets `CARGO_HOME` to `rust/target/ci-cargo-home` and
`CARGO_TARGET_DIR` to `rust/target`. Before either path is created, `ci doctor`
inspects Cargo registry and git caches, `rust/target`, and `sc2/obj`. The receipt
must show all required paths absent. The workflow therefore builds its xtask copy
under the hosted runner's temporary directory, uses no cache action, and then
starts required execution.

## Evidence and failures

Each gate captures authority-bounded stdout and stderr prefixes. Its supervision
receipt records whether either stream exceeded its limit; overflow fails the gate.
The evidence index records source SHA, tuple, cache mode, gate and step identity,
exact producing command vector, role, media type, relative path, byte length, and
SHA-256. Offline validation
re-reads every payload and rejects malformed metadata, missing files, path
traversal, duplicate paths, hash or size drift, and an invalid first-failure
contract. A failure records the exact gate or a typed gate subcontract. Each
workflow job seeds a `uqm-s4-transport-finalizer-fallback-v1` index before checkout
or execution. The normal `always()` finalizer atomically replaces that index only
after it has built the complete transport manifest, so a finalizer failure retains
a detached-replayable `transport.finalize` receipt instead of stale evidence. Files
left beside a fallback are diagnostic rather than members of a normal evidence
bundle. Replay does not follow or trust them; the separate upload receipt binds the
complete uploaded artifact digest and size.

`required-gates` has no source checkout and no job-level expression that parses plan
JSON. It consumes the plan job's validated base-authority projection, then validates
a present retention output against that projection before exposing retention and API
timeout values to later steps. Missing retention selects complementary artifact
uploads that omit the `retention-days` input. Malformed retention fails validation
and selects the same fallback uploads. Both the aggregate evidence and its upload
receipt therefore retain an `always()` upload path when authority fetch, controller
build, source checkout, plan derivation, or plan validation fails.

The package gate retains the S2 artifact identities and determinism proof, rewrites the
manifest's executable path to the packaged executable, publishes that executable with
exact mode `0500`, and captures the production ownership report, package files, and
target dependencies. The bootstrap gate launches that package path with the fixed S3
`rust/scripts/main-menu-v1.json` profile, validates its LCAR offline, and captures
teardown.

The hosted tests gate no longer contains the `native-acceptance` step. The text that follows
describes how the local native suite executes on a macOS Aqua session.
Linux executes the step under the dedicated identity and records that native-window
acceptance is not required there. On macOS, the base-owned controller permits the
current Aqua session only for the exact hidden command derived from this step. Cocoa
window creation requires matching Unix credentials and launchd session ownership, so
running a dedicated UID inside another user's Aqua session is not accepted. The Aqua
coordinator still uses bounded process-group supervision. It starts from a cleared
environment and restores only the host toolchain/runtime allowlist, explicit
authority-derived values, and the three containment bindings needed by the linked
build. Runner command files and credential channels are excluded. The coordinator
runs the linked Cargo build under the dedicated identity, stages read-only proof
inputs, and then launches the retained executable directly in its credential-matched
Aqua session. The dedicated wrapper sets `umask 0027`, which gives the runner's shared
primary group read and traversal access to ordinary generated outputs while excluding
other users. All other gate commands remain under dedicated-UID containment.

The native manifest records the real UID, effective UID, launchd manager UID, and
launchd manager name. Validation requires a non-root process, equal real and effective
UIDs, a matching manager UID, manager name `Aqua`, and runtime-contract bytes equal to
the authority. The workflow derives the content URL, filename, version, byte length,
and SHA-256 from `rust/ci/gates.json`. It downloads `uqm-0.8.0-content.uqm`, requires
exactly 11,547,353 bytes and SHA-256
`77d75ac25e6fb755a33c4ba3b38a7b7bc41fcbc02896891b0cc9ac9214b72eef`, and
provides the verified directory through `UQM_CI_NATIVE_CONTENT_ROOT`. The runner
retains that exact package in its evidence bundle. Detached replay checks the
retained bytes against both the native manifest and embedded authority.

Native acceptance launches the retained linked executable directly. It binds OS
window observations and native screenshots to the child PID, start time,
executable digest, unique layer-zero OS window identity, and a fresh nonce. The
stable screenshot is taken at 120 post-visibility committed presentations. The
playable screenshot is taken no earlier than 300 presentations and only after the
trace reports accepted player input and at least 300 battle frames. The gameplay
trace is produced by the exact-head executable, so it is evidence about that subject,
not an independent observation by the base-owned controller. The controller checks
that at least one percent of screenshot pixels change by 24 or more in an RGB
channel between the stable and playable captures. This rejects identical captures
and isolated compositor noise without assigning semantic meaning to pixels. The
script requests another presentation after its battle-frame assertion so the parent
can capture a compositor-visible frame with that evidence already committed.
Renderer readbacks remain supplementary because they do not prove compositor
visibility.

## Ownership boundary

Immutable ownership ledger v7 assigns S4 control-plane authority and no native
implementation authority. Its SHA-256 is
`d8d90624ff846bfa24fcfdfecd684649b0f81b49a447955f63bfc3d6a97a747a`.
The S4 delta is zero native additions: native code may only shrink. The gate
(`ownership.zero_native_delta`) reports a verdict for every path or manifest
entry that differs between the merge base and the head, across tracked native
sources, bridges, generated bindings, transitional flag definitions, objects,
providers, and internal symbols.

Path-based categories (tracked sources, bridges, generated bindings, transitional
flags):

- A path deleted at head passes.
- A path added at head fails, whatever it contains.
- A path edited in place passes only if every hunk of its `git diff -U0` is
  removal-only: the multiset of word and punctuation tokens on the hunk's added
  lines must be a sub-multiset of the tokens on the same hunk's removed lines.
  Whitespace is ignored, so `#<tab>include "x.h"` becoming `#include "x.h"`, a
  shortened comment, a shorter file list, and a reordered block all pass. Any
  token the hunk did not remove fails, so a new `extern` declaration or a new
  call fails even though its surrounding lines look familiar. A change with no
  textual hunk (binary or mode-only) fails.

Manifest categories (objects, providers, and internal symbols in
`rust/ownership/native-provider-manifest.json`): the head's set of entries,
identified by object path or recompiled canonical source (objects), those plus
provider and archive decision (providers), and symbol, owner, and active
provider (internal symbols), must be a subset of the base's set. Changed hashes
and other fields of kept entries pass; a new entry fails.

`measured_delta` counts failing verdicts and must be zero. The report keeps
`changed_paths` (every path with a verdict) and `verdicts` (`path`, `verdict` of
`pass`, `fail`, or `waived`, and `reason`). The transitional native input count
may not increase and may not exceed its ceiling; that ratchet is unchanged.

A commit between the base and the head whose message has a line starting exactly
`Adds-native: <reason>` with a non-empty reason waives the failures of that run.
Waived entries stay in the report as `waived` verdicts and the commit and reason
are listed under `waivers`. The evidence validator rejects a `waived` verdict
without a recorded waiver and a `fail` verdict alongside one. Agents may add an
`Adds-native:` line only with Andrew's explicit approval for that change. S1's
provider report remains the provider and symbol authority.
