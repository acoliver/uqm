# Reproducible Transitional Build Contract

Issue #22/S2 provides one Rust-owned command surface from the repository root. It orchestrates Cargo directly; the legacy C build never invokes Cargo and no command consumes `sc2/obj`.

## Root commands

```sh
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- debug
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- release
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- test
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- probe
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- harness
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- production
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- prove
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- package
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- verify
```

The separately scoped `capture-dependencies` command performs a target-scoped bootstrap build and writes a review candidate under `rust/target`; it never edits or broadens production authority. Production and `verify` always fail closed on both missing and stale dependency declarations.

`doctor` validates the host tuple, source manifest, git-tracked status of every native input, monotonic trend, and all target prerequisites. `matrix` is an intentional pure-inspection exception that only prints the checked-in machine-readable matrix. `test` validates the matrix and tracked-input contract but intentionally skips external production-package probes before running workspace tests.

## Explicit transitional native inputs

This contract is authorized by immutable ownership ledger v7
(`uqm-native-ownership-ledger-v7`): raw revision
`d35f6156bff0b202306cca57d517f800234951df`, gist history revision
`46eb961886e2aefe8b2085a3af4b1afbc5e82a77`, and SHA-256
`d8d90624ff846bfa24fcfdfecd684649b0f81b49a447955f63bfc3d6a97a747a`.
The pinned raw URL is
<https://gist.githubusercontent.com/acoliver/03378acffcc0d62e7cfd094fc77c223c/raw/d35f6156bff0b202306cca57d517f800234951df/uqm-native-ownership-ledger.json>.
Both the provider manifest and native-input trend report encode this identity;
repository validation does not fetch mutable network state.

`rust/build/native-inputs.json` is the exact source/object/profile authority, bound field-for-field to `rust/ownership/native-provider-manifest.json`; `rust/build/native-dependencies.json` is the hashed tracked transitive header/config/include authority. Each entry declares a git-tracked canonical source, SHA-256, unique output name, producing command, canonical owner, and profile. The root preflight rejects an untracked declaration before build work, and `rust/build.rs` verifies every declaration and compiles all 321 entries into its current Cargo `OUT_DIR`; ignored object trees and stale outputs cannot authorize archive membership. Canonical domain ownership does not transfer to S2.

`rust/build/native-input-trend.json` fixes the current and maximum count at 321, down from 339 assessment objects. The count may only decrease. The tracked native-file delta is zero. Infrastructure deltas are one removed recursive Cargo invocation, one removed ambient-object path, one stale `heap.c.o` provider, two Rust hash-table provider cutovers, and all active hardcoded workstation paths. The report pins ledger v7 and names both cutovers: `native/charhashtable.c.o` to the sole Rust provider with RESOURCE/#22 retaining source-deletion ownership, and `native/stringhashtable.c.o` to the same Rust provider with CORE_NATIVE/#22 retaining source-deletion ownership. `CharHashTable` and `StringHashTable` are Rust-owned C ABI providers in `rust/src/collections/hash_table.rs`; their superseded C objects are excluded from production archive membership. No generic hash-table C template is tracked, embedded, generated, or staged.

S1's source-derived provider manifest independently enforces exact archive membership, one provider per internal symbol, duplicate/unassigned rejection, `displist.c.o` exclusion, strict final linking, and provider reports. `heap.c.o` has no source and is absent; `heap.h` remains COLLECTIONS-owned.

S4 consumes this matrix and the accepted S2 commands through
`rust/ci/gates.json`; it does not redefine target, prerequisite, determinism,
verification, or package semantics. Required CI executes every S2 tuple from an
isolated empty Cargo home and absent build-output paths. The S4 native-input delta
is zero and the maximum transitional input count remains 321.

## Supported matrix and prerequisites

`rust/ci/gates.json` is the machine authority for the exact supported tuples and runner mappings. `rust/build/supported-matrix.json` remains the S2 semantic input and must derive exactly the same tuple set. Supported hosts are current macOS and Linux on `aarch64` and `x86_64`, with SDL2 software rendering/input, UQM 0.8 content, cpal audio (ALSA on Linux), full networking, and directory-manifest packaging. Any other tuple fails before native compilation and reports every dimension.

Prerequisites are discovered for the active target with `pkg-config`; no Homebrew, `/usr/local`, volume, or user path is configured. Required packages are SDL2, libpng, liblzma, bzip2, and ALSA on Linux, plus `cc`, `ar`, `nm`, Cargo, and rustc. A missing tool or package reports the exact command and target package set that failed.

## Deterministic artifacts

Production resolves one target-aware toolchain before Cargo runs: canonical executable path, version output, executable SHA-256, and effective arguments for rustc, Cargo, CC, AR, NM, pkg-config, and the target linker. Target-qualified environment selectors take precedence and are normalized for build.rs, helper compilation, archive creation, pkg-config, strict symbol checks, and final linking. Ambient CFLAGS/CPPFLAGS/LDFLAGS/RUSTFLAGS are rejected rather than appended. `SOURCE_DATE_EPOCH` is mandatory build identity (defaulted by xtask to the Git commit epoch only when the caller omits it) and is recorded with package versions/cflags/libs and the complete ordered typed compile profile.

Production uses sorted source and archive input order, unique fixed member names, optimized non-debug C compilation, `ZERO_AR_DATE=1`, a unified `rust/Cargo.lock`, disabled incremental compilation, and deterministic `__DATE__`/`__TIME__` definitions. Repeated builds from the same checkout, target, toolchain, package metadata, dependency set, compile profile, and source epoch must produce byte-identical `uqm` bytes.

`rust/target/production-artifacts.json` records the executable, Rust static archive, C static archive, exact object sidecar, and provider report. Every entry includes role, repository-relative path, MIME type, byte length, SHA-256, and producing command. `prove` performs two genuinely empty/full release builds (removing the entire `rust/target/release` directory between each build), compares source/toolchain identity first, then all five artifacts by byte length and SHA-256, and records both digest sets and build identities as the determinism proof consumed by CI. `package` first runs the full `prove` determinism proof and validates staged evidence. It installs the executable with exact mode `0500` at `rust/target/uqm-package/<target>/uqm` and rewrites the packaged manifest's executable path to that file, so bootstrap proof launches the package artifact itself.

## Clean-checkout replay

```sh
git clean -ndx                   # review only; do not remove user files
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- doctor
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- production
rust/ownership/verify-production.sh
```

Production requires only tracked source plus external prerequisites. It does not require `sc2/build.vars`, `sc2/obj`, or a prior native build.


## Canonical proof and live verification

`cargo run --locked --manifest-path rust/xtask/Cargo.toml -- prove` performs two clean Cargo builds and writes the five-artifact manifest. The manifest records full Git HEAD, a deterministic digest and count of every tracked worktree file, honest tracked dirty status, complete rustc/Cargo/cc/pkg-config identities, host target, release profile, exact features, and each artifact hash. A clean CI checkout therefore produces clean LCAR evidence; local development evidence records `dirty: true` rather than omitting that fact.

`cargo run --locked --manifest-path rust/xtask/Cargo.toml -- verify` never silently rebuilds. It recomputes all live source, toolchain, target/profile/feature, path, length, and digest identities and rejects stale evidence. `rust/ownership/verify-production.sh` invokes this operation before strict ownership symbol checks.

Production native objects are source-derived. The compiler uses canonical absolute paths for the 321 tracked translation units, emits dependency files, and rejects any repository-local dependency outside the active explicit target subset in `rust/build/native-dependencies.json`. Every dependency declaration is tracked and hash-checked; `sc2/config_unix.h` is included. Ambient or ignored `sc2/obj` content never grants authority.

CI proves all declared build tuples on official runners and checks `uname -m`: Ubuntu 24.04 x86_64, Ubuntu 24.04 ARM aarch64, macOS 15 Intel x86_64, and macOS 15 Apple Silicon aarch64. Issue 23 additionally executes native gameplay acceptance on macOS 14 arm64, the maintained macOS Intel runner, Ubuntu 22.04 x86_64, and `ubuntu-24.04-arm` aarch64. GitHub has retired the requested `macos-13` hosted label, so CI records that tuple as an explicit machine-readable excluded execution rather than claiming a run. Applicable jobs run the production executable rather than treating cross-compilation as runtime evidence, retain typed passing/failing LCARs, and compare two real battle runs from one production artifact.


## Issue #23 boundary

The broad all-feature linked-harness gate is currently failing and is explicitly owned by issue #23. Issue #22 validates only its focused production feature pair and strict linked production artifact contract; this document does not claim the broad all-feature harness passed, and no gate is weakened or excluded here.

## Scaler pixel format contract: only_u8x4

UQM's scaler contract is RGBA U8x4 (RGBA8888). The `fast_image_resize` dependency is configured with the `only_u8x4` feature, which restricts the compiled scaler to exclusively support the U8x4 (four u8 channels) pixel format. This is required because:

1. UQM's framebuffer is always RGBA8888; no other pixel format is ever used.
2. The `only_u8x4` feature eliminates dead code paths for U8, U16x2, U16x3, and F32 pixel types, reducing binary size and improving reproducibility.
3. Any future change to this feature must be accompanied by a ledger amendment documenting the new pixel format contract.

The resolved locked Cargo feature graph — including `fast_image_resize/only_u8x4` — is recorded in production artifact evidence (`rust/target/production-artifacts.json` under `cargo_feature_graph`) and verified by `verify` against the live `Cargo.lock`.