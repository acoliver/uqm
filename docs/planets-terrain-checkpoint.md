# Craters, storms and gas bands have a Rust checkpoint

Issue #267 continues chunk 3 of `tmp/planets-plan/PLAN.md` as a child of
PR #266 at `3a38f94e78c3748c65b7da7c22f1efafd79045f6`. Local and remote
parent heads matched, and the parent source worktree was clean before work.
This checkpoint adds `make_crater`, `make_storms` and `make_gas_giant` to
`planets::render::topo_gen`. C remains the runtime provider of `plangen.c`.

## Behavior and input contract

The code uses the existing `RandomContext`, signed `CPoint`/`CRect` geometry,
`delta_topography` and `dither_map` owners. `MAP_HEIGHT` still follows
`planets.h` and the existing `SAFE_Y` owner. `RANGE_SHIFT`, `NUM_BAND_COLORS`
and `MAX_STORMS` follow the original `plangen.c` definitions. `compiler.h`
defines `COUNT` as unsigned 16-bit and `SIZE` as signed 16-bit; `gfxlib.h`
defines signed 16-bit coordinates. No second RNG or geometry owner is added.

Craters preserve both midpoint regions, arithmetic right shifts of signed
64-bit decision terms, consumed rim counters, inclusive even-axis edges,
zero-axis behavior, clear-before-add mode and signed-byte wrap on every rim
and floor write. There is no clipping. The rectangle must be nonnegative,
and corner plus twice the half-extent must lie inside the complete map.
Map offsets must fit the original unsigned 16-bit row-index domain.

Storms preserve reverse insertion, both candidate draws on every rejected
attempt, the original four-cell overlap margin against already inset stored
rectangles, all size classes, the small-height correction, the bitmask used
to select the band elevation, and every nested crater/inset in order. Counts
are zero through eight. Maps must admit every candidate: width at least 70
for the current height, with no more than 65535 cells. Rejection is not
capped or replaced with a different placement strategy.

Gas generation takes a complete map, the domain used by its sole production
caller, rather than an arbitrary subrectangle. It preserves uneven-band
error accumulation, boundary jitter, 50 faults before filling each internal
band, signed-byte additions, four cyclic band colors, the last-band draw,
storm selection, storm rejection and final dithering. Band count is one
through `MAP_HEIGHT / 3`; at smaller band heights C divides by zero or reads
outside storage. Every signed 16-bit fault depth is accepted. Invalid
geometry/counts fail before RNG or map mutation. The C routines return void;
success is established by complete responses and a zero oracle exit, with
separate Rust tests for the invalid-input contract.

## Original-C proof

The oracle compiles the complete unchanged `plangen.c` and the unchanged
historical `gentopo.c` from `05feeddf5^` using Clang `-O2 -std=gnu99`.
A test-only dispatch calls their static functions. Dead stripping removes
unused graphics paths. The request driver uses the existing unchanged Rust
RNG module, which already supplies the C runtime's RNG symbols. Gas fault
buffers therefore compare original C DeltaTopography against its Rust
owner too. No algorithm, graphics effect, resource effect or RNG is faked.
No harness is installed as production C or a shim.

The sealed proof contains 1302 cases: 480 craters, 516 storm maps and 306
gas maps. Every response includes the entire map, 32-byte guards on each
side, final caller-owned system RNG state, eight following system draws,
and the next draw of a separately seeded global TFB stream. C receives
`SysGenRNG = &caller_context`; it must not advance the independent global
stream. Rust does not share a context across the two streams either.

Widths are 242, 243, 244, 245, 562 and 873. They exercise each four-cell
RNG tail, widened maps, horizontal fault wrapping, bottom rows and the
largest width fitting the 16-bit map domain. Inputs include zero, -128,
127, -1 and mixed signed-byte maps. Crater cases cover twenty degenerate,
odd, even, wide, tall and almost full-map shapes, both opposite map edges,
both clear modes and signed 16-bit delta extrema. Storm counts cover 0..8
and all 32 size-selection tags. Gas counts cover 1..25 with divisible and
nondivisible heights, all four starting colors, both band directions and
fault depths -32768, -127, -1, 0, 2, 8, 127 and 32767. Current PlanData gas
entries use ten bands and depth two, which are included.

An independent parameter/RNG trace checks its final states against original
C records. It reports 3540 accepted storms, 439 overlap rejections, both
outcomes of both optional insets and 888 small-height corrections. This
trace measures branch coverage only; it never writes pixel expectations.
The input reconstruction initially missed a signed offset, and the new
crater implementation initially failed to consume a rim counter. Focused
checks exposed both; fixes changed Rust only, not C captures. A separate
six-case original-C capture supplies the initially uncovered zero band
color without changing the first sealed 1296 records.

The tests always compare every fixture byte, guard and RNG response, check
input/output digests and EOF, and pin the aggregate fixture digests. With
`UQM_TERRAIN_ORIGINAL_C_BUFFERS` and `UQM_TERRAIN_BAND_ZERO_C_BUFFERS`, they
also read every retained original C byte. With
`UQM_TERRAIN_ORIGINAL_C_ORACLE`, they replay each full request to original
compiled C and require the same fixture response and successful exit.
No fixture writer is present in the final source or active evidence tools.
Capture-writer source is retained only as non-executable text.

Main fixture SHA-256:
`9b396857902a452780d37a8499166340e256b26082302c3286e641aadca438cb`.
Supplemental fixture SHA-256:
`c42875fde1a31949fe833b20132216fe52b0660d8ab0d2739d8929d46a286c9a`.
Full C buffers, requests, parameter records, source/header closure, RNG
source, executable, compiler identities, capture receipts, raw command
outputs and exit receipts are retained at
`/Volumes/XS1000/acoliver/uqm-planets-terrain-ft71xfpc/`.

The proof is bound to the retained arm64 Clang build. Original C contains
negative signed shifts in gas band stepping and fault-line generation and
implementation-defined signed-byte narrowing. Rust spells out wrapping or
multiplication. Matching this compiler's original executable does not claim
portable ISO C semantics for those expressions. Negative geometry, invalid
counts, zero divisors and out-of-buffer C requests are excluded, not
executed as meaningful parity cases. The matrices are synthetic valid maps,
not an exhaustive seed corpus or rendered-planet acceptance.

## Delivery and remaining work

The external disk had insufficient space for a complete tracked-asset
checkout. Only the new failed checkout was removed. The complete child
worktree and unique Cargo target are on the internal disk; `storage.json`
records both paths. Durable evidence remains external. No foreign build,
worktree, system log or prior evidence was removed. Targeted before/after
hashes reference the parent proof instead of copying its large artifacts.
Read-only seals bind original C, fixtures and tested source before and
after validation and commit. The supported xtask route creates its own
native toolchain handoff and dependency evidence; no marker is fabricated.

There are no production exports, provider deletions, C changes, new
dependencies, fallback paths or ownership/gate changes. The active C-input
count remains 286. Parent checkpoint code and fixtures are unchanged.
No functional bug in the parent checkpoint was found.

Still unported from `plangen.c`: `RenderTopography`; P3 vector operations,
`GenerateSphereMask`, `create_aa_points`, channel averaging and
`CreateSphereTiltMap`; `CreateShieldMask`, `shield_level`,
`SetShieldThrobEffect`, `ApplyShieldTint`; map-light/pixel/elevation helpers
and `RenderPlanetSphere`; `planet_orbit_init`; persistent `frandom`,
`TopoVarianceFactor`, `TopoVarianceCalc` and `TopoScale4x`; and
`GeneratePlanetSurface` orchestration, frame/resource ownership and cleanup.
All of `pl_stuff.c` also remains C. The persistent scaling variance stream
needs separate proof over consecutive calls before porting or cutover.

This is an unwired headless checkpoint. It establishes neither colored
surface/sphere pixels nor game behavior or real-window acceptance. No
game, GUI, images, lane025, foreign process, stress run, formal OCR review,
hosted CI retry/cancel, merge, retarget or protection change was performed.
Review skipped per Andrew 2026-10-09.
