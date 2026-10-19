# Oval rendering has a Rust implementation; C still runs it

This is chunk 3, checkpoint A of `tmp/planets-plan/PLAN.md` in the main
checkout. The parent is PR #256's verified head,
`a9d6f54520e6e8e59ec2fb8ae1dd1cd235562c0e`. Issue #262 tracks this leaf.
The parent has the generation ports through Sol, but no Rust oval renderer.
`oval.c` is the smallest complete rendering leaf in the next planned chunk.

## Behavior and dependencies

`planets::render::oval` implements both functions from `oval.c`. The outline
retains the four quadrant visibility tests, `_clip_line` fallback, midpoint
ellipse regions, unsigned off-step countdown and reverse quadrant submission
order. The filled variant emits bottom and top single-height rectangles in
that order and retains the repeated centre row. Both keep the degenerate
rectangle-to-line branch, signed coordinate narrowing, even-extent asymmetry
and duplicate axis points. They do not consume RNG.

The existing `comm::locdata` point and rectangle types and `starbase::ids`
SIS layout are reused. Header-generated primitives and clipping-line layouts
come from the existing bindgen owner. `CffiOvalGfx` uses the existing C
`DrawBatch`, `DrawLine`, `_clip_line` and foreground-color operations. It does
not export or replace `DrawOval` or `DrawFilledOval`. The battle bridge now
reexports that same header-generated `DrawBatch` declaration instead of
maintaining its incompatible opaque-pointer/int-flags declaration.

All supported targets have a 64-bit C `long`. For signed 16-bit semiaxes,
`2 * A * A * B` fits that type, so the original overflow-reduction loop's
first iteration always succeeds. Rust uses those resulting squared terms
with 64-bit decision arithmetic. It does not introduce a numeric tolerance.

## Proof scope

The linked `planets_parity` target invokes the actual C oval symbols from the
unchanged native archive. Both implementations draw through the same C
clipping and graphics dependencies into RAM drawables owned by the existing
`CapturedSurface`. No video subsystem, window or game is started. Every C
and Rust pixel is compared, including background and alpha. A central case
must change pixels, preventing inactive graphics from passing as empty parity.
The input rectangle and system RNG are checked for preservation.

The bounded case set covers two runtime screen sizes, odd and even extents,
both ellipse regions, thin and degenerate rectangles, negative dimensions,
all viewport edges and corners, wholly invisible and enclosing ovals, signed
coordinate narrowing, four outline cadences and filled rendering. Opaque and
partly transparent foregrounds exercise duplicate axis points and centre
rows. A wrapped-corner input with zero height is excluded because the C
original divides by zero after bypassing its degenerate check; there is no
C result to use as an oracle for that undefined input.

Seven pure recording tests check degenerate calls, point order and duplicates,
continuous dotted cadence, culling and fallback endpoints, paired filled
rows, and coordinate narrowing. Their clipping responses are scripted. They
prove the oval's use of the graphics contract, not the C clipping algorithm.
The linked tests use real clipping. Full-buffer parity and the retained C
fixture establish headless rendering equivalence for this case set, not
real-window appearance or every representable rectangle.

## Original-C fixture provenance

The original-C capture passed the supported full xtask route on this branch
with Rust 1.97.1, locked dependencies and four build jobs. The fixture has
840 draw records from 42 geometries at each of two screen sizes, two
foreground alphas and five draw modes. Each record identifies its inputs,
readback byte count and SHA-256. The live differential comparison covered
533,211,840 RGBA readback bytes. The checked-in fixture is read-only test
input; the temporary capture writers were removed before final verification.

`rust/tests/fixtures/planets/oval.txt` is 101,432 bytes with SHA-256
`bc270ce794ea6cc7fc48df48f96df2500969fcd07b0f0e7b3b5f1d51307fb212`.
The raw original-C buffers are 533,211,840 bytes with SHA-256
`2ef36aee9679faafb011be5e915bb5634637b6bd8ef7a5d1f6cd5091c34cc310`.
They concatenate the full buffers in fixture-record order, so each record's
byte count also defines its raw-buffer offset.

Retained source copies, native archive, capture harness and executable,
raw buffers, command stdout/stderr and exit receipts are under
`/Volumes/XS1000/acoliver/uqm-planets-next-leaf-oval-_kcuv867`.
`capture-receipts.json` and `original-c-source-receipts.json` record absolute
paths, byte lengths and hashes. Capture came only from original linked C;
Rust output was compared against it, never written as expected output.
Disk exhaustion required moving only this checkpoint's build intermediates
to `/private/tmp/uqm-planets-next-leaf-oval-_kcuv867-target`; durable evidence
stays in the external artifact root. `storage-relocation.json` records it.

The first linked attempt exposed a test-setup error: non-alpha drawable
conversion reads `SDL_Screen->format`. The fixture now supplies separate
1x1 RGB and RGBA RAM format surfaces, as existing `CapturedSurface` tests do.
The failed run and LLDB stop in `TFB_DisplayFormatAlpha` remain retained.
Neither format surface starts SDL video or creates a window.

## Inherited exact-count test repair

Issue #263 records stale inventory expectations inherited from PR #256.
Earth and Pluto added two required scripts and five captures, but existing
library and xtask tests still expected the prior totals. This checkpoint
raises the exact expectations to 34 required scripts, 38 policy pins,
35 restart-menu scripts and 164 captures. The default-deadline count becomes
24; the approved long-deadline count stays 14. Both new scripts have a
300-second budget, so that exact budget-class count rises by two. Readiness,
presentation-budget, timeout ordering, pin-mutation rejection and per-script
assertions are unchanged. No policy, script, threshold or enforcement was
relaxed.

## Production and live work still pending

The original C source, linked provider, native-input inventory, ownership
bundle, gates and workflow remain unchanged. There is no runtime cutover,
merge, main-branch write, hosted workflow action or lane025 operation here.
No screenshot or scenario is accepted by these tests. Formal review is
skipped per Andrew's instruction.

The next production checkpoint must install the Rust oval entry points,
remove the original C provider and update ownership artifacts without
relaxing gates. That checkpoint still needs unchanged real-window orbit and
star-map oval coverage when the GUI lane is available. The rest of chunk 3,
`plangen.c` and `pl_stuff.c`, remains separate rendering work. Pending merge
authorization does not prevent this implementation checkpoint and does not
authorize any merge.
