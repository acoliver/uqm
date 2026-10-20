# RenderTopography has an unwired Rust checkpoint

Issue #269 continues PR #268 from `5e687c619` in the existing terrain
worktree. `render::topography::render_topography` implements the complete
`plangen.c` leaf. `render::c_api::render_system_topography` reads the current
solar-system resources and uses the existing context renderer. Neither is
installed as a C export. `GeneratePlanetSurface` still calls C, and the active
C-input count remains 286.

## Behavior and ownership

The input is a tightly packed row-major array of signed elevations. This
function does not wrap coordinates, resample elevations, apply an albedo or
lighting model, or infer a source pitch. Those operations belong elsewhere.
Destination pitch, bounds and draw-mode effects remain with the existing
RAM-frame renderer. The leaf renders both the ordinary map and the supplied
4× map without changing their layout.

Rocky elevations add the planet's signed 16-bit base, narrow to signed `SIZE`,
then clamp to 0..255. Narrowing before clamping matters at the signed extrema.
Gas elevations reinterpret the signed byte as unsigned and ignore the base.
The algorithm uses the existing planet type, with the original 0x0c mask.
The host selects the existing `PLANET_ARRAY` entry after clearing
`PLANET_SHIELDED` from the orbital descriptor's index.

`XLAT_DESC` contains three `SIZE` levels followed by 256 translation bytes.
The levels are skipped here. The palette is a two-byte header followed by RGB
triples. Translation minus the first header byte selects a triple; the second
header byte does not bound the lookup in C. Each channel is shifted right once,
then expanded with `CC5TO8`, including unsigned-byte narrowing for inputs above
the normal palette range. Alpha is 255. `BUILD_COLOR` discards its old VGA
palette-index argument, so the Rust point carries RGBA rather than a second
palette-index channel.

Points fill slots 63 down to 0 in each 64-slot batch. DrawBatch follows links
in ascending slot order, including the final partial batch's nonzero start.
Batch state continues across rows. The host supplies POINT_PRIM, next links,
END_OF_LIST and zero batch flags to the existing DrawBatch.

The frame is selected before saving origin and effective clipping. Setting
origin changes the destination frame's hotspot. The leaf resets origin and
clip for drawing, restores clip and origin, then restores the previous frame.
It leaves the current context, foreground color and draw mode alone. Missing
translation still selects/restores the frame but does not inspect elevations,
palette, orbital descriptor or dimensions. Frames and resource handles remain
borrowed; the adapter captures, frees and transfers none of them.

Positive dimensions must fit signed COORD. Nonpositive dimensions produce no
points. Source storage may contain unused trailing bytes. Invalid dimensions,
short storage, short palette header, palette underflow or missing RGB bytes
return an error before graphics effects. The host also rejects an invalid
planet index or short translation resource before touching graphics. C returns
void and has no defined invalid-storage contract; these error tests are Rust
contract tests, not executions of invalid C requests.

## Original-C and RAM proof

`tools/render-topography-oracle.py` extracts the unchanged complete function
body from retained original `plangen.c` and compiles a disposable native oracle.
Its operation table injects the linked test's real context functions,
GetColorMapAddress and DrawBatch. The batch observer records active primitive
fields and then calls real DrawBatch. No graphics effect or algorithm is
implemented by the oracle harness. No new production C, shim, dependency,
provider change or ownership exception is added.

The 108-record, 6,616,219-byte regression fixture stores full RGBA readback
for both destination and previous frame, effective clip state, and the full
ordered active primitive and context-operation trace. It contains 1,582,752
state/pixel bytes, 5,032,603 trace bytes and record-length prefixes. Its SHA-256
is `8ca6a84e393a64f0b390a81e43d6b992f619908841549ae1822766c6e7529496`. Each original-C record is compared directly with
the Rust core using the same RAM renderer. The resource adapter independently
compares full pixel/state output for every case using the unchanged planet
table. Custom base-extrema cases use a caller-owned table only in the C/core
comparison. Inputs, palette bytes and translation bytes remain unchanged.
System RNG state and consecutive independent global RNG draws are checked.

The matrix covers every signed byte for all 59 planet entries, shield-bit
selection, missing translation, mixed signed-byte maps, base extrema,
forward/reverse/nonzero-offset
translation, high channel values, zero/negative extents, full and partial
64-point batches, cross-row tails, cropped destinations, 242×75 and 968×300
maps, undefined and explicit clip state, nonzero hotspots, and replace,
additive, subtractive and alpha draw modes. Small unit recorders prove only
core dependency calls and validation. Their comments explicitly exclude pixel,
resource and real-context proof; the linked suite supplies those effects.

This proof uses synthetic valid resource records and the actual resource
getters. It does not prove resource loading, planet-generation orchestration,
rotating spheres, screen presentation or real-window acceptance. The original-C
comparison is scoped to the current arm64 Clang build and its signed narrowing
behavior. The final source contains no fixture writer or capture bypass.

## Verification and retention

Use Rust 1.97.1 with `--locked`, four build jobs, and unset SDKROOT and
MACOSX_DEPLOYMENT_TARGET. All task paths stay beneath the UQM root. The only
Cargo target is `tmp/build/current`; native dependency/toolchain handoff comes
from the supported xtask capture route. Final validation runs the supported
xtask test route, strict workspace/all-target pure-feature Clippy with warnings
denied, formatting and workflow checks, followed by the newly built linked
parity binary directly from `rust/`. Commit precedes HEAD-dependent inventories.
Only the new fixture and changed files are hashed for tested-byte identity.

The evidence directory is `tmp/current-render-topography/`. It holds small
current logs and the generated oracle source, not another source checkout or
binary archive. The oracle binary is disposable in the shared cache. The
existing retained oval pixel bank is read in place and never copied. Historical
proofs relocated to `tmp/retained-proof/` are referenced only where their files
still exist; deleted binaries and old worktrees are not reconstructed.

Remaining work includes sphere geometry/lighting/rotation, shield rendering,
persistent variance and TopoScale4x, planet-orbit frame/resource lifecycle,
GeneratePlanetSurface orchestration, and `pl_stuff.c`. Those require separate
parity and eventual complete runtime cutover. No game, window, image inspection,
GUI/lane025, foreign worker, stress lane, hosted CI retry/cancel, merge or
protection change is part of this checkpoint. Review skipped per Andrew
2026-10-09.
