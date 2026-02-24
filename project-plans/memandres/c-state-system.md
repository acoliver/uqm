# UQM C State & Save/Load System — Architectural Analysis

## 1. System Overview

The UQM state and persistence system is composed of two distinct but cooperating subsystems:

1. **State File I/O** — An in-memory virtual file system (`GAME_STATE_FILE`) that provides fread/fwrite-style access to three named binary buffers. These buffers hold star system scan data, random battle group info, and defined battle group info. They are **not** written to disk directly; instead, their contents are serialized into the save file during `SaveGame` and deserialized back during `LoadGame`.

2. **Game State Bits** — A packed bit-field array (`BYTE GameState[]`) stored inside the `GAME_STATE` structure. Individual flags of 1–8 bits are accessed via `GET_GAME_STATE` / `SET_GAME_STATE` macros that expand to bit-manipulation functions. A companion `GET_GAME_STATE_32` / `SET_GAME_STATE_32` pair handles 32-bit values (used for DEFGRP file offsets).

### Interaction During Save/Load

```
SaveGame:
  1. Serialize SUMMARY_DESC (SIS_STATE + metadata) → file
  2. Serialize GAME_STATE (includes GameState[] byte array) → file
  3. Serialize queues (race_q, ship_q, event_q, encounter_q) → file
  4. Read state files via OpenStateFile(…, "rb"), serialize contents → file
  5. Close disk file

LoadGame:
  1. Read SUMMARY_DESC from file → GlobData.SIS_state
  2. Read GAME_STATE (includes GameState[]) from file → GlobData.Game_state
  3. Read tagged chunks → populate queues
  4. Reconstruct state files by writing chunk data into state file buffers
  5. Close disk file
```

### Threading Model

The save system is explicitly **single-threaded**. A static `io_ok` boolean in `save.c` tracks write errors. A comment in `save.c` states: *"If for some insane reason you need to save games in different threads, you'll need to protect your calls to SaveGame with a mutex."*

State files have an `open_count` field with warning logging when count > 1 after open or < 0 after close, but there is no actual mutex protection — correctness relies on single-threaded access.

---

## 2. State File I/O Architecture

### 2.1 GAME_STATE_FILE Type

Defined as an opaque `typedef struct GAME_STATE_FILE GAME_STATE_FILE` in `state.h`, with the full struct definition in `state.c`:

```c
struct GAME_STATE_FILE
{
    const char *symname;   // Human-readable name: "STARINFO", "RANDGRPINFO", "DEFGRPINFO"
    DWORD size_hint;       // Initial allocation size (and grows-to watermark)
    int   open_count;      // Reference count for open/close balance
    BYTE *data;            // Heap-allocated buffer (HMalloc/HRealloc)
    DWORD used;            // Logical file size (highest byte written)
    DWORD size;            // Physical allocation size
    DWORD ptr;             // Current read/write position (cursor)
};
```

This is purely an **in-memory buffer** — there is no on-disk file. The data lives in heap-allocated `BYTE*` arrays that grow dynamically via `HRealloc`.

### 2.2 File Index Enum

Three state files exist, stored in a static array `state_files[3]`:

| Index | Symbolic Name | `symname` | Initial Size Hint |
|-------|--------------|-----------|-------------------|
| `0` | `STARINFO_FILE` | `"STARINFO"` | `STAR_BUFSIZE` = `NUM_SOLAR_SYSTEMS * 4 + 3800 * 12` |
| `1` | `RANDGRPINFO_FILE` | `"RANDGRPINFO"` | `RAND_BUFSIZE` = 4096 |
| `2` | `DEFGRPINFO_FILE` | `"DEFGRPINFO"` | `DEF_BUFSIZE` = 10240 |

### 2.3 Path Resolution

There is **no path resolution** — these files never touch disk. The `int stateFile` parameter is an index into the static `state_files[]` array. Validated as `0 <= stateFile < NUM_STATE_FILES` (3).

### 2.4 Buffer Management

- **Allocation**: On first `OpenStateFile`, `HMalloc(size_hint)` creates the buffer.
- **Growth**: On `WriteStateFile`, if `ptr + bytes > size`, the buffer grows to `max(ptr + bytes, size * 3/2)` via `HRealloc`. The `size_hint` watermark is also updated.
- **Deallocation**: On `DeleteStateFile`, `HFree(data)` releases the buffer and resets all fields.
- **Open mode "w"**: Resets `used = 0` (clears logical content). In `DEBUG` builds, paints buffer with `0xCC` for tracking.
- **Open mode "r"**: No-op on content.
- **Open mode "r+"**: Not explicitly handled — falls into the "unsupported mode" warning path, but since read/write are always permitted regardless of mode, it still works.

### 2.5 Operation Semantics

#### OpenStateFile(int stateFile, const char *mode) → GAME_STATE_FILE*
- Validates index range; returns `NULL` if out of range.
- Increments `open_count`; warns if > 1 after open.
- Allocates buffer on first open.
- Mode `"w"/"wb"`: clears content (`used = 0`).
- Mode `"r"/"rb"/"r+b"`: preserves content.
- Resets `ptr = 0`.

#### CloseStateFile(GAME_STATE_FILE *fp)
- Resets `ptr = 0`.
- Decrements `open_count`; warns if < 0 after close.
- Does **not** free the buffer (data persists across open/close cycles).

#### ReadStateFile(void *lpBuf, COUNT size, COUNT count, GAME_STATE_FILE *fp) → int
- fread-style: reads `size * count` bytes from current position.
- Returns number of complete elements read (bytes / size).
- Checks against `fp->size` (physical allocation), not `fp->used` (logical size).
- Truncates to available data, rounding down to whole elements.

#### WriteStateFile(const void *lpBuf, COUNT size, COUNT count, GAME_STATE_FILE *fp) → int
- fwrite-style: writes `size * count` bytes at current position.
- Grows buffer with 1.5x strategy if needed.
- Advances `ptr`; updates `used` if `ptr > used`.
- Returns number of complete elements written.

#### SeekStateFile(GAME_STATE_FILE *fp, long offset, int whence) → int
- Supports `SEEK_SET`, `SEEK_CUR`, `SEEK_END`.
- `SEEK_CUR`: adds offset to `ptr`.
- `SEEK_END`: adds offset to `used`.
- Clamps negative results to 0.
- Returns 1 on success, 0 if clamped to 0.

#### LengthStateFile(GAME_STATE_FILE *fp) → DWORD
- Returns `fp->used` (logical file size, not allocation size).

#### DeleteStateFile(int stateFile)
- Validates index range.
- Warns if `open_count != 0`.
- Resets `used`, `ptr` to 0.
- Frees `data` buffer via `HFree`, sets `data = 0`.

### 2.6 sread_* / swrite_* Serialization Helpers

Defined as `static inline` functions in `state.h`, these wrap `ReadStateFile`/`WriteStateFile`:

```c
sread_8(fp, &v)   → ReadStateFile(v, 1, 1, fp)   // 1 byte
sread_16(fp, &v)  → ReadStateFile(v, 2, 1, fp)   // 2 bytes, NATIVE endian
sread_16s(fp, &v) → sread_16 + cast to SWORD
sread_32(fp, &v)  → ReadStateFile(v, 4, 1, fp)   // 4 bytes, NATIVE endian
sread_a32(fp, ar, count) → loop of sread_32      // array of DWORDs

swrite_8(fp, v)   → WriteStateFile(&v, 1, 1, fp)
swrite_16(fp, v)  → WriteStateFile(&v, 2, 1, fp)
swrite_32(fp, v)  → WriteStateFile(&v, 4, 1, fp)
swrite_a32(fp, ar, count) → loop of swrite_32
```

**Critical endianness note**: The `sread_*`/`swrite_*` helpers use `ReadStateFile`/`WriteStateFile` which are simple `memcpy` operations. They do **NOT** perform endianness conversion. This is safe because the state file buffers are in-memory only and never cross platform boundaries directly. However, when state file data is serialized *into* a save file, the save file's own `write_32`/`read_32` functions handle endianness (little-endian on disk).

---

## 3. Game State Bits Architecture

### 3.1 The GameState[] Byte Array

```c
// In GAME_STATE struct (globdata.h):
BYTE GameState[(NUM_GAME_STATE_BITS + 7) >> 3];
```

`NUM_GAME_STATE_BITS` = **1238**, making the array **155 bytes** long.

### 3.2 Bit Field Definitions

Fields are defined using preprocessor macros:

```c
#define START_GAME_STATE enum {
#define ADD_GAME_STATE(SName, NumBits) SName, END_##SName = SName + NumBits - 1,
#define END_GAME_STATE NUM_GAME_STATE_BITS };
```

Each `ADD_GAME_STATE(NAME, N)` creates two enum constants:
- `NAME` — the starting bit index
- `END_NAME` — the ending bit index (inclusive) = `NAME + N - 1`

The enum auto-increments, so each field is packed immediately after the previous one, creating a contiguous bit stream.

There are **448** defined game state fields totaling **1238 bits**.

### 3.3 GET_GAME_STATE / SET_GAME_STATE Macro Expansion

```c
#define GET_GAME_STATE(SName) \
    getGameState(GLOBAL(GameState), (SName), (END_##SName))

#define SET_GAME_STATE(SName, val) \
    setGameState(GLOBAL(GameState), (SName), (END_##SName), (val))
```

#### Actual Bit Manipulation — getGameState()

```c
BYTE getGameState(BYTE *state, int startBit, int endBit)
{
    return (BYTE)(
        ((startBit >> 3) == (endBit >> 3)
            // Same byte: shift right by bit offset within byte
            ? (state[startBit >> 3] >> (startBit & 7))
            // Spans two bytes: combine low bits of first byte with high bits of second
            : ((state[startBit >> 3] >> (startBit & 7))
              | (state[endBit >> 3] << (endBit - startBit - (endBit & 7))))
        )
        // Mask to the correct number of bits
        & ((1 << (endBit - startBit + 1)) - 1)
    );
}
```

**How it works**:
1. Compute which byte(s) the field occupies: `startBit >> 3` and `endBit >> 3`.
2. If same byte: right-shift to align, then mask.
3. If spanning two bytes: extract remaining bits from first byte (right-shifted), OR with beginning bits from second byte (left-shifted), then mask.
4. Fields are limited to 8 bits maximum (BYTE return type).

#### Actual Bit Manipulation — setGameState()

```c
void setGameState(BYTE *state, int startBit, int endBit, BYTE val)
{
    // Clear and set bits in the first byte
    state[startBit >> 3] =
        (state[startBit >> 3]
        & (BYTE)~(((1 << (endBit - startBit + 1)) - 1) << (startBit & 7)))
        | (BYTE)((val) << (startBit & 7));

    // If field spans two bytes, handle the second byte
    if ((startBit >> 3) < (endBit >> 3)) {
        state[endBit >> 3] =
            (state[endBit >> 3]
            & (BYTE)~((1 << ((endBit & 7) + 1)) - 1))
            | (BYTE)((val) >> (endBit - startBit - (endBit & 7)));
    }
}
```

**How it works**:
1. Create a mask of the correct width, shifted to the field's position within the byte.
2. Clear those bits in the target byte(s) using AND with inverted mask.
3. OR in the new value, shifted to the correct position.
4. If the field spans two bytes, repeat for the second byte with the overflow bits.

#### 32-bit Variants

```c
DWORD getGameState32(BYTE *state, int startBit)
{
    DWORD v;
    int shift;
    for (v = 0, shift = 0; shift < 32; shift += 8, startBit += 8)
        v |= getGameState(state, startBit, startBit + 7) << shift;
    return v;
}

void setGameState32(BYTE *state, int startBit, DWORD val)
{
    DWORD v = val;
    int i;
    for (i = 0; i < 4; ++i, v >>= 8, startBit += 8)
        setGameState(state, startBit, startBit + 7, v & 0xff);
}
```

These read/write 32-bit values as four consecutive 8-bit fields in little-endian order.

#### copyGameState()

```c
void copyGameState(BYTE *dest, DWORD target, BYTE *src, DWORD begin, DWORD end)
{
    while (begin < end)
    {
        BYTE b;
        DWORD delta = 7;
        if (begin + delta > end)
            delta = end - begin;
        b = getGameState(src, begin, begin + delta);
        setGameState(dest, target, target + delta, b);
        begin += 8;
        target += 8;
    }
}
```

Copies bit ranges between state arrays in 8-bit chunks. Used by the legacy save loader to transpose old bit layouts to new ones.

### 3.4 Categories of Flags

| Category | Count (approx.) | Examples |
|----------|-----------------|---------|
| Alien race visit counters | ~60 | `SHOFIXTI_VISITS(3)`, `SPATHI_HOME_VISITS(3)` |
| Alien conversation stacks | ~50 | `MELNORME_YACK_STACK0(2)`, `SYREEN_STACK0(2)` |
| Quest/mission progress | ~80 | `CHMMR_BOMB_STATE(2)`, `UTWIG_SUPOX_MISSION(3)` |
| Item possession flags | ~40 | `TALKING_PET_ON_SHIP(1)`, `AQUA_HELIX(1)` |
| Discussion flags | ~25 | `DISCUSSED_PORTAL_SPAWNER(1)`, `DISCUSSED_UTWIG_BOMB(1)` |
| Scan/exploration state | ~15 | `RAINBOW_WORLD0(8)`, `SCANNED_MAIDENS(1)` |
| Combat/encounter state | ~15 | `BATTLE_SEGUE(1)`, `BATTLE_PLANET(8)` |
| Lander upgrades | ~5 | `LANDER_SHIELDS(4)`, `IMPROVED_LANDER_SPEED(1)` |
| Starbase state | ~10 | `STARBASE_AVAILABLE(1)`, `STARBASE_BULLETS0(8)` |
| DEFGRP file offsets | 60 | `SHOFIXTI_GRPOFFS0-3(8)` through `SAMATRA_GRPOFFS0-3(8)` |
| Temporal state | ~15 | `YEHAT_SHIP_MONTH(4)`, `PKUNK_SHIP_YEAR(5)` |
| Misc gameplay flags | ~75 | `PLANETARY_LANDING(1)`, `KOHR_AH_FRENZY(1)` |

The 60 GRPOFFS fields (15 groups × 4 bytes each = 480 bits) store 32-bit offsets into the DEFGRPINFO state file. These are historical artifacts that the new save format separates into `BATTLE_GROUP_TAG` chunks.

---

## 4. Save Game Format

### 4.1 File Naming & Location

Save files are named `uqmsave.NN` (where NN is 00-99) and written to the `saveDir` directory via `res_OpenResFile`.

### 4.2 Overall File Structure

The new format is a tagged chunk format. All multi-byte values are stored **little-endian** on disk.

```
[SAVEFILE_TAG: 4 bytes]          // 0x01534d55 = "UMS\x01"
[SUMMARY_TAG: 4 bytes] [size: 4 bytes] [Summary Data]
[GLOBAL_STATE_TAG: 4] [size: 4] [Global State Data]
[GAME_STATE_TAG: 4]   [size: 4] [GameState[] bytes]
[Optional tagged chunks in any order:]
  [RACE_Q_TAG]    [size] [Race Queue Data]
  [IP_GRP_Q_TAG]  [size] [IP Group Queue Data]
  [NPC_SHIP_Q_TAG][size] [NPC Ship Queue Data]
  [SHIP_Q_TAG]    [size] [Built Ship Queue Data]
  [EVENTS_TAG]    [size] [Events Data]
  [ENCOUNTERS_TAG][size] [Encounters Data]
  [SCAN_TAG]      [size] [Star Info / Scan Masks]
  [GROUP_LIST_TAG][size] [Random Group List]
  [BATTLE_GROUP_TAG][size] [Battle Group] (may appear multiple times)
  [STAR_TAG]      [size] [Star Descriptor]
```

### 4.3 Tag Values

| Tag | Hex | ASCII | Description |
|-----|-----|-------|-------------|
| `SAVEFILE_TAG` | `0x01534d55` | `UMS\x01` | File magic |
| `SUMMARY_TAG` | `0x6d6d7553` | `Summ` | Summary chunk |
| `GLOBAL_STATE_TAG` | `0x74536c47` | `GlSt` | Global state |
| `GAME_STATE_TAG` | `0x74536d47` | `GmSt` | Game state bits |
| `EVENTS_TAG` | `0x73747645` | `Evts` | Events |
| `ENCOUNTERS_TAG` | `0x74636e45` | `Enct` | Encounters |
| `RACE_Q_TAG` | `0x51636152` | `RacQ` | Race queue |
| `IP_GRP_Q_TAG` | `0x51704749` | `IGpQ` | IP group queue |
| `NPC_SHIP_Q_TAG` | `0x5163704e` | `NpcQ` | NPC ship queue |
| `SHIP_Q_TAG` | `0x51706853` | `ShpQ` | Built ship queue |
| `STAR_TAG` | `0x72617453` | `Star` | Star descriptor |
| `SCAN_TAG` | `0x6e616353` | `Scan` | Scan masks |
| `BATTLE_GROUP_TAG` | `0x70477442` | `BtGp` | Battle group |
| `GROUP_LIST_TAG` | `0x73707247` | `Grps` | Group list |

### 4.4 SUMMARY_DESC Structure

```c
typedef struct {
    SIS_STATE SS;                           // Ship state
    BYTE Activity;                          // Current activity enum
    BYTE Flags;                             // Lander shields + upgrades
    BYTE day_index, month_index;            // Game date
    COUNT year_index;                       // Game year
    BYTE MCreditLo, MCreditHi;             // Melnorme credits (16-bit split)
    BYTE NumShips, NumDevices;              // Escort count, device count
    BYTE ShipList[MAX_BUILT_SHIPS];        // 12 bytes: escort race IDs
    BYTE DeviceList[MAX_EXCLUSIVE_DEVICES]; // 16 bytes: device IDs
    UNICODE SaveName[SAVE_NAME_SIZE];       // 64 bytes: save game name
} SUMMARY_DESC;
```

Written to file as:
- `SIS_STATE` serialized (see below)
- Activity, Flags, day, month: 4 × BYTE
- year: UWORD (2 bytes LE)
- MCreditLo, MCreditHi, NumShips, NumDevices: 4 × BYTE
- ShipList: 12 bytes
- DeviceList: 16 bytes
- SaveName: variable length (chunk size = 160 + strlen(SaveName))

### 4.5 SIS_STATE Serialization

```c
typedef struct {
    SDWORD log_x, log_y;           // 4+4 bytes: universe coordinates
    DWORD ResUnits;                // 4 bytes: resource units
    DWORD FuelOnBoard;             // 4 bytes: fuel (scaled)
    COUNT CrewEnlisted;            // 2 bytes
    COUNT TotalElementMass;        // 2 bytes
    COUNT TotalBioMass;            // 2 bytes
    BYTE ModuleSlots[16];          // 16 bytes
    BYTE DriveSlots[11];           // 11 bytes
    BYTE JetSlots[8];              // 8 bytes
    BYTE NumLanders;               // 1 byte
    COUNT ElementAmounts[8];       // 16 bytes (8 × UWORD)
    UNICODE ShipName[16];          // 16 bytes
    UNICODE CommanderName[16];     // 16 bytes
    UNICODE PlanetName[16];        // 16 bytes
} SIS_STATE;
```

Total serialized size: 4+4+4+4+2+2+2+16+11+8+1+16+16+16+16 = **122 bytes**.

### 4.6 GAME_STATE Serialization (Global State Chunk)

Fixed 75 bytes:
- `glob_flags`: 1 byte
- `CrewCost`, `FuelCost`: 2 bytes
- `ModuleCost[NUM_MODULES]`: NUM_MODULES bytes
- `ElementWorth[NUM_ELEMENT_CATEGORIES]`: 8 bytes
- `CurrentActivity`: 2 bytes (UWORD)
- Clock state: 8 bytes (day, month, year, tick_count, day_in_ticks)
- `autopilot`: 4 bytes (2 × SWORD)
- `ip_location`: 4 bytes (2 × SWORD)
- `ShipStamp.origin`: 4 bytes (2 × SWORD)
- `ShipFacing`: 2 bytes (UWORD)
- `ip_planet`, `in_orbit`: 2 bytes
- Velocity: 18 bytes (9 × UWORD/SWORD)

Followed by GAME_STATE_TAG chunk containing the raw `GameState[]` array (155 bytes).

### 4.7 Queue Entry Sizes

| Queue | Bytes per Entry | Contents |
|-------|----------------|----------|
| Ship (SHIP_Q / NPC_SHIP_Q) | 11 | index(2), captain(1), race(1), idx(1), crew(2), max_crew(2), energy(1), max_energy(1) |
| Race (RACE_Q) | 30 | index(2), allied_state(2), days_left(1), growth_fract(1), crew(2), max_crew(2), growth(1), max_energy(1), loc(4), strengths(4), known_loc(4), growth_err(1), func_idx(1), dest_loc(4) |
| IP Group (IP_GRP_Q) | 13 | counter(2), race(1), sys_loc(1), task(1), in_system(1), dest_loc(1), orbit_pos(1), group_id(1), loc(4) |
| Event | 5 | day(1), month(1), year(2), func_index(1) |
| Encounter | 65 | transition(2), origin(4), radius(2), loc_pt(4), race(1), num_ships(1), flags(1), ships(6×MAX_HYPER_SHIPS), log_x(4), log_y(4) |

### 4.8 Endianness in write_* / read_* Helpers (save.c / load.c)

**Save file helpers** (`save.c`) explicitly write little-endian:
```c
static inline void write_16(void *fp, UWORD v) {
    write_8(fp, (BYTE)( v        & 0xff));   // Low byte first
    write_8(fp, (BYTE)((v >>  8) & 0xff));   // High byte second
}

static inline void write_32(void *fp, DWORD v) {
    write_8(fp, (BYTE)( v        & 0xff));   // Byte 0 (least significant)
    write_8(fp, (BYTE)((v >>  8) & 0xff));   // Byte 1
    write_8(fp, (BYTE)((v >> 16) & 0xff));   // Byte 2
    write_8(fp, (BYTE)((v >> 24) & 0xff));   // Byte 3 (most significant)
}
```

**Load file helpers** (`load.c`) explicitly read little-endian:
```c
static inline size_t read_16(void *fp, UWORD *v) {
    UWORD t = 0;
    int shift, i;
    for (i = 0, shift = 0; i < 2; ++i, shift += 8) {
        BYTE b;
        if (read_8(fp, &b) != 1) return 0;
        t |= ((UWORD)b) << shift;
    }
    if (v) *v = t;
    return 1;
}
```

**Important**: The legacy loader (`load_legacy.c`) uses `ReadResFile(v, 2, 1, fp)` for 16-bit values, which does **NOT** do endianness conversion. This means legacy save files are only compatible with the platform they were created on. The loader includes a crude endianness check using `year_index` range validation.

### 4.9 Save Slot Directory Structure

Each save occupies a single file `uqmsave.NN` in the `saveDir`. Legacy saves use the name `starcon2.NN`. There are no subdirectories per slot — each save is a self-contained flat file.

---

## 5. Load Game Format

### 5.1 LoadGame Flow (`load.c`)

```
LoadGame(which_game, SummPtr):
  1. Open "uqmsave.NN"
  2. If open fails → fall back to LoadLegacyGame
  3. LoadSummary:
     a. Read 4 bytes → expect SAVEFILE_TAG (0x01534d55)
     b. If not SAVEFILE_TAG → close, try LoadLegacyGame
     c. Read SUMMARY_TAG + size
     d. Read SIS_STATE + metadata + variable-length SaveName
  4. If SummPtr provided (display mode) → copy summary, close, return TRUE
  5. Copy SIS_state from summary to GlobData
  6. Reinitialize all queues
  7. Clear GameState[] to zero
  8. LoadGameState:
     a. Read GLOBAL_STATE_TAG + size (must be 75)
     b. Read all GAME_STATE fields
     c. Read GAME_STATE_TAG + size
     d. Read GameState[] bytes (handle size mismatch: truncate or skip excess)
  9. Save/restore CurrentActivity
  10. Enter chunk processing loop:
      while (read_32 → tag, read_32 → size):
        switch(tag):
          RACE_Q_TAG     → LoadRaceQueue (size / 30 entries)
          IP_GRP_Q_TAG   → LoadGroupQueue (size / 13 entries)
          NPC_SHIP_Q_TAG → LoadShipQueue (size / 11 entries)
          SHIP_Q_TAG     → LoadShipQueue (size / 11 entries)
          EVENTS_TAG     → LoadEvent × (size / 5)
          ENCOUNTERS_TAG → LoadEncounter × (size / 65)
          SCAN_TAG       → LoadScanInfo → write to STARINFO state file
          GROUP_LIST_TAG → InitGroupInfo, LoadGroupList → write to RANDGRPINFO
          BATTLE_GROUP_TAG → InitGroupInfo, LoadBattleGroup → write to DEF/RAND
          STAR_TAG       → LoadStarDesc
          default        → skip_8(size) — forward compatibility
  11. Resolve CurStarDescPtr from loaded STAR_DESC
  12. Set up NextActivity flags
```

### 5.2 Legacy Format Support (`load_legacy.c`)

Legacy saves (`starcon2.NN`) use a different format:
- No tagged chunks; data is in fixed sequential order.
- Summary is read uncompressed from the file header.
- Game state and queues are compressed (using the `cread` decode library).
- State file data is embedded as length-prefixed compressed blobs.
- Game state bits are in a different layout (155 bytes, different bit positions for GRPOFFS fields).

**InterpretLegacyGameState()** transposes legacy bit layout to current layout using a `GAMESTATE_TRANSPOSE` table:

```c
static GAMESTATE_TRANSPOSE transpose[] = {
    {   0,   51,   0 },    // Bits 0-51 → target bits 0-51
    { 404,  450,  52 },    // Bits 404-450 → target bits 52-98
    { 483,  878,  99 },    // Bits 483-878 → target bits 99-494
    { 911,  930, 495 },    // Bits 911-930 → target bits 495-514
    { 963, 1237, 515 },    // Bits 963-1237 → target bits 515-789
    {  -1,   -1,  -1 }     // Sentinel
};
```

The DEFGRP offsets (which occupied interleaved positions 52-403 and 451-482 and 879-910 and 931-962 in the old layout) are extracted separately and stored at new dedicated positions.

### 5.3 State Restoration

On load:
1. `GlobData.SIS_state` ← from summary
2. `GlobData.Game_state` ← from GLOBAL_STATE + GAME_STATE chunks
3. All queues reinitialized and populated from chunk data
4. State file buffers rebuilt by writing chunk data via `OpenStateFile("wb")` + `swrite_*`
5. `CurStarDescPtr` resolved by `FindStar()` from loaded coordinates
6. `NextActivity` set from loaded `CurrentActivity`, with `START_INTERPLANETARY` flag added if appropriate

### 5.4 Error Handling

- **File not found**: Falls back to `LoadLegacyGame` automatically.
- **Invalid magic**: Falls back to legacy loader.
- **Corrupt summary**: Returns `FALSE`.
- **Wrong chunk size**: `LoadGameState` returns `FALSE` if GLOBAL_STATE_TAG missing or size ≠ 75.
- **GameState size mismatch**: If chunk is larger than array, reads array size then skips remainder. If smaller, reads only what's available.
- **Unknown chunks**: Skipped with `skip_8(chunkSize)` — forward compatibility.
- **Legacy endianness mismatch**: Detected via `year_index` range check; returns `FALSE` with error log.
- **Write errors during save**: Global `io_ok` boolean tracks any failed write; if `FALSE` at end, save file is deleted and `SaveGame` returns `FALSE`.

---

## 6. State File Call Site Catalog

### OpenStateFile

| File | Line | Context |
|------|------|---------|
| `state.c` | 214 | `InitPlanetInfo()` — opens STARINFO with "wb" to initialize planet info offset table |
| `state.c` | 250 | `GetPlanetInfo()` — opens STARINFO with "rb" to read scan masks |
| `state.c` | 294 | `PutPlanetInfo()` — opens STARINFO with "r+b" to write scan masks |
| `grpinfo.c` | 153 | `InitGroupInfo()` — opens RANDGRPINFO with "wb" to initialize group header |
| `grpinfo.c` | 164 | `InitGroupInfo()` — opens DEFGRPINFO with "wb" (first-time only) to write sentinel byte |
| `grpinfo.c` | 521 | `GetGroupInfo()` — opens DEFGRPINFO with "r+b" for defined groups |
| `grpinfo.c` | 523 | `GetGroupInfo()` — opens RANDGRPINFO with "r+b" for random groups |
| `grpinfo.c` | 564 | `GetGroupInfo()` — opens RANDGRPINFO with "wb" to erase expired groups |
| `grpinfo.c` | 800 | `PutGroupInfo()` — opens DEFGRPINFO with "r+b" for defined groups |
| `grpinfo.c` | 802 | `PutGroupInfo()` — opens RANDGRPINFO with "r+b" for random groups |
| `save.c` | 595 | `SaveStarInfo()` — opens STARINFO with "rb" to serialize to save file |
| `save.c` | 675 | `SaveGroups()` — opens RANDGRPINFO with "rb" to serialize random groups |
| `save.c` | 711 | `SaveGroups()` — opens DEFGRPINFO with "rb" to serialize defined groups |
| `load.c` | 456 | `LoadScanInfo()` — opens STARINFO with "wb" to write scan data from save file |
| `load.c` | 473 | `LoadGroupList()` — opens RANDGRPINFO with "rb" to append group list |
| `load.c` | 529 | `LoadBattleGroup()` — opens DEFGRPINFO with "rb" for defined battle groups |
| `load.c` | 537 | `LoadBattleGroup()` — opens RANDGRPINFO with "rb" for random battle groups |
| `load_legacy.c` | 744 | `LoadLegacyGame()` — opens STARINFO with "wb" to restore from legacy save |
| `load_legacy.c` | 764 | `LoadLegacyGame()` — opens DEFGRPINFO with "wb" to restore from legacy save |
| `load_legacy.c` | 784 | `LoadLegacyGame()` — opens RANDGRPINFO with "wb" to restore from legacy save |

### CloseStateFile

| File | Line | Context |
|------|------|---------|
| `state.c` | 228 | `InitPlanetInfo()` — after writing offset table |
| `state.c` | 285 | `GetPlanetInfo()` — after reading scan masks |
| `state.c` | 351 | `PutPlanetInfo()` — after writing scan masks |
| `grpinfo.c` | 161 | `InitGroupInfo()` — after writing RANDGRPINFO header |
| `grpinfo.c` | 170 | `InitGroupInfo()` — after writing DEFGRPINFO sentinel |
| `grpinfo.c` | 562,568 | `GetGroupInfo()` — closing RANDGRPINFO |
| `grpinfo.c` | 655,659,666 | `GetGroupInfo()` — closing state file after reading |
| `grpinfo.c` | 741 | `GetGroupInfo()` — after reading GROUP_LIST |
| `grpinfo.c` | 788 | `GetGroupInfo()` — after reading specific group |
| `grpinfo.c` | 861 | `PutGroupInfo()` — after writing group data |
| `save.c` | 615 | `SaveStarInfo()` — after reading star info for save |
| `save.c` | 709 | `SaveGroups()` — after reading RANDGRPINFO for save |
| `save.c` | 726 | `SaveGroups()` — after reading DEFGRPINFO for save |
| `load.c` | 466 | `LoadScanInfo()` — after writing scan data |
| `load.c` | 510 | `LoadGroupList()` — after writing group list |
| `load.c` | 590 | `LoadBattleGroup()` — after writing battle group |
| `load_legacy.c` | 760 | `LoadLegacyGame()` — STARINFO restored |
| `load_legacy.c` | 780 | `LoadLegacyGame()` — DEFGRPINFO restored |
| `load_legacy.c` | 800 | `LoadLegacyGame()` — RANDGRPINFO restored |

### DeleteStateFile

| File | Line | Context |
|------|------|---------|
| `state.c` | 235 | `UninitPlanetInfo()` — deletes STARINFO |
| `grpinfo.c` | 177 | `UninitGroupInfo()` — deletes DEFGRPINFO |
| `grpinfo.c` | 178 | `UninitGroupInfo()` — deletes RANDGRPINFO |

### SeekStateFile

| File | Line | Context |
|------|------|---------|
| `state.c` | 265 | `GetPlanetInfo()` — seek to star's offset entry |
| `state.c` | 280 | `GetPlanetInfo()` — seek to planet's scan record |
| `state.c` | 310,323,327,347 | `PutPlanetInfo()` — seek for read/write of offset and scan data |
| `grpinfo.c` | 433,448,495 | `FlushGroupInfo()` — seek for writing headers and group data |
| `grpinfo.c` | 528,586 | `GetGroupInfo()` — seek for reading headers and groups |
| `grpinfo.c` | 682,701 | `GetGroupInfo()` — seek for GROUP_LIST reading |
| `grpinfo.c` | 769 | `GetGroupInfo()` — seek for specific group reading |
| `grpinfo.c` | 810,823 | `PutGroupInfo()` — seek for reading/writing headers |
| `save.c` | 625,630,648 | `SaveBattleGroup()` — seek for reading group data |
| `save.c` | 683 | `SaveGroups()` — seek for reading group list |
| `load.c` | 484,486 | `LoadGroupList()` — seek for header update |
| `load.c` | 555,565,588 | `LoadBattleGroup()` — seek for writing group data |

### ReadStateFile / WriteStateFile

Used indirectly through `sread_*`/`swrite_*` helpers at all the above call sites, plus directly:

| File | Line | Context |
|------|------|---------|
| `load_legacy.c` | 756 | `WriteStateFile` — copying compressed STARINFO data |
| `load_legacy.c` | 776 | `WriteStateFile` — copying compressed DEFGRPINFO data |
| `load_legacy.c` | 796 | `WriteStateFile` — copying compressed RANDGRPINFO data |

---

## 7. Game State Macro Usage Analysis

### By File (Top 20)

| File | Count | Subsystem |
|------|-------|-----------|
| `comm/starbas/starbas.c` | 138 | Starbase dialog |
| `comm/druuge/druugec.c` | 97 | Druuge dialog |
| `comm/utwig/utwigc.c` | 96 | Utwig dialog |
| `comm/melnorm/melnorm.c` | 96 | Melnorme dialog |
| `comm/thradd/thraddc.c` | 85 | Thraddash dialog |
| `comm/arilou/arilouc.c` | 81 | Arilou dialog |
| `comm/pkunk/pkunkc.c` | 80 | Pkunk dialog |
| `comm/spahome/spahome.c` | 71 | Spathi homeworld dialog |
| `comm/slyhome/slyhome.c` | 66 | Slylandro homeworld dialog |
| `comm/orz/orzc.c` | 64 | Orz dialog |
| `comm/yehat/yehatc.c` | 59 | Yehat dialog |
| `comm/supox/supoxc.c` | 57 | Supox dialog |
| `planets/devices.c` | 54 | Device handling |
| `gameev.c` | 50 | Game events/clock |
| `comm/vux/vuxc.c` | 46 | VUX dialog |
| `comm/syreen/syreenc.c` | 44 | Syreen dialog |
| `comm/talkpet/talkpet.c` | 41 | Talking Pet dialog |
| `hyper.c` | 39 | HyperSpace navigation |
| `comm/comandr/comandr.c` | 37 | Commander dialog |
| `comm/mycon/myconc.c` | 35 | Mycon dialog |

### By Subsystem

| Subsystem | Files | Total Usage |
|-----------|-------|-------------|
| **Alien Communication** | 28 files in `comm/` | ~1,467 |
| **Planet Generation** | 22 files in `planets/generate/` | ~150 |
| **Planets/Lander** | `devices.c`, `lander.c`, `solarsys.c`, `scan.c`, `pstarmap.c` | ~82 |
| **Core Gameplay** | `hyper.c`, `gameev.c`, `encount.c`, `comm.c`, `commglue.c`, `starcon.c`, `battle.c` | ~129 |
| **UI/Outfitting** | `starbase.c`, `shipyard.c`, `outfit.c`, `gameopt.c`, `sis.c` | ~32 |
| **Save/Load** | `save.c`, `load.c`, `globdata.c`, `grpinfo.c` | ~14 |
| **Debug** | `uqmdebug.c` | 33 |
| **Ships** | `sis_ship.c`, `shofixti.c`, `lastbat.c`, `ilwrath.c` | ~7 |
| **Other** | `restart.c`, `ipdisp.c`, `galaxy.c`, `starmap.c` | ~16 |

---

## 8. Requirements (EARS Format)

### REQ-SFILE: State File I/O Requirements

#### Open/Close Semantics

**REQ-SFILE-001**: The state file system shall maintain exactly three in-memory state files: STARINFO (index 0), RANDGRPINFO (index 1), and DEFGRPINFO (index 2).

**REQ-SFILE-002**: When `OpenStateFile` is called with a valid index (0–2), the system shall return a pointer to the corresponding state file and increment its open count.

**REQ-SFILE-003**: If `OpenStateFile` is called with an index outside the range [0, 2], then the system shall return NULL.

**REQ-SFILE-004**: When `OpenStateFile` is called for a file that has no allocated buffer, the system shall allocate a buffer of `size_hint` bytes using `HMalloc`.

**REQ-SFILE-005**: If buffer allocation fails during `OpenStateFile`, then the system shall return NULL.

**REQ-SFILE-006**: When `OpenStateFile` is called with mode starting with `'w'`, the system shall reset the logical file size (`used`) to zero and set the cursor to zero.

**REQ-SFILE-007**: When `OpenStateFile` is called with mode starting with `'r'`, the system shall preserve existing file content and set the cursor to zero.

**REQ-SFILE-008**: While a state file's open count exceeds 1 after an open, the system shall emit a warning log message.

**REQ-SFILE-009**: When `CloseStateFile` is called, the system shall decrement the open count and reset the cursor to zero without freeing the buffer.

**REQ-SFILE-010**: While a state file's open count is below 0 after a close, the system shall emit a warning log message.

#### Read/Write Operations

**REQ-SFILE-011**: When `ReadStateFile` is called with size and count, the system shall read up to `size × count` bytes from the current cursor position, copying data into the caller's buffer.

**REQ-SFILE-012**: When reading would exceed the allocated buffer size, the system shall truncate to available bytes, rounded down to a whole number of elements.

**REQ-SFILE-013**: When the cursor is at or past the allocated buffer size during a read, the system shall return 0 (EOF).

**REQ-SFILE-014**: The system shall return the number of complete elements read (total bytes read / size).

**REQ-SFILE-015**: When `WriteStateFile` is called, the system shall write `size × count` bytes at the current cursor position.

**REQ-SFILE-016**: When writing would exceed the allocated buffer size, the system shall grow the buffer to at least `cursor + bytes`, preferring `current_size × 1.5` if that is larger.

**REQ-SFILE-017**: If buffer reallocation fails during write, then the system shall return 0.

**REQ-SFILE-018**: When a write advances the cursor past the current logical file size (`used`), the system shall update `used` to the new cursor position.

**REQ-SFILE-019**: When the buffer grows beyond the original `size_hint`, the system shall update `size_hint` to the new size.

#### Seek Operations

**REQ-SFILE-020**: When `SeekStateFile` is called with `SEEK_SET`, the system shall set the cursor to the specified offset.

**REQ-SFILE-021**: When `SeekStateFile` is called with `SEEK_CUR`, the system shall add the offset to the current cursor position.

**REQ-SFILE-022**: When `SeekStateFile` is called with `SEEK_END`, the system shall set the cursor to `used + offset`.

**REQ-SFILE-023**: If a seek would result in a negative cursor position, then the system shall clamp the cursor to 0 and return 0.

**REQ-SFILE-024**: When a seek results in a non-negative cursor position, the system shall return 1.

#### File Length

**REQ-SFILE-025**: The `LengthStateFile` function shall return the logical file size (highest byte ever written), not the allocated buffer size.

#### Delete

**REQ-SFILE-026**: When `DeleteStateFile` is called with a valid index, the system shall free the data buffer, reset `used` and `ptr` to 0, and set the data pointer to NULL.

**REQ-SFILE-027**: If `DeleteStateFile` is called while the file's open count is non-zero, then the system shall emit a warning log message.

**REQ-SFILE-028**: If `DeleteStateFile` is called with an index outside [0, 2], then the system shall do nothing.

### REQ-STATE: Game State Bits Requirements

#### Bit Get/Set

**REQ-STATE-001**: The `getGameState` function shall extract up to 8 bits from the `GameState[]` byte array at the specified bit range [startBit, endBit] and return them as a BYTE.

**REQ-STATE-002**: When a bit field is contained within a single byte (startBit/8 == endBit/8), the system shall extract bits by right-shifting and masking within that byte.

**REQ-STATE-003**: When a bit field spans two bytes, the system shall combine the high bits of the first byte with the low bits of the second byte.

**REQ-STATE-004**: The `setGameState` function shall write up to 8 bits into the `GameState[]` byte array, clearing the target bits first, then setting the new value.

**REQ-STATE-005**: When setting a field that spans two bytes, the system shall update both bytes correctly, preserving adjacent bits.

#### 32-bit Get/Set

**REQ-STATE-006**: The `getGameState32` function shall read 32 bits from the `GameState[]` array as four consecutive 8-bit reads in little-endian order.

**REQ-STATE-007**: The `setGameState32` function shall write a 32-bit value into the `GameState[]` array as four consecutive 8-bit writes in little-endian order.

#### Bit Copy

**REQ-STATE-008**: The `copyGameState` function shall copy a range of bits from a source state array to a target state array, processing 8 bits at a time.

**REQ-STATE-009**: When the remaining bits in a `copyGameState` operation are fewer than 8, the system shall copy only the remaining bits.

#### State Reset

**REQ-STATE-010**: When a game is loaded, the system shall zero-fill the entire `GameState[]` array before populating it from save data.

### REQ-SAVE: Save/Load Requirements

#### Serialization

**REQ-SAVE-001**: The save system shall write all multi-byte values in little-endian byte order.

**REQ-SAVE-002**: The save file shall begin with a 4-byte magic tag (`SAVEFILE_TAG` = 0x01534d55).

**REQ-SAVE-003**: Each data section after the magic tag shall be preceded by a 4-byte tag and a 4-byte chunk size.

**REQ-SAVE-004**: The Summary chunk (tag `SUMMARY_TAG`) shall be the first chunk and shall contain the SIS_STATE, activity, date, Melnorme credits, ship list, device list, and variable-length save name.

**REQ-SAVE-005**: The Global State chunk (tag `GLOBAL_STATE_TAG`, fixed 75 bytes) shall be the second chunk and shall contain game configuration, clock state, ship position, and velocity.

**REQ-SAVE-006**: The Game State Bits chunk (tag `GAME_STATE_TAG`) shall be the third chunk and shall contain the raw `GameState[]` byte array.

**REQ-SAVE-007**: The system shall serialize the contents of all three state file buffers (STARINFO, RANDGRPINFO, DEFGRPINFO) into appropriate tagged chunks (`SCAN_TAG`, `GROUP_LIST_TAG`, `BATTLE_GROUP_TAG`).

**REQ-SAVE-008**: The system shall only write queue chunks (RACE_Q, SHIP_Q, etc.) when the queue is non-empty.

#### Save Creation

**REQ-SAVE-009**: When `SaveGame` is called, the system shall call `SaveFlagshipState()` to snapshot the current ship position before serializing.

**REQ-SAVE-010**: When the player is in interplanetary space without an active encounter, the system shall call `PutGroupInfo(GROUPS_RANDOM, GROUP_SAVE_IP)` to flush IP group state before saving.

**REQ-SAVE-011**: The save file shall be named `uqmsave.NN` where NN is the zero-padded save slot number.

**REQ-SAVE-012**: If any write operation fails during save (io_ok becomes FALSE), then the system shall delete the partially written save file and return FALSE.

**REQ-SAVE-013**: The system shall track write success via a global `io_ok` boolean, checked after every individual write operation.

#### Load Restoration

**REQ-SAVE-014**: When `LoadGame` is called, the system shall attempt to open `uqmsave.NN` first.

**REQ-SAVE-015**: If the new-format file does not exist or has an invalid header, then the system shall automatically fall back to `LoadLegacyGame` to try loading `starcon2.NN`.

**REQ-SAVE-016**: When loading a summary only (SummPtr non-NULL), the system shall read only the summary chunk and return without loading full game state.

**REQ-SAVE-017**: When loading full game state, the system shall reinitialize all queues before populating them.

**REQ-SAVE-018**: When a `GAME_STATE_TAG` chunk is larger than the current `GameState[]` array, the system shall read only `sizeof(GameState)` bytes and skip the remainder.

**REQ-SAVE-019**: When a `GAME_STATE_TAG` chunk is smaller than the current `GameState[]` array, the system shall read only the available bytes (remaining bytes stay zero from the prior memset).

**REQ-SAVE-020**: When an unknown chunk tag is encountered during loading, the system shall skip `chunkSize` bytes and continue processing.

#### Legacy Detection

**REQ-SAVE-021**: The legacy loader shall validate `year_index` against the range `[START_YEAR, START_YEAR + YEARS_TO_KOHRAH_VICTORY + 27)` to detect endianness incompatibilities.

**REQ-SAVE-022**: When loading legacy saves, the system shall decompress data using the `cread`/`copen`/`cclose` decode library.

**REQ-SAVE-023**: When loading legacy game state bits, the system shall transpose bit positions from the legacy layout to the current layout using the `transpose[]` table and re-extract DEFGRP offsets from their old interleaved positions.

#### Error Handling

**REQ-SAVE-024**: If the save directory cannot be opened or the file cannot be created, then `SaveGame` shall return FALSE.

**REQ-SAVE-025**: If `LoadSummary` fails (corrupt header, unreadable fields), then `LoadGame` shall close the file and return FALSE.

**REQ-SAVE-026**: If `LoadGameState` fails (wrong tag or size), then `LoadGame` shall close the file and return FALSE.

**REQ-SAVE-027**: If a chunk read fails mid-stream during loading, then `LoadGame` shall close the file and return FALSE.

**REQ-SAVE-028**: If `LoadLegacyGame` detects an endianness-incompatible save (year_index out of range), then the system shall log a warning and return FALSE.

#### State File Reconstruction on Load

**REQ-SAVE-029**: When a `SCAN_TAG` chunk is encountered, the system shall open STARINFO_FILE with mode "wb" and write the chunk data as DWORD values.

**REQ-SAVE-030**: When a `GROUP_LIST_TAG` chunk is encountered, the system shall read the existing RANDGRPINFO header, append the group list data, and update the header's GroupOffset[0].

**REQ-SAVE-031**: When a `BATTLE_GROUP_TAG` chunk with `encounter_id == 0` is encountered, the system shall write the battle group to RANDGRPINFO_FILE.

**REQ-SAVE-032**: When a `BATTLE_GROUP_TAG` chunk with `encounter_id > 0` is encountered, the system shall write the battle group to DEFGRPINFO_FILE and update the corresponding `*_GRPOFFS` game state bits with the file offset.

**REQ-SAVE-033**: When the first group-related chunk is encountered during loading, the system shall call `InitGroupInfo(TRUE)` and reset `BattleGroupRef` before processing.

**REQ-SAVE-034**: When a battle group chunk has `current == 1`, the system shall set `GLOBAL(BattleGroupRef)` to the written offset.
