# PR250 hosted failure diagnosis

## First failures on 080f19d01

[Push run 39198432057](https://github.com/acoliver/uqm/actions/runs/39198432057)
failed `ownership.zero_native_delta` on all four tuples. Its controller was the
source commit, `080f19d011e77a4f7205a85583ff9a8eb3ec19ae`. The comparison base was
`08242c489b7df76139cd7d5b4221b95f811c8200`. That controller counted changed or
removed native paths as failures. The input count check itself passed: 321 base
inputs, 288 head inputs and a 288-input maximum. The aggregate non-merge job
failed because its matrix jobs failed; it was not an independent first failure.

[PR run 39198476916](https://github.com/acoliver/uqm/actions/runs/39198476916)
used installed main controller `9dff9edb78aaac25138d61ab99a109c28e96d4d2`
and comparison base `83ae74da6003cae35217169eac215266c5016b1d`. Its native
ownership contract passed without waivers on Linux arm64, Linux x86_64 and macOS
arm64: 293 base inputs, 288 head inputs and a 288-input maximum. The installed
controller includes the shrink-only implementation from #240. Merging that
installed controller into the source branch removes the push/PR controller
mismatch without changing the branch's admitted ceiling or policy limits.

Linux arm64's first failed PR contract was `coverage.exec`, not ownership or
policy admission. Cargo llvm-cov exited 101 because
`hung_screencapture_without_barrier_budget_fails_and_leaves_nothing_behind`
received an error other than `CaptureTimedOut`. The original assertion did not
print that error. The same test passed in the retained Linux x86_64 and macOS
arm64 coverage outputs. These jobs used Rust 1.97.1, cargo-llvm-cov 0.9.0, the
same source, admitted authority hash and controller, and the same workspace,
all-targets, pure-feature coverage command. The retained Intel macOS job was
still executing its authoritative gates when the initial evidence was captured;
it was not counted as passing.

## Capture fixture defect and repair

The capture test fixture wrote an executable shell script with `fs::write` and
then executed that inode directly. A concurrent fork can inherit the writable
file descriptor before the write closes it. Close-on-exec does not close that
descriptor until the child reaches exec. While any process retains it, Linux
rejects execution of the script with `ETXTBSY`.

A deterministic unprivileged Linux arm64 regression retains a writable script
handle across helper launch. Before the repair, its unchanged required error
type assertion failed with:

```text
expected the helper's capture timeout, got Os("spawn native capture helper: spawn failed: Text file busy (os error 26)")
```

The test helper now runs `/bin/sh` with the fixture script as its first argument.
The executable digest describes the actual interpreter. The script remains an
input file, so a writable descriptor cannot prevent its interpreter from
executing. This configuration exists only under `cfg(test)`. Production helper
launching, deadlines, retry decisions and failure classifications are unchanged.
Both original capture tests retain their assertions. The new regression also
requires the timeout type and complete scratch cleanup while its writer stays
open.

The deterministic failure establishes this fixture defect. The original hosted
artifact cannot establish its hidden inner error retrospectively; `ETXTBSY` is
consistent with the Linux-only intermittent failure, rather than a recovered
historical error message. The timeout assertion now prints unexpected errors so
future failures preserve that distinction.

## Evidence boundaries

Raw hosted artifacts, original logs, deterministic failing and passing regression
logs, and new-head verification receipts are retained separately under
`tmp/pr250-hosted-fix/`. Earlier proof bundles and fixtures are not rewritten.
The interim `head-9b5e9b329` local full-route directory is diagnostic only: source
was edited while it ran, and its `SOURCE-SCOPE.txt` explicitly excludes it from
clean committed-head verification. Final verification uses a new directory and
compares complete tracked-source inventories before and after execution.

Local Linux checks use the existing Podman arm64 VM and Rust 1.97.1, four build
jobs and an isolated target. A first root-run diagnostic hit the expected
permission-denial save-slot test because root could write the protected file.
An initial unprivileged coverage setup encountered a root-owned unreadable
Cargo registry member. Both setup failures are retained. Subsequent execution
uses UID 65534 and a private readable Cargo home, rather than weakening either
test or changing the shared registry.

No GUI, native-window acceptance or screenshot result is claimed. No unchanged
hosted rerun, workflow cancellation, policy waiver or enforcement relaxation is
part of this repair. Review skipped per Andrew 2026-10-09.

## Changed-source validation and remaining controller defect

On `979570fa8`, both clean local full routes passed: 6,183 tests for PR250 and
6,200 for PR251, each with zero failures and 15 existing ignored tests. Both
fresh linked parity executions, strict clippy, formatting and workflow checks
passed. The exact-source Linux arm64 workspace coverage run also passed. Its
raw-profile files are retained as diagnostic outputs, not admitted clean-source
CI evidence.

[Changed-source push run 39207702596](https://github.com/acoliver/uqm/actions/runs/39207702596)
used the merged shrink controller and exposed narrower failures. The native
object/provider reductions and 321-to-288 input ratchet passed. Its only native
source failure was the cross-file comment replacement in `hyper.h`. The existing
rule explicitly counts comment tokens. Removing that comment is a removal-only
source correction; the coordinate macros and the token rule are unchanged.

Five Rust paths were also reported under transitional flags because their uses
of the existing `linked_c_archive` feature caused whole Rust files to be judged
as native paths. [Issue #252](https://github.com/acoliver/uqm/issues/252) tracks
the controller projection defect. Its repair is isolated on a main-targeted
branch and must be installed before this stack can use it as the base-owned PR
controller. No merge is authorized. The final Linux arm64 and Linux x86_64 PR
gates in run `39207704569` passed before that controller repair; their narrower
stack-parent comparison did not expose the same false positives as the push
comparison against main.
