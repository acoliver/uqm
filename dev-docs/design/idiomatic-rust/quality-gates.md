# Quality gates

Every check is a Rust `xtask` subcommand or a lint table. None pins our own files
by hash. Existing debt is recorded in one baseline file whose values only go down,
with finite budgets for the migration's transition counters. Companion documents:
[architecture.md](architecture.md), [plan.md](plan.md).

## 1. What exists today

`.github/workflows/ci.yaml`: `cargo fmt --check`, `abi-check` (run as `cargo run
--locked --manifest-path xtask/Cargo.toml -- abi-check`, `ci.yaml:36`), `cargo clippy
--workspace --all-targets -D warnings` (default lints), `cargo test`, a shell C-file
ratchet (`ci.yaml:40-56`, the only job that fetches the base ref), release builds on
five targets with `bundled-sdl` (`ci.yaml:97`), the four native failure scenarios,
and release-tool tests. There is no `clippy.toml`, no `[lints]` table and no Cargo
alias (`rust/.cargo/config.toml`). PR #282 removed the lizard complexity gate, the
70% line-coverage gate and `cargo audit`. `rust/xtask` already depends on `syn 2`
(`full`, `visit`), `quote` and `proc-macro2` with `span-locations`.

## 2. Lints

### 2.1 `rust/Cargo.toml`

```toml
[workspace.lints.rust]
unsafe_op_in_unsafe_fn = "warn"          # debt lint (2.3)

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
dbg_macro = "deny"
todo = "deny"
unimplemented = "deny"
mem_forget = "deny"
undocumented_unsafe_blocks = "warn"      # debt lint
unwrap_used = "warn"                     # debt lint
expect_used = "warn"                     # debt lint

[lints]
workspace = true                         # in both the uqm package and rust/xtask
```

### 2.2 `rust/clippy.toml`

```toml
msrv = "1.98"                        # the toolchain pinned in ci.yaml
too-many-arguments-threshold = 7     # clippy default; the 12 existing allows are baselined
type-complexity-threshold = 250
allow-unwrap-in-tests = true
allow-expect-in-tests = true
```

Phase 0 runs each `deny` lint once. Any lint with hits is either fixed in Phase 0
(when the hits are few) or moved to the debt set.

### 2.3 Deny set and debt set

`cargo xtask clippy` runs clippy once with `--message-format=json` and
`--force-warn` for each debt lint, then sorts the diagnostics:
- an error, or a warning from any lint outside the debt set, fails (the effect of
  `-D warnings` today);
- warnings from the **debt set** (`unsafe_op_in_unsafe_fn`,
  `clippy::undocumented_unsafe_blocks`, `clippy::unwrap_used`, `clippy::expect_used`)
  are counted per file against the baseline `[lint]` section. `--force-warn`
  overrides in-source `allow`s, so a broad suppression such as
  `allow(clippy::all)` over the ABI include (`ships/native.rs:4-12`) cannot hide
  debt moved under it.

In Phase 8 the debt set becomes `deny`. Where a compiler lint can enforce a rule it
is used instead of a count: when a phase closes a subsystem, that subsystem's module
root gets `#![deny(unsafe_code)]`.

Not adopted: crate-wide `pedantic`/`nursery` (thousands of hits in ported code), and
jefe's 60-line and cognitive-15 thresholds, which would force splits of functions that
mirror one C function (`process_collisions`, `cyborg_native::tactical_intelligence`)
and make side-by-side review against the C original harder.

## 3. `cargo xtask` subcommands

All accept `--base <git-ref>` (growth check, section 4) and `--tighten` (rewrite the
baseline downward). Production code is defined by the source walk of section 3.2.

| Command | Checks | Limit |
|---|---|---|
| `quality loc` | Effective LOC per production file: lines with a token, excluding comments, doc comments and `#[cfg(test)]` items | 800 per file |
| `quality complexity` | Per function: effective LOC, cyclomatic, cognitive (syn walk) | 80 LOC, cyclomatic 25, cognitive 30 |
| `quality ffi` | Per subsystem (top-level module): `static mut` (incl. `global!` expansions), `extern "C" fn` definitions, foreign items, `#[no_mangle]`/`#[export_name]`, `#[repr(C)]`, `link_name`, raw-pointer parameters, `include!` of `abi/*`, `app::bridge` call sites, `Hook::Legacy` values | baseline; 0 when absent |
| `quality unsafe` | Per subsystem: `unsafe` blocks, `unsafe fn`, `unsafe impl`, lines inside them | baseline; 0 outside `UNSAFE_ALLOWED` when absent |
| `quality allows` | Every `allow`/`expect`/`#![allow]`/`cfg_attr(.., allow(..))`, keyed by file, lint and covered item, valued by the effective LOC it covers; `#![cfg(not(test))]` modules | baseline; 0 when absent |
| `quality layers` | Imports against the layer table (architecture.md section 2); in engine/content/modes, uses of `std::time::{Instant, SystemTime}`, `std::thread`, `std::env`, `std::fs`, `std::net`, `std::process`, `sdl2` | baseline; 0 when absent |
| `quality native-files` | Tracked files ending `.c .h .cc .cpp .hpp .m .mm .s .S .asm` (`git ls-files`) | 0, absolute |
| `quality deps` | Section 3.4 | absolute |
| `clippy` | Section 2.3 | deny set: 0; debt set: baseline |
| `coverage` | `cargo llvm-cov --workspace --no-default-features --features audio_heart,debug-process --summary-only`, compared per file (section 4) | floor from baseline; target 70 |
| `deny` | `cargo deny --locked check advisories bans licenses sources` with `rust/deny.toml` | 0 findings |
| `parity record\|check` | Replay corpus (plan.md section 2) against `rust/tests/golden/parity/` | exact match |
| `abi-check` | Existing (`xtask/src/abi_check.rs`); extended in Phase 0 to foreign `static` types | kept until foreign items are 0 |

### 3.1 LOC and complexity

Effective LOC follows `llxprt-code-rs/xtask/src/loc.rs` (lines carrying a token by
`proc-macro2` spans, doc comments blanked). Cyclomatic and cognitive complexity follow
`llxprt-code-rs/xtask/src/complexity.rs` (365 lines, one syn walk per function). Not
lizard: an external Python tool that needed a patched 1.24.0 to parse Rust char
literals. Not clippy `cognitive_complexity`/`too_many_lines`: they baseline existing
functions only through `#[allow]`. Data tables (`hq_rules.rs::rules`,
`planets/tables.rs`) are ordinary baseline entries.

### 3.2 Source walk and identity

- **Walk.** From `src/lib.rs`, `src/main.rs`, `src/bin/*.rs` and `xtask/src`, the
  walker follows `mod`, `#[path]` and `include!` (as `abi_check.rs` does) and skips
  `#[cfg(test)]` items. It evaluates `cfg` for each CI configuration (linux, macOS
  and Windows targets; features `audio_heart`, `debug-process`, `bundled-sdl`); an item
  present in any configuration counts once. Item-generating first-party
  `macro_rules!` are expanded by a per-macro handler (`global!` today). Any other
  first-party macro that expands to items, or a `cfg` predicate the walker cannot
  evaluate, fails the gate, as unsupported export macros already fail `abi-check`
  (`abi_check.rs:610-623,1075-1090`).
- **Identity.** Keys are full paths from the crate root, with the impl self type and
  trait: `graphics::filegfx::load` and `graphics::sdl::png2sdl::load` are distinct;
  `<ships::races::arilou::Arilou as Race>::preprocess`. Moves and renames are debt
  transfers (section 4, rule 3).
- **Tests.** xtask unit tests cover those two `load` functions, the `include!` under
  `allow(clippy::all)` in `ships/native.rs:4-12`, `global!`, and a `cfg`-split item.

### 3.3 Where `unsafe` is permitted

`UNSAFE_ALLOWED` is a `const` list in xtask, changed only by a reviewed code change:

```rust
const UNSAFE_ALLOWED: &[&str] = &[
    "src/platform/sdl",      // texture upload, sdl2-sys calls
    "src/platform/macos",    // objc/CoreGraphics/CoreFoundation/sandbox
    "src/platform/net",      // only if sockets stay on libc/WinSock
    "src/threading/qos.rs",  // pthread QoS
];
```

Until a subsystem's phase closes, its `unsafe` is baseline debt; afterwards the limit
is 0. Every `unsafe` block needs `// SAFETY:`.

### 3.4 Native code and supply chain

`quality deps` enforces `dev-docs/project-standards.md` section 1 (zero first-party
native code; external libraries declared by Cargo, never vendored or patched):
- first-party packages (`uqm`, `xtask`) have no build script except `rust/build.rs`,
  whose `[build-dependencies]` are limited to `pkg-config`; no first-party `links`
  key; no direct dependency on `cc`, `cmake`, `bindgen` or `cxx-build`;
- no `[patch]` or `[replace]`, no `path` dependency outside the workspace, no `[source]`
  replacement in `.cargo/config.toml`, and no tracked `Cargo.toml` outside workspace
  members (a vendored crate);
- registry crates that compile native code are listed in a reviewed `const` in xtask:
  today `sdl2-sys` (bundled SDL through `cmake`) and `oboe-sys` (through `cc`). A new
  one fails until added. `cc` and `cmake` as transitive build tools are allowed.

The gate must pass on `main` as it is, including the `bundled-sdl` release builds.

`rust/deny.toml` covers advisories, licences and sources:

```toml
[graph]
all-features = true
[advisories]
version = 2
yanked = "deny"
ignore = []    # each entry: { id = "RUSTSEC-…", reason = "… #<issue>" }; count is baselined
[licenses]
version = 2
allow = []     # filled in Phase 0 from `cargo deny list`, checked against GPL-2.0-or-later
[bans]
multiple-versions = "warn"
wildcards = "deny"
[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

cargo-deny was chosen over cargo-audit because one pinned tool covers advisories,
licences and sources. Advisory data is fetched at run time; the tool is pinned.

## 4. Baseline file

`rust/xtask/quality-baseline.toml`, written only by xtask. An absent entry means 0 for
counters and the global limit for LOC/complexity. Values below are measured where
stated, otherwise placeholders until Phase 0 regenerates the file.

```toml
[coverage."src/battle/process_native.rs"]
lines_percent = 0.0          # per file; Phase 0 measurement

[subsystem.battle]           # line-regex measurement at 0ba2845ad
static_mut = 22
extern_c_fn = 160
no_mangle = 120
unsafe_lines = 4263          # about 40% of 10,657 production lines

[function."battle::process_native::process_collisions"]
loc = 236                    # placeholder
cyclomatic = 58              # placeholder

[allow."src/ships/native.rs"."clippy::all"."ships::native::abi"]
loc = 0                      # placeholder: effective LOC covered

[budget]                     # set once by the Phase 1 opening PR; limits only fall
bridge_calls = { limit = 0, ends = "phase-8" }   # placeholder limit
legacy_hooks = { limit = 0, ends = "phase-2" }   # placeholder limit

[moved]                      # debt transfers, "old key" = "new key"

[counter]
cfg_not_test_modules = 16
advisory_ignores = 0
```

Rules enforced by xtask:
1. **Measured ≤ baseline.** A value above its entry, or a violator with no entry,
   fails. A budget counter above its limit fails.
2. **Headroom is reported, not failed.** A value below its entry prints a CI notice.
   `cargo xtask quality --tighten` rewrites the baseline deterministically (sorted
   keys, measured values) in one command; phase-exit PRs run it.
3. **No growth against the base.** With `--base <ref>`, syn-measured entries are
   compared with values measured from the base tree (`git archive <ref>` into
   `target/xtask-base`, no build), so headroom from an earlier PR cannot be spent;
   lint counts are compared with the base's baseline file. Allowed without a label:
   - a budget counter growing up to its limit (raising a limit is growth);
   - a debt transfer: `cargo xtask quality move <old> <new>` adds a `[moved]` line;
     the new entry is accepted if its source existed at base and is now absent, each
     source credits one target, and no metric's crate total rises. A moved function
     keeps its own entry and is not split. `--tighten` drops `[moved]` lines whose
     target is already in the base baseline.
4. **Coverage per file.** Covered-line percentage may not fall for files present at
   both refs. Deleted files and files newly compiled under test are excluded, so
   removing tested dead code or making a module testable is not a drop.
5. **Owner sign-off.** Any other growth needs the label `quality-growth-approved`,
   applied only by the owner, with the reason in the PR description. Agents never
   apply it. Setting `[budget]` in the Phase 1 opening PR is one such approval.

There is no hashing, no provenance bundle and no self-check of xtask.

## 5. CI wiring (`.github/workflows/ci.yaml`)

Phase 0 adds the alias to `rust/.cargo/config.toml`; jobs run in `rust/`, where the
relative path resolves:

```toml
[alias]
xtask = "run --locked --manifest-path xtask/Cargo.toml --"
```

Each job that compares with the base starts with:

```yaml
env:
  BASE: ${{ github.base_ref }}
steps:
  - uses: actions/checkout@v4
  - if: github.event_name == 'pull_request'
    run: git fetch --no-tags --depth=1 origin "+refs/heads/$BASE:refs/remotes/origin/$BASE"
  - run: echo "BASE_ARG=${BASE:+--base origin/$BASE}" >> "$GITHUB_ENV"
```

On a push to `main`, `BASE` is empty, `BASE_ARG` is empty, and only rules 1, 2 and 4
apply. `pull_request` adds `types: [opened, synchronize, reopened, labeled,
unlabeled]` so a label change re-runs. `GROWTH` and `GOLDEN` stand for
`${{ contains(github.event.pull_request.labels.*.name, '<label>') && '--allow-…' || '' }}`.

| Job | Runner | Steps |
|---|---|---|
| `rust` | ubuntu | `cargo fmt --all --check`; `cargo xtask quality $BASE_ARG GROWTH`; `cargo xtask clippy $BASE_ARG GROWTH` (replaces the raw clippy step); `cargo test --workspace --locked --no-default-features --features audio_heart,debug-process`; `cargo xtask abi-check` until deleted |
| `supply` | ubuntu | install pinned cargo-deny (`--locked`); `cargo xtask deny` |
| `coverage` | ubuntu | install pinned cargo-llvm-cov; `cargo xtask coverage $BASE_ARG GROWTH` under SDL dummy drivers |
| `parity` | ubuntu | `cargo xtask parity check $BASE_ARG GOLDEN`; fails if `rust/tests/golden/` differs from the base unless the PR carries `parity-golden-update` |
| `game-build` | 5 targets | unchanged |
| `failure-scenarios` | macOS | unchanged (4 scenarios) |
| `release-tools` | ubuntu | unchanged |

The `c-ratchet` job is deleted; `quality native-files` replaces it with an absolute
zero. Phase 0 is accepted when these jobs pass on one PR and on the following push.

## 6. Sources

Effective LOC, the 800/80 limits and syn complexity 25/30: llxprt-code-rs
`xtask/src/{loc,complexity,lib}.rs`. Every `allow` as a finding: llxprt-code-rs
`xtask/src/analyze.rs:281-287` and jefe `xtask/src/clippy_policy.rs`. Shrink-only
ledger against the base ref: llxprt-code-rs `xtask/src/coupling.rs`. Reviewed exception
lists: jefe `xtask/src/architecture.rs`. Banned-pattern scan for `layers`:
llxprt-code-rs `xtask/src/compat.rs`. `[lints]` table and `clippy.toml` thresholds:
jefe and vibetools. Coverage inside xtask: jefe `xtask/src/toolchain.rs:114-128`. 70%
coverage target: UQM before PR #282. No SHA pinning or self-referential proof: owner
requirement, PR #282. References read: vybestack/llxprt-code-rs `main`,
vybestack/llxprt-jefe at 9b7e09ca, acoliver/vibetools `branch-1` a6cb22e.
