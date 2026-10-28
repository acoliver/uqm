# Scenario counts goldens

`parity record/check` stores each scenario in
`rust/tests/golden/parity/counts/<scenario>.toml`. `parity counts record/check --all
--out DIR` uses the same directory layout. Each file contains one `[[scenario]]`
table, and its name must match the filename. `rust/tests/golden/manifest.json`
records the size and SHA256 of every file.

Phase rows retain the existing counter deltas and action/phase dictionaries.
A scenario whose uncompressed table exceeds 5,000,000 bytes uses
`delta-dictionary-rle-v1`: each JSON-string entry in `action_phases` is
`[[input_delta,present_delta,step_delta,action_index,phase_index],repeat]`.
Decoding applies the deltas once per repetition, preserving every sample.
Smaller scenarios keep `delta-dictionary-v1`. Checks compare decoded fields and
serialized bytes, so both behavior and deterministic storage remain enforced.

## Reproduce the monolith migration

The ignored migration test compares all 188 scenarios with an original
monolithic golden, including every phase and terminal/failure field. From the
repository root, extract the source revision that still has the monolith:

```sh
mkdir -p tmp/verify
# BASE is the source revision being converted, for example origin/main before #732.
git show "$BASE:rust/tests/golden/parity/counts.toml" > tmp/verify/legacy-counts.toml
cd rust
UQM_LEGACY_COUNTS="$PWD/../tmp/verify/legacy-counts.toml" \
  cargo test -p uqm-xtask --no-default-features --locked \
  split_legacy_counts_matches_every_decoded_value -- --ignored --nocapture
```

To regenerate the split after a replay repair lands, additionally set
`UQM_SPLIT_COUNTS="$PWD/tests/golden/parity/counts"`. The test writes the files
with the production encoder before checking decoded equality. Regenerate the
manifest hashes afterward; no replay values should be edited by hand.
