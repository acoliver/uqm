# Native suite teardown and interrupted publication

The native runner records process-group cleanup, pipe drain and any surviving
descendants. A clean native acceptance result requires `verified-empty` cleanup,
exit 0, no signal, no timeout and no termination signal. Unsupported or unknown
cleanup cannot establish successful teardown.

Finalization retains the prior unfinalized status in `suite-finalization-intent.json`,
bound to its immutable journal event. Recovery validates that binding, removes
incomplete final documents and the provisional final event, durably replaces
`suite-status.json` with the prior status, then removes the intent. An interruption
after cleanup can resume from the retained intent or restored journal. Missing
members during repeated removal are expected; other I/O errors propagate.

Recovery never runs gameplay. It publishes failure even if scenario proofs had
completed before final publication was interrupted. It preserves those proofs
and their journal history, and does not claim teardown it did not observe.
A fully finalized bundle is not recovered again.

The implementation and regression tests are in `rust/xtask/src/native_suite.rs`;
process supervision is in `rust/xtask/src/process.rs`. From the repository root:

```sh
cargo test --locked --manifest-path rust/xtask/Cargo.toml native_suite
```

Tests exercise torn finalization, restartable cleanup, journal/intent mismatch
and recovery without execution or a success claim. See
[scenario tooling](scenarios.md) for normal run and validation commands.
