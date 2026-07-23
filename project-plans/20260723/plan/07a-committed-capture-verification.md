# Phase 07a: Verify Presentation and Capture

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P07.VERIFY`

Require P06 marker/P07 evidence. Independently run `automation-present-boundary`, input harness regression, `nm -A`, production build, focused graphics/capture tests, and all strict gates.

FAIL if:

- “success” means more than normal `Canvas::present()` return or observer runs before it;
- skip/no-redraw can increment or capture;
- real production swap/flush symbols are not linked/tested or C implementation is copied;
- SDL metadata/MUSTLOCK relies on the partial handwritten `SDL_Surface` layout rather than `sdl2::sys`/linked C accessors;
- no production-linked real lock-required surface or linked forced-lock-failure/no-read test exists, or any exit omits real unlock;
- raw copy ignores pitch/masks/checked lengths;
- harness double-calls `Init_DrawCommandQueue`, does not use proven dummy+hidden software SDL, or silently fakes an unsupported setup;
- graphics/SDL/file wait or I/O overlaps runtime mutex, or runtime and ordered-I/O mutex nest;
- full extern panic can cross C, inactive automation subcall allocates/works, or ABI/active counters conflate;
- generation is zero/stale/duplicate/unvalidated or capture record/advance precedes exclusive publication, sync/close/directory classification and ordered trace;
- metadata/window claims or 320x240 decode are wrong;
- current `rust_gfx_postprocess` user edit was overwritten rather than integrated.

Mutation checks move observer before present, make skip-swap notify, bypass lock, omit unlock on error, and ignore sync failure; each must fail.

On PASS emit `Phase 07: PASS`, update tracker, create `.completed/P07.md` with production symbol origins, lock/durability/error evidence, command exits, and preservation. Otherwise no marker.
