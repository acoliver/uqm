# P10: Consolidate CommData ownership to Rust

## Worker scope

Make Rust's `CommData` (in `comm/types.rs`) the single source of truth for alien
encounter data. Eliminate the C `LOCDATA CommData` global and the 32 field
accessors in `rust_comm.c`.

### Current state (the problem)

```
C owns:   LOCDATA CommData (global in comm.c:72)
          47 direct CommData field accesses in rust_comm.c
          32 LOCDATA field accessor functions

Rust has: CommData struct in comm/types.rs (25+ fields)
          Separate copy, populated through bridge accessors

Bridge:   rust_comm.c reads C LOCDATA fields and copies to Rust
          189 bridge functions, 2156 lines of C bridge code
```

### Target state (the solution)

```
Rust owns: CommData (in comm/types.rs) — single source of truth
           init_race() populates Rust CommData directly

C reads:   Through FFI getters when needed (e.g., CommData.AlienFont for rendering)

Bridge:   LOCDATA field accessors ELIMINATED
          rust_comm.c shrunk or eliminated
```

### What needs to happen

1. **Make Rust CommData the single source of truth**
   - Rust `CommData` already exists with all 25+ fields
   - `init_race()` must populate Rust `CommData` directly (not C `LOCDATA`)
   - C code that reads `CommData.AlienFont` etc. must go through FFI

2. **Create FFI getters for C-side reads**
   - C's `HailAlien()` (or `rust_HailAlien()`) reads CommData fields for
     resource loading (AlienFrameRes, AlienFontRes, etc.)
   - Already has bridge: `c_GetCommDataAlienFrameRes()` etc. — but these read
     from C's LOCDATA. Change them to read from Rust's CommData.

3. **Port init_race dispatch table**
   - C's `init_race(comm_id)` returns `LOCDATA*` — needs to return/populate Rust CommData
   - The per-race `init_X_comm()` functions set function pointers + segue mode
   - For now, keep per-race init in C but have it write to Rust CommData through FFI
   - P12-P15 will port per-race init to Rust entirely

4. **Eliminate rust_comm.c LOCDATA accessors**
   - 32 `c_locdata_get_*` functions become unnecessary once Rust owns CommData
   - Remove them (or leave as dead code until full elimination)

### Test plan

**Unit tests**:
- CommData field equivalence: after init_race, Rust CommData matches what C LOCDATA had
- Each field getter returns correct value

**Automation proof** (`scripts/comm-data-v1.json`):
- Start game, reach encounter
- Verify CommData is populated (can check via capture — if alien sprite renders, data is correct)
- Capture, finish

### Dependencies
- P09 (game state ownership — CommData depends on activity flags)

### Files to create/modify
- MODIFY: `rust/src/comm/types.rs` (ensure CommData is the single source)
- CREATE: `rust/src/comm/comm_data_ffi.rs` (FFI getters for C-side reads)
- MODIFY: `sc2/src/uqm/rust_comm.c` (eliminate LOCDATA accessors, route to Rust)
- MODIFY: `sc2/src/uqm/comm.c` (CommData global → Rust-owned)
- MODIFY: `sc2/src/uqm/comm.h` (extern declaration → FFI)