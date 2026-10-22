# Planet ovals now use the Rust ABI provider

This production cutover follows checkpoint A, commit
`e45d226e4d519fc6ea911298787f5724c30c84f3`, on issue #262. It is stacked on
PR #256. The algorithm and original-C fixture from that checkpoint are
unchanged. This change installs the two runtime exports and removes
`sc2/src/uqm/planets/oval.c` from the source tree, Makeinfo and active native
input and provider inventories.

## Runtime boundary

`rust/src/planets/render/c_api.rs` exports `DrawOval(RECT *, BYTE)` and
`DrawFilledOval(RECT *)`. Bindgen reads their declarations from the real
`planets.h`. Compile-time function assignments check both signatures;
rectangle size, alignment, corner and extent offsets are checked against the
header layout. The adapter copies rectangle fields before any graphics call.
It holds no reference to caller memory, context memory or a frame across
primitive submission.

Both exports run synchronously on the game thread with the active context
and foreground frame kept alive by their caller. Screen dimensions come from
the header-declared globals and the existing SIS layout. The private adapter
captures the foreground color for the call. Existing `DrawBatch`, `DrawLine`
and `_clip_line` apply the graphics context's primitive rendering and clipping.
The oval code does not change the context, foreground frame, color, clip,
draw mode, frame hotspot, screen dimensions or RNG.

The algorithm retains its signed 16-bit narrowing, quadrant visibility,
submission order, dotted cadence, duplicate axis points, repeated filled
centre rows and degenerate line branch. Decision arithmetic remains signed
64-bit, matching C `long` on the supported native targets. The original's
undefined zero-semiaxis divide after a wrapped corner remains outside the
parity claim. There is no new C, shim, provider fallback or renderer rewrite.

## Tests and retained oracle

The linked `planets_parity` test now calls the exported ABI, rather than
comparing a Rust helper with a still-linked C oval. It requires the complete
840-record original-C fixture to match, including each full-buffer SHA-256,
input identity and byte count. It retains a changed-pixel assertion and the
rectangle and RNG preservation checks. It also checks context identity,
foreground frame and color, clip rectangle, draw mode, frame hotspot and
screen dimensions before and after every call.

Setting `UQM_OVAL_ORIGINAL_C_PIXELS` to the retained raw capture adds a
streaming, byte-for-byte comparison of all 533,211,840 bytes. That path is
read-only. The test requires exact EOF and the original aggregate hash.
Only one expected buffer is read at a time. Standard tests always enforce
the immutable per-buffer digest fixture, whether or not the external raw
capture is supplied.

The fixture remains SHA-256
`bc270ce794ea6cc7fc48df48f96df2500969fcd07b0f0e7b3b5f1d51307fb212`.
The raw capture remains SHA-256
`2ef36aee9679faafb011be5e915bb5634637b6bd8ef7a5d1f6cd5091c34cc310`.
Original C source, archive, capture harness, executable and raw buffers stay
under `/Volumes/XS1000/acoliver/uqm-planets-next-leaf-oval-_kcuv867/`.
No new copy of that raw capture is needed.

## Ownership accounting

The active native input count and ceiling decrease from 287 to 286. The
provider object's identity projection decreases from 304 to 303 entries;
the new projection hash is
`b80f9e594c8aa43f7118ed87b878bde5c38e0bc1e45ccf4c41b71eaf84a1f570`.
All headers used by oval.c remain dependencies of other native inputs.
Historical ownership ledgers and their hashes remain unchanged.

Both oval symbols now have explicit Rust-provider contracts. Mutation tests
reject a missing definition, duplicate Rust definition, C definition or
unresolved executable import for either symbol. The existing ownership
checks still apply to the entire native archive. Gates, trend, exact-count
expectations and evidence fixtures reflect the smaller active inventory.
The native input ceiling decreases by one.

The generated clip-query declaration exposed an inherited starbase declaration
with a missing return type and a separate rectangle mirror. That adapter now
uses the header-generated `GetContextClipRect` and rectangle type, then copies
its fields into the starbase model. No pointer cast or suppression is used.

The inherited issue #263 repairs correspond to the two actual script
additions in `a9d6f54520e6e8e59ec2fb8ae1dd1cd235562c0e`: Earth and Pluto add
two required scripts, two policy pins and five captures. Their deadlines
remain the default class, and both carry a 300-second script budget. No
correction to those count repairs is needed. They do not establish that
either scenario has been played.

## Evidence and limits

Cutover commands, exit receipts, source manifests, prior-artifact hash
receipts and provider observations are retained under
`/Volumes/XS1000/acoliver/uqm-planets-oval-cutover-jvdwxlby/`.
Owned build intermediates reuse
`/private/tmp/uqm-planets-next-leaf-oval-_kcuv867-target`; durable evidence
stays on the external volume. The evidence receipt binds the tested commit
and retained oracle without copying prior snapshots or raw buffers.

This is bounded headless ABI and rendering proof. It does not establish
real-window orbit or star-map appearance, complete graphics parity, or
acceptance of the unplayed Earth and Pluto scenarios. Original OS screenshots
and live GUI acceptance remain pending. No GUI, lane025, stress run, hosted
workflow action or merge is part of this cutover. `plangen.c` and `pl_stuff.c`
remain separate work. Review skipped per Andrew 2026-10-09.
