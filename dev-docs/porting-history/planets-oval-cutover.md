> Historical porting reference. C paths, ABI providers, build commands and
> checkpoint status below describe the migration at the time of writing.
> For current development, see [the development guide](../README.md).

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
