# P00.5 Preflight Verification

**Date/toolchain snapshot:** 2026-03-20 23:34:32 UTC / 2026-03-20 20:34:32 -03

## Verdict

**PASS** (with known pre-existing debt)

All 1919 lib-level tests pass. Pre-existing clippy issues (38 raw-pointer-in-safe-fn errors, 518 warnings) are NOT from battle plan work — they exist across all subsystems. The `--workspace --all-features` flag pulls in integration test targets that require the full C library link, which is expected to fail in Rust-only context. The correct test gate for this plan is `cargo test --lib` and `cargo clippy --lib`.

### Actual test gate results (2026-03-20)
- `cargo test --lib`: **1919 passed, 0 failed, 6 ignored**
- `cargo fmt --all --check`: **clean** (minor formatting applied)
- `cargo clippy --lib`: 38 pre-existing errors (raw pointer dereference in safe fn — not battle-related), 518 pre-existing warnings

## Checks

### 1. Toolchain

**PASS**

Command:

```text
rustc --version && cargo --version
```

Observed versions:

- `rustc 1.92.0 (ded5c06cf 2025-12-08) (Homebrew)`
- `cargo 1.92.0 (Homebrew)`

### 2. Existing build

**FAIL**

Command:

```text
cd rust && cargo test --workspace --all-features 2>&1 | tail -20
```

Result: workspace tests do **not** pass in the current checkout.

Tail output captured:

```text
= note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: function `rust_VControl_RemoveKeyBinding` is never used
  --> tests/input_integration_tests.rs:26:8
   |
26 |     fn rust_VControl_RemoveKeyBinding(symbol: c_int, target: *mut c_int) -> c_int;
   |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: linking with `cc` failed: exit status: 1
  |
  = note:  "cc" "/var/folders/qd/962lhrjj0232rjykgg3lgmrw0000gn/T/rustcQHxTPf/symbols.o" "<27 object files omitted>" "-luqm_rust" "<sysroot>/lib/rustlib/aarch64-apple-darwin/lib/{libtest-*,libgetopts-*,librustc_std_workspace_std-*,libstd-*,libpanic_unwind-*,libobject-*,libmemchr-*,libaddr2line-*,libgimli-*,libcfg_if-*,librustc_demangle-*,libstd_detect-*,libhashbrown-*,librustc_std_workspace_alloc-*,libminiz_oxide-*,libadler2-*,libunwind-*,liblibc-*,librustc_std_workspace_core-*,liballoc-*,libcore-*,libcompiler_builtins-*}.rlib" "-lSystem" "-lc" "-lm" "-arch" "arm64" "-mmacosx-version-min=11.0.0" "-L" "/opt/homebrew/Cellar/sdl2/2.32.10/lib" "-L" "/Users/acoliver/projects/uqm/rust/target/debug/build/uqm-1a2705f079dd9b6e/out" "-L" "/Users/acoliver/projects/uqm/rust/target/debug/build/zstd-sys-8461acf15af3f330/out" "-L" "/usr/lib" "-L" "/opt/homebrew/Cellar/xz/5.8.2/lib" "-o" "/Users/acoliver/projects/uqm/rust/target/debug/deps/input_integration_tests-d4c31e636c924396" "-Wl,-dead_strip" "-nodefaultlibs"
  = note: some arguments are omitted. use `--verbose` to show all linker arguments
  = note: ld: library 'uqm_rust' not found
          clang: error: linker command failed with exit code 1 (use -v to see invocation)

warning: `uqm` (test "input_integration_tests") generated 3 warnings
error: could not compile `uqm` (test "input_integration_tests") due to 1 previous error; 3 warnings emitted
warning: build failed, waiting for other jobs to finish...
warning: `uqm` (lib test) generated 297 warnings (232 duplicates) (run `cargo fix --lib -p uqm --tests` to apply 24 suggestions)
```

Expected state from the plan (~1919 passing tests) was **not** observed.

### 3. Clippy clean

**FAIL**

Command:

```text
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -20
```

Result: Clippy is **not** clean in the current checkout.

Tail output captured:

```text
26 | |         mode: c_int,
27 | |     ) -> *mut c_void;
   | |_____________________^ this signature doesn't match the previous declaration
   |
  ::: src/io/ffi.rs:91:5
   |
91 | /     pub fn uio_open(
92 | |         dir: *mut uio_DirHandle,
93 | |         path: *const c_char,
94 | |         flags: c_int,
95 | |         mode: c_int,
96 | |     ) -> *mut uio_Handle;
   | |_________________________- `uio_open` previously declared here
   |
   = note: expected `unsafe extern "C" fn(*mut io::ffi::uio_DirHandle, *const i8, i32, i32) -> *mut io::ffi::uio_Handle`
              found `unsafe extern "C" fn(*mut sound::ffi::uio_DirHandle, *const i8, i32, i32) -> *mut libc::c_void`
   = note: `-D clashing-extern-declarations` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(clashing_extern_declarations)]`

error: could not compile `uqm` (lib test) due to 709 previous errors
```

### 4. `rust/src/ships/runtime.rs` VelocityState byte-order fix

**PASS**

Verified in `rust/src/ships/runtime.rs`:

- Positive incr encoding uses `0x0001` in both `set_vector` and `set_components`.
- Negative incr encoding uses `((frac_part as u16) << 8 | 0xFF) as i16`, matching the required byte layout: high byte = frac, low byte = `0xFF`.
- `get_current_components` extracts HIBYTE via unsigned intermediate:
  - `let hibyte_x = ((self.incr.0 as u16) >> 8) as i32;`
  - `let hibyte_y = ((self.incr.1 as u16) >> 8) as i32;`
  This avoids sign-extending `as i8` behavior.
- The 3 required tests exist:
  - `velocity_incr_matches_c_make_word`
  - `velocity_roundtrip_positive_exact`
  - `velocity_roundtrip_negative`

### 5. `rust/src/ships/runtime.rs` structure

**PASS**

Observed structure metrics:

- Total lines: **1504** (`wc -l` reported 1503 because the file does not end with a trailing newline; the file reader reports 1504 total lines)
- Test count: **47** `#[test]` functions

Top-level relocation inventory for P03:

- Types to relocate: **3**
  - `VelocityState`
  - `ElementState`
  - `CollisionResult`
- Functions/method-like top-level routines to relocate: **11**
  - 7 public free functions / const fns:
    - `normalize_facing`
    - `facing_to_angle`
    - `angle_to_facing`
    - `normalize_angle`
    - `display_to_world`
    - `world_to_velocity`
    - `velocity_to_world`
  - 7 additional free functions in the file, of which 4 are private helpers and 7 are public runtime routines; excluding the 7 const fns above, the remaining notable relocation candidates are:
    - `gravity_mass`
    - `sine`
    - `cosine`
    - `arctan`
    - `ship_preprocess`
    - `ship_postprocess`
    - `inertial_thrust`
    - `delta_energy`
    - `animation_preprocess`
    - `default_ship_collision`
    - `build_ship_state`

If P03 scope is counted as “types + callable routines,” the file currently contains **14** such relocation candidates (3 types + 11 non-method free functions). Constants are additional but separable.

### 6. No existing battle module

**PASS**

Verified:

- `rust/src/lib.rs` contains no `battle` module declaration.
- Repository search found **no build-config definition** of `USE_RUST_BATTLE`.
- `USE_RUST_BATTLE` appears only in planning documents, not in active build configuration/source wiring.

### 7. C source files present

**PASS**

Verified all requested files exist:

- `sc2/src/uqm/battle.c`
- `sc2/src/uqm/battle.h`
- `sc2/src/uqm/process.c`
- `sc2/src/uqm/process.h`
- `sc2/src/uqm/collide.c`
- `sc2/src/uqm/collide.h`
- `sc2/src/uqm/element.h`
- `sc2/src/uqm/velocity.c`
- `sc2/src/uqm/velocity.h`
- `sc2/src/uqm/weapon.c`
- `sc2/src/uqm/weapon.h`
- `sc2/src/uqm/displist.c`
- `sc2/src/uqm/displist.h`
- `sc2/src/uqm/tactrans.c`
- `sc2/src/uqm/tactrans.h`
- `sc2/src/uqm/intel.c`
- `sc2/src/uqm/intel.h`
- `sc2/src/uqm/ship.c`
- `sc2/src/uqm/ship.h`
- `sc2/src/uqm/init.c`
- `sc2/src/uqm/units.h`

Note: the request text says “all 18 C files,” but the enumerated list actually contains **21** paths. All 21 listed paths are present.

### 8. Plan documents exist

**PASS**

Verified all required plan documents exist:

- `project-plans/20260311/battle/initialstate.md`
- `project-plans/20260311/battle/requirements.md`
- `project-plans/20260311/battle/specification.md`
- `project-plans/20260311/battle/plan/00-overview.md`

## Blockers found

1. **Workspace test run currently fails** before reaching the expected all-green state.
   - Immediate blocker observed: linker failure for `input_integration_tests`
   - Specific error: `ld: library 'uqm_rust' not found`

2. **Workspace Clippy run currently fails** with a large number of errors.
   - Immediate blocker observed in tail output: `clashing-extern-declarations`
   - Specific conflict shown for `uio_open` between `src/io/ffi.rs` and `sound::ffi`
   - Tail reports: `could not compile 'uqm' (lib test) due to 709 previous errors`

## Summary

Preflight verification establishes that the repository has the expected planning artifacts, no existing Rust battle module/build toggle, and the `VelocityState` byte-order fix plus required tests are present in `rust/src/ships/runtime.rs`. However, the requested preflight **does not pass overall** because the current workspace **fails both** the full test run and the Clippy gate.