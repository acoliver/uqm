# Graphics Subsystem Requirements

## Document authority

These requirements govern all externally visible obligations of the graphics subsystem. When a conflict exists between these requirements, the specification, current C code behavior, or current Rust code behavior, the precedence defined in the specification's "Document authority and precedence" section applies. In summary: requirements take precedence over specification text, which takes precedence over C code (except in explicitly delegated parity areas), which takes precedence over Rust code.

## Ownership vocabulary

This document uses the ownership vocabulary defined in the specification's front matter. When a requirement says "ownership" without qualification, it means resource ownership (allocation and deallocation responsibility). Behavioral ownership, execution ownership, and end-state ownership are stated explicitly where they apply.

## Rendering lifecycle requirements

### REQ-RL-001 Initialization
When the graphics subsystem is initialized, the subsystem shall create and own the rendering backend state required to render UQM frames, including the display-facing presentation context, the logical game screens, and any format-conversion resources needed by externally visible graphics APIs.

### REQ-RL-002 Logical screen model
The graphics subsystem shall expose exactly three logical screens for integration compatibility: main, extra, and transition.

### REQ-RL-003 Logical resolution
The graphics subsystem shall preserve a logical rendering coordinate space of 320 by 240 pixels for all externally visible drawing and presentation behavior.

### REQ-RL-004 Single final present per frame
When a frame is presented, the subsystem shall perform exactly one final display present operation per frame. The presentation sequence shall consist of preprocess, screen-layer compositing, fade/color compositing when active, optional postprocessing effects, and final present. Internal implementation steps (such as multiple off-screen compositing passes, intermediate texture uploads, or multi-step rendering within a single frame) do not violate this requirement provided only one present call makes the result visible to the display.

### REQ-RL-005 Main-screen visibility
When a frame is presented, the subsystem shall composite the main screen as the primary visible game image.

### REQ-RL-006 Transition overlay visibility
When a transition effect is active, the subsystem shall composite the transition screen according to the active transition amount and clip region.

### REQ-RL-007 Extra-screen visibility
The subsystem shall not composite the extra screen to the display unless a separately defined externally visible integration contract requires it.

### REQ-RL-008 Fade redraw continuity
When no draw commands are pending and a fade or transition is still active, the subsystem shall continue presentation updates sufficient to advance the visible effect.

### REQ-RL-009 Idle behavior
When no draw commands are pending and no visible fade or transition update is required, the subsystem shall return from the flush/present cycle without altering visible output.

### REQ-RL-010 Shutdown
When the graphics subsystem is shut down, the subsystem shall release all graphics resources that it owns in an order that preserves backend validity during teardown.

### REQ-RL-011 Reinitialization
When the video subsystem is reinitialized through the graphics interface, the subsystem shall either restore a valid rendering backend configuration or report failure through the established external error path. On failure, the subsystem shall attempt to revert to the prior configuration. If reversion also fails, the subsystem shall terminate the process, consistent with the existing C backend's irrecoverable-failure behavior.

### REQ-RL-012 System-box visibility through fades
When the system box is active during presentation, the subsystem shall re-composite the designated main-screen subregion after fade overlay and before final present, preserving the visible behavior of keeping the system UI readable through fades and transitions.

## Draw queue requirements

### REQ-DQ-001 Single drawing ingress
All externally visible drawing work submitted for deferred execution shall enter the rendering pipeline through the draw-command queue. Operations that are not deferred render mutations (such as immediate read-only surface access, lifecycle calls, or transitional direct writes by C during migration) are not subject to this requirement; see the specification's operation category table for the full boundary definition.

### REQ-DQ-002 FIFO ordering
When draw commands are flushed, the subsystem shall execute commands in first-in, first-out order.

### REQ-DQ-003 Batch visibility
When draw batching is active, the subsystem shall defer visibility of batched commands until the corresponding batch scope is exited.

### REQ-DQ-004 Nested batching
When nested batching scopes are used, the subsystem shall not expose queued commands for execution until the outermost active batch scope is exited.

### REQ-DQ-005 Screen targeting
When a draw command is enqueued, the subsystem shall preserve the command's targeted logical destination screen through flush execution.

### REQ-DQ-006 Flush completion signal
When a flush cycle completes, the subsystem shall notify waiting integration code through the established synchronization mechanism used by UQM rendering.

### REQ-DQ-007 Empty-queue fade handling
When the queue is empty and a visible fade or transition update is required, the subsystem shall execute the presentation path without requiring a synthetic draw command.

### REQ-DQ-008 Queue backpressure
When producer activity would otherwise prevent flush completion, the subsystem shall apply queue backpressure or equivalent throttling sufficient to guarantee forward progress.

### REQ-DQ-009 Callback execution
When a callback command is flushed, the subsystem shall invoke the callback on the graphics/rendering thread, in FIFO order relative to all other queued commands.

### REQ-DQ-010 Deferred destruction
When image or data destruction is submitted through the draw queue, the subsystem shall defer destruction until the corresponding command reaches flush execution.

### REQ-DQ-011 Screen copy operations
When copy commands are submitted between logical screens or between a screen and an image, the subsystem shall preserve source selection, destination selection, clipping, and ordering semantics visible to UQM.

### REQ-DQ-012 Bounding update tracking
When visible drawing modifies the main screen, the subsystem shall track the modified region as a bounding box that is a correct superset of all main-screen pixels modified during the flush cycle. If no external integration code consumes draw-damage information, the bounding box may be treated as an internal optimization. After each flush cycle, the tracked region shall be reset.

### REQ-DQ-013 Signal delivery ordering
When a signal command is flushed, the subsystem shall deliver the signal only after all prior queued commands affecting visibility have been executed.

## Canvas, image, and font requirements

### REQ-CAN-001 Opaque canvas handles
The subsystem shall treat canvases exposed through the external graphics interface as opaque handles rather than requiring callers to manipulate backend-native canvas internals.

### REQ-CAN-002 Canvas extent fidelity
When a canvas is created or wrapped, the subsystem shall preserve its width, height, and pixel-format properties as required by externally visible drawing behavior.

### REQ-CAN-003 Canvas clipping
When a clipping or scissor region is active on a canvas, the subsystem shall clip subsequent drawing operations to that region.

### REQ-CAN-004 Coordinate clipping
When drawing coordinates extend outside canvas bounds, the subsystem shall clip the operation to the valid canvas extent rather than requiring callers to preclip.

### REQ-CAN-005 Primitive support
The subsystem shall support canvas operations sufficient to implement UQM's existing line, rectangle, fill, copy, image-blit, and font-glyph rendering behavior.

### REQ-CAN-006 Surface-backed canvas coherence
When a canvas wraps a backend surface that is later read for presentation, transition capture, or interoperability, the subsystem shall preserve pixel coherence between the canvas view and the underlying surface-visible pixel data. Pixels written through the canvas shall be visible in the surface's pixel memory before the presentation layer composites that surface, before transition-source capture reads that surface, and before any interoperability read that returns current pixel data to external code. The mechanism used to maintain coherence is an implementation choice.

### REQ-IMG-001 Image lifecycle
The subsystem shall provide externally compatible creation, use, cache maintenance, and destruction behavior for image objects used by UQM rendering.

### REQ-IMG-002 Image resource ownership
When an image object holds resource ownership of auxiliary cached or derived image resources, the subsystem shall release those resources when the image is destroyed.

### REQ-IMG-003 Scaling cache reuse
When an image is drawn repeatedly with the same scale parameters and the same scaling mode, the subsystem shall be permitted to reuse a cached scaled representation.

### REQ-IMG-004 Scaling cache invalidation
When an image's effective scale parameters, scaling mode, colormap dependency, or other cache-affecting state changes, the subsystem shall invalidate or refresh derived cached image representations before reuse.

### REQ-IMG-005 Mipmap association
When a mipmap or equivalent lower-resolution image is associated with an image, the subsystem shall preserve that association for image-scaling behavior that depends on it.

### REQ-IMG-006 Filled-image behavior
When a filled-image draw is requested, the subsystem shall render the image shape using the requested fill color while preserving the source image's visible coverage semantics.

### REQ-IMG-007 Rotation compatibility
When externally visible APIs request a rotated image object, the subsystem shall return an image object whose rendered orientation matches the requested rotation semantics.

### REQ-IMG-008 Hot-spot compatibility
The subsystem shall preserve externally visible image hot-spot behavior used by positioning and scaling logic.

### REQ-FONT-001 Glyph alpha semantics
When a font glyph is rendered from alpha coverage data, the subsystem shall blend the requested foreground color according to glyph alpha values.

### REQ-FONT-002 Glyph positioning
When a font glyph is drawn, the subsystem shall honor the glyph hot-spot/origin data used by UQM text placement.

### REQ-FONT-003 Glyph metrics compatibility
The subsystem shall preserve externally visible glyph extent and display-advance semantics relied upon by UQM text layout.

### REQ-FONT-004 Backing-image composition
When the text-rendering path supplies glyph backing imagery, the subsystem shall composite the backing image first, then blend the glyph alpha map on top, preserving the externally visible compositing order. Clipping shall apply identically to both the backing-image draw and the glyph draw.

## Colormap and fade requirements

### REQ-CMAP-001 Colormap capacity
The subsystem shall support the externally visible colormap capacity required by UQM, including at least 250 concurrently addressable colormap slots.

### REQ-CMAP-002 Colormap shape
Each externally visible colormap shall represent 256 color entries with RGB channel data in the format expected by existing UQM integrations.

### REQ-CMAP-003 Colormap indexing
When a caller selects or requests a colormap by index, the subsystem shall preserve that index-based lookup behavior.

### REQ-CMAP-004 Colormap versioning
When colormap contents change, the subsystem shall advance a colormap change indicator sufficient for dependent cached image state to detect staleness.

### REQ-CMAP-005 Colormap lifetime
When a colormap is shared or retained by multiple graphics objects, the subsystem shall preserve resource-ownership semantics sufficient to prevent premature release while references remain active.

### REQ-FADE-001 Fade intensity model
The subsystem shall preserve the externally visible fade intensity model in which normal display intensity is represented distinctly from full-black and full-white intensity.

### REQ-FADE-002 Fade progression
When a fade or color transform is active, the subsystem shall provide a mechanism to advance the transform over time or over discrete update steps.

### REQ-FADE-003 Immediate fade completion
When integration code requests immediate completion of active fade or color-transform state, the subsystem shall complete that state without requiring intermediate frames.

### REQ-FADE-004 Fade presentation ordering
When fade presentation is active, the subsystem shall composite fade output after visible screen-layer composition and before final present.

### REQ-FADE-005 Black fade behavior
When the active fade intensity is below normal intensity, the subsystem shall produce a visible darkening effect equivalent to compositing toward black.

### REQ-FADE-006 White fade behavior
When the active fade intensity is above normal intensity, the subsystem shall produce a visible brightening effect equivalent to compositing toward white.

## Scaling and presentation requirements

### REQ-SCAL-001 Logical-to-physical presentation
When presenting to a physical display surface or window, the subsystem shall preserve the logical 320 by 240 coordinate space while mapping presentation to the actual display size.

### REQ-SCAL-002 Presentation scaling support
The subsystem shall support the presentation scaling modes externally selectable through UQM graphics configuration, including nearest/identity presentation and any configured software upscale modes that are part of the integration contract.

### REQ-SCAL-003 Software scaler behavior
When a configured software presentation scaler is active, the subsystem shall apply that scaler to visible screen-layer content before final presentation.

### REQ-SCAL-004 Sprite scaling behavior
When image draw commands request scaled sprite rendering, the subsystem shall preserve externally visible sprite-scaling semantics independently of presentation-time display scaling.

### REQ-SCAL-005 Bilinear/trilinear compatibility
When UQM selects bilinear, trilinear, or equivalent smoothing behavior for image scaling, the subsystem shall preserve the corresponding externally visible sampling behavior.

### REQ-SCAL-006 Scanline effect
When the scanline presentation option is enabled, the subsystem shall apply a scanline-like postprocess effect before final present. The visual result shall be consistent with the C reference backend's scanline behavior; visually equivalent dimming of alternating horizontal lines is acceptable.

### REQ-SCAL-007 Alpha-modulated screen compositing
When a screen layer is composited with partial opacity, the subsystem shall modulate the layer's visible opacity according to the requested alpha value.

### REQ-SCAL-008 Rectangular screen compositing
When a presentation call supplies a clip or destination rectangle for screen compositing, the subsystem shall limit compositing to the externally visible region defined by that rectangle.

### REQ-SCAL-009 Transition presentation semantics
When transition presentation depends on reading current transition-screen pixel data during screen compositing, the subsystem shall produce correct transition compositing output. The mechanism by which transition-screen pixels reach the renderer (separate upload step, per-frame read, or other approach) is an implementation choice, provided the transition-screen content is correctly composited at the specified alpha and clip region each frame.

## Transition capture requirements

### REQ-TRANS-001 Transition-source capture content
When transition source imagery is captured from the main screen, the captured content shall consist of already-flushed main-screen pixel data at the point of capture. Queued draw commands that have not yet been flushed to the main screen's surface pixel memory shall not be included in the capture.

### REQ-TRANS-002 Transition-source capture stability
When transition source imagery has been captured, the captured content on the transition screen shall remain stable for the duration of the transition effect.

### REQ-TRANS-003 Transition capture synchronization
When transition-source capture occurs during mixed C/Rust execution of the graphics subsystem, the capture shall read surface pixel memory that is consistent with all preceding graphics-thread operations. The existing UQM threading model, in which all draw-command dispatch and capture execute on the single graphics thread, provides this serialization. If a future architecture change introduces concurrent surface writers, the subsystem shall provide an explicit serialization mechanism to preserve capture consistency.

## Error handling requirements

### REQ-ERR-001 No unwinding across FFI
When called through an external language boundary, the subsystem shall not permit unwinding or equivalent unchecked exception propagation across that boundary.

### REQ-ERR-002 Initialization guards
When a graphics API is called before initialization or after shutdown, the subsystem shall fail safely using the established return-value or no-op conventions for that API.

### REQ-ERR-003 Null-input safety
When externally supplied pointers or handles are null or invalid, the subsystem shall fail safely rather than dereferencing invalid memory.

### REQ-ERR-004 Return convention compatibility
The subsystem shall preserve the externally visible success, failure, null, and no-op conventions expected by UQM integration code.

### REQ-ERR-005 Partial-initialization cleanup
When initialization fails after partially allocating graphics resources, the subsystem shall release already-owned resources before returning failure.

### REQ-ERR-006 Logging of graphics failures
When a graphics operation fails in a way that affects initialization, resource creation, presentation, or other externally significant behavior, the subsystem shall emit diagnostic logging through UQM's established bridge/logging path.

### REQ-ERR-007 Defensive range checking
When an externally visible API accepts an indexed screen, colormap, or similar bounded identifier, the subsystem shall range-check the value before use.

## Ownership and lifecycle obligations

### REQ-OWN-001 Backend resource ownership
The graphics subsystem shall hold resource ownership of the rendering backend resources that it creates, including display, renderer, screen surfaces, presentation textures or buffers, and scaling buffers.

### REQ-OWN-002 No external free obligation for subsystem-owned resources
When the subsystem returns opaque graphics objects through externally visible creation APIs, integration code shall not be required to free backend-native subresources directly; the subsystem shall provide and honor corresponding graphics-level destruction APIs.

### REQ-OWN-003 Surface access compatibility
When integration code obtains raw screen-surface pointers for compatibility, the subsystem shall ensure that resource ownership of those surfaces remains with the graphics subsystem. In the intended end state, such access is read-only. During migration, write access through the existing C draw path (which retains execution ownership of draw-command dispatch) is permitted; as each C draw path is replaced and execution ownership transfers to Rust, the corresponding write-access permission is retired.

### REQ-OWN-004 Destruction ordering
When the subsystem tears down owned graphics resources, it shall do so in an order that does not invalidate still-dependent resources before they are released.

### REQ-OWN-005 Colormap retention obligations
When image objects or draw operations depend on colormap data, the subsystem shall preserve colormap validity for the duration required by those dependencies.

### REQ-OWN-006 Deferred free ordering
When destruction is deferred through the draw queue, the subsystem shall not release the targeted object before all earlier queued uses of that object have completed.

### REQ-OWN-007 Image synchronization obligations
When externally visible image metadata may be observed concurrently with rendering activity, the subsystem shall preserve the locking or equivalent synchronization guarantees required by the existing ABI contract.

## Integration obligations with the rest of UQM

### REQ-INT-001 Existing API compatibility
The subsystem shall preserve the externally visible behavior required by the existing UQM graphics API surface, including drawing, image, font, colormap, fade, transition, and presentation entry points.

### REQ-INT-002 Backend-vtable compatibility
When UQM selects the graphics backend through the established backend vtable, the subsystem shall provide behaviorally compatible implementations of preprocess, postprocess, screen-layer compositing, color overlay compositing, and any required transition-related hooks.

### REQ-INT-003 FFI symbol compatibility
Where integration depends on named exported FFI entry points, the subsystem shall preserve those externally referenced entry points or provide an ABI-compatible replacement layer.

### REQ-INT-004 Build-flag behavioral responsibility
When the build enables the Rust graphics path, the subsystem shall assume behavioral ownership of all externally visible graphics behavior otherwise provided by the legacy backend path. Compatibility shim layers in C may participate as thin forwarding wrappers, but behavioral ownership of the resulting rendered output rests with the Rust subsystem. During migration, execution ownership of individual domains (draw-command dispatch, canvas operations, etc.) may remain with C code; this does not reduce the Rust subsystem's behavioral accountability for correct final output.

### REQ-INT-005 UIO asset-loading compatibility
When graphics assets are loaded from UQM's UIO-backed resource system, the subsystem shall preserve externally visible asset-loading behavior sufficient for existing game resources to render correctly, including resource-file parsing, frame/animation extraction, and color-key or alpha interpretation where applicable.

### REQ-INT-006 Transition-source compatibility
When UQM saves transition source imagery from the main screen and later uses it for transition rendering, the subsystem shall preserve the visible behavior of that workflow. The captured transition-screen content shall reflect already-flushed main-screen pixels at the time of capture (see REQ-TRANS-001) and shall remain stable for the duration of the transition effect (see REQ-TRANS-002).

### REQ-INT-007 Extra-screen workflow compatibility
When UQM uses the extra screen as an off-screen staging or copy surface, the subsystem shall preserve the visible results of copying to and from that screen.

### REQ-INT-008 Context-driven draw compatibility
When higher-level UQM graphics/context code supplies draw mode, color, clipping, font, scale, or target-screen state, the subsystem shall honor that state in the resulting rendered output.

### REQ-INT-009 Synchronization compatibility
When UQM waits for rendering completion through existing condition-variable, semaphore, or signal-based mechanisms, the subsystem shall preserve those synchronization points.

### REQ-INT-010 Presentation call count
For each completed visible frame, the subsystem shall perform no more than one final display present operation. Internal rendering steps (off-screen compositing, texture uploads, intermediate render passes) are not constrained by this requirement; the contract applies only to the single present call that makes the frame visible to the display.

### REQ-INT-011 Language-agnostic implementation freedom
The subsystem may be implemented in any language or internal architecture, provided that all externally visible behavior and ABI-sensitive contracts required by UQM integration are preserved.

### REQ-INT-012 Asset-loader behavioral parity
When the graphics subsystem takes behavioral ownership of asset loading, the subsystem shall preserve the following loader-visible behaviors of the existing C loading path: accepted resource encodings and container formats, frame ordering and hotspot extraction from animation resources, font glyph extraction semantics, and safe handling of malformed or partial resource data. If specific loader contracts are intentionally defined by code parity rather than prose, the governing interfaces and data structures in the C loading path (`gfxload.c`, `png2sdl.c`, `sdluio.c`, `TFB_Image`, `TFB_Char`, `TFB_Canvas`) are the authoritative reference. This code-parity delegation is expected to narrow as the prose specification gains fuller loader coverage.

---

## Appendix A — Parity risk traceability (non-normative)

This appendix is a **non-normative traceability aid** for implementers. It maps current-state parity risks identified in the initial-state analysis to the requirement IDs that govern them. It does not define requirements, modify their scope, or add obligations beyond what is stated in the normative sections above.

| Parity risk domain | Governing requirements |
|---|---|
| Postprocess / presentation duplication | REQ-RL-004, REQ-INT-002, REQ-INT-010 |
| Transition handling and source capture | REQ-INT-006, REQ-SCAL-009, REQ-RL-006, REQ-TRANS-001, REQ-TRANS-002, REQ-TRANS-003 |
| Raw surface coherence and ownership seam | REQ-OWN-003, REQ-CAN-006, REQ-OWN-001 |
| Queue synchronization and signal ordering | REQ-INT-009, REQ-DQ-006, REQ-DQ-013 |
| Scanline parity with C backend | REQ-SCAL-006 |
| Asset loader compatibility | REQ-INT-005, REQ-INT-012 |
| System-box visibility through fades | REQ-RL-012, REQ-RL-004 |
| Callback execution context | REQ-DQ-009 |
| Reinit failure behavior | REQ-RL-011 |
