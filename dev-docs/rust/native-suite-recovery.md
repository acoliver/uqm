# Native suite teardown and interrupted publication

A launched native acceptance child establishes clean process state only through `verified-empty` process-group cleanup, complete pipe drain and no observed surviving descendant. `not-supported` cannot attest to macOS native teardown. This requirement belongs to native acceptance accounting; generic platform supervision applicability is unchanged. Completed supervision also requires exit0, no signal, no timeout and no termination signal. Failed supervision with unknown cleanup remains diagnostic evidence with a false process-clear claim.

Finalization retains the prior unfinalized status in `suite-finalization-intent.json`, bound byte-for-byte to its immutable journal event. Recovery validates that binding even after the prior status has been restored. It removes incomplete final documents and the provisional final event, durably replaces `suite-status.json` with the prior status, then removes the intent. An interruption after any cleanup operation can resume from either the retained intent or the restored journal. Missing members during repeated removal are expected; other I/O errors propagate.

Recovery never runs gameplay. It publishes failure even when all scenario proofs had completed before final publication was interrupted, preserves those proofs and their journal history, and does not attest to teardown it did not observe. A fully finalized valid bundle is not recovered again.

## Tests

`cargo +1.97.1 test --locked --manifest-path rust/xtask/Cargo.toml native_suite` exercises synthetic fault accounting, eight torn-finalization boundaries and seventeen interruption points during recovery cleanup and publication. The unsupported-cleanup regression and intent-removal regression must fail without their respective fixes.

Two additional tests require authentic retained native evidence and are explicitly ignored in the portable default test run. Execute them separately:

```sh
UQM_COMPLETED_NATIVE_FIXTURE=/absolute/path/to/retained-suite \
UQM_COMPLETED_NATIVE_SOURCE_SHA=source-sha-from-linked-receipt \
cargo +1.97.1 test --locked --manifest-path rust/xtask/Cargo.toml \
  retained_completed_native -- --ignored
```

The fixture must contain a completed first authority-pinned battle scenario and its shared inputs. The accepted control passes both public accounting and S4 success validation. A reindexed mutation retains completed exit0/no signals/complete pipes but changes process cleanup to `not-supported`; both validators must reject it at teardown. Completed-suite recovery tests cover torn would-be-success finalization, every remove/replace cleanup boundary and an index-publication budget failure. Every recovered bundle must validate as failed, retain the completed scenario unchanged and preserve the immutable pre-final event. These tests reuse retained proof; they do not claim fresh gameplay execution.
