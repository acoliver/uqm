# Clean-checkout CI gates

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
network access. Coverage includes applicable first-party workspace targets and must reach 80 percent. The exact pure feature profile is
`audio_heart,debug-process`; the exact linked profile adds `linked_c_archive`.

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
the exact pull-request head and exact base commit and requires byte equality. It then
builds the supervisor and xtask from the exact base commit, then checks out the
pull-request head with persisted credentials disabled. The base-owned xtask evaluates
the pull-request working tree. Probe and harness shell programs that decide gate results
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
authority contract to equal the retained, base-matched authority snapshot. This
literal tuple set prevents pull-request code from selecting another `runs-on` label.
Matrix values enter Bash only through quoted step environment variables.

Gate-tool versions and installation identities come from the base-matched authority.
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
runs merge-deciding gate subprocesses. The base-owned controller starts each
subprocess under that identity and, before returning, kills and checks for every
process with its real or effective UID. This includes a process that calls `setsid`,
double-forks, and loses every observable ancestor before the controller's first
process snapshot. A base-owned containment check exercises that escape pattern before
authoritative gates run, and successful transport retains
`containment-check.result.json`. An always-running, marker-gated cleanup removes the
identity after killing and checking its remaining processes. The outer workflow
supervisor's receipt describes only its own process-tree boundary; dedicated-UID
containment is established by the separate containment-check receipt. After untrusted
gates return, a base-owned supervised check revalidates the exact source SHA, tracked
and untracked state, and authority bytes, retaining `source-revalidation.result.json`.

Normal pull requests cannot change gate authority and production code together. An
authority update first needs a separately reviewed base-policy change, after which
production changes can target those bytes. Initial deployment is a one-time bootstrap:
a maintainer must install this workflow and authority on the protected base branch
after detached review because the previous base has neither file. Branch-protection
administration must reserve the merge-required `Required S4 gates` context for
`pull_request_target`; push runs publish `Required S4 gates (non-merge)` and cannot
satisfy that context.

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

On macOS tuples, the tests gate also runs the direct native-window acceptance
runner against the linked executable. The workflow derives the content URL,
filename, version, byte length, and SHA-256 from `rust/ci/gates.json`. It downloads
`uqm-0.8.0-content.uqm`, requires exactly 11,547,353 bytes and SHA-256
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
The S4 delta is zero tracked native sources, providers, objects, internal symbols,
bridges, generated bindings, and transitional ownership flags. The transitional
native input count may not increase. S1's provider report remains the provider and
symbol authority.
