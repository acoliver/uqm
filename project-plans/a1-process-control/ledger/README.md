# Ledger amendment required to land the C removal

PR #185 deletes all nine first-party C sources under `rust/`. CI rejects it at
`ownership.zero_native_delta` because those nine paths are still tracked native sources in the
published ownership ledger. This directory holds the amended ledger and the exact follow-on edits.

## Why this cannot be done in-repo alone

`rust/xtask/src/ci/delta.rs` pins the vendored ledger to the published one:

- line 79: the sha256 of `rust/ci/native-ownership-ledger-v7.json` must equal
  `gates.json .ledger_identity.sha256`.
- lines 84-88: the inventory must be exactly 913 tracked sources, 124 FFI files, 48 flags.

`gates.json .ledger_identity` names the gist raw URL, gist revision, and content hash. Editing the
vendored copy without republishing would either break the hash check or require forging the
authority, which defeats the point of an externally published ledger.

## What CI measured

Run 33675441033, all four tuples, first failed contract `ownership.zero_native_delta`:

- `tracked_sources` measured delta 5: `menu_binding_accessor.c`, `menu_binding_accessor.h`,
  `menu_binding_probe.c`, `p00_harness.c`, `p00_harness.h`. The other four removals were measured
  against an earlier base commit in the same series.
- `transitional_flags` measured delta 4: `run_menu_binding_probe.sh`, `run_p00_harness.sh`,
  `probes/menu_binding_probe.rs`, `probes/p00_symbol_harness.rs`. These count because
  `is_flag_definition_path` treats any file under `rust/` outside `rust/ci/` and `rust/xtask/` as a
  flag definition site, and each mentions `linked_c_archive`.

The authority declares both must be 0.

## The amendment

`uqm-native-ownership-ledger-v8.json` in this directory is `v7` with:

- the nine `RUST_HELPERS` entries removed, all of which are the files this PR deletes;
- `counts.sources` 913 to 904;
- `schema` set to `uqm-native-ownership-ledger-v8`;
- `supersedes` set to the v7 identity (revision `d35f6156bff0b202306cca57d517f800234951df`,
  sha256 `d8d90624ff846bfa24fcfdfecd684649b0f81b49a447955f63bfc3d6a97a747a`).

Removed paths:

```
rust/harness/menu_binding_accessor.c
rust/harness/menu_binding_accessor.h
rust/harness/menu_binding_probe.c
rust/harness/p00_harness.c
rust/harness/p00_harness.h
rust/harness/sdl_surface_accessors.c
rust/harness/sdl_surface_accessors.h
rust/src/io/uio_vfprintf_helper.c
rust/src/mainloop/rust_test_bridge.c
```

sha256 of the file as generated here:
`49073df15a115e790d2e72c02387359bf2eef321f25f2d4f0306b361f55dc789`

That hash holds only if the published bytes are byte-identical, including the trailing newline.
Recompute it from the published raw URL rather than trusting this line.

## Steps to unblock, in order

1. Publish `uqm-native-ownership-ledger-v8.json` as a new revision of gist
   `03378acffcc0d62e7cfd094fc77c223c`.
2. Record the new gist revision and raw revision hashes, and recompute the sha256 from the raw URL.
3. Update `rust/ci/gates.json .ledger_identity`: `schema`, `assessment_commit` if it moves,
   `raw_revision`, `url`, `gist_revision`, `sha256`.
4. Replace the vendored `rust/ci/native-ownership-ledger-v7.json` with the published v8 bytes and
   update `LEDGER_PATH` in `rust/xtask/src/ci/delta.rs`.
5. Update the expected inventory in `delta.rs` lines 84-88 from 913 to 904 sources.
6. Decide the declared delta in `gates.json .zero_native_delta`. With the nine paths no longer
   tracked, `tracked_sources` returns to 0 on its own. `transitional_flags` still measures the four
   touched flag-definition files, so either that declaration carries the intended non-zero value for
   this change, or the ledger records the new probe paths as the successors of the removed ones.
   This is a policy decision, not a mechanical one, which is why it is not pre-decided here.
7. Also update the prose that still assigns these files to `RUST_HELPERS`: the S3
   `rust_helpers_boundary` text and the `rust/harness/**` control-plane responsibility, both of which
   say helper deletion remains issue 156.

## Status of the code itself

The Rust replacements are complete and verified on 1.97.1: format, strict Clippy with `-D warnings`,
the full workspace test suite, 80 evidence tests and 7 mutation tests all pass. The blocker is
ownership bookkeeping, not the port.
