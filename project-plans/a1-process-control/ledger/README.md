# Ledger amendment for the C removal

PR #185 deletes all nine first-party C sources under `rust/`. Those nine paths were tracked native
sources in ownership ledger v7, so the removal required an amended ledger. This directory holds the
amendment that was published and adopted.

## What was published

`uqm-native-ownership-ledger-v8.json` is v7 with:

- the nine `RUST_HELPERS` entries removed, which are exactly the files this PR deletes;
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

Published as gist revision `5aece912bec7e8a2a646bd1bfc95d18289f55020`. The bytes served by the raw
URL hash to `49073df15a115e790d2e72c02387359bf2eef321f25f2d4f0306b361f55dc789`, which matches the
vendored copy at `rust/ci/native-ownership-ledger-v8.json` and the value recorded in
`rust/ci/gates.json .ledger_identity.sha256`.

## What was updated to adopt it

- `rust/ci/gates.json .ledger_identity`: schema, history revision, raw revision, url, sha256.
- `rust/ci/native-ownership-ledger-v7.json` renamed to `-v8.json` and replaced with the published
  bytes; `LEDGER_PATH` in `rust/xtask/src/ci/delta.rs` follows the rename.
- `delta.rs` expected inventory: 913 tracked sources to 904.
- `rust/build/native-input-trend.json .ownership_ledger` and
  `rust/ownership/native-provider-manifest.json .generated_from_ledger`: same identity fields. The
  manifest's `projection_sha256` is unchanged because it digests the 338 sc2 objects, not the
  tracked-source list.
- `rust/ownership/src/lib.rs` expected ledger constants.
- `dev-docs/rust/ownership-and-strict-linking.md` and `dev-docs/rust/reproducible-build.md`.

CI run 33695338836 confirmed the adoption: `tracked_sources` measured delta returned to zero.

## Note on the transitional-flags category

`delta.rs is_flag_definition_path` treats any file under `rust/` outside `rust/ci/` and
`rust/xtask/` as a flag-definition site, and the delta is measured against the immediately preceding
commit. Adding, editing or deleting such a file in a commit therefore registers a non-zero
`transitional_flags` delta for that commit even when the ledger inventory itself is unchanged. The
new Rust probes and harness scripts name `linked_c_archive`, so the commits that introduced and then
removed them each registered one. This is a property of per-commit measurement on a feature branch,
not of the ledger.
