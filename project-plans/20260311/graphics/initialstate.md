# Graphics subsystem initial state

## Scope and responsibilities

The graphics subsystem still owns both rendering presentation and most 2D image/canvas operations for the game. In C, it is responsible for:

- maintaining global screen state and presentation timing (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfx_common.c:27-40`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:42-57`)
- creating and destroying drawables, frames, canvases, and image wrappers (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/drawable.c:62-92`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:289-317`)
- loading graphics assets and converting them into `TFB_Image` / `TFB_Char` data (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfxload.c:45-109`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfxload.c:111-152`)
- queueing screen draw commands and flushing them onto SDL surfaces (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:26-235`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/dcqueue.c:396-621`)
- presenting the composed frame through an SDL/OpenGL backend (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:275-330`)
- supporting transition and fade presentation paths (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfx_common.c:143-196`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:300-329`)

In the Rust port, that responsibility is split: the SDL presentation backend, scaling path used by presentation, colormap manager, DCQ model, canvas bridge, and some graphics data-model code exist in Rust, but the primary draw-command producers and most concrete pixel operations still remain in C.

## Current C structure

### Core C graphics modules still compiled

The top-level graphics library still compiles the legacy C implementation set, including colormap, context, drawable, draw queue, loaders, font, frame, pixmap, and draw wrappers:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/Makeinfo:5-11`

The SDL graphics subdirectory also still compiles the full C backend set, including `canvas.c`, `pure.c`, `sdl2_pure.c`, `opengl.c`, scaler files, `png2sdl.c`, and `sdluio.c`:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/Makeinfo:1-9`

This means the Rust graphics port is not replacing the whole graphics subtree at build level; it is selectively taking over backend entry points behind `USE_RUST_GFX` while the bulk of C graphics code remains in the binary.

### C draw pipeline remains authoritative for game drawing

Game code still enqueues draw commands through the C `TFB_DrawScreen_*` wrappers in `tfb_draw.c`:

- line/rect/image/font/copy/reinit enqueueing: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:26-235`

Those commands are still executed by the C draw-command queue flush loop in `dcqueue.c`, which dispatches to C canvas/image functions and then presents by calling `TFB_SwapBuffers`:

- empty-queue fade/transition redraw path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/dcqueue.c:330-355`
- command dispatch onto canvases/images: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/dcqueue.c:396-615`
- final presentation call: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/dcqueue.c:621-623`

The concrete pixel work is still done by `canvas.c`, not Rust. Evidence includes:

- primitive drawing and fill/blit implementations: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:73-193`
- image blit/scaling path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:196-273`
- filled-image path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:373-507`
- font-char path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:507-603`
- canvas creation/deletion and screen-format conversion: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:608-870`

### C object/resource ownership remains broad

Drawables and frame/image ownership are still created in C:

- drawable/frame allocation and screen-display creation: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/drawable.c:62-123`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/drawable.c:158-198`

`TFB_Image` lifecycle and scaling cache remain C-owned:

- image creation and screen-format conversion: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:289-317`
- create-for-screen: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:319-340`
- image deletion: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:405-434`
- image scaling cache maintenance: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:436-464`

Asset loading is also still C-owned, including PNG/font ingestion and UIO-backed reads:

- image/frame setup in `gfxload.c`: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfxload.c:45-109`
- font-char extraction in `gfxload.c`: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfxload.c:111-152`
- UIO-based animation/resource file loading start: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfxload.c:154-220`

## Current Rust structure

The Rust graphics module tree is broad and includes both core data types and several FFI bridges:

- module declarations: `/Users/acoliver/projects/uqm/rust/src/graphics/mod.rs:4-20`
- exported public surface of the Rust graphics crate: `/Users/acoliver/projects/uqm/rust/src/graphics/mod.rs:22-73`

Important Rust-side graphics modules present in-tree:

- backend/presentation FFI: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs`
- canvas bridge: `/Users/acoliver/projects/uqm/rust/src/graphics/canvas_ffi.rs`
- colormap bridge: `/Users/acoliver/projects/uqm/rust/src/graphics/cmap_ffi.rs`
- draw-command queue bridge: `/Users/acoliver/projects/uqm/rust/src/graphics/dcq_ffi.rs`
- graphics data/model code: `drawable.rs`, `frame.rs`, `pixmap.rs`, `font.rs`, `context.rs`, `render_context.rs`, `gfx_common.rs`, `dcqueue.rs`, `tfb_draw.rs`, `scaling.rs` under `/Users/acoliver/projects/uqm/rust/src/graphics/`

The Rust crate is built as a static library and links SDL2 plus image/scaling dependencies needed by the graphics subsystem:

- crate type: `/Users/acoliver/projects/uqm/rust/Cargo.toml:6-8`
- SDL2 / OpenGL / image / xBRZ / resize deps: `/Users/acoliver/projects/uqm/rust/Cargo.toml:19-35`

## Build and configuration wiring

`USE_RUST_GFX` is a configured build variable and exported into the make/build environment:

- build vars definition/export: `/Users/acoliver/projects/uqm/sc2/build.vars.in:67-99`

The corresponding preprocessor symbol is injected into generated config headers:

- symbol placeholder in config header template: `/Users/acoliver/projects/uqm/sc2/src/config_unix.h.in:95-96`

At compile time, `sdl_common.c` uses `#ifdef USE_RUST_GFX` to switch the active backend wiring to Rust:

- Rust include gate: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:37-40`
- Rust wrapper/vtable definitions: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:58-92`
- Rust init branch: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:108-141`
- Rust uninit branch: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:187-193`

The scaler build is also partially conditioned for Rust graphics. `scalers.c` skips CPU-platform detection under `USE_RUST_GFX` and hardwires the platform enum instead:

- Rust graphics scaler guard: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/scalers.c:213-279`

Architecture-specific C scaler implementations are compiled but fully guarded out when Rust graphics is enabled, e.g.:

- SSE scaler file disabled under Rust gfx: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/2xscalers_sse.c:19-20`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/2xscalers_sse.c:103-105`

## C↔Rust integration points

### Backend FFI surface

The C header `rust_gfx.h` declares the Rust graphics FFI exported to C:

- init/uninit and surface accessors: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/rust_gfx.h:19-27`
- backend vtable entry points: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/rust_gfx.h:29-34`
- event/fullscreen/gamma/size helpers: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/rust_gfx.h:36-48`
- canvas/cmap/dcq bridge declarations: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/rust_gfx.h:50-125`

The currently wired integration point is the `TFB_GRAPHICS_BACKEND` vtable in `sdl_common.c`, which forwards into Rust:

- wrapper calls: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:60-82`
- `rust_backend` vtable: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:85-91`
- vtable activation in init: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:118-124`

### Shared SDL surface ownership boundary

Rust creates and owns the real `SDL_Surface` objects used as C draw targets:

- raw C SDL surface declarations: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:38-78`
- Rust graphics state storing surface pointers: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:88-117`
- screen-surface creation loop: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:268-302`
- `format_conv_surf` creation: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:304-316`
- Rust-side destruction order: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:363-393`

C then retrieves those pointers and continues drawing directly into them:

- pointer handoff from Rust to C globals: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:126-138`
- screen-canvas getter used by C draw code: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:368-372`

This is the key partial-port boundary: Rust owns SDL initialization and presentation surfaces, but C still owns most writes into `SDL_Screens[]` pixel memory.

### Presentation call sequence boundary

C still controls when presentation happens through `TFB_SwapBuffers`:

- preprocess/screen/color/postprocess call sequence: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:275-330`

Rust implements the corresponding vtable functions:

- preprocess: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:448-463`
- postprocess: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:477-598`
- upload transition screen: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:600-615`
- screen layer: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:617-816`
- color layer: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:818-858`

### Other declared bridges are not yet integrated from C

Rust has implemented canvas, colormap, and DCQ bridge modules and exports:

- module presence: `/Users/acoliver/projects/uqm/rust/src/graphics/mod.rs:17-20`
- canvas bridge exports begin at: `/Users/acoliver/projects/uqm/rust/src/graphics/canvas_ffi.rs:72-125`
- colormap bridge exports begin at: `/Users/acoliver/projects/uqm/rust/src/graphics/cmap_ffi.rs:87-138`
- DCQ bridge exports begin at: `/Users/acoliver/projects/uqm/rust/src/graphics/dcq_ffi.rs:88-145`

However, there are no C call sites for `rust_canvas_*`, `rust_cmap_*`, or `rust_dcq_*` under `sc2/src/**/*.c` at the time of inspection, indicating these bridges are present but not yet wired into the active C graphics path.

## What is already ported

### SDL backend initialization and presentation are Rust-owned

When `USE_RUST_GFX` is enabled, Rust owns:

- SDL initialization/window/canvas/event pump setup: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:196-266`
- creation of the three screen surfaces and format-conversion surface: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:268-316`
- state storage including scaler buffers and fullscreen bookkeeping: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:318-355`
- clean shutdown order for surfaces, canvas, video, SDL context: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:363-393`

### Screen compositing exists in Rust

Unlike earlier stub states, `rust_gfx_screen` is implemented and now composites either unscaled or software-scaled screen layers onto the SDL2 renderer:

- uninitialized/range/extra-screen guards: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:632-647`
- null-surface guard and rect conversion: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:649-658`
- software-scale selection: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:659-663`
- scaled compositing path: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:663-777`
- unscaled compositing path: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:778-816`

### Color-layer fade overlay is Rust-owned

The fade/solid color overlay path is implemented in Rust:

- `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:826-858`

### Software scaling for presentation is partly Rust-owned

Rust allocates per-screen soft-scale buffers during init when non-bilinear scaling is requested:

- scale-buffer allocation: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:335-351`

The actual scale-factor selection and RGBX↔RGBA conversions used by the presentation scaler are in Rust:

- scale-factor helper: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:884-912`
- conversion helpers start at: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:914+`

### Rust-side bridge implementations exist for future ownership transfer

The repository already contains Rust implementations for:

- canvas operations on opaque `SurfaceCanvas` handles: `/Users/acoliver/projects/uqm/rust/src/graphics/canvas_ffi.rs:43-125`, `/Users/acoliver/projects/uqm/rust/src/graphics/canvas_ffi.rs:154-569`
- colormap/fade/xform management: `/Users/acoliver/projects/uqm/rust/src/graphics/cmap_ffi.rs:42-458`
- Rust DCQ enqueue/flush model: `/Users/acoliver/projects/uqm/rust/src/graphics/dcq_ffi.rs:36-600+`

These are real code, not absent placeholders, but they are not yet the active C execution path.

## What remains C-owned

### Draw-command production and execution

The active enqueue and flush path is still entirely C:

- enqueue wrappers: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:26-235`
- queue flush and dispatch: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/dcqueue.c:396-621`

### Pixel primitives, blits, image draws, font draws, and copy paths

The active implementations remain in `canvas.c`:

- primitive drawing: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:73-138`
- blit core: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:140-193`
- image draw path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:196-273`
- filled-image path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:373-507`
- font-char path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:507-603`
- canvas creation and screen-format conversion: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:608-870`

### Drawable, frame, image, and loader lifecycle

Still C-owned:

- drawable/frame creation/destruction: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/drawable.c:62-123`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/drawable.c:158-198`
- `TFB_Image` lifecycle and image-side scaling cache: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:289-464`
- graphics asset/font loading via UIO and SDL canvas loaders: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfxload.c:45-220`

### Transition source setup and orchestration

Transition source copying and transition progression remain controlled in C:

- copy main→transition screen: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfx_common.c:143-155`
- transition orchestration and upload trigger: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfx_common.c:157-196`

### SDL image I/O adapter and loader path

The SDL-side UIO adapter remains C-owned and still compiled through `sdluio.c` via `Makeinfo`:

- build inclusion: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/Makeinfo:1-9`

## Partial-port boundaries, guards, stubs, and fallback behavior

### Boundary 1: Rust backend selected, C drawing retained

The highest-value split is in `TFB_InitGraphics`:

- Rust selected under `USE_RUST_GFX`: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:108-141`
- legacy C backend retained in the `#else` branch: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:142-160`

This is the primary partial-port seam: backend init/presentation is replaced, while most graphics logic is not.

### Boundary 2: Rust owns surfaces, C mutates them

Rust surface creation:

- `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:268-316`

C stores the returned pointers and passes them to all legacy canvas code:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:126-138`
- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl_common.c:368-372`

### Boundary 3: transition upload semantics diverge

C reference backend marks the transition texture dirty:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl2_pure.c:303-311`

Rust backend intentionally turns `uploadTransitionScreen` into a no-op:

- `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:600-615`

This is an evidence-backed behavior split, not just a comment-level difference.

### Boundary 4: postprocess retains fallback upload path in Rust

The Rust file explicitly documents that `postprocess` should eventually be present-only, but still contains upload/scaling/copy logic as a temporary fallback:

- rationale comment: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:465-475`
- fallback upload block: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:478-596`

That is a live partial-port/fallback boundary. `postprocess` is not yet parity-clean with the C backend's presentation contract.

### Boundary 5: extra screen compositing is guarded out in Rust

Rust `rust_gfx_screen` returns early for `TFB_SCREEN_EXTRA`:

- `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:644-647`

The C SDL2 reference backend also treats screen 1 as inactive:

- inactive extra screen at init: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl2_pure.c:167-179`

### Boundary 6: C scaler code partially compiled but bypassed/guarded

Rust graphics bypasses the C scaler platform-selection logic:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/scalers.c:213-279`

Architecture-specific scaler implementations are guarded out completely under Rust gfx:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/2xscalers_sse.c:19-20`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/2xscalers_sse.c:103-105`

### Boundary 7: Rust bridge modules exist but are effectively inactive from C

Canvas/cmap/dcq functions are declared in the C header:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/rust_gfx.h:50-125`

But there are no current C call sites for `rust_canvas_*`, `rust_cmap_*`, or `rust_dcq_*` in `sc2/src/**/*.c`, so these bridges are present as integration points without active adoption.

## Parity gaps

### Postprocess is still architecturally non-parity

In the C SDL2 backend, `postprocess` only applies optional scanlines and presents:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl2_pure.c:456-463`

In Rust, `postprocess` still uploads `surfaces[0]` into a temporary texture, applies scaling logic, copies to the canvas, and then presents:

- `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:478-596`

This means the Rust backend currently duplicates part of the compositing responsibility that the C backend assigns to `screen()`. The file itself marks this as temporary.

### Scanline parity is missing on the Rust path

C SDL2 backend applies scanlines when `TFB_GFXFLAGS_SCANLINES` is set:

- scanline implementation: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl2_pure.c:344-356`
- scanline call in postprocess: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl2_pure.c:456-463`

No equivalent scanline path appears in Rust `rust_gfx_postprocess`, `rust_gfx_screen`, or `rust_gfx_color`; the Rust postprocess ends at `canvas.present()` after its upload block:

- `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:478-598`

### Dirty-rect / cached-texture behavior differs materially

C reference backend tracks per-screen `dirty` and `updated` state and only updates the affected texture region:

- dirty rect prep: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl2_pure.c:358-384`
- transition dirty marking: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl2_pure.c:303-311`
- region upload in unscaled path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl2_pure.c:387-404`
- region upload in scaled path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdl2_pure.c:407-445`

Rust uses temporary streaming textures and full-surface upload paths in both `screen()` and `postprocess()`:

- unscaled full upload in `screen()`: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:778-813`
- scaled upload in `screen()`: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:731-777`
- additional upload in `postprocess()`: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:478-596`

### C graphics ownership is much broader than Rust usage

Although Rust contains implementations for DCQ, canvas, and colormap bridges, the active runtime still uses:

- C enqueue/flush path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:26-235`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/dcqueue.c:396-621`
- C canvas/image code: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/canvas.c:73-870`
- C loaders and drawable/image lifecycle: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/drawable.c:62-198`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfxload.c:45-220`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:289-464`

So the Rust module inventory overstates active Rust ownership if read without integration context.

### Rust canvas bridge likely does not yet round-trip SDL surface pixels

`SurfaceCanvas` stores both a raw `SDL_Surface*` and a Rust `Canvas`, but its creation path initializes a fresh `Canvas::new_rgba(w, h)` rather than mapping existing surface pixels:

- struct fields: `/Users/acoliver/projects/uqm/rust/src/graphics/canvas_ffi.rs:56-66`
- constructor allocating new Rust canvas: `/Users/acoliver/projects/uqm/rust/src/graphics/canvas_ffi.rs:94-115`
- destroy only frees the wrapper box, not an SDL sync step: `/Users/acoliver/projects/uqm/rust/src/graphics/canvas_ffi.rs:127-148`

Given the absence of C call sites, this bridge is not currently authoritative, but if wired as-is it appears to need additional synchronization logic to become a real drop-in replacement for direct SDL surface drawing.

### Rust DCQ bridge includes explicit placeholder behavior

`rust_dcq_push_setpalette()` does not enqueue a dedicated palette command; it enqueues a callback that only logs the request for future wiring:

- `/Users/acoliver/projects/uqm/rust/src/graphics/dcq_ffi.rs:497-525`

This is a concrete stub/placeholder in the partially ported subsystem.

## Notable risks and unknowns

### Risk: double-present / duplicated compositing path remains in Rust backend

`rust_gfx_screen()` now composites layers, but `rust_gfx_postprocess()` still uploads and renders `surfaces[0]` before presenting:

- `screen()` compositing: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:632-816`
- `postprocess()` fallback upload+copy: `/Users/acoliver/projects/uqm/rust/src/graphics/ffi.rs:478-596`

This is the most important current risk. The file comments explicitly say the upload/scaling block is retained only until `ScreenLayer` fully takes over, but the code already has a real `ScreenLayer`. That means the architecture is in a transitional state and may be sensitive to ordering, overdraw, or incorrect final composition depending on runtime paths.

### Risk: inactive Rust bridge code may drift from C semantics

Canvas/cmap/dcq bridges are implemented but unwired from C. Since the authoritative path remains C, these Rust bridges can silently diverge from production behavior. Evidence:

- declarations exist: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/rust_gfx.h:50-125`
- modules exist: `/Users/acoliver/projects/uqm/rust/src/graphics/mod.rs:17-20`
- no current C call sites found for those export prefixes under `sc2/src/**/*.c`

### Risk: graphics asset loading remains tied to C UIO/SDL loader stack

Asset loading still flows through C `gfxload.c` plus SDL/UIO helpers, so a future full Rust ownership transition must either preserve the exact data contracts of `TFB_Image`, `TFB_Char`, and `TFB_Canvas` or replace those call sites end-to-end.

Evidence:

- image/font processing in C: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfxload.c:45-152`
- file parsing and UIO dependency: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/gfxload.c:154-220`
- SDL-side canvas/file loader implementation compiled in C: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/Makeinfo:1-9`

### Unknown: exact runtime use of Rust-side drawable/frame/pixmap/context modules

Rust contains substantial non-FFI graphics model code, but the inspected active C call graph still uses the C implementations for drawables, images, frames, contexts, and loaders. Without additional direct call-site evidence, these Rust modules should be treated as implemented library code, not confirmed active ownership.

### Unknown: build-level linkage of Rust staticlib into the final executable outside `USE_RUST_GFX`

The Rust crate clearly builds as `uqm_rust` staticlib (`/Users/acoliver/projects/uqm/rust/Cargo.toml:6-8`), and C headers/ifdefs show the graphics-side link contract, but the exact top-level link orchestration was not required to establish the graphics subsystem state and was not needed to verify the active split.

## Summary

The graphics subsystem is partially ported in a narrow but important way:

- **Rust-owned today:** SDL backend init/uninit, presentation surfaces, screen/color vtable handlers, presentation-time scaling helpers, and inactive-but-implemented canvas/cmap/dcq bridges.
- **Still C-owned today:** draw-command production and execution, most pixel operations, drawable/image/frame lifecycle, graphics asset loading, transition orchestration, and much of the scaler/canvas support stack.

The dominant present seam is:

1. Rust creates/owns the `SDL_Surface` screens and renderer.
2. C continues drawing into those surfaces and managing most graphics objects.
3. C calls Rust through the backend vtable to present the final frame.

The most significant evidence-backed parity concern is that `rust_gfx_postprocess()` still contains a temporary upload/render fallback even though `rust_gfx_screen()` is implemented, leaving the backend in an explicitly transitional state rather than a clean final architecture.

## Implications for specification and requirements

The following observed-state findings have the most direct impact on what the specification and requirements documents must address:

- **Raw-surface ownership seam:** Rust owns the SDL surfaces but C writes directly into their pixel memory. The specification and requirements must define access rules for this shared boundary, including whether external writes are permitted, transitional, or forbidden in the end state.
- **Inactive bridge drift risk:** The canvas, colormap, and DCQ bridges exist in Rust but have no active C callers. Any specification of these bridges' contracts must account for the possibility that the Rust implementations have diverged from the C semantics they are intended to replace.
- **Transition parity risk:** The Rust backend's no-op `uploadTransitionScreen` and reliance on per-frame full-surface reads in `screen()` is a deliberate design choice, not a gap, but it differs from the C backend's dirty-marking approach. The specification must define the transition contract in terms of observable behavior rather than mechanism.
- **Postprocess duplication risk:** The Rust `postprocess` still contains upload/render logic that overlaps with `screen()`. The specification must clearly define postprocess as present-only (matching the C contract) and treat the current fallback as a transitional exception.
- **Scanline parity gap:** The Rust path does not yet implement scanlines. The requirements must cover scanline behavior as a compatibility obligation.
- **Loader ownership gap:** Asset loading is entirely C-owned. The specification's claim of loader ownership is aspirational; the requirements must define loader behavioral parity contracts sufficient for a future Rust takeover without data-dependent regressions.
