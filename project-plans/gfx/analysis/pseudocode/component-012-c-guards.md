# Pseudocode: Component 012 — C Guard Insertion & Dual-Path Build Verification

Plan ID: `PLAN-20260223-GFX-FULL-PORT`
Requirements: REQ-GUARD-010..180, REQ-COMPAT-010, REQ-COMPAT-030,
              REQ-COMPAT-070

---

## 012A: Guard Insertion Algorithm

> Determines which C files get `USE_RUST_GFX` guards, in what order,
> and with what granularity (whole-file vs selective function).
> Reference: technical.md §9.1–9.6, functional.md §17

```
 1: ALGORITHM insert_guards()
 2:   // --- Phase 1: Validate dependency graph ---
 3:   // CONSTRAINT: A file cannot be guarded if unguarded files depend on it
 4:   //             (REQ-COMPAT-070)
 5:   dep_graph ← build_dependency_graph()                      // technical §9.2
 6:   FOR EACH file IN files_to_guard:
 7:     dependents ← dep_graph.reverse_deps(file)
 8:     FOR EACH dep IN dependents:
 9:       IF dep NOT IN files_to_guard AND dep NOT IN already_guarded THEN
10:         ERROR "Cannot guard {file}: {dep} depends on it and is not guarded"
11:         ABORT
12:       END IF
13:     END FOR
14:   END FOR
15:
16:   // --- Phase 2: Classify files by guard granularity ---
17:   whole_file_guards ← []                                     // entire file body guarded
18:   selective_guards ← []                                      // only some functions guarded
19:
20:   // Level 0 — no dependencies (technical §9.2)              // REQ-GUARD-080, REQ-GUARD-110
21:   whole_file_guards.add("sdl/primitives.c")                  // REQ-GUARD-080
22:   whole_file_guards.add("sdl/hq2x.c")                       // REQ-GUARD-110
23:   whole_file_guards.add("sdl/biadv2x.c")                    // REQ-GUARD-110
24:   whole_file_guards.add("sdl/bilinear2x.c")                 // REQ-GUARD-110
25:   whole_file_guards.add("sdl/nearest2x.c")                  // REQ-GUARD-110
26:   whole_file_guards.add("sdl/triscan2x.c")                  // REQ-GUARD-110
27:   whole_file_guards.add("sdl/2xscalers.c")                  // REQ-GUARD-110
28:   whole_file_guards.add("sdl/rotozoom.c")                   // REQ-GUARD-110
29:
30:   // Level 1 — depends on Level 0                            // REQ-GUARD-070
31:   whole_file_guards.add("sdl/canvas.c")                     // REQ-GUARD-070
32:
33:   // Level 2 — depends on Level 1
34:   whole_file_guards.add("dcqueue.c")                        // REQ-GUARD-010
35:   whole_file_guards.add("cmap.c")                           // REQ-GUARD-100
36:   whole_file_guards.add("context.c")                        // REQ-GUARD-030
37:
38:   // Level 3 — depends on Level 2
39:   whole_file_guards.add("tfb_draw.c")                       // REQ-GUARD-020
40:   whole_file_guards.add("tfb_prim.c")                       // REQ-GUARD-090
41:   whole_file_guards.add("frame.c")                          // REQ-GUARD-040
42:
43:   // Level 4 — depends on Level 3
44:   whole_file_guards.add("drawable.c")                       // REQ-GUARD-060
45:   whole_file_guards.add("pixmap.c")                         // REQ-GUARD-130
46:   whole_file_guards.add("sdl/palette.c")                    // REQ-GUARD-150
47:
48:   // Selective guards (only specific functions)
49:   selective_guards.add("font.c", [                           // REQ-GUARD-050
50:     "font_DrawText",
51:     "font_DrawTracedText",
52:     "TextRect",                                              // may be needed by C code
53:   ])
54:   selective_guards.add("gfx_common.c", [                    // REQ-GUARD-120
55:     "FlushGraphics",
56:     "BatchGraphics",
57:     "UnbatchGraphics",
58:     "SetTransitionSource",
59:     "ScreenTransition",
60:   ])
61:
62:   // --- Phase 3: Apply guards ---
63:   FOR EACH file IN whole_file_guards:
64:     apply_whole_file_guard(file)                             // see 012B
65:   END FOR
66:   FOR EACH (file, functions) IN selective_guards:
67:     apply_selective_guard(file, functions)                   // see 012C
68:   END FOR
69:
70:   // --- Phase 4: Verify header files ---                    // REQ-GUARD-180
71:   FOR EACH header IN affected_headers:
72:     ASSERT no_type_guards(header)                            // types always available
73:     // Function declarations may be guarded if needed
74:   END FOR
75: END ALGORITHM
```

## 012B: Whole-File Guard Pattern

> Applied to C files where the entire implementation is replaced by Rust.
> Reference: technical.md §9.1

```
 1: FUNCTION apply_whole_file_guard(filepath: &str)
 2:   content ← read_file(filepath)
 3:
 4:   // --- Validate file has no external dependents that aren't also guarded ---
 5:   // (Already verified in Phase 1)
 6:
 7:   // --- Locate insertion point ---
 8:   // Insert after all #include directives and any file-level comments
 9:   insert_pos ← find_last_include(content) + 1
10:
11:   // --- Insert guard ---
12:   // Pattern:                                                // technical §9.1
13:   //   #include "..."
14:   //   #ifndef USE_RUST_GFX
15:   //
16:   //   ... entire original C implementation ...
17:   //
18:   //   #endif /* USE_RUST_GFX */
19:
20:   modified ← content[0..insert_pos]
21:     + "\n#ifndef USE_RUST_GFX\n\n"
22:     + content[insert_pos..]
23:     + "\n#endif /* USE_RUST_GFX */\n"
24:
25:   write_file(filepath, modified)
26:
27:   // --- Post-condition: file compiles to empty translation unit ---
28:   //     when USE_RUST_GFX is defined                       // REQ-GUARD-160
29: END FUNCTION
```

## 012C: Selective Guard Pattern

> Applied to C files where only some functions are replaced.
> Non-replaced functions remain available to all C code.
> Reference: technical.md §9.1

```
 1: FUNCTION apply_selective_guard(filepath: &str, functions: [&str])
 2:   content ← read_file(filepath)
 3:
 4:   FOR EACH func_name IN functions:
 5:     // --- Locate function boundaries ---
 6:     func_start ← find_function_start(content, func_name)
 7:     func_end ← find_function_end(content, func_start)
 8:
 9:     IF func_start IS None OR func_end IS None THEN
10:       ERROR "Function {func_name} not found in {filepath}"
11:       CONTINUE
12:     END IF
13:
14:     // --- Wrap individual function ---
15:     // Pattern:
16:     //   #ifndef USE_RUST_GFX
17:     //   void replaced_function(void) {
18:     //       // original implementation
19:     //   }
20:     //   #endif
21:
22:     content ← content[0..func_start]
23:       + "#ifndef USE_RUST_GFX\n"
24:       + content[func_start..func_end]
25:       + "#endif /* USE_RUST_GFX */\n"
26:       + content[func_end..]
27:   END FOR
28:
29:   write_file(filepath, content)
30:
31:   // --- Post-condition: guarded functions excluded, others remain ---
32: END FUNCTION
```

## 012D: Dual-Path Build Verification

> Ensures both build paths (USE_RUST_GFX defined and undefined) compile
> and link successfully.
> Reference: REQ-COMPAT-030, REQ-GUARD-170

```
 1: ALGORITHM verify_dual_path_build()
 2:   // --- Step 1: Build WITHOUT USE_RUST_GFX ---              // REQ-COMPAT-030
 3:   result_c ← build(flags: [])
 4:   IF result_c IS FAILURE THEN
 5:     ERROR "C-only build failed — original code broken by guard insertion"
 6:     REPORT result_c.errors
 7:     ABORT
 8:   END IF
 9:
10:   // --- Step 2: Build WITH USE_RUST_GFX ---
11:   result_rust ← build(flags: ["USE_RUST_GFX"])
12:   IF result_rust IS FAILURE THEN
13:     // --- Diagnose: link errors indicate missing FFI exports ---
14:     FOR EACH error IN result_rust.errors:
15:       IF error.type == UNDEFINED_SYMBOL THEN
16:         REPORT "Missing FFI export: {error.symbol}"         // REQ-GUARD-170
17:       ELSE IF error.type == DUPLICATE_SYMBOL THEN
18:         REPORT "Duplicate symbol: {error.symbol} — guard missing in C file"
19:       ELSE
20:         REPORT "Build error: {error}"
21:       END IF
22:     END FOR
23:     ABORT
24:   END IF
25:
26:   // --- Step 3: Verify empty translation units ---          // REQ-GUARD-160
27:   FOR EACH file IN whole_file_guards:
28:     obj_size ← get_object_size(file, flags: ["USE_RUST_GFX"])
29:     // Empty translation units should produce minimal .o files
30:     // (only debug info, no code sections)
31:     code_size ← get_text_section_size(obj_size)
32:     IF code_size > 0 THEN
33:       WARN "File {file} has residual code when guarded: {code_size} bytes"
34:     END IF
35:   END FOR
36:
37:   // --- Step 4: Verify Rust library exports ---             // REQ-GUARD-170
38:   rust_symbols ← nm_list_exports("libuqm_rust.a")
39:   required_symbols ← collect_guarded_function_names()
40:
41:   FOR EACH sym IN required_symbols:
42:     // Account for name mangling: Rust exports use #[no_mangle]
43:     // so symbol names match C exactly (or have rust_ prefix)
44:     rust_name ← IF sym.starts_with("TFB_") THEN
45:       "rust_" + sym.to_lowercase()                           // convention from technical §8.2.1
46:     ELSE
47:       sym                                                     // direct match
48:     END IF
49:
50:     IF rust_name NOT IN rust_symbols AND sym NOT IN rust_symbols THEN
51:       ERROR "Missing Rust export for guarded C function: {sym}"
52:     END IF
53:   END FOR
54:
55:   // --- Step 5: Runtime smoke test (if available) ---
56:   // Run game with USE_RUST_GFX and verify:
57:   //   (a) Window appears
58:   //   (b) No crash within 5 seconds
59:   //   (c) At least one non-black frame rendered
60:   // This is a manual verification step, not automated.
61:
62:   REPORT "Dual-path build verification PASSED"
63: END ALGORITHM
```

## 012E: Guard Application Order Per Level

> The precise order in which guards must be applied to avoid
> intermediate build failures.
> Reference: technical.md §9.2

```
 1: ALGORITHM apply_guards_in_order()
 2:   // --- Level 0: Independent files (no reverse deps) ---
 3:   // Can be guarded in any order within this level.
 4:   // All Level 0 must complete before Level 1.
 5:   GUARD "sdl/primitives.c"                                  // REQ-GUARD-080
 6:   GUARD "sdl/hq2x.c"                                       // REQ-GUARD-110
 7:   GUARD "sdl/biadv2x.c"                                    // REQ-GUARD-110
 8:   GUARD "sdl/bilinear2x.c"                                 // REQ-GUARD-110
 9:   GUARD "sdl/nearest2x.c"                                  // REQ-GUARD-110
10:   GUARD "sdl/triscan2x.c"                                  // REQ-GUARD-110
11:   GUARD "sdl/2xscalers.c"                                  // REQ-GUARD-110
12:   GUARD "sdl/rotozoom.c"                                    // REQ-GUARD-110
13:   VERIFY build(["USE_RUST_GFX"])                             // checkpoint
14:
15:   // --- Level 1: Depends on Level 0 ---
16:   GUARD "sdl/canvas.c"                                      // REQ-GUARD-070
17:   VERIFY build(["USE_RUST_GFX"])                             // checkpoint
18:
19:   // --- Level 2: Depends on Level 1 ---
20:   // dcqueue.c + tfb_draw.c MUST be guarded together         // technical §9.4
21:   GUARD "dcqueue.c"                                          // REQ-GUARD-010
22:   GUARD "tfb_draw.c"                                        // REQ-GUARD-020
23:   GUARD "cmap.c"                                            // REQ-GUARD-100
24:   GUARD "context.c"                                         // REQ-GUARD-030
25:   VERIFY build(["USE_RUST_GFX"])                             // checkpoint
26:
27:   // --- Level 3: Depends on Level 2 ---
28:   GUARD "tfb_prim.c"                                        // REQ-GUARD-090
29:   GUARD "frame.c"                                           // REQ-GUARD-040
30:   SELECTIVE_GUARD "gfx_common.c" [functions]                // REQ-GUARD-120
31:   VERIFY build(["USE_RUST_GFX"])                             // checkpoint
32:
33:   // --- Level 4: Depends on Level 3 ---
34:   // drawable.c + canvas.c SHOULD be guarded together         // technical §9.4
35:   GUARD "drawable.c"                                         // REQ-GUARD-060
36:   SELECTIVE_GUARD "font.c" [drawing functions]               // REQ-GUARD-050
37:   GUARD "pixmap.c"                                          // REQ-GUARD-130
38:   GUARD "sdl/palette.c"                                     // REQ-GUARD-150
39:   VERIFY build(["USE_RUST_GFX"])                             // checkpoint
40:
41:   // --- Final verification ---
42:   verify_dual_path_build()                                   // algorithm 012D
43: END ALGORITHM
```

## 012F: Cross-Dependency Safety Checks

> Files with bidirectional dependencies must be guarded simultaneously.
> Reference: technical.md §9.4

```
 1: CONST SIMULTANEOUS_GUARD_GROUPS: [[&str]] = [
 2:   // Group 1: DCQ + enqueue (bidirectional)
 3:   ["dcqueue.c", "tfb_draw.c"],                              // technical §9.4
 4:
 5:   // Group 2: Canvas + DCQ dispatch
 6:   ["sdl/canvas.c", "dcqueue.c"],                            // technical §9.4
 7:
 8:   // Group 3: Context + Frame
 9:   ["context.c", "frame.c"],                                  // technical §9.4
10:
11:   // Group 4: Drawable + Canvas
12:   ["drawable.c", "sdl/canvas.c"],                            // technical §9.4
13: ]
14:
15: FUNCTION validate_simultaneous_guards()
16:   FOR EACH group IN SIMULTANEOUS_GUARD_GROUPS:
17:     all_guarded ← TRUE
18:     any_guarded ← FALSE
19:     FOR EACH file IN group:
20:       IF is_guarded(file) THEN
21:         any_guarded ← TRUE
22:       ELSE
23:         all_guarded ← FALSE
24:       END IF
25:     END FOR
26:
27:     IF any_guarded AND NOT all_guarded THEN
28:       ERROR "Partial guard in simultaneous group: {group}"
29:       // All files in the group MUST be guarded together
30:       REPORT "Guarded: {guarded_files}, Missing: {unguarded_files}"
31:     END IF
32:   END FOR
33: END FUNCTION
```

### Validation Points
- 012A line 6–14: Dependency graph validation before any guards applied
- 012B line 9: Guard insertion after includes (not before)
- 012C line 9–12: Function not found → error with filename
- 012D line 4–8: C-only build must still pass after guards
- 012D line 14–16: Link errors specifically diagnosed as missing exports
- 012D line 32–34: Empty translation unit verification
- 012D line 50–52: Every guarded C function has a Rust export
- 012E line 13, 17, 25, 31, 39: Build checkpoint after each level
- 012F line 27–31: Partial guard detection in cross-dep groups

### Error Handling
- Dependency violation: ABORT — cannot proceed with partial guards
- Missing function in selective guard: ERROR + continue (non-fatal)
- Build failure without USE_RUST_GFX: ABORT — original code broken
- Build failure with USE_RUST_GFX: diagnose missing exports
- Residual code in guarded file: WARN (may be valid for selective guards)

### Ordering Constraints
- Level 0 BEFORE Level 1 BEFORE Level 2 BEFORE Level 3 BEFORE Level 4
- Cross-dependency groups guarded atomically (all or none)               // technical §9.4
- Build verification AFTER each level (early failure detection)
- Header files: type definitions NEVER guarded                           // REQ-GUARD-180
- C-only build MUST be verified FIRST (no regression)                    // REQ-COMPAT-030
- USE_RUST_GFX MUST NOT be defined until all deps have Rust exports      // REQ-COMPAT-070

### Integration Boundaries
- Operates on: 41 C files in sc2/src/libs/graphics/ and sdl/ subdirectory
- Requires: All Rust FFI exports from components 008–011 implemented
- Requires: libuqm_rust.a built and linkable
- Build system: Makefile/build.sh defines USE_RUST_GFX compile flag
- Linker: resolves guarded C symbols from Rust library

### Side Effects
- C source files modified (guard preprocessor directives inserted)
- Object files may become empty when USE_RUST_GFX defined                // REQ-GUARD-160
- Build output changes: some symbols now from Rust library, not C
- No runtime side effects (compile-time only)
