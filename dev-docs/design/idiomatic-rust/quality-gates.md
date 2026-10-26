# Quality gates

Every check is a Rust `xtask` subcommand or a lint table. None pins our own files
by hash. Existing debt is recorded in one baseline file whose values only go down,
with finite budgets for the migration's transition counters. Layering is enforced by
the Cargo crate graph as crates are split (architecture.md section 2.1); `quality
layers` checks the graph and counts what still blocks the next split. Companion
documents: [architecture.md](architecture.md), [plan.md](plan.md),
[work-items.md](work-items.md).

## 1. What exists today

`.github/workflows/ci.yaml`: `cargo fmt --check`, `abi-check` (run as `cargo run
--locked --manifest-path xtask/Cargo.toml -- abi-check`, `ci.yaml:36`), `cargo clippy
--workspace --all-targets -D warnings` (default lints), `cargo test`, a shell C-file
ratchet (`ci.yaml:40-56`, the only job that fetches the base ref), release builds on
five targets with `bundled-sdl` (`ci.yaml:97`), the four native failure scenarios,
and release-tool tests. There is no `clippy.toml`, no `[lints]` table and no Cargo
alias (`rust/.cargo/config.toml`). PR #282 removed the lizard complexity gate, the
70% line-coverage gate and `cargo audit`. `rust/xtask` already depends on `syn 2`
(`full`, `visit`), `quote` and `proc-macro2` with `span-locations`, and on the game
package itself (`uqm-rust` with `audio_heart,debug-process`, `xtask/Cargo.toml`), so
every xtask command compiles the whole game first. The workspace has two members
(`uqm`, `xtask`); `rust/.cargo/config.toml:2` sets `incremental = false`.

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
workspace = true                         # in every workspace member
```

Crate roots carry the `unsafe` attribute of architecture.md section 2.1:
`#![forbid(unsafe_code)]` for `uqm-save`, `uqm-content` and `uqm-modes` from creation,
for `uqm-engine` from Phase 7 exit and for the root from Phase 8;
`#![deny(unsafe_code)]` with module allows for `uqm-platform`.

### 2.2 `rust/clippy.toml`

```toml
msrv = "1.98"                        # the toolchain pinned in ci.yaml
too-many-arguments-threshold = 7     # clippy default; the 12 existing allows are baselined
type-complexity-threshold = 250
allow-unwrap-in-tests = true
allow-expect-in-tests = true
```

Phase 0 runs each `deny` lint once. Any lint with hits is either fixed in Phase 0
(when the hits are few) or moved to the debt set. U0-02 adds the deny set only, while
CI still runs raw clippy with `-D warnings`; the four debt lints arrive in U0-03
together with `cargo xtask clippy`, which baselines them and replaces the raw step in
the same PR, so no PR runs a debt lint under `-D warnings`.

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

Debt baselines are generated and checked by CI on Linux x86_64 with Rust 1.98.0
and features `audio_heart,debug-process,native`. That target is the authority:
Clippy sees only compiled `cfg` branches, so another host can report different
counts. Local runs remain useful checks but must not raise the Linux baseline.
Document or remove unsafe blocks behind target-specific branches, or use the CI
measurement when tightening entries (decision [#708](https://github.com/acoliver/uqm/issues/708)).

In Phase 8 the debt set becomes `deny`. Where a compiler lint can enforce a rule it
is used instead of a count: when a phase closes a subsystem, that subsystem's module
root gets `#![deny(unsafe_code)]`, and a crate root gets `forbid` as above.

Not adopted: crate-wide `pedantic`/`nursery` (thousands of hits in ported code), and
jefe's 60-line and cognitive-15 thresholds, which would force splits of functions that
mirror one C function (`process_collisions`, `cyborg_native::tactical_intelligence`)
and make side-by-side review against the C original harder.

## 3. `cargo xtask` subcommands

Baseline-backed checks accept `--base <git-ref>` (growth check, section 4) and
`--tighten` (rewrite the baseline downward). Coverage accepts `--base <git-ref>`
to identify added production files and has no baseline or growth override.
Production code is defined by the source walk of section 3.2.

| Command | Checks | Limit |
|---|---|---|
| `quality loc` | Effective LOC per production file: lines with a token, excluding comments, doc comments and `#[cfg(test)]` items | 800 per file |
| `quality complexity` | Per function: effective LOC, cyclomatic, cognitive (syn walk) | 80 LOC, cyclomatic 25, cognitive 30 |
| `quality ffi` | Per subsystem (top-level module): `static mut` (incl. macro expansions), `extern "C" fn` definitions, foreign items, `#[no_mangle]`/`#[export_name]`, `#[repr(C)]`, `link_name`, raw-pointer parameters, `include!` of `abi/*`, `app::bridge` call sites, `Hook::Legacy` values; `raw_records` (references to `ELEMENT`, `HELEMENT`, `STARSHIP`, `HSTARSHIP`, `RACE_DESC`, `RaceDescPtr`, `DisplayArray`, `disp_q`, `race_q` outside the record module and pure re-exports); `queue_calls` (calls into `collections::queue` outside its typed views) | baseline; 0 when absent |
| `quality unsafe` | Per subsystem: `unsafe` blocks, `unsafe fn`, `unsafe impl`, lines inside them | baseline; 0 outside `UNSAFE_ALLOWED` when absent |
| `quality allows` | Every `allow`/`expect`/`#![allow]`/`cfg_attr(.., allow(..))`, keyed by file, lint and covered item, valued by the effective LOC it covers; `#![cfg(not(test))]` modules | baseline; 0 when absent |
| `quality layers` | Crate graph (section 3.5): workspace edges, OS crates per crate, references that block a planned crate, banned `std` APIs; inside the root, imports against the layer table (architecture.md section 2) | edges: absolute; others baseline, 0 when absent |
| `quality native-files` | Tracked files ending `.c .h .cc .cpp .hpp .m .mm .s .S .asm` (`git ls-files`) | 0, absolute |
| `quality deps` | Section 3.4 | absolute |
| `clippy` | Section 2.3 | deny set: 0; debt set: baseline |
| `coverage` | `cargo llvm-cov --workspace --no-default-features --features audio_heart,debug-process,native --summary-only`; workspace and added production files (section 4 rule 4, [#708](https://github.com/acoliver/uqm/issues/708)) | workspace ≥ 70%; each added production file ≥ 70%, absolute |
| `deny` | `cargo deny --locked check advisories bans licenses sources` with `rust/deny.toml` | 0 findings |
| `parity record\|check` | Replay corpus (plan.md section 2) against `rust/tests/golden/parity/` | exact match |
| `quality report <counter>[:<scope>]` | Prints one counter for a subsystem, file or crate; `--expect N` fails unless it equals N, `--expect-only <scopes>` fails if any other scope is nonzero. Acceptance commands use it for zero checks | as asked |
| `quality ident <Name>... [--defs]` | Counts the identifier and path tokens naming each argument, by syn tokens, in every target of every workspace member (test targets and `#[cfg(test)]` items included, unlike the counters), outside comments, doc comments and string literals; C traceability doc lines (`/// C: <name>`) do not count. With `--defs` it counts definitions only: a `fn` with a body, `struct`, `enum`, `union`, type alias, `trait`, `const`, `static` with an initializer, `mod`, `macro_rules!` or field; extern-block declarations and `use` re-exports are not definitions. `--expect N` and `--expect-only <scopes>` work as for `quality report`. Added in U0-02; every Deletes list uses it for zero-count checks | as asked |
| `abi-check` | Existing (`xtask/src/abi_check.rs`); extended in U0-01 to foreign `static` types and to every workspace crate; a foreign declaration whose definition is in a crate the declaring crate does not depend on fails | deleted in U8-01 |

### 3.1 LOC and complexity

Effective LOC follows `llxprt-code-rs/xtask/src/loc.rs` (lines carrying a token by
`proc-macro2` spans, doc comments blanked). Cyclomatic and cognitive complexity follow
`llxprt-code-rs/xtask/src/complexity.rs` (365 lines, one syn walk per function). Not
lizard: an external Python tool that needed a patched 1.24.0 to parse Rust char
literals. Not clippy `cognitive_complexity`/`too_many_lines`: they baseline existing
functions only through `#[allow]`. Data tables (`hq_rules.rs::rules`,
`planets/tables.rs`) are ordinary baseline entries.

### 3.2 Source walk and identity

- **Walk.** From the targets of every workspace member as listed by `cargo metadata
  --no-deps` (today `src/lib.rs`, `src/main.rs`, `src/bin/*.rs` and `xtask/src`), the
  walker follows `mod`, `#[path]` and `include!` (as `abi_check.rs` does) and skips
  `#[cfg(test)]` items. It evaluates `cfg` for each CI configuration (linux, macOS
  and Windows targets; features `audio_heart`, `debug-process`, `bundled-sdl`); an item
  present in any configuration counts once. Item-generating first-party
  `macro_rules!` are expanded by a per-macro handler (today the two `global!` macros,
  `game_init/kernel_abi.rs:17` and `mainloop/options_abi.rs:9`, and `screen_global!`,
  `graphics/gfx_common_abi.rs:15`). Any other
  first-party macro that expands to items, or a `cfg` predicate the walker cannot
  evaluate, fails the gate, as unsupported export macros already fail `abi-check`
  (`abi_check.rs:610-623,1075-1090`). At 35d33879b there are 16 such definitions, so
  U0-02 ships 16 handlers: besides the three above, `accessor!`
  (`mainloop/state_abi.rs:14`), `operation!` (`mainloop/main2_abi.rs:2`,
  `mainloop/restart_menu/menu_abi.rs:6`, `ships/native_adapters.rs:53`),
  `define_hash_table!` (`collections/hash_table.rs:396`), `energy_hook!`
  (`planets/generate/home_exports.rs:268`), `moons_hook!`, `story_energy!`,
  `story_pickup!` (`planets/generate/late_exports.rs:216,332,411`), `menu_handler!`
  (`front_end/setupmenu.rs:146`), `callbacks!` (`comm/races/comandr.rs:587`),
  `game_state_fields!` (`state/schema.rs:6`) and `assert_c_layout!`
  (`c_bindings.rs:112`). A handler is deleted with its macro.
- **Identity.** Keys are the package name plus the full path from the crate root,
  with the impl self type and trait: `uqm::graphics::filegfx::load` and
  `uqm::graphics::sdl::png2sdl::load` are distinct;
  `<uqm::ships::races::arilou::Arilou as Race>::preprocess`. Moves and renames are
  debt transfers (section 4, rule 3). A crate split moves whole directories, so
  `cargo xtask quality move --prefix uqm::graphics uqm-engine::graphics` records one
  `[moved]` prefix line that transfers every counter and lint entry under it.
- **Tests.** xtask unit tests cover those two `load` functions, the `include!` under
  `allow(clippy::all)` in `ships/native.rs:4-12`, `global!`, and a `cfg`-split item.

### 3.3 Where `unsafe` is permitted

`UNSAFE_ALLOWED` is a `const` list in xtask, changed only by a reviewed code change:

```rust
const UNSAFE_ALLOWED: &[&str] = &[
    "crates/platform/src/sdl",    // texture upload, sdl2-sys calls
    "crates/platform/src/macos",  // objc/CoreGraphics/CoreFoundation/sandbox
    "crates/platform/src/net",    // only if sockets stay on libc/WinSock
    "crates/platform/src/qos.rs", // pthread QoS, today threading/qos.rs
    "crates/platform/src/os",     // process, descriptor, fd-relative file and errno calls (U7-06)
];
```

Until a subsystem's phase closes, its `unsafe` is baseline debt; afterwards the limit
is 0. Every `unsafe` block needs `// SAFETY:`. During the migration `unsafe` may move
from a caller into an engine wrapper, the `src/engine/` staging accessors or
`engine::platform::os` when the workspace total of each `unsafe` metric does not
rise; new `unsafe` needs an owner-approved allowance (AGENTS.md).

### 3.4 Native code and supply chain

`quality deps` enforces `dev-docs/project-standards.md` section 1 (zero first-party
native code; external libraries declared by Cargo, never vendored or patched):
- first-party packages (the workspace members) have no build script except the
  SDL probe (`rust/build.rs` today; it moves with the `sdl2` dependency to
  `crates/engine/build.rs` in Phase 1 and `crates/platform/build.rs` in Phase 7),
  whose `[build-dependencies]` are limited to `pkg-config`; no first-party `links`
  key; no direct dependency on `cc`, `cmake`, `bindgen` or `cxx-build`;
- no `[patch]` or `[replace]`, no `path` dependency outside the workspace, no `[source]`
  replacement in `.cargo/config.toml`, and no tracked `Cargo.toml` outside workspace
  members (a vendored crate);
- registry crates that compile native code are listed in a reviewed `const` in xtask:
  today `sdl2-sys` (bundled SDL through `cmake`) and `oboe-sys` (through `cc`). A new
  one fails until added. `cc` and `cmake` as transitive build tools are allowed.

The gate must pass on `main` as it is, including the `bundled-sdl` release builds.
New members set `version.workspace = true` and `publish = false`.

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

### 3.5 Crate graph

`quality layers` reads `cargo metadata --locked --format-version 1` (no build) and
checks four things.

1. **Workspace edges.** Every normal, dev and build dependency of a member on another
   member must be listed in a reviewed table in xtask. An unlisted edge fails; there
   is no baseline for it. Cargo itself rejects cycles.

   ```rust
   const CRATE_DEPS: &[(&str, &[&str])] = &[
       ("uqm-save", &[]),
       ("uqm-engine", &["uqm-save"]),
       ("uqm-content", &["uqm-engine"]),
       ("uqm-modes", &["uqm-engine", "uqm-content", "uqm-save"]),
       ("uqm-platform", &["uqm-engine"]),
       ("uqm", &["uqm-engine", "uqm-content", "uqm-modes", "uqm-save", "uqm-platform"]),
       ("uqm-xtask", &["uqm", "uqm-save"]), // uqm only under feature `native`
   ];
   ```
2. **OS crates.** Direct dependencies on `sdl2`, `sdl2-sys`, `gl`, `libc`,
   `windows-sys`, `rodio` and `cpal` are allowed in `uqm-platform` and `uqm-xtask`.
   For `uqm-engine` and `uqm` they are baseline entries until Phase 7 and may only be
   removed.
3. **Planned crates.** From U0-03, `PLANNED` lists, for every crate not yet split
   (`uqm-engine`, `uqm-content`, `uqm-modes`, `uqm-platform`), the root modules the
   layer table assigns to it (`uqm-engine`: `graphics`, `sound`, `resource`, `video`,
   `strings`, `threading`, `input`, `io`, `network`, `callback`, `collections`,
   `math`, `memory`, `logging`, `log_abi`, `bridge_log`, `platform`,
   `time::counter_abi`, `time::source`). Each `crate::` path, and each foreign declaration that
   `abi-check`'s resolver finds defined, from a planned member to a module that will
   not be in or below that crate counts against `planned.<crate>`. At 0ba2845ad
   `planned.uqm-engine` is 20 (13 paths, 7 foreign statics; architecture.md section
   2.2). The move PR requires 0. The staging module `rust/src/engine/` is counted the
   same way but as the budget counter `planned_engine_core` (set in U1-01, 0 at
   U4-04), because its Stage B code reaches `GlobData` until Phase 4.
   `planned.uqm-content` and `planned.uqm-modes` must be 0 in U6-04's semantic
   commits, before its move commit.
4. **Banned `std` APIs** in `uqm-save`, `uqm-content`, `uqm-modes`, and `uqm-engine`
   from Phase 7 exit: `std::time::{Instant, SystemTime}`, `std::thread`, `std::env`,
   `std::fs`, `std::net`, `std::process`. Per-file baseline, 0 when absent.

Inside the root package the module layer table of architecture.md section 2 still
applies to modules not yet in a crate.

## 4. Baseline file

`rust/xtask/quality-baseline.toml`, written only by xtask. An absent entry means 0 for
counters and the global limit for LOC/complexity. Values below are measured where
stated, otherwise placeholders until Phase 0 regenerates the file.

```toml
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

[budget]                     # set by the PR that creates each counter (U1-01, U1-02); limits only fall
bridge_calls = { limit = 0, ends = "phase-6" }   # placeholder limit; 0 before the modes split
legacy_hooks = { limit = 0, ends = "phase-2" }   # placeholder limit
planned_engine_core = { limit = 0, ends = "phase-4" } # placeholder limit; staging module

[moved]                      # debt transfers, "old key" = "new key"; a split adds
                             # prefix lines: "uqm::graphics" = "uqm-engine::graphics"

[crate_os."uqm-engine"]      # OS crates outside uqm-platform; removable only
deps = ["cpal", "gl", "libc", "rodio", "sdl2", "sdl2-sys", "windows-sys"]

[planned]
"uqm-engine" = 20            # references blocking the split; 0 before the move PR

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
     target is already in the base baseline. For `unsafe` metrics moved across
     subsystems or crates (a caller's `unsafe` into an engine wrapper), the check is
     that the workspace total does not rise.
4. **Coverage floors.** Workspace line coverage must be at least 70%, and each
   production file added relative to `--base` must have at least 70% line coverage.
   These are absolute checks, with no per-file baseline or growth override. Files
   added by a move also count as added; an added production file absent from the
   report fails. Existing files contribute to the workspace total without an exact
   per-file ratchet. Decision [#708](https://github.com/acoliver/uqm/issues/708)
   replaces that ratchet because correct subprocess tests exercise different
   timeout/poll branches under OS scheduling, changing measured coverage between
   passing runs. No files are excluded from the workspace measurement.
5. **Owner sign-off.** Any other growth needs the label `quality-growth-approved`,
   applied only by the owner, with the reason in the PR description. Agents never
   apply it. Setting the `[budget]` counters (U1-01, U1-02) and U1-02's unsafe
   allowance are such approvals.

There is no hashing, no provenance bundle and no self-check of xtask.

## 5. CI wiring (`.github/workflows/ci.yaml`)

Phase 0 adds the aliases to `rust/.cargo/config.toml`, which keeps
`[build] incremental = false`; jobs run in `rust/`, where
the relative path resolves. The game dependency of xtask moves behind its feature
`native` (default on), and the gate alias turns it off, so `quality`, `clippy`,
`deny`, `coverage`, `parity` and `abi-check` start without compiling the game.
`parity` builds the `uqm` binary it replays with `cargo build --release --locked
--bin uqm` as a subprocess, as `native run` does today. The `FEATURES` variable of
the `rust` and `coverage` jobs becomes `audio_heart,debug-process,native`, because
`--no-default-features` on a workspace command also turns off xtask's default
`native` feature and would drop its native-harness tests.

```toml
[alias]
xtask = "run --locked --manifest-path xtask/Cargo.toml --no-default-features --"
xtask-native = "run --locked --release --manifest-path xtask/Cargo.toml --"
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
| `rust` | ubuntu | `cargo fmt --all --check`; `cargo xtask quality $BASE_ARG GROWTH`; `cargo xtask clippy $BASE_ARG GROWTH` (replaces the raw clippy step in U0-03); `cargo test --workspace --locked --no-default-features --features audio_heart,debug-process,native`; `cargo xtask abi-check` until U8-01 |
| `supply` | ubuntu | install pinned cargo-deny (`--locked`); `cargo xtask deny` |
| `coverage` | ubuntu | install Clippy, llvm-tools-preview and pinned cargo-llvm-cov; `cargo xtask coverage $BASE_ARG` under SDL dummy drivers |
| `parity` | ubuntu | `cargo xtask parity check $BASE_ARG GOLDEN` and `parity counts check --all`; fails if a file under `rust/tests/golden/` that exists at base is modified or deleted without `parity-golden-update`, or a new golden arrives without its corpus manifest entry (U0-06 creates the first set with the owner's approval) |
| `game-build` | 5 targets | unchanged until U7-06, which moves `examples/platform_smoke.rs` to `uqm-platform`: then `cargo run --release --locked -p uqm-platform --example platform_smoke --features bundled-sdl` |
| `failure-scenarios` | macOS | unchanged (4 scenarios) |
| `release-tools` | ubuntu | unchanged |

The `c-ratchet` job is deleted; `quality native-files` replaces it with an absolute
zero. Phase 0 is accepted when these jobs pass on one PR and on the following push.

Crate splits change no job. `members = ["xtask", "crates/*"]` adds each new crate to
`--workspace`, and `default-members` stays `["."]`, so commands without `--workspace`
build the app and its dependencies only. Release packaging is unchanged:
`scripts/release/package.py` and `release.yml` build `--bin uqm --features bundled-sdl`
in the root package (the feature is forwarded), find `sdl2-sys` through `cargo
metadata`, read `workspace.package.version`, and take the binary from
`rust/target/<target>/release`.

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
