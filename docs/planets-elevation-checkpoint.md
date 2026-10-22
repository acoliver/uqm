# Elevation conditioning and light-map computation have a Rust checkpoint

Issue #265 advances chunk 3 of `tmp/planets-plan/PLAN.md` from PR #264,
commit `6c8bed5e756c77eac16d2d770ab0071ccad3f5c5`. Local and remote parent
heads matched before the child worktree was created. This checkpoint adds
plain Rust implementations of `DitherMap`, `ValidateMap`, `get_vblock_avg`
and `GenerateLightMap` from the actual `sc2/src/uqm/planets/plangen.c`.
Original C remains the runtime provider for that file.

## Preserved behavior

`planets::render::topo_gen` takes explicit complete elevation buffers and
widths. `MAP_HEIGHT` comes from the `planets.h` expression, reusing the
existing Rust `SAFE_Y` owner. Width remains the current SIS screen width,
not a fixed 242-pixel constant. The RNG comes from the existing
`math::RandomContext` owner.

Dithering consumes one random word per four cells, shifts that word by eight
bits between cells, and narrows each elevation adjustment back to a signed
byte. Validation retains the first-row scan, the two wrapping byte counters,
remembered-byte replacement after a counter wraps, side changes only across
differences greater than 128, and the side-one tie rule. Its second pass
updates the previous byte after any replacement.

Lighting retains the signed division truncation, both divisions in the
vertical average, the original negative maximum initializer, the exclusive
vertical upper bound, horizontal wrapping, three spread factors, running
block slots, and signed-byte narrowing. The transform is in place. Later
rows and newly computed blocks must observe earlier writes; using an
immutable source snapshot would change the output. Flat negative and
positive surfaces are levelled to zero. There are no floating-point
expressions in this subset.

The API requires full maps. Dither and validation enforce the original
16-bit loop-count domain; lighting requires width greater than its averaging
radius. The original production caller supplies `MAP_WIDTH` by `MAP_HEIGHT`
for all three operations. This checkpoint does not claim behavior for
mismatched buffer dimensions or original-C inputs that run outside its
storage or loop-count domain.

## Original-C proof

The retained oracle includes the entire unchanged original `plangen.c` in
a test-only translation unit. A dispatch function makes its static routines
callable for the capture. It does not rewrite or extract their algorithms.
Clang builds it with `-O2 -std=gnu99`; dead stripping removes unused graphics
paths. The small Rust request driver compiles the unchanged existing RNG
module and passes its context to the original C routines. No RNG, graphics,
or resource implementation is faked. This harness is outside production and
is not installed as an ABI provider or vtable.

The case matrix has 960 records: five widths (242, 243, 244, 245, 562),
24 explicit input patterns, two seeds, and four operations (dither,
validation, lighting, and their ordered composition). Widths cover every
four-byte RNG tail and first-row byte-counter wrapping. Patterns include
signed-byte extrema, differences of 128 and 129, counter-side changes,
first-row voting, flat surfaces, both sides of light-spread thresholds,
edge impulses, horizontal seams, row boundaries, and mixed elevations.
These are synthetic full maps, not an exhaustive planet-type/seed corpus.

Each fixture row identifies the operation, width, pattern, input seed,
final RNG seed, byte count, and full input/output SHA-256 digests. The
checked-in fixture SHA-256 is
`88badb0e3e0ebce4440af58b6ecdb9cc64fd68009e8390e0ae384052c0db947f`.
The retained original-C output stream has 22,122,240 bytes: 22,118,400
elevation bytes and 960 four-byte RNG states. Its SHA-256 is
`aa7f5f2709ab3c862e0f658a36295b72b47faec5b11c506fa6a357468991db8e`.
The complete requests, including every input byte, are retained separately.

`rust/tests/planets_elevation.rs` always checks every fixture record and its
pinned digest. With `UQM_PLANGEN_ORIGINAL_C_BUFFERS`, it also reads and
compares every retained output byte, checks EOF and the aggregate hash.
With `UQM_PLANGEN_ORIGINAL_C_ORACLE`, it sends each exact input to the
retained original-C executable, compares every returned elevation byte and
RNG seed, and requires EOF and successful process exit. Neither path writes
expected output. Two additional tests check RNG draw count/tail handling and
flat-map/discontinuity behavior independently of the digests.

Original source, compiler dependency headers, unchanged RNG source, test
harnesses, object, executable, raw requests and responses, capture receipts,
command outputs, and exit records are retained under
`/Volumes/XS1000/acoliver/uqm-planets-elevation-ax4mfnrr/`.
The fixture writer was removed before read-only verification. Its source
is retained as a non-executable text artifact. Captures are read-only files.
No Rust output was used to write expected buffers or digests.

Durable evidence must be retained. Only owned build intermediates may be
removed after handoff. Low external capacity required serial reuse of the
previous leaf's owned Cargo target at
`/private/tmp/uqm-planets-next-leaf-oval-_kcuv867-target`; supported xtask
rebuild and dependency capture establish new-worktree provenance.
`storage.json` records that policy. Prior fixtures and sealed manifests are
referenced rather than copied. Before/after targeted hash receipts check
that they remain unchanged, and tested-source receipts bind the commands
to the committed source.

## Runtime boundary and remaining work

There are no production exports, partial vtables, native-input deletions,
new dependencies, C shims, fallback providers, or ownership/gate changes.
The active C-input count remains 286. Existing oval Rust ownership stays
unchanged. The inherited issue #263 exact-count repairs are untouched.

The rest of `plangen.c` remains C: topography color rendering; crater,
storm and gas-giant generation; fourfold topography scaling and its separate
persistent random stream; sphere shadow and tilt maps; antialias sampling;
shield masks, tint and throbbing; sphere pixel rendering; orbit allocation;
and the `GeneratePlanetSurface` orchestration and resource/frame lifecycle.
All of `pl_stuff.c` remains C, including draw/tint calls, rotation state,
frame alternation, zoom timing and cleanup. Those functions need their own
full-buffer, state and ordered-host-effect coverage before a file cutover.

This is headless elevation-buffer proof. It does not establish colored
surface pixels, rotating sphere appearance, frame/resource ownership,
real-window acceptance, or full chunk-3 parity. No game, GUI, image, lane025,
foreign process, stress run, hosted CI action, merge, retarget, or protection
change is part of this checkpoint. Parent/controller PRs remain unmerged.
Review skipped per Andrew 2026-10-09.
