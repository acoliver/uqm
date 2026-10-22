# Sphere geometry port checkpoint

This checkpoint records the sphere geometry design, original-C fixture proof, and focused verification. The extended fixture retains the original 607,765-byte capture as an unchanged prefix. Final supported-suite, strict-linked, and direct-binary results are recorded separately under the repository root's `tmp/current-sphere-geometry/` and in the PR after committing; they are not established by the focused results below.

This ports `P3mult`, `P3sub`, `P3dot`, `P3norm`, `GenerateSphereMask`, `create_aa_points`, `get_color_channel`, `get_avg_channel`, and `CreateSphereTiltMap` from `sc2/src/uqm/planets/plangen.c`. It adds no production C, C shim, ABI export, call-site wiring, or dependency.

## Types, domains, and dimensions

`SphereMapPoint.points` and the sun location use the existing `graphics::drawable::Point`, whose coordinates are signed `i16` and which already implements `Debug`, `Clone`, `Copy`, and `Eq`. No traits were added to other modules. `Point3` remains a local three-dimensional floating-point type. Serialization writes each signed coordinate's little-endian bits, each `u32` weight, and each `f64` bit pattern separately. It never reads raw Rust struct storage. For example, the map sentinel `0xa5a5` is represented as signed -23131 and retains those exact bits.

The C definitions are independent of logical screen width:

- `sc2/src/uqm/units.h` defines `SAFE_Y=0`.
- `sc2/src/uqm/planets/planets.h` defines `MAP_HEIGHT=(75-SAFE_Y)`, hence 75.
- The enabled branch in `plangen.c` defines `SPHERE_SPAN_X=MAP_HEIGHT`, hence 75. Its `MAP_WIDTH >> 1` alternative is disabled.
- `RADIUS=37` and `DIAMETER=75` determine the mask and tilt array sizes.

`MAP_WIDTH=SIS_SCREEN_WIDTH` belongs to other topography algorithms. These sphere algorithms do not use it, so the unused Rust sphere `MAP_WIDTH` constant was removed. No scaled sphere array, dimension argument, or unsupported-dimension fallback is needed.

The native ownership check followed the assignments of `ScreenWidth` and `ScreenHeight`. `sc2/src/libs/graphics/sdl/sdl_common.c` sets them to 320 and 240 under `USE_RUST_GFX`, while setting `ScreenWidthActual`/`ScreenHeightActual` from the requested output size. The legacy SDL/OpenGL backends also assign 320 and 240. The current Rust backends, `rust/src/graphics/sdl/sdl2.rs` and `opengl.rs`, define logical dimensions as 320x240 and report configuration width/height as actual output dimensions. `gfx_common.rs` allocates screen canvases from those driver-reported logical dimensions. Larger windows and output scaling therefore do not introduce another supported logical width. No artificial 640-wide logical variant was added to this proof. Source inspection establishes the dimension ownership; no window-scale or live rendering experiment was run here.

Vector cases require a finite, positive computed squared norm. They exclude zero norms and overflowing norms, while covering component magnitudes from `1e-150` through `3e150`. Mask locations are nonzero and their squared coordinates must sum to at most `INT_MAX`. The only invalid signed-coordinate extreme pair, `(-32768,-32768)`, is excluded. AA inputs are finite; infinities and NaNs that would lead to undefined C conversions are excluded. Every tested tilt angle, including both signed-int extrema, is converted to double before multiplication and is valid for the original C body.

## Fixture matrix and byte format

`rust/tests/planets_render/sphere_cases.rs` owns the cases, input stream, Rust result assembly, original-C loading, and fixture assertions. Both the pure tests and linked parity route use it. The tests and oracle builder have no fixture writer or capture-mode bypass.

| Case family | Count | Coverage |
| --- | ---: | --- |
| Vector result groups | 40 | Original vector pair plus 13 additional pairs in separate-output, first-operand alias, and second-operand subtraction alias modes. Each group contains multiply, subtract, dot, and normalize results. |
| Full diffuse masks | 16 | Original `(1,0)`, `(17,-23)`, `(1000,-2000)`; both signs of both axes; all sun quadrants; signed extrema with valid squared sums. |
| Full tilt maps | 13 | Original `-45,0,37`; `-180,-90,-1,1,90,179,180,360,INT_MIN,INT_MAX`. |
| AA records | 26 | Original five; exact 0/74/75 bounds; adjacent representable values around 1/74/75; integral/fractional axes; asymmetric fractions; small positive and large finite clamped coordinates. |
| Averaging weight sets | 14 | Original wrapping set; zero; each 65536-only contributor; quarters; unequal normalized weights; truncation boundaries; saturation; multiply/add wrap. Each set exercises channels -1 through 3. |

There are 109 case records. All 90,000 mask cells and 73,125 tilt records are compared, including every inactive point and weight. Tilt maps start with `0xa5` bytes; AA records start with `0x6d` bytes. The initial five unit-test scenarios, including the separate guarded mask `(11,-19)` and tilt `37` with `0xbeef` point sentinels, remain.

The fixture consists of 2,704,382 result bytes, 55 before/after array-or-AA guard records totaling 7,040 bytes, 128 C capture-buffer guard bytes, and the unchanged 3,145-byte input stream. Total: **2,714,695 bytes**. The input stream has one case tag followed by explicit fixed-width fields; its vector alias selector drives the actual C alias calls and equivalent Rust assignments.

| Identity | SHA-256 |
| --- | --- |
| Whole original `plangen.c` | `0b1605af93c4c5a2c4b99200ad03a027255e3e5d8b7479c744c5d1afba153ec0` |
| Extracted original bodies, 7,116 bytes | `df7feecb5c700ca27b71a9ed9ab69cf08e6af89c62b338359eb1f74adf8e3f39` |
| Original 607,765-byte fixture prefix | `b6b8a81f17952218cbb506bb21b9cfc788af1e9af3371a0551c82f91bcfa0044` |
| Extended 2,714,695-byte fixture | `913bedd679cefcce48026df05ef53964474427944417bb37e16ef9fb9008bfbf` |

The pure and linked fixture checks enforce the source, prefix, and full fixture hashes, output length, and every output byte. The oracle builder separately enforces the extracted-body hash.

## Arithmetic and original-C guards

The extension initially exposed 18 differing bytes, all in the `INT_MAX` tilt map. The oracle used Apple clang 17.0.0 (`clang-1700.6.3.2`), targeting `arm64-apple-darwin25.4.0`. Disassembly of its `-O2` output showed `fmadd` for `da += M_DEG2RAD * angle`. It also showed `P3dot` contracting the x product into the separately rounded y product, then contracting the z product into that sum. Rust now uses `mul_add` at those observed sites. No C function body or compiler contraction setting was changed to obtain agreement.

The added cancellation cases distinguish operation order, not just approximate numerical agreement. For all three alias modes, a left-associated cancellation case produces dot bits `3ff0000000000000` (1.0). The x-product contraction discriminator produces `3970000000000000` (2^-104), its swapped x/y counterpart produces `0000000000000000`, and the z-product discriminator produces `3970000000000000`. Non-dyadic, signed-zero, axis, small-magnitude, and large-magnitude vector results are also compared as exact `f64` bits. The focused original-C replay and Rust comparison have zero differing bytes for this source, compiler, target, and fixture matrix.

`tools/sphere-geometry-oracle.py` extracts the original types and body region. Test-only guarded structs own `light_diff` and `map_rotate`; macros alias the original names to their array members. The original 7,116 body bytes remain unchanged in the compiled harness. Every mask, tilt map, and AA record has 16 `0x51a7cafe` words before and after it, initialized before each call, asserted afterward, and serialized into the fixture. The C internal capture buffer has the same guards. Rust also guards the caller's output allocation with 64 `0xcd` bytes on each side and checks the returned C length.

C compile-time assertions check signed `COORD`, `POINT` size and field offsets, `DWORD`, RGBA `Color`, original `MAP3D_POINT` size/offsets, original `POINT3` size/offsets, dimensions, and adjacency of each guarded array/record/buffer. C asserts unchanged non-aliased vector operands and averaging inputs. The complete encoded input buffer is echoed into the output, and the caller separately compares it against its pre-call copy. Rust checks and serializes its corresponding array/AA canaries. No raw Rust layout is assumed for parity.

## Linked RNG route and remaining limits

`rust/tests/planets_render/sphere_geometry.rs` is called by the existing `exported_symbols_reproduce_the_c_fixtures` test in `rust/tests/planets_parity.rs`, alongside oval and topography parity. It adds no concurrent global-state test. Following the topography oracle convention, live C replay is optional in both pure and linked tests. If `UQM_SPHERE_GEOMETRY_ORACLE` is absent, the linked route still compares the complete Rust fixture and checks the actual global state. If supplied, the oracle must load and pass every original-C fixture, guard, input-preservation, and result-length assertion; an invalid path or symbol fails the test. The linked route logs `SphereGeometry: cases=109 bytes=2714695 live_C=true/false` after its checks. The pure replay logs the same coverage fields. Ordinary supported tests need no manually supplied oracle; final acceptance runs supply the freshly built oracle and must report `live_C=true`.

The linked route snapshots the actual `SysGenRNG` pointer and its seed. It snapshots the actual global TFB seed using `TFB_SeedRandom`'s returned previous seed, restores it, records eight consecutive actual `TFB_Random` draws, and restores that seed again. After the optional C replay and independently after the mandatory Rust run, it checks the pointer, context seed, global seed, logical width, and the same eight actual draws. These state checks run even when no live C oracle is supplied. A drop guard restores the saved global state. It never uses an isolated `RandomContext` as a substitute for checking the real TFB stream.

The focused verification recorded here does not build or execute the linked RNG route. Its results belong to the separately retained supported/strict/direct verification records. No toolchain marker is constructed by these test helpers; linked builds must use the supported xtask toolchain/build proof.

The fixture seals the original source hash and the extracted-body hash. A change to `plangen.c` requires investigating and updating the proof rather than treating the sealed fixture as evidence for changed source. Exact floating-point parity is established here for Apple clang 17.0.0 on the recorded arm64 target, with the observed contractions and host math library. It does not establish identical C contraction or transcendental behavior under other compilers, flags, architectures, or math libraries. Production wiring, live game behavior, automation reproduction, window scaling experiments, and real-window screenshots are outside this focused proof.

## Focused verification record

Cargo and clang commands use the shared build environment, with `SDKROOT` and `MACOSX_DEPLOYMENT_TARGET` unset:

```sh
export CARGO_TARGET_DIR=/Volumes/XS1000/acoliver/projects/uqm/tmp/build/current
export CARGO_BUILD_JOBS=4
export TMPDIR=/Volumes/XS1000/acoliver/projects/uqm/tmp/sg
unset SDKROOT MACOSX_DEPLOYMENT_TARGET
```

The original-C oracle build, run from the repository root, exited 0:

```sh
python3 tools/sphere-geometry-oracle.py \
  --output "$CARGO_TARGET_DIR/sphere-geometry-oracle.dylib" \
  --evidence "$PWD/tmp/current-sphere-geometry"
```

Clang emitted two existing header typedef-redefinition warnings. Keep the sphere oracle and the existing `render-topography-oracle.dylib` available in the shared cache for live-C acceptance runs.

The focused command, run from `rust/`, is:

```sh
cargo +1.97.1 test --locked --lib --features audio_heart,debug-process \
  planets::render::sphere_geometry::tests -- --nocapture
```

Both environment variants exited 0 with **5 passed, 0 failed**:

| Oracle environment | Logged coverage | Focused log |
| --- | --- | --- |
| `UQM_SPHERE_GEOMETRY_ORACLE=$CARGO_TARGET_DIR/sphere-geometry-oracle.dylib` | `cases=109 bytes=2714695 live_C=true` | `focused-tests-oracle-present.log` |
| `UQM_SPHERE_GEOMETRY_ORACLE` unset | `cases=109 bytes=2714695 live_C=false` | `focused-tests-oracle-absent.log` |

The env-present run executes original C as well as the separate full Rust fixture comparison. The env-absent run exercises the optional loader branch and retains the full Rust fixture and guard checks. Neither focused run substitutes for executing the linked global-state assertions.

During initial fixture development, a focused run exited 101 from a test stack overflow caused by nesting the full fixture helper inside the standalone large-map guard test. Removing that redundant nested call and borrowing serialized map rows fixed the allocation problem without increasing the stack limit. Temporary case/fixture extraction code was removed before the passing focused runs.

Targeted `rustfmt +1.97.1 --check` for the touched Rust files and `git diff --check` exited 0. Focused commands, exits, oracle/compiler identity, assembly, hashes, and arithmetic evidence are retained under the repository root's `tmp/current-sphere-geometry/`. See `commands.tsv`, `oracle-build.log`, `clang-version.log`, `oracle-assembly.log`, `arithmetic-evidence.log`, `capture-evidence.log`, the focused logs above, and `fmt-check.log`.

## Supported acceptance evidence

Final supported-suite, strict-linked, and direct-binary results are recorded separately in `tmp/current-sphere-geometry/` and the PR after committing. This checkpoint makes no claim that those runs have completed. Live-C acceptance requires both oracle variables:

```sh
export UQM_SPHERE_GEOMETRY_ORACLE="$CARGO_TARGET_DIR/sphere-geometry-oracle.dylib"
export UQM_RENDER_TOPOGRAPHY_ORACLE="$CARGO_TARGET_DIR/render-topography-oracle.dylib"
```

From the repository root, the supported suite entry is `cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- test`; it produces the real linked toolchain/build proof. Retain the oracle environment when running the resulting direct `planets_parity` binary from `rust/`, and require the sphere coverage log to report `live_C=true` in both supported and direct acceptance records. An env-absent `live_C=false` result is valid for ordinary supported testing but does not satisfy live-C acceptance.
