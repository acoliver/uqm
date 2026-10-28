# Scoped transition budgets

The `stage_b_raw` budget expires at phase-2 and covers only exact finding identities
for `ffi|raw_records` and `ffi|raw_pointer_parameters`. Record it with:

```sh
cargo xtask quality budget stage_b_raw 3 \
  --scope 'ffi|raw_records|uqm::engine::battle::table::ElementTable=2' \
  --scope 'ffi|raw_pointer_parameters|uqm::app::bridge::dispatch=1'
```

The example identities are illustrative. Use measured finding identities, not
subsystem totals or wildcard prefixes. Each scope has an absolute measured limit.
The overall finite limit must equal their sum. Each identity is checked separately,
so spare capacity cannot move between identities. Existing transition budgets keep
their workspace-wide behavior.

Creating a positive budget or adding or raising a scope against the base requires
`quality-growth-approved`. Replacing a scope at the same total also requires it.
A local scope expansion requires `--allow-growth`. Lowering or removing scopes does
not. Baseline serialization and tightening preserve the scopes and their expiry.

Scoped findings are removed from both sides of the base comparison before the
ordinary subsystem ratchet runs. All other findings retain their existing ratchet.
The finite limits apply even with `--allow-growth`. This records the temporary
adapter allowance approved in #745 without increasing permanent raw-debt entries.
