# State System Rust Migration — Functional & Technical Specification

## 1. Current State Architecture

### 1.1 C State File I/O (`state.c` / `state.h`)

The C state system provides **in-memory file I/O** for three named buffers that store runtime game data (star scan info, random encounter groups, and scripted encounter groups). These are **not** on-disk files — they are `malloc`'d byte arrays exposed through a `FILE*`-like API.

#### `GAME_STATE_FILE` Structure

```c
struct GAME_STATE_FILE {
    const char *symname;   // "STARINFO", "RANDGRPINFO", "DEFGRPINFO"
    DWORD size_hint;       // initial allocation size
    int   open_count;      // reference count
    BYTE *data;            // heap-allocated buffer
    DWORD used;            // high-water mark (logical length)
    DWORD size;            // physical allocation size
    DWORD ptr;             // current read/write position
};
```

Three static instances are defined in `state.c`:

| Index | Symbolic Name       | Size Hint     |
|-------|---------------------|---------------|
| 0     | `STARINFO_FILE`     | `STAR_BUFSIZE` (NUM_SOLAR_SYSTEMS × 4 + 3800 × 12) |
| 1     | `RANDGRPINFO_FILE`  | `RAND_BUFSIZE` (4 KB) |
| 2     | `DEFGRPINFO_FILE`   | `DEF_BUFSIZE` (10 KB) |

#### State File API

| Function | Signature | Behavior |
|----------|-----------|----------|
| `OpenStateFile` | `(int stateFile, const char *mode) → GAME_STATE_FILE*` | Returns pointer to static file entry. `"w"` clears data; `"r"` preserves. Increments open_count. |
| `CloseStateFile` | `(GAME_STATE_FILE *fp)` | Resets ptr to 0, decrements open_count. No deallocation. |
| `DeleteStateFile` | `(int stateFile)` | Frees data buffer, resets used/ptr to 0. |
| `LengthStateFile` | `(GAME_STATE_FILE *fp) → DWORD` | Returns `fp->used` (high-water mark). |
| `ReadStateFile` | `(void *lpBuf, COUNT size, COUNT count, GAME_STATE_FILE *fp) → int` | `memcpy` from `data+ptr`. Returns items read. Clamps to available data. |
| `WriteStateFile` | `(const void *lpBuf, COUNT size, COUNT count, GAME_STATE_FILE *fp) → int` | `memcpy` to `data+ptr`. Auto-grows buffer (1.5× strategy). Updates high-water mark. |
| `SeekStateFile` | `(GAME_STATE_FILE *fp, long offset, int whence) → int` | Standard SEEK_SET/SEEK_CUR/SEEK_END. Clamps negative to 0. |

**Key insight**: The `GAME_STATE_FILE*` is an **opaque pointer** to callers. Callers never inspect the struct — they only pass it to the State File API functions and to `sread_*`/`swrite_*` inline helpers defined in `state.h`.

#### Serialization Helpers (`state.h`)

`state.h` defines inline wrappers that call `ReadStateFile`/`WriteStateFile`:

- `sread_8`, `sread_16`, `sread_16s`, `sread_32`, `sread_a32`
- `swrite_8`, `swrite_16`, `swrite_32`, `swrite_a32`

**Critical endianness note**: These helpers perform **raw memory reads** — `ReadStateFile(v, 2, 1, fp)` copies 2 bytes directly into a `UWORD*`. There is **no** byte-swapping. The state files are in-memory only and always match the host's native byte order. This is safe because the state files are never written to disk directly — save/load uses a separate byte-order-aware serialization layer (see Section 5).

### 1.2 C Game State Bits (`globdata.h` / `globdata.c`)

Game state is stored as a **packed bit array** in `GAME_STATE.GameState[]`, a byte array sized `(NUM_GAME_STATE_BITS + 7) >> 3`.

#### Bit Layout

The `globdata.h` header defines ~300 named state variables using the macro pattern:

```c
#define START_GAME_STATE enum {
#define ADD_GAME_STATE(SName, NumBits) SName, END_##SName = SName + NumBits - 1,
#define END_GAME_STATE NUM_GAME_STATE_BITS };
```

Each `ADD_GAME_STATE(NAME, N)` creates two enum values:
- `NAME` — the starting bit index
- `END_NAME` — `NAME + N - 1` (the ending bit index, inclusive)

This gives compile-time constants used by the macros:

```c
#define GET_GAME_STATE(SName) getGameState(GLOBAL(GameState), (SName), (END_##SName))
#define SET_GAME_STATE(SName, val) setGameState(GLOBAL(GameState), (SName), (END_##SName), (val))
```

There are also 32-bit variants (`GET_GAME_STATE_32`/`SET_GAME_STATE_32`) used for group offsets.

#### Underlying Functions (`globdata.c`)

```c
BYTE getGameState(BYTE *state, int startBit, int endBit);
void setGameState(BYTE *state, int startBit, int endBit, BYTE val);
DWORD getGameState32(BYTE *state, int startBit);
void setGameState32(BYTE *state, int startBit, DWORD val);
void copyGameState(BYTE *dest, DWORD target, BYTE *src, DWORD begin, DWORD end);
```

The bit extraction/insertion logic handles cross-byte boundaries. `getGameState32` reads 4 consecutive bytes via `getGameState`. `copyGameState` is used only in legacy save conversion.

#### Bit Array Contents

The bit array stores approximately 300 named fields spanning ~1240 bits of "real" game state, plus 15 × 32-bit group offsets (480 bits) for scripted encounter group positions in `DEFGRPINFO_FILE`. The group offsets were historically interleaved with game state bits in the legacy format but were reorganized into a contiguous block.

### 1.3 Rust State Module (`rust/src/state/`)

The existing Rust implementation provides:

| File | What It Implements |
|------|--------------------|
| `game_state.rs` | `GameState` struct with `[u8; 256]` backing array, `get_state`/`set_state`/`get_state_32`/`set_state_32`/`copy_state`/`reset`/`as_bytes`/`from_bytes`. Matches C bit manipulation logic. |
| `state_file.rs` | `StateFile` (in-memory buffer with read/write/seek) and `StateFileManager` (manages 3 files). Matches C `GAME_STATE_FILE` semantics. |
| `planet_info.rs` | `PlanetInfoManager` and `ScanRetrieveMask` for planet scan data. Partial implementation — `get_planet_info` doesn't account for moon counts properly. |
| `ffi.rs` | `extern "C"` functions: `rust_init_game_state`, `rust_get_game_state_bits`, `rust_set_game_state_bits`, `rust_get/set_game_state_32`, `rust_open/close/delete/length/read/write/seek_state_file`, `rust_get_game_state_bytes`, `rust_restore_game_state_from_bytes`. Uses `Mutex<Option<...>>` globals. |
| `mod.rs` | Module re-exports. |

**Current status**: The Rust module has 59 unit tests. The FFI layer is functional but has notable gaps:
1. `rust_get_game_state`/`rust_set_game_state` (string key variants) only handle 3 hardcoded state names — not usable in production.
2. `rust_copy_game_state` has a deadlock: it tries to lock `GLOBAL_GAME_STATE` twice (immutable + mutable).
3. The `StateFile::seek` clamps to `data.len()` rather than allowing seeks past EOF (the C version allows `ptr` to exceed `size`).
4. `NUM_GAME_STATE_BITS` is hardcoded at 2048, which may differ from the C enum's actual terminal value.
5. No `sread_*`/`swrite_*` equivalents exist; the FFI uses raw byte-buffer read/write.

---

## 2. State File I/O Call Site Catalog

### 2.1 Core Implementation (`state.c`)

| Line | Function | Call | Context |
|------|----------|------|---------|
| 53 | `OpenStateFile` | (definition) | Opens state file by index |
| 100 | `CloseStateFile` | (definition) | Closes state file |
| 112 | `DeleteStateFile` | (definition) | Frees state file data |
| 132 | `LengthStateFile` | (definition) | Returns used bytes |
| 138 | `ReadStateFile` | (definition) | Reads from state buffer |
| 161 | `WriteStateFile` | (definition) | Writes to state buffer |
| 192 | `SeekStateFile` | (definition) | Seeks in state buffer |

### 2.2 Planet Info (`state.c` — data operations)

| Line | Call | Context |
|------|------|---------|
| 214 | `OpenStateFile(STARINFO_FILE, "wb")` | `InitPlanetInfo` — writes zero offsets for all stars |
| 228 | `CloseStateFile(fp)` | End of `InitPlanetInfo` |
| 235 | `DeleteStateFile(STARINFO_FILE)` | `UninitPlanetInfo` — frees star info buffer |
| 250 | `OpenStateFile(STARINFO_FILE, "rb")` | `GetPlanetInfo` — reads scan masks |
| 265 | `SeekStateFile(fp, ..., SEEK_SET)` | Seek to star's offset record |
| 280 | `SeekStateFile(fp, offset, SEEK_SET)` | Seek to planet's scan record |
| 285 | `CloseStateFile(fp)` | End of `GetPlanetInfo` |
| 294 | `OpenStateFile(STARINFO_FILE, "r+b")` | `PutPlanetInfo` — writes scan masks |
| 310 | `SeekStateFile(fp, ..., SEEK_SET)` | Seek to star's offset record |
| 320 | `LengthStateFile(fp)` | Get EOF offset for new record |
| 323 | `SeekStateFile(fp, ..., SEEK_SET)` | Seek to write offset back-pointer |
| 327 | `SeekStateFile(fp, offset, SEEK_SET)` | Seek to new record location |
| 347 | `SeekStateFile(fp, offset, SEEK_SET)` | Seek to target scan record |
| 351 | `CloseStateFile(fp)` | End of `PutPlanetInfo` |

### 2.3 Group Info (`grpinfo.c`)

`grpinfo.c` is the **heaviest user** of the State File API with ~40 calls. It manages random and scripted encounter groups.

| Line | Call | Context |
|------|------|---------|
| 153 | `OpenStateFile(RANDGRPINFO_FILE, "wb")` | `InitGroupInfo` — init random group header |
| 161 | `CloseStateFile(fp)` | End of random init |
| 164 | `OpenStateFile(DEFGRPINFO_FILE, "wb")` | `InitGroupInfo(TRUE)` — init defined group file |
| 170 | `CloseStateFile(fp)` | End of defined init |
| 177 | `DeleteStateFile(DEFGRPINFO_FILE)` | `UninitGroupInfo` — cleanup |
| 178 | `DeleteStateFile(RANDGRPINFO_FILE)` | `UninitGroupInfo` — cleanup |
| 389 | `LengthStateFile(fp)` | `FlushGroupInfo` — get EOF for new group list |
| 430 | `LengthStateFile(fp)` | `FlushGroupInfo` — get EOF for new group data |
| 433 | `SeekStateFile(fp, offset, SEEK_SET)` | Seek to header position |
| 448 | `SeekStateFile(fp, ..., SEEK_SET)` | Seek to group list position |
| 495 | `SeekStateFile(fp, ..., SEEK_SET)` | Seek to specific group position |
| 521 | `OpenStateFile(DEFGRPINFO_FILE, "r+b")` | `GetGroupInfo` — read defined groups |
| 523 | `OpenStateFile(RANDGRPINFO_FILE, "r+b")` | `GetGroupInfo` — read random groups |
| 528 | `SeekStateFile(fp, offset, SEEK_SET)` | Seek to group header |
| 562 | `CloseStateFile(fp)` | End of group validation |
| 564 | `OpenStateFile(RANDGRPINFO_FILE, "wb")` | Erase expired random groups |
| 568 | `CloseStateFile(fp)` | End of erase |
| 586 | `SeekStateFile(fp, ..., SEEK_SET)` | Read specific group ships |
| 655 | `CloseStateFile(fp)` | End of `GetGroupInfo(GROUP_INIT_IP)` |
| 659 | `CloseStateFile(fp)` | End of `GetGroupInfo(GROUP_LIST)` |
| 666 | `CloseStateFile(fp)` | End of `GetGroupInfo(which_group)` |
| 682 | `SeekStateFile(fp, ..., SEEK_SET)` | Read group list for `GROUP_LIST` case |
| 701 | `SeekStateFile(fp, ..., SEEK_SET)` | Read past group list byte count |
| 741 | `CloseStateFile(fp)` | End of `GROUP_LIST` read |
| 769 | `SeekStateFile(fp, ..., SEEK_SET)` | Read specific group's ships |
| 788 | `CloseStateFile(fp)` | End of specific group read |
| 800 | `OpenStateFile(DEFGRPINFO_FILE, "r+b")` | `PutGroupInfo` — write defined group |
| 802 | `OpenStateFile(RANDGRPINFO_FILE, "r+b")` | `PutGroupInfo` — write random group |
| 809 | `LengthStateFile(fp)` | Get EOF for new group |
| 810 | `SeekStateFile(fp, offset, SEEK_SET)` | Seek to write position |
| 823 | `SeekStateFile(fp, offset, SEEK_SET)` | Seek after header write |
| 861 | `CloseStateFile(fp)` | End of `PutGroupInfo` |

### 2.4 Save Game (`save.c`)

| Line | Call | Context |
|------|------|---------|
| 595 | `OpenStateFile(STARINFO_FILE, "rb")` | `SaveStarInfo` — read star data for serialization to save file |
| 598 | `LengthStateFile(fp)` | Get total star info length |
| 615 | `CloseStateFile(fp)` | End of `SaveStarInfo` |
| 620–648 | Multiple `SeekStateFile` calls | `SaveBattleGroup` — read group data for serialization |
| 675 | `OpenStateFile(RANDGRPINFO_FILE, "rb")` | `SaveGroups` — read random groups for serialization |
| 676 | `LengthStateFile(fp)` | Check if random groups have data |
| 683 | `SeekStateFile(fp, ..., SEEK_SET)` | Read group list |
| 709 | `CloseStateFile(fp)` | End of random group save |
| 711 | `OpenStateFile(DEFGRPINFO_FILE, "rb")` | Read defined groups for serialization |
| 712 | `LengthStateFile(fp)` | Check if defined groups have data |
| 726 | `CloseStateFile(fp)` | End of defined group save |

### 2.5 Load Game (`load.c`)

| Line | Call | Context |
|------|------|---------|
| 456 | `OpenStateFile(STARINFO_FILE, "wb")` | `LoadScanInfo` — write star scan data from save file |
| 466 | `CloseStateFile(fp)` | End of `LoadScanInfo` |
| 473 | `OpenStateFile(RANDGRPINFO_FILE, "rb")` | `LoadGroupList` — read existing random header |
| 483 | `LengthStateFile(fp)` | Get EOF for group list append |
| 484–486 | `SeekStateFile` calls | Update header, seek to group list pos |
| 510 | `CloseStateFile(fp)` | End of `LoadGroupList` |
| 529 | `OpenStateFile(DEFGRPINFO_FILE, "rb")` | `LoadBattleGroup` — read defined group header |
| 530 | `LengthStateFile(fp)` | Get EOF for new group |
| 537 | `OpenStateFile(RANDGRPINFO_FILE, "rb")` | `LoadBattleGroup` — read random group header |
| 555 | `SeekStateFile(fp, offset, SEEK_SET)` | Write header at offset |
| 564 | `LengthStateFile(fp)` | Get group offset |
| 565 | `SeekStateFile(fp, ..., SEEK_SET)` | Seek to group position |
| 588 | `SeekStateFile(fp, offset, SEEK_SET)` | Re-write finalized header |
| 590 | `CloseStateFile(fp)` | End of `LoadBattleGroup` |

### 2.6 Load Legacy Game (`load_legacy.c`)

| Line | Call | Context |
|------|------|---------|
| 744 | `OpenStateFile(STARINFO_FILE, "wb")` | Load legacy star info blob |
| 756 | `WriteStateFile(buf, num_bytes, 1, fp)` | Direct bulk write of legacy star data |
| 760 | `CloseStateFile(fp)` | End of star info load |
| 764 | `OpenStateFile(DEFGRPINFO_FILE, "wb")` | Load legacy defined group blob |
| 776 | `WriteStateFile(buf, num_bytes, 1, fp)` | Direct bulk write of legacy group data |
| 780 | `CloseStateFile(fp)` | End of defined group load |
| 784 | `OpenStateFile(RANDGRPINFO_FILE, "wb")` | Load legacy random group blob |
| 796 | `WriteStateFile(buf, num_bytes, 1, fp)` | Direct bulk write of legacy random data |
| 800 | `CloseStateFile(fp)` | End of random group load |

### 2.7 Summary

**Total call sites**: ~103 across 6 files (excluding definitions)

| File | Open | Close | Delete | Length | Read | Write | Seek | Total |
|------|------|-------|--------|--------|------|-------|------|-------|
| `state.c` (data ops) | 5 | 4 | 1 | 1 | 0† | 0† | 7 | 18 |
| `grpinfo.c` | 7 | 8 | 2 | 3 | 0† | 0† | 11 | 31 |
| `save.c` | 3 | 3 | 0 | 3 | 0† | 0† | 5 | 14 |
| `load.c` | 4 | 3 | 0 | 3 | 0† | 0† | 5 | 15 |
| `load_legacy.c` | 3 | 3 | 0 | 0 | 0 | 3 | 0 | 9 |

†These files call `sread_*`/`swrite_*` helpers which in turn call `ReadStateFile`/`WriteStateFile`. Direct `ReadStateFile`/`WriteStateFile` calls are only in `load_legacy.c` and the inline helpers in `state.h`.

---

## 3. Game State Macro Call Site Analysis

### 3.1 Total Count

**1964 macro invocations** of `GET_GAME_STATE`, `SET_GAME_STATE`, `GET_GAME_STATE_32`, and `SET_GAME_STATE_32` across **83 `.c` files**.

### 3.2 Breakdown by Module

#### Communication Scripts (~1,192 invocations, ~61%)

| File | Count | Role |
|------|-------|------|
| `comm/starbas/starbas.c` | 138 | Starbase commander dialog |
| `comm/druuge/druugec.c` | 97 | Druuge trade dialog |
| `comm/utwig/utwigc.c` | 96 | Utwig dialog |
| `comm/melnorm/melnorm.c` | 96 | Melnorme trade dialog |
| `comm/thradd/thraddc.c` | 85 | Thraddash dialog |
| `comm/arilou/arilouc.c` | 81 | Arilou dialog |
| `comm/pkunk/pkunkc.c` | 80 | Pkunk dialog |
| `comm/spahome/spahome.c` | 71 | Spathi homeworld dialog |
| `comm/slyhome/slyhome.c` | 66 | Slylandro homeworld dialog |
| `comm/orz/orzc.c` | 64 | Orz dialog |
| `comm/yehat/yehatc.c` | 59 | Yehat dialog |
| `comm/supox/supoxc.c` | 57 | Supox dialog |
| `comm/vux/vuxc.c` | 46 | VUX dialog |
| `comm/syreen/syreenc.c` | 44 | Syreen dialog |
| `comm/talkpet/talkpet.c` | 41 | Talking Pet dialog |
| `comm/comandr/comandr.c` | 37 | Commander dialog |
| `comm/mycon/myconc.c` | 35 | Mycon dialog |
| `comm/chmmr/chmmrc.c` | 35 | Chmmr dialog |
| `comm/blackur/blackurc.c` | 33 | Black Urquan dialog |
| `comm/umgah/umgahc.c` | 32 | Umgah dialog |
| `comm/shofixt/shofixt.c` | 32 | Shofixti dialog |
| `comm/spathi/spathic.c` | 31 | Spathi dialog |
| `comm/ilwrath/ilwrathc.c` | 28 | Ilwrath dialog |
| `comm/urquan/urquanc.c` | 26 | Urquan dialog |
| `comm/zoqfot/zoqfotc.c` | 22 | Zoq-Fot-Pik dialog |
| `comm/slyland/slyland.c` | 19 | Slylandro space dialog |
| `comm/rebel/rebel.c` | 16 | Yehat rebel dialog |

#### Planet Generation (~133 invocations, ~7%)

| File | Count | Role |
|------|-------|------|
| `planets/devices.c` | 54 | Device activation/inventory |
| `planets/generate/genmyc.c` | 16 | Mycon world generation |
| `planets/generate/genutw.c` | 15 | Utwig world generation |
| `planets/generate/gensol.c` | 15 | Sol system generation |
| `planets/generate/genshof.c` | 14 | Shofixti world generation |
| `planets/generate/genvux.c` | 13 | VUX world generation |
| `planets/generate/genthrad.c` | 13 | Thraddash world generation |
| `planets/generate/genspa.c` | 13 | Spathi world generation |
| (+ 17 more `gen*.c` files) | ~66 total | Other world generation |
| `planets/lander.c` | 13 | Lander operations |
| `planets/solarsys.c` | 7 | Solar system management |
| `planets/pstarmap.c` | 7 | Starmap navigation |
| `planets/scan.c` | 1 | Planet scanning |

#### Core Game Logic (~639 invocations, ~32%)

| File | Count | Role |
|------|-------|------|
| `gameev.c` | 50 | Game event processing |
| `hyper.c` | 39 | Hyperspace/Quasispace logic |
| `uqmdebug.c` | 33 | Debug cheats |
| `comm.c` | 15 | Communication framework |
| `starbase.c` | 12 | Starbase state management |
| `encount.c` | 11 | Encounter setup/resolution |
| `commglue.c` | 10 | Comm subsystem glue |
| `save.c` | 9 | Save game preparation |
| `starcon.c` | 9 | Main game loop |
| `shipyard.c` | 8 | Ship building |
| `outfit.c` | 6 | Ship outfitting |
| `globdata.c` | 2 | `inHyperSpace`/`inQuasiSpace` |
| `grpinfo.c` | 2 | `BuildGroups` encounter rates |
| `battle.c` | 2 | Battle setup |
| `load.c` | 1 | `SET_GAME_STATE_32` for group offsets |
| (+ 8 more files) | ~12 | Misc |

### 3.3 Macro Mechanics

The macros expand to function calls:

```c
GET_GAME_STATE(SHOFIXTI_VISITS)
// expands to:
getGameState(GLOBAL(GameState), SHOFIXTI_VISITS, END_SHOFIXTI_VISITS)
```

`GLOBAL(GameState)` resolves to `GlobData.Game_state.GameState` — a `BYTE[]` member of the global `GAME_STATE` struct. The enum constants (`SHOFIXTI_VISITS = 0`, `END_SHOFIXTI_VISITS = 2`) are compile-time integers.

The macros therefore call through to `getGameState`/`setGameState` functions defined in `globdata.c`, passing the bit array pointer and compile-time bit indices.

---

## 4. Migration Approach

### 4.1 Design Principles

1. **One guard, zero scattered `#ifdef`s.** All 1964+ call sites must continue to work unmodified.
2. **Opaque pointer preservation.** The `GAME_STATE_FILE*` type is already opaque; callers cannot break if the underlying implementation changes.
3. **Binary compatibility of the bit array.** The `GameState[]` byte array must remain at the same offset in `GAME_STATE` for save/load compatibility.
4. **No new dependencies for C callers.** Existing `.c` files should not need `#include` changes.

### 4.2 Option A: Function-Level Redirect in `state.c` (RECOMMENDED for State File I/O)

**Strategy**: When `USE_RUST_STATE` is defined, the 7 state file functions in `state.c` call through to their `rust_*` equivalents. The function signatures remain identical. No other file changes.

```c
// state.c — one-time ifdef block
#ifdef USE_RUST_STATE

#include "rust_state_ffi.h"

GAME_STATE_FILE *
OpenStateFile (int stateFile, const char *mode)
{
    // Rust implementation via FFI
    // Returns an opaque handle (cast from file index)
}
// ... all 7 functions redirect

#else
// ... existing C implementation
#endif
```

**Complication**: The current C code returns `GAME_STATE_FILE*` (a real pointer to a struct). Callers pass this pointer back to subsequent calls. The Rust FFI currently uses `file_index: c_int`. Options:
- **4.2a**: Return a sentinel pointer (e.g., `(GAME_STATE_FILE*)(uintptr_t)(file_index + 1)`) and decode it back to an index in every function call. Ugly but zero-change to callers.
- **4.2b** (RECOMMENDED): Have the Rust FFI accept and return `GAME_STATE_FILE*` by maintaining a parallel array of opaque handles on the Rust side, or simply use the C static array indices as the Rust already does. Since the C always passes `&state_files[i]`, we can recover `i` from `fp - state_files`. The redirect functions just compute the index and pass it to Rust.

**Impact**: 0 lines changed in caller files. 1 `#ifdef` block in `state.c` (~50 lines). No header changes needed.

### 4.3 Option B: `#error` Guard on `state.c`, Symbols from Rust

**Strategy**: When `USE_RUST_STATE` is defined, `state.c` emits `#error` (or is excluded from the build) and all 7 symbols are provided by the Rust static library.

**Problem**: The Rust FFI must match the exact C signatures including the `GAME_STATE_FILE*` parameter type. This requires the Rust code to define or accept a C-compatible `GAME_STATE_FILE` struct or opaque pointer. Currently, the Rust FFI uses `c_int` file indices, not `GAME_STATE_FILE*`.

**Verdict**: Possible but requires more FFI work than Option A. Not recommended unless there's a desire to completely eliminate `state.c` from the build.

### 4.4 Option C: Macro Redirect for Game State Bits (RECOMMENDED)

**Strategy**: Redirect the macros in `globdata.h` to call Rust functions when `USE_RUST_STATE` is defined.

```c
#ifdef USE_RUST_STATE
#define GET_GAME_STATE(SName) \
    rust_get_game_state_bits((SName), (END_##SName))
#define SET_GAME_STATE(SName, val) \
    rust_set_game_state_bits((SName), (END_##SName), (val))
#define GET_GAME_STATE_32(SName) \
    rust_get_game_state_32((SName))
#define SET_GAME_STATE_32(SName, val) \
    rust_set_game_state_32((SName), (val))
#else
// ... existing macro definitions
#endif
```

**Impact**: ~10 lines changed in `globdata.h`. Zero changes to the 83 caller files. The enum constants remain as-is (they're just integers).

**Critical requirement**: The Rust `GameState` byte array must use **exactly the same bit layout** as the C code. The Rust `get_state`/`set_state` implementation must be **bit-for-bit identical** to `getGameState`/`setGameState` in `globdata.c`.

### 4.5 Option D: Redirect `getGameState`/`setGameState` Functions

**Strategy**: Instead of redirecting the macros (Option C), redirect the underlying functions in `globdata.c`.

```c
#ifdef USE_RUST_STATE
BYTE getGameState(BYTE *state, int startBit, int endBit) {
    (void)state;
    return rust_get_game_state_bits(startBit, endBit);
}
// ...
#endif
```

**Problem**: The `state` parameter is ignored — the Rust side uses its own global state. But the C code passes `GLOBAL(GameState)` — meaning the C-side byte array and the Rust-side byte array could diverge. Any C code that reads `GameState[]` directly (not through macros) would see stale data.

**Verdict**: Fragile. Option C is safer because the macros are the **only** access path for 1964 call sites — but we must also redirect `getGameState`/`setGameState`/`getGameState32`/`setGameState32`/`copyGameState` since `load_legacy.c` calls them directly with explicit state arrays. For `load_legacy.c`, the direct function calls operate on local byte arrays (not `GLOBAL(GameState)`), so they must continue to work as pure C. The macro redirect (Option C) handles the 1964 sites; the legacy functions can remain in C.

### 4.6 Recommended Approach

**Combine Options A + C:**

1. **State File I/O** (Option A): Redirect the 7 functions in `state.c` via a `USE_RUST_STATE` ifdef block. Compute file index from `fp` pointer. ~50 lines, 1 file.

2. **Game State Bits** (Option C): Redirect the 4 macros in `globdata.h` via a `USE_RUST_STATE` ifdef block. ~10 lines, 1 file.

3. **Underlying functions** (`getGameState` etc.): Leave in C. They are called directly only by `load_legacy.c` (operating on local byte arrays) and by the macros (which we're redirecting). Once the macros are redirected, `globdata.c`'s functions are only needed for legacy save loading.

4. **Synchronization point**: On load game, after `GameState[]` is populated from the save file, call `rust_restore_game_state_from_bytes(GameState, sizeof(GameState))` to sync the Rust state. On save game, call `rust_get_game_state_bytes()` to export Rust state back to the C byte array before serialization.

**Total C-side changes**: ~60 lines across 2 files (`state.c`, `globdata.h`), plus a sync call in `load.c` and `save.c` (~4 lines each).

---

## 5. Save/Load Compatibility

### 5.1 Save File Format

The UQM save format (`uqmsave.NN`) uses a **tagged chunk** system:

```
[SAVEFILE_TAG: 4 bytes]
[SUMMARY_TAG: 4 bytes] [size: 4 bytes] [SIS state + summary data]
[GLOBAL_STATE_TAG: 4 bytes] [size: 4 bytes = 75] [GAME_STATE struct fields]
[GAME_STATE_TAG: 4 bytes] [size: 4 bytes] [GameState[] byte array]
[RACE_Q_TAG] [size] [fleet info entries]
[NPC_SHIP_Q_TAG/SHIP_Q_TAG] [size] [ship entries]
[EVENTS_TAG] [size] [event entries]
[ENCOUNTERS_TAG] [size] [encounter entries]
[SCAN_TAG] [size] [star info data — copied from STARINFO state file]
[GROUP_LIST_TAG] [size] [IP group list — copied from RANDGRPINFO state file]
[BATTLE_GROUP_TAG] [size] [battle group — copied from DEF/RANDGRPINFO state files]
[STAR_TAG] [size] [star descriptor]
```

**All multi-byte values in the save file are little-endian**, using explicit byte decomposition:

```c
static inline void write_32(void *fp, DWORD v) {
    write_8(fp, (BYTE)(v & 0xff));
    write_8(fp, (BYTE)((v >> 8) & 0xff));
    write_8(fp, (BYTE)((v >> 16) & 0xff));
    write_8(fp, (BYTE)((v >> 24) & 0xff));
}
```

### 5.2 How State Files Interact with Saves

On **save**: `SaveStarInfo()` reads the `STARINFO` state file via `OpenStateFile`/`sread_32` and writes each DWORD to the save file via `write_32` (with explicit LE encoding). `SaveGroups()` similarly reads `RANDGRPINFO` and `DEFGRPINFO` state files.

On **load**: `LoadScanInfo()` reads from the save file via `read_32` (explicit LE decoding) and writes to the `STARINFO` state file via `swrite_32` (native-endian in-memory write). `LoadGroupList()` and `LoadBattleGroup()` similarly populate `RANDGRPINFO`/`DEFGRPINFO`.

The `GameState[]` bit array is written/read directly:
- **Save**: `write_a8(fh, GSPtr->GameState, sizeof(GSPtr->GameState))` — raw byte copy to save file.
- **Load**: `read_a8(fh, GSPtr->GameState, magic)` — raw byte copy from save file.

### 5.3 Impact of Rust Migration on Saves

**State File I/O**: Save/load compatibility is **unaffected** as long as:
1. The Rust `StateFile` buffers contain **exactly the same bytes** as the C buffers would for any given sequence of operations.
2. `LengthStateFile` returns the same value (high-water mark semantics).
3. `SeekStateFile` allows seeking past EOF without error (the C version allows `ptr > size` — the Rust version currently clamps).

**WARNING: Known Rust bug**: `StateFile::seek` clamps `ptr` to `data.len()`. The C version allows `ptr` to go arbitrarily past `used`/`size`. This must be fixed — `FlushGroupInfo` relies on seeking past the current data length before writing (which auto-extends the buffer).

**Game State Bits**: Save/load compatibility is **unaffected** as long as:
1. The Rust `GameState.bytes[]` array is **bit-for-bit identical** to `GLOBAL(GameState)` for any given sequence of get/set operations.
2. Before saving, the Rust state is synced back to the C byte array.
3. After loading, the C byte array is synced to the Rust state.

**The `GameState[]` byte array is the save format's canonical representation.** As long as the Rust code faithfully reproduces the exact same byte array, saves are compatible.

### 5.4 Legacy Save Compatibility

Legacy saves (`starcon2.NN`) use a different format with compressed streams. `load_legacy.c` handles these by:
1. Reading compressed data with `cread_*` functions
2. Calling `InterpretLegacyGameState()` which uses `getGameState`/`setGameState`/`getGameState32`/`setGameState32`/`copyGameState` on **local byte arrays** (not `GLOBAL(GameState)`)
3. Bulk-writing state file data via `WriteStateFile`

Since legacy loading operates on local arrays and calls the underlying functions (not the macros), it will continue to use the C implementations even with macro redirection. This is correct and requires no changes.

### 5.5 Endianness

- **Save files**: Explicitly little-endian (byte-by-byte serialization). Platform-safe.
- **State files (in-memory)**: Native endian (raw `memcpy` of multi-byte values). This is safe because state files never persist to disk — they are rebuilt from save files at load time.
- **GameState[] bit array**: Byte-oriented. The bit extraction logic indexes bytes and shifts within bytes. It is endian-neutral as long as the byte-level logic matches exactly.

**Rust concern**: The Rust `GameState` must use `u8` arrays (not larger types) for the backing store to avoid endianness issues. The current implementation correctly uses `[u8; NUM_GAME_STATE_BYTES]`.

---

## 6. Risk Assessment

### 6.1 Severity: CRITICAL

State file I/O and game state bits are the two most central data systems in the game:

- **State files** store all explored planet data, all encounter group compositions and locations. Corruption = lost progress, broken game world.
- **Game state bits** control every dialog branch, every quest flag, every alliance status, every item acquisition. A single off-by-one bit error could silently break game logic that only manifests dozens of hours into a playthrough.
- **Save compatibility** is non-negotiable. Players must be able to save with the C backend and load with the Rust backend (and vice versa).

### 6.2 Specific Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Bit extraction/insertion differs between Rust and C | **Critical** — wrong quest flags, silent game logic corruption | Exhaustive bit-level comparison tests against C reference implementation |
| `SeekStateFile` clamping behavior differs | **High** — corrupted group data, crashes during IP encounter | Fix Rust `StateFile::seek` to match C semantics exactly (allow past-EOF seeks) |
| `LengthStateFile` semantics differ (Rust returns `data.len()`, C returns `used` high-water mark) | **High** — wrong offsets for group records | Verify that Rust's `Vec::len()` after writes matches C's `used` tracking |
| State sync timing — Rust and C byte arrays diverge | **Medium** — save/load produces wrong state | Clearly defined sync points; assert byte arrays match in debug builds |
| `WriteStateFile` growth strategy differs | **Low** — memory usage difference, not correctness | Test with realistic group counts |
| `NUM_GAME_STATE_BITS` mismatch | **Medium** — Rust array too small/large, serialization mismatch | Derive from C enum or static_assert equivalence |
| `copy_game_state` FFI deadlock | **High** — program hangs on legacy load | Fix the double-lock in `ffi.rs` `rust_copy_game_state` |
| `open_count` tracking differs | **Low** — harmless warning messages | Match C behavior exactly |

### 6.3 The 1964 Call Sites Problem

The 1964 macro invocations span 83 files. There is no way to test these incrementally — either the macro points to C or to Rust for all 1964 at once. This is why the holistic approach (macro redirect) is correct. But it means:

1. The Rust bit-manipulation code must be **proven correct** before flipping the switch.
2. A comprehensive test suite must compare Rust and C outputs for the same inputs.
3. A dual-execution mode (call both, compare results) should be available in debug builds.

---

## 7. Test Plan

### 7.1 Existing Rust Tests

| File | Tests | Coverage |
|------|-------|----------|
| `game_state.rs` | 11 | Single-bit, multi-bit, cross-byte, 32-bit, copy, reset, clone, value validation |
| `state_file.rs` | 21 | Read/write, append, seek (all modes), open/close, delete, u32 helpers, file manager, edge cases |
| `planet_info.rs` | 15 | Scan masks, init/get/put with planets/moons/stars, uninit |
| `ffi.rs` | 12 | Init, get/set bits, 32-bit, reset, state file open/read/write/seek/length/delete, bytes export/restore |
| **Total** | **59** | |

### 7.2 Additional Tests Required

#### 7.2.1 Bit-Level Equivalence Tests (CRITICAL)

These must compare the Rust `GameState` implementation against the C `getGameState`/`setGameState` **for every possible combination** that matters:

- **All named state fields**: For each of the ~300 `ADD_GAME_STATE` entries, set a value, verify `get_state` returns the same bits.
- **Cross-byte boundary fields**: Fields that straddle byte boundaries (e.g., a 3-bit field starting at bit 6).
- **Full-width fields**: 8-bit fields (e.g., `MELNORME_CREDIT0`) with all 256 values.
- **Adjacent field isolation**: Setting field A must not corrupt adjacent field B.
- **32-bit fields**: Set/get all 15 group offset fields.
- **Round-trip from C byte array**: Set known bytes in a raw array, verify Rust extracts the same values C would. Generate these expected values from the C code or by hand-computing.

**Implementation**: A test harness that generates a known `GameState[]` byte array (or obtains one from a real save file), feeds it to both the C functions (via a test shim) and the Rust functions, and asserts bit-for-bit equality of all outputs.

#### 7.2.2 State File Behavioral Equivalence Tests

- **Seek past EOF**: Verify that seeking to position 100 in a 50-byte file sets ptr to 100, and that a subsequent write at position 100 extends the buffer.
- **Write growth**: Write 1 byte at a time to a 4KB initial buffer until 10KB. Verify `length()` matches.
- **Open mode semantics**: Verify `"wb"` clears, `"rb"` preserves, `"r+b"` preserves.
- **Interleaved read/write**: Open a file, write 10 bytes, seek to 0, read 5, write 5. Verify buffer contents.
- **Multiple open/close**: Open a file twice, close once. Verify `open_count` tracking.

#### 7.2.3 Save/Load Round-Trip Tests (CRITICAL)

These test the full save-load cycle:

1. **Reference save file test**: Take a known save file (from the existing game test corpus or generated by the C code). Load it using the C path. Capture the resulting `GameState[]` bytes and all 3 state file buffers. Load the same save file using the Rust path. Compare all captured data byte-for-byte.

2. **Save round-trip**: Starting from a known game state, save using the C path. Load the resulting file using the Rust path. Compare all state. Then save using the Rust path and load using the C path. Compare again.

3. **Legacy save compatibility**: Load a `starcon2.NN` format save. Verify the legacy conversion produces identical `GameState[]` bytes whether the runtime macros point to C or Rust.

#### 7.2.4 Group Info Integration Tests

Group info operations are the most complex state file users. Tests needed:

- `InitGroupInfo(TRUE)` → verify RANDGRPINFO and DEFGRPINFO file contents match C behavior.
- `PutGroupInfo` → write a group, verify file layout matches C byte-for-byte.
- `GetGroupInfo(GROUP_INIT_IP)` → read back groups, verify identical `ip_group_q` state.
- `GetGroupInfo(GROUP_LIST)` → verify group list round-trip.
- Expired groups (date validation) → verify identical invalidation behavior.

#### 7.2.5 Debug-Mode Dual Execution

When `USE_RUST_STATE` and `DEBUG` are both defined:

```c
#define GET_GAME_STATE(SName) ({ \
    BYTE _c_val = getGameState(GLOBAL(GameState), (SName), (END_##SName)); \
    BYTE _r_val = rust_get_game_state_bits((SName), (END_##SName)); \
    assert(_c_val == _r_val); \
    _r_val; \
})
```

This allows running the actual game with both backends in parallel, catching any divergence at runtime. Should be used during the QA period before fully switching.

### 7.3 Test Matrix Summary

| Category | Priority | New Tests Needed | Approach |
|----------|----------|------------------|----------|
| Bit equivalence | P0 | ~20 | Property-based + known-vector |
| State file seek/write edge cases | P0 | ~10 | Unit tests |
| Save/load round-trip | P0 | ~5 | Integration with real save files |
| Group info operations | P1 | ~8 | Integration tests |
| Dual execution mode | P1 | ~1 (infrastructure) | Build flag + runtime assertions |
| Legacy save loading | P2 | ~3 | Integration with `starcon2.*` files |
| Planet info moon accounting | P2 | ~5 | Unit tests with variable moon counts |

---

## 8. Known Rust Implementation Gaps

The following must be fixed before the switch can be made:

| Issue | File | Description | Severity |
|-------|------|-------------|----------|
| `seek` clamps to `data.len()` | `state_file.rs` | C allows `ptr > size` — Rust must too. Buffer extends on write, not on seek, but ptr must be settable beyond current length. | **Blocker** |
| `rust_copy_game_state` deadlocks | `ffi.rs` | Locks `GLOBAL_GAME_STATE` twice (read + write). Must use a single lock or copy-then-mutate pattern. | **Blocker** |
| `NUM_GAME_STATE_BITS` hardcoded | `game_state.rs` | Set to 2048; must match C enum's `NUM_GAME_STATE_BITS`. If C value is smaller, the Rust array wastes space (acceptable). If larger, UB. Should be derived from C or verified at build time. | **High** |
| String-key `rust_get/set_game_state` | `ffi.rs` | Only handles 3 hardcoded names. Either delete (not needed for macro redirect approach) or make comprehensive. | **Low** (not needed for Option C) |
| `StateFile` size hints differ | `state_file.rs` | Uses 256KB/64KB/64KB vs C's computed `STAR_BUFSIZE`/4KB/10KB. Doesn't affect correctness but wastes memory. | **Low** |
| `planet_info.rs` moon counting | `planet_info.rs` | `get_planet_info` doesn't properly account for per-planet moon counts when skipping scan records. | **Medium** (not used via FFI yet) |
| No `sread_*`/`swrite_*` equivalents | N/A | The FFI uses raw byte reads. `read_u32_le`/`write_u32_le` exist in `state_file.rs` but aren't exposed via FFI. Not strictly needed if the C wrappers in `state.h` continue to be used — they call `ReadStateFile`/`WriteStateFile` which would be redirected. | **Low** |
