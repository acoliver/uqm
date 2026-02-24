# UQM Rust State System — Functional & Technical Specification

## 1. Purpose and Scope

### 1.1 What This Document Covers

This specification defines the **behavioral contract** for a Rust replacement of the C state and save/load system in The Ur-Quan Masters (UQM). It describes:

- **State File I/O**: An in-memory virtual file system providing fread/fwrite-style access to three named binary buffers. These buffers hold star system scan data, random encounter group info, and scripted encounter group info.
- **Game State Bits**: A packed bit-field array storing ~448 named game flags (1238 bits / 155 bytes) accessed via bit-manipulation functions.
- **FFI Boundary**: The C-compatible interface through which existing C code calls into the Rust system without modification.

### 1.2 What This Document Does NOT Cover

- **Implementation details**: Data structure choices, algorithms, module layout, or Rust idioms are not prescribed. The spec defines *what* the system must do, not *how*.
- **Timeline or estimates**: No scheduling information.
- **Game logic**: Quest progression, dialog branching, combat mechanics, and other game systems that *use* state are out of scope.
- **Save/load of queues, encounters, and star descriptors**: The spec covers state file serialization and game state bit serialization. Queue serialization (race_q, ship_q, event_q, encounter_q) is a separate concern that uses different data structures.

---

## 2. System Context

### 2.1 Role in UQM

The state system is one of the two most central data systems in UQM (the other being the resource/content system). It provides:

1. **Runtime game flag storage** — Every dialog branch, quest progression flag, item possession flag, alliance status, and encounter state is stored as packed bits in a single byte array. 1,964 macro call sites across 83 C source files read and write these flags.
2. **In-memory virtual file I/O** — Three named binary buffers store structured data about star systems (scan masks), random encounter groups, and scripted encounter groups. ~103 call sites across 6 C source files use this subsystem.
3. **Save/load persistence** — Both subsystems serialize into and deserialize from a tagged-chunk save file format.

### 2.2 Subsystem Relationship

```
┌─────────────────────────────────────────────────────┐
│                   C Game Code                        │
│  83 files, 1964 macro sites  │  6 files, ~103 sites │
│  GET/SET_GAME_STATE macros   │  State File API calls │
└──────────┬───────────────────┴──────────┬────────────┘
           │                              │
     ┌─────▼──────┐                ┌──────▼──────────┐
     │ FFI / Macro │                │ FFI / Function  │
     │  Redirect   │                │   Redirect      │
     └─────┬──────┘                └──────┬──────────┘
           │                              │
     ┌─────▼──────────────────────────────▼────────────┐
     │              Rust State System                   │
     │  ┌─────────────────┐  ┌───────────────────────┐ │
     │  │  Game State Bits │  │   State File I/O      │ │
     │  │  (155-byte array)│  │   (3 memory buffers)  │ │
     │  └────────┬────────┘  └───────────┬───────────┘ │
     │           │                       │              │
     │     ┌─────▼───────────────────────▼─────┐       │
     │     │       Save / Load Integration      │       │
     │     │  (serialization to/from disk file) │       │
     │     └────────────────────────────────────┘       │
     └──────────────────────────────────────────────────┘
```

### 2.3 The FFI Boundary

C callers interact with the Rust state system through two mechanisms:

1. **Game State Bits** — The C macros `GET_GAME_STATE(name)` and `SET_GAME_STATE(name, value)` expand to function calls passing compile-time bit indices. Under the Rust backend, these macros redirect to Rust FFI functions that accept the same integer parameters.

2. **State File I/O** — The seven C functions (`OpenStateFile`, `CloseStateFile`, `ReadStateFile`, `WriteStateFile`, `SeekStateFile`, `LengthStateFile`, `DeleteStateFile`) redirect to Rust FFI equivalents. The C `GAME_STATE_FILE*` opaque pointer type is translated to a file index at the redirect boundary.

The boundary is controlled by a compile-time `USE_RUST_STATE` flag. When not defined, the original C implementation is used. When defined, the C functions redirect to Rust. **No C caller file requires modification.**

---

## 3. State File I/O Public API

### 3.1 Overview

The state file system provides three in-memory binary buffers accessible through a `FILE*`-like API. These are **not** disk files — they are heap-allocated byte arrays with a read/write cursor. Data persists across open/close cycles (close does not free the buffer). The buffers are serialized to disk only during `SaveGame` and restored from disk during `LoadGame`.

### 3.2 Function Specifications

#### 3.2.1 OpenStateFile

**C Signature:**
```c
GAME_STATE_FILE* OpenStateFile(int stateFile, const char *mode);
```

**Rust FFI Signature:**
```c
// At the C redirect layer, GAME_STATE_FILE* is translated to/from file_index
extern int rust_open_state_file(int file_index, const char *mode);
// Returns: 1 on success, 0 on failure
```

**Behavioral Contract:**
1. Validates `stateFile` is in range [0, 2]. If out of range, returns NULL (C) / 0 (Rust FFI).
2. Increments the file's `open_count`.
3. If `open_count > 1` after increment, emits a warning log message.
4. If the file has no allocated buffer, allocates one of `size_hint` bytes. If allocation fails, returns NULL/0.
5. If `mode` starts with `'w'`: resets logical file size (`used`) to 0. Cursor set to 0.
6. If `mode` starts with `'r'`: preserves existing content. Cursor set to 0.
7. Regardless of mode, read and write operations are always permitted on the returned handle.
8. Returns a pointer to the file (C) or success indicator (Rust FFI).

**Edge Cases:**
- Mode `"r+b"`: treated as read mode (preserves content), but write operations work normally.
- Mode with unrecognized first character: emits a warning, but the file is still opened (content preserved, cursor reset to 0).
- Opening an already-open file: allowed, increments `open_count`, resets cursor.

#### 3.2.2 CloseStateFile

**C Signature:**
```c
void CloseStateFile(GAME_STATE_FILE *fp);
```

**Rust FFI Signature:**
```c
extern void rust_close_state_file(int file_index);
```

**Behavioral Contract:**
1. Resets the cursor to 0.
2. Decrements `open_count`.
3. If `open_count < 0` after decrement, emits a warning log message.
4. Does **not** free the buffer. Data persists.

**Edge Cases:**
- Closing a file that was never opened: `open_count` goes negative, warning emitted.
- The buffer remains valid after close; a subsequent open will find existing data intact (unless opened with `"w"` mode).

#### 3.2.3 ReadStateFile

**C Signature:**
```c
int ReadStateFile(void *lpBuf, COUNT size, COUNT count, GAME_STATE_FILE *fp);
```

**Rust FFI Signature:**
```c
extern size_t rust_read_state_file(int file_index, uint8_t *buf, size_t size, size_t count);
```

**Behavioral Contract:**
1. Attempts to read `size × count` bytes from the current cursor position.
2. Reads against the **physical allocation size** (`size`), not the logical size (`used`). This means bytes between `used` and the physical allocation can be read (they contain uninitialized/zero-fill/debug-paint data).
3. If `cursor >= physical_size`: returns 0 (EOF).
4. If `cursor + requested_bytes > physical_size`: truncates to available bytes, rounded down to a whole number of elements (`(available_bytes / size) * size`).
5. Copies data from buffer to caller's `lpBuf` via memcpy-equivalent.
6. Advances cursor by the number of bytes actually read.
7. Returns the number of complete elements read (`bytes_read / size`).

**Edge Cases:**
- `size == 0`: returns 0.
- `count == 0`: returns 0.
- `buf` is NULL: undefined behavior in C; Rust FFI returns 0.
- Cursor positioned past physical size (via seek-past-end followed by buffer growth via write on another handle): reads from extended buffer.

#### 3.2.4 WriteStateFile

**C Signature:**
```c
int WriteStateFile(const void *lpBuf, COUNT size, COUNT count, GAME_STATE_FILE *fp);
```

**Rust FFI Signature:**
```c
extern size_t rust_write_state_file(int file_index, const uint8_t *buf, size_t size, size_t count);
```

**Behavioral Contract:**
1. Writes `size × count` bytes at the current cursor position.
2. If `cursor + bytes > physical_size`: grows the buffer. New size is `max(cursor + bytes, physical_size × 3/2)`. If reallocation fails, returns 0.
3. If the new physical size exceeds the stored `size_hint`, updates `size_hint` to the new size.
4. Copies data from caller's buffer to the file buffer via memcpy-equivalent.
5. Advances cursor by the number of bytes written.
6. If cursor now exceeds `used` (logical file size), updates `used` to the new cursor position.
7. Returns the number of complete elements written (`bytes / size`). Under normal conditions (no allocation failure), this equals `count`.

**Edge Cases:**
- Writing at a position past `used` but within physical allocation: the gap between old `used` and the write position contains uninitialized data. `used` jumps to `cursor + bytes`.
- Writing at a position past physical allocation (after seek-past-end): buffer grows to accommodate. Gap is zero-filled by the allocation strategy.

#### 3.2.5 SeekStateFile

**C Signature:**
```c
int SeekStateFile(GAME_STATE_FILE *fp, long offset, int whence);
```

**Rust FFI Signature:**
```c
extern int rust_seek_state_file(int file_index, int64_t offset, int whence);
```

**Behavioral Contract:**
1. `whence == SEEK_SET (0)`: new position = `offset`.
2. `whence == SEEK_CUR (1)`: new position = `cursor + offset`.
3. `whence == SEEK_END (2)`: new position = `used + offset`.
4. If new position < 0: clamps cursor to 0, returns 0.
5. If new position ≥ 0: sets cursor to new position, returns 1.
6. **The cursor may exceed both `used` and physical buffer size.** There is no upper-bound clamp. A subsequent write at a position past the physical size will grow the buffer. A subsequent read at a position past the physical size will return 0 (EOF).

**Critical Note — Known Blocker:** The existing Rust implementation clamps the cursor to `data.len()`. This is **incorrect**. The C implementation allows `ptr` to be set to any non-negative value without clamping. The group info subsystem (`grpinfo.c`) relies on seeking past the current data length and then writing at that position, which auto-extends the buffer. The Rust implementation must allow seek-past-end.

#### 3.2.6 LengthStateFile

**C Signature:**
```c
DWORD LengthStateFile(GAME_STATE_FILE *fp);
```

**Rust FFI Signature:**
```c
extern size_t rust_length_state_file(int file_index);
```

**Behavioral Contract:**
1. Returns the logical file size (`used`): the highest byte position ever written.
2. This is distinct from the physical allocation size.
3. Does not modify the cursor or any other state.

#### 3.2.7 DeleteStateFile

**C Signature:**
```c
void DeleteStateFile(int stateFile);
```

**Rust FFI Signature:**
```c
extern void rust_delete_state_file(int file_index);
```

**Behavioral Contract:**
1. Validates `stateFile` is in range [0, 2]. If out of range, does nothing.
2. If `open_count != 0`, emits a warning log message.
3. Resets `used` and `ptr` to 0.
4. Frees the data buffer and sets the data pointer to NULL.
5. After deletion, a subsequent `OpenStateFile` will allocate a fresh buffer.

### 3.3 Serialization Helpers

The C header `state.h` defines inline helper functions that wrap `ReadStateFile`/`WriteStateFile`:

| Helper | Behavior |
|--------|----------|
| `sread_8(fp, &v)` | `ReadStateFile(v, 1, 1, fp)` — reads 1 byte |
| `sread_16(fp, &v)` | `ReadStateFile(v, 2, 1, fp)` — reads 2 bytes, native endian |
| `sread_16s(fp, &v)` | `sread_16` + cast to signed |
| `sread_32(fp, &v)` | `ReadStateFile(v, 4, 1, fp)` — reads 4 bytes, native endian |
| `sread_a32(fp, ar, count)` | Loop of `sread_32` — reads array of DWORDs |
| `swrite_8(fp, v)` | `WriteStateFile(&v, 1, 1, fp)` — writes 1 byte |
| `swrite_16(fp, v)` | `WriteStateFile(&v, 2, 1, fp)` — writes 2 bytes, native endian |
| `swrite_32(fp, v)` | `WriteStateFile(&v, 4, 1, fp)` — writes 4 bytes, native endian |
| `swrite_a32(fp, ar, count)` | Loop of `swrite_32` — writes array of DWORDs |

**Endianness:** These helpers perform raw memory copies — no byte-swapping. The state file buffers are in-memory only and always match host byte order. This is safe because state files are never directly persisted to disk; the save/load system uses a separate byte-order-aware serialization layer.

**Rust interaction:** These helpers remain as C inline functions in `state.h`. They call `ReadStateFile`/`WriteStateFile`, which redirect to Rust. The Rust system does not need to provide equivalents — the C wrappers call through the redirected functions. Rust-only code that needs similar functionality uses `read_u32_le`/`write_u32_le` etc.

---

## 4. Game State Bits Public API

### 4.1 Overview

Game state is stored as a **packed bit array** in a `BYTE GameState[]` array of 155 bytes (1238 bits). 448 named fields are defined, each occupying 1–8 bits. Fields are packed contiguously with no padding. Access is through macros that expand to bit-manipulation function calls.

### 4.2 Bit Field Layout

Fields are defined using preprocessor macros that create a C enum:

```c
#define START_GAME_STATE enum {
#define ADD_GAME_STATE(SName, NumBits) SName, END_##SName = SName + NumBits - 1,
#define END_GAME_STATE NUM_GAME_STATE_BITS };
```

Each `ADD_GAME_STATE(NAME, N)` creates:
- `NAME` — the starting bit index (auto-incremented from previous field's end + 1)
- `END_NAME` — the ending bit index (inclusive) = `NAME + N - 1`

The final enum value `NUM_GAME_STATE_BITS` equals **1238**. The byte array size is `(1238 + 7) >> 3` = **155 bytes**.

### 4.3 Macro Definitions

```c
#define GET_GAME_STATE(SName) \
    getGameState(GLOBAL(GameState), (SName), (END_##SName))

#define SET_GAME_STATE(SName, val) \
    setGameState(GLOBAL(GameState), (SName), (END_##SName), (val))

#define GET_GAME_STATE_32(SName) \
    getGameState32(GLOBAL(GameState), (SName))

#define SET_GAME_STATE_32(SName, val) \
    setGameState32(GLOBAL(GameState), (SName), (val))
```

Under `USE_RUST_STATE`, these macros redirect to Rust FFI functions:

```c
#define GET_GAME_STATE(SName) \
    rust_get_game_state_bits((SName), (END_##SName))

#define SET_GAME_STATE(SName, val) \
    rust_set_game_state_bits((SName), (END_##SName), (val))

#define GET_GAME_STATE_32(SName) \
    rust_get_game_state_32((SName))

#define SET_GAME_STATE_32(SName, val) \
    rust_set_game_state_32((SName), (val))
```

### 4.4 Function Specifications

#### 4.4.1 getGameState / rust_get_game_state_bits

**C Signature:**
```c
BYTE getGameState(BYTE *state, int startBit, int endBit);
```

**Rust FFI Signature:**
```c
extern uint8_t rust_get_game_state_bits(int start_bit, int end_bit);
```

**Behavioral Contract:**
1. Extracts up to 8 bits from the backing byte array at the bit range `[startBit, endBit]` inclusive.
2. `endBit - startBit + 1` must be ≤ 8.
3. If the field is contained within a single byte (`startBit / 8 == endBit / 8`):
   - Right-shift the byte by `startBit % 8`, then mask to the field width.
4. If the field spans two bytes (`startBit / 8 < endBit / 8`):
   - Extract the high bits of the first byte (right-shifted by `startBit % 8`).
   - Extract the low bits of the second byte.
   - Combine with OR and mask to the field width.
5. Returns the extracted value as a `BYTE` (u8).

**Rust FFI Difference:** The Rust version operates on a global byte array (not a passed-in pointer). The `state` parameter is implicit.

#### 4.4.2 setGameState / rust_set_game_state_bits

**C Signature:**
```c
void setGameState(BYTE *state, int startBit, int endBit, BYTE val);
```

**Rust FFI Signature:**
```c
extern void rust_set_game_state_bits(int start_bit, int end_bit, uint8_t value);
```

**Behavioral Contract:**
1. Writes up to 8 bits into the backing byte array at bit range `[startBit, endBit]` inclusive.
2. Clears the target bits first (AND with inverted mask), then sets the new value (OR with shifted value).
3. If the field spans two bytes, both bytes are updated correctly.
4. Adjacent bits outside the field must be preserved exactly.

#### 4.4.3 getGameState32 / rust_get_game_state_32

**C Signature:**
```c
DWORD getGameState32(BYTE *state, int startBit);
```

**Rust FFI Signature:**
```c
extern uint32_t rust_get_game_state_32(int start_bit);
```

**Behavioral Contract:**
1. Reads 32 bits as four consecutive 8-bit reads in little-endian order.
2. `result = byte0 | (byte1 << 8) | (byte2 << 16) | (byte3 << 24)`.
3. Each byte is read via the 8-bit getter at offsets `startBit`, `startBit+8`, `startBit+16`, `startBit+24`.

#### 4.4.4 setGameState32 / rust_set_game_state_32

**C Signature:**
```c
void setGameState32(BYTE *state, int startBit, DWORD val);
```

**Rust FFI Signature:**
```c
extern void rust_set_game_state_32(int start_bit, uint32_t value);
```

**Behavioral Contract:**
1. Writes a 32-bit value as four consecutive 8-bit writes in little-endian order.
2. Byte 0 = `val & 0xFF`, byte 1 = `(val >> 8) & 0xFF`, etc.
3. Each byte is written via the 8-bit setter at offsets `startBit`, `startBit+8`, `startBit+16`, `startBit+24`.

#### 4.4.5 copyGameState

**C Signature:**
```c
void copyGameState(BYTE *dest, DWORD target, BYTE *src, DWORD begin, DWORD end);
```

**Rust FFI Signature:**
```c
extern void rust_copy_game_state(int dest_bit, int src_start_bit, int src_end_bit);
```

**Behavioral Contract:**
1. Copies a range of bits `[begin, end)` from a source state array to a destination state array starting at `target`.
2. Processes 8 bits at a time. If fewer than 8 bits remain, copies only the remainder.
3. Used exclusively by the legacy save loader to transpose old bit layouts.

**Critical Note — Known Blocker:** The existing Rust FFI implementation of `rust_copy_game_state` deadlocks because it attempts to lock `GLOBAL_GAME_STATE` twice (once for reading the source, once for writing the destination). Since the source and destination are the same global in the FFI path, this causes a deadlock on a non-reentrant `Mutex`. The fix is to acquire the lock once and operate on the state within a single critical section, or to use a reentrant lock, or to snapshot the source data before mutating.

**Additional Note:** The C `copyGameState` accepts separate `dest` and `src` pointers, which may be different arrays. The legacy loader calls it with `dest = src = GLOBAL(GameState)` (same array, different bit ranges). The Rust FFI version must support self-copy correctly.

### 4.5 Backing Storage

- The Rust system maintains a `[u8; 155]` byte array (or equivalent) as its backing store.
- This array must be byte-for-byte identical to what the C `GameState[]` array would contain for any given sequence of get/set operations.
- The array uses `u8` element types only — never larger types — to ensure endian-neutral bit manipulation.

### 4.6 Multi-Bit Values Spanning Byte Boundaries

Many fields span byte boundaries. Example: a 3-bit field starting at bit 6 occupies bits 6–7 of byte 0 and bit 0 of byte 1.

**Extraction (get):**
```
byte0 = state[0], byte1 = state[1]
low_bits = byte0 >> 6          // bits 6-7 → positions 0-1
high_bits = byte1 & 0x01       // bit 0 → position 2
result = low_bits | (high_bits << (8 - 6 - (0 + 1)))  [simplified from C formula]
mask = (1 << 3) - 1 = 0x07
return result & mask
```

**Insertion (set):** Clear bits 6–7 of byte 0 and bit 0 of byte 1, then OR in the new value at the correct positions.

The bit extraction/insertion logic must match the C implementation exactly, including the specific shift amounts used when combining bytes.

### 4.7 State Synchronization with C

When the Rust backend is active, the canonical `GameState[]` data lives in Rust. However, the C save/load system needs access to the raw bytes. Two synchronization functions are required:

**Export (Rust → C, before save):**
```c
extern const uint8_t* rust_get_game_state_bytes(void);
extern size_t rust_get_game_state_size(void);
```

**Import (C → Rust, after load):**
```c
extern void rust_restore_game_state_from_bytes(const uint8_t *bytes, size_t size);
```

**Reset:**
```c
extern void rust_reset_game_state(void);
```

---

## 5. State File Types

### 5.1 File Index Enumeration

| Index | Constant | Symbolic Name | Description |
|-------|----------|---------------|-------------|
| 0 | `STARINFO_FILE` | `"STARINFO"` | Planet scan mask data — records which biological, mineral, and energy scans have been performed on each planet/moon |
| 1 | `RANDGRPINFO_FILE` | `"RANDGRPINFO"` | Random encounter group data — transient IP (interplanetary) groups that appear and expire |
| 2 | `DEFGRPINFO_FILE` | `"DEFGRPINFO"` | Defined (scripted) encounter group data — persistent battle groups tied to specific story events |

### 5.2 STARINFO_FILE (Index 0)

**Initial Size Hint:** `STAR_BUFSIZE` = `NUM_SOLAR_SYSTEMS × 4 + 3800 × (3 × 4)` bytes. (~50–150 KB depending on star count.)

**Structure:**
- **Header region:** One `DWORD` (4 bytes) per star system, containing the byte offset into the data region where that star's scan records begin. Offset 0 means "no records yet."
- **Data region:** For each visited star system, one scan record per planet and moon. Each scan record is 3 × `DWORD` (12 bytes): `[biological_mask, mineral_mask, energy_mask]`.
- Records for a star system are laid out as: planet 0 record, planet 0's moons records, planet 1 record, planet 1's moons records, etc.

**Access Pattern:** Seek to `star_index × 4`, read the offset DWORD. If non-zero, seek to that offset and skip forward past preceding planets' (and their moons') records to reach the target planet/moon's scan record.

### 5.3 RANDGRPINFO_FILE (Index 1)

**Initial Size Hint:** `RAND_BUFSIZE` = 4096 bytes.

**Structure:**
- **Header:** A `GROUP_HEADER` structure containing group counts, the offset to the group list, and metadata.
- **Group entries:** Variable-size records describing encounter groups (ship types, counts, positions).
- **Group list:** An index of all active random groups.

**Access Pattern:** `GetGroupInfo`/`PutGroupInfo` in `grpinfo.c` perform complex read-modify-write sequences with multiple seeks.

### 5.4 DEFGRPINFO_FILE (Index 2)

**Initial Size Hint:** `DEF_BUFSIZE` = 10240 bytes.

**Structure:** Same layout as RANDGRPINFO but for scripted encounter groups. The first byte is a sentinel (initialized to non-zero on creation). Offsets into this file are stored as 32-bit values in the game state bit array (`*_GRPOFFS0-3` fields).

**Access Pattern:** Similar to RANDGRPINFO. Group offsets are stored in game state bits and used to seek directly to group data.

### 5.5 Path Resolution

There is **no path resolution** for state files. They are purely in-memory buffers identified by integer index (0–2). They never touch the filesystem directly. The `int stateFile` parameter is an array index, validated as `0 <= stateFile < 3`.

---

## 6. In-Memory Buffer Model

### 6.1 Core Principle

State files are **in-memory byte buffers with a position cursor**, not disk files. They provide a `FILE*`-like API over heap-allocated memory.

### 6.2 Buffer Lifecycle

```
                     ┌─────────────────────┐
                     │    Not Allocated     │
                     │  (data = NULL)       │
                     └──────────┬──────────┘
                                │ OpenStateFile (first time)
                                │ allocates size_hint bytes
                                ▼
                     ┌─────────────────────┐
               ┌────▶│     Allocated        │◀────┐
               │     │  (data != NULL)      │     │
               │     │  open_count tracks   │     │
               │     │  concurrent opens    │     │
               │     └──────────┬──────────┘     │
               │                │                 │
               │    Open/Close cycles             │
               │    (data persists)               │
               │                │                 │
               │                │ DeleteStateFile  │
               │                ▼                 │
               │     ┌─────────────────────┐     │
               │     │   Freed / Reset      │     │
               │     │  (data = NULL)       │─────┘
               │     └─────────────────────┘
               │                │ OpenStateFile (re-open)
               └────────────────┘
```

### 6.3 Buffer Operations

- **Open**: Allocates buffer on first use. Optionally clears content (mode "w"). Resets cursor to 0.
- **Read**: Copies bytes from buffer at cursor position to caller's buffer. Advances cursor.
- **Write**: Copies bytes from caller's buffer to buffer at cursor position. Grows buffer if needed. Advances cursor. Updates high-water mark.
- **Seek**: Sets cursor to absolute or relative position. **No upper bound clamp.** Cursor may exceed both `used` and physical allocation size.
- **Close**: Resets cursor to 0. Decrements open count. Buffer remains allocated.
- **Delete**: Frees buffer. Resets all metadata.

### 6.4 Seek-Past-End Behavior

This is a critical behavioral requirement. The C implementation allows:

```c
// C SeekStateFile — no upper bound check
fp->ptr = offset;  // offset can be any non-negative value
```

This means:
- `SeekStateFile(fp, 1000000, SEEK_SET)` is valid even if the buffer has only 100 bytes.
- A subsequent `ReadStateFile` at this position returns 0 (EOF, since cursor ≥ physical size).
- A subsequent `WriteStateFile` at this position grows the buffer to at least 1000000 + write_size, zero-filling the gap.

**Why this matters:** `FlushGroupInfo` in `grpinfo.c` seeks to `LengthStateFile(fp)` (end of data) and then writes, expecting the buffer to grow. Other code seeks to computed offsets that may temporarily exceed the current buffer size.

### 6.5 Read Boundary: Physical vs Logical

`ReadStateFile` checks against the **physical allocation size** (`size` field), not the logical size (`used` field). This means:
- Bytes between `used` and `size` are readable (they contain allocation artifacts: zeroes, 0xCC debug paint, or old data).
- The Rust implementation must replicate this behavior. If using `Vec<u8>` as the backing store (where `len()` tracks `used`), reads must also allow reading the gap between `used` and physical capacity, or the implementation must track `used` and physical allocation separately.

**Practical impact:** In practice, most callers respect the `used` boundary via `LengthStateFile`. But the contract allows reads up to the physical size, and any behavioral difference could cause subtle bugs.

### 6.6 Write Growth Strategy

When a write exceeds the physical buffer:
1. Compute `new_size = max(cursor + bytes, current_physical_size × 3/2)`.
2. Reallocate the buffer to `new_size`.
3. Update `size_hint` if `new_size > size_hint`.
4. If reallocation fails, return 0.

---

## 7. Save/Load Integration

### 7.1 Save Game Process

When `SaveGame` is called:

1. **Pre-save state flush:**
   - `SaveFlagshipState()` snapshots current ship position.
   - If in interplanetary space without active encounter, `PutGroupInfo(GROUPS_RANDOM, GROUP_SAVE_IP)` flushes IP group state to RANDGRPINFO.

2. **File creation:**
   - Save file is named `uqmsave.NN` (NN = zero-padded slot 00–99).
   - Written to the configured `saveDir` directory.

3. **Serialization order:**
   - 4-byte file magic: `SAVEFILE_TAG` (0x01534d55 = "UMS\x01")
   - SUMMARY chunk (tag + size + data)
   - GLOBAL_STATE chunk (tag + size + 75 bytes of game config/clock/position)
   - GAME_STATE chunk (tag + size + raw `GameState[]` byte array)
   - Queue chunks (RACE_Q, SHIP_Q, NPC_SHIP_Q, EVENTS, ENCOUNTERS) — only if non-empty
   - SCAN chunk — serialized from STARINFO state file
   - GROUP_LIST chunk — serialized from RANDGRPINFO state file
   - BATTLE_GROUP chunks — serialized from RANDGRPINFO and DEFGRPINFO state files
   - STAR chunk — star descriptor

4. **State file serialization:** During save, the state file buffers are read via `OpenStateFile(idx, "rb")` / `sread_*` / `CloseStateFile`. The data is re-encoded with explicit little-endian byte order using `write_32` / `write_16` / `write_8` helpers from `save.c`.

5. **Error tracking:** A global `io_ok` boolean is set to `TRUE` before saving and checked after every write. If any write fails, `io_ok` becomes `FALSE`, the partial save file is deleted, and `SaveGame` returns `FALSE`.

### 7.2 Load Game Process

When `LoadGame` is called:

1. **File opening:** Attempts to open `uqmsave.NN`.
2. **Format detection:** Reads first 4 bytes. If they match `SAVEFILE_TAG`, proceeds with new format. Otherwise, falls back to `LoadLegacyGame` for `starcon2.NN` format.
3. **Summary loading:** Reads SUMMARY chunk. If `SummPtr` is non-NULL (display mode), returns summary without loading full state.
4. **Full state loading:**
   - Reinitializes all queues.
   - Zero-fills `GameState[]` array.
   - Reads GLOBAL_STATE chunk (75 bytes).
   - Reads GAME_STATE chunk (raw bytes into `GameState[]`).
   - Processes tagged chunks in a loop until EOF:
     - `SCAN_TAG` → opens STARINFO with "wb", writes scan data
     - `GROUP_LIST_TAG` → reads existing RANDGRPINFO header, appends group list
     - `BATTLE_GROUP_TAG` → writes group to DEFGRPINFO or RANDGRPINFO
     - Queue tags → populates queues
     - `STAR_TAG` → loads star descriptor
     - Unknown tags → skipped (forward compatibility)

5. **State synchronization:** After `GameState[]` is populated from the save file, `rust_restore_game_state_from_bytes()` must be called to sync the Rust state with the loaded bytes.

### 7.3 SUMMARY_DESC Metadata

The summary contains enough information to display a save slot without loading the full game:

| Field | Type | Size | Description |
|-------|------|------|-------------|
| SIS_STATE | struct | 122 bytes | Ship name, commander name, coordinates, fuel, crew, modules, etc. |
| Activity | BYTE | 1 | Current activity enum |
| Flags | BYTE | 1 | Lander shields + upgrades |
| day_index | BYTE | 1 | Game date — day |
| month_index | BYTE | 1 | Game date — month |
| year_index | UWORD | 2 | Game date — year (little-endian) |
| MCreditLo | BYTE | 1 | Melnorme credits (low byte) |
| MCreditHi | BYTE | 1 | Melnorme credits (high byte) |
| NumShips | BYTE | 1 | Escort fleet size |
| NumDevices | BYTE | 1 | Device count |
| ShipList | BYTE[12] | 12 | Escort race IDs |
| DeviceList | BYTE[16] | 16 | Device IDs |
| SaveName | UNICODE[] | variable | Save game name (chunk size = 160 + strlen) |

### 7.4 Endianness Helpers

**Save file (`save.c`)** — writes little-endian:
```c
write_16(fp, v): writes v & 0xFF, then (v >> 8) & 0xFF
write_32(fp, v): writes bytes 0, 1, 2, 3 from least to most significant
```

**Load file (`load.c`)** — reads little-endian:
```c
read_16(fp, &v): reads 2 bytes, reconstructs v = byte0 | (byte1 << 8)
read_32(fp, &v): reads 4 bytes, reconstructs v = byte0 | ... | (byte3 << 24)
```

**State file helpers (`state.h`)** — native endian (raw memcpy):
```c
sread_32(fp, &v): ReadStateFile(v, 4, 1, fp) — no byte-swapping
swrite_32(fp, v): WriteStateFile(&v, 4, 1, fp) — no byte-swapping
```

The distinction matters: state files are in-memory and use native endianness. Save files are on-disk and use explicit little-endian.

### 7.5 Save Slot Directory Structure

- Each save occupies a single file `uqmsave.NN` in `saveDir`.
- Legacy saves use `starcon2.NN`.
- No subdirectories per slot — each save is self-contained.
- Slot numbers range from 00 to 99.

### 7.6 Legacy Format Compatibility

Legacy saves (`starcon2.NN`) use a different format:
- No tagged chunks; fixed sequential layout.
- Data is compressed using the `cread` decode library.
- Game state bits use a **different bit layout** — the legacy loader transposes positions using a `GAMESTATE_TRANSPOSE` table.
- DEFGRP offsets were interleaved with game state bits in old positions; the legacy loader extracts them and stores them at new dedicated positions.

`InterpretLegacyGameState()` calls `copyGameState()` with local byte arrays (not the global `GameState[]`), so it operates entirely in C. The Rust system does not need to handle legacy format conversion directly — it only needs to accept the final byte array after the C legacy loader has finished transposing.

---

## 8. Game State Flag Catalog

### 8.1 Summary Statistics

| Metric | Value |
|--------|-------|
| Total named fields | 448 |
| Total bits | 1238 |
| Byte array size | 155 bytes |
| 1-bit flags | ~280 (62.5%) |
| 2-bit fields | ~70 (15.6%) |
| 3-bit fields | ~65 (14.5%) |
| 4-bit fields | ~15 (3.3%) |
| 5-bit fields | ~8 (1.8%) |
| 8-bit fields | ~10 (2.2%) |
| 32-bit composite fields (via *_GRPOFFS0-3) | 15 groups = 60 × 8-bit entries |

### 8.2 Categories

| Category | Approx Count | Examples |
|----------|-------------|---------|
| Alien race visit counters | ~60 | `SHOFIXTI_VISITS(3)`, `SPATHI_HOME_VISITS(3)` |
| Alien conversation stacks | ~50 | `MELNORME_YACK_STACK0(2)`, `SYREEN_STACK0(2)` |
| Quest/mission progress | ~80 | `CHMMR_BOMB_STATE(2)`, `UTWIG_SUPOX_MISSION(3)` |
| Item possession flags | ~40 | `TALKING_PET_ON_SHIP(1)`, `AQUA_HELIX(1)` |
| Discussion flags | ~25 | `DISCUSSED_PORTAL_SPAWNER(1)`, `DISCUSSED_UTWIG_BOMB(1)` |
| Scan/exploration state | ~15 | `RAINBOW_WORLD0(8)`, `SCANNED_MAIDENS(1)` |
| Combat/encounter state | ~15 | `BATTLE_SEGUE(1)`, `BATTLE_PLANET(8)` |
| Lander upgrades | ~5 | `LANDER_SHIELDS(4)`, `IMPROVED_LANDER_SPEED(1)` |
| Starbase state | ~10 | `STARBASE_AVAILABLE(1)`, `STARBASE_BULLETS0(8)` |
| DEFGRP file offsets | 60 | `SHOFIXTI_GRPOFFS0-3(8)` through `SAMATRA_GRPOFFS0-3(8)` — 15 groups × 4 bytes |
| Temporal state | ~15 | `YEHAT_SHIP_MONTH(4)`, `PKUNK_SHIP_YEAR(5)` |
| Misc gameplay flags | ~75 | `PLANETARY_LANDING(1)`, `KOHR_AH_FRENZY(1)` |

### 8.3 Heavily Used Flags (by Call Site Count)

The communication scripts account for ~61% of all macro invocations. The top files:
- `starbas.c`: 138 sites (Starbase commander dialog)
- `druugec.c`: 97 sites
- `utwigc.c`: 96 sites
- `melnorm.c`: 96 sites
- `thraddc.c`: 85 sites
- `arilouc.c`: 81 sites

### 8.4 DEFGRP Offset Fields

15 defined battle groups store their file offsets as 32-bit values in the game state array:

| Group | Bit Fields |
|-------|-----------|
| SHOFIXTI | `SHOFIXTI_GRPOFFS0-3` (bits 814–845) |
| ZOQFOT | `ZOQFOT_GRPOFFS0-3` (bits 846–877) |
| MELNORME0–8 | `MELNORMEn_GRPOFFS0-3` (9 groups, bits 878–1165) |
| URQUAN_PROBE | `URQUAN_PROBE_GRPOFFS0-3` (bits 1166–1197) |
| COLONY | `COLONY_GRPOFFS0-3` (bits 1198–1229) |
| SAMATRA | `SAMATRA_GRPOFFS0-3` (bits 1230–1261) |

Note: The last GRPOFFS ends at bit 1261, but `NUM_GAME_STATE_BITS` = 1238 based on the enum auto-increment. The discrepancy arises because `END_GAME_STATE` captures the final enum value correctly; the actual last bit used is `END_SAMATRA_GRPOFFS3` = bit 1237, and `NUM_GAME_STATE_BITS` = 1238 (one past the last).

---

## 9. Thread Safety

### 9.1 C Threading Model

The C state system is explicitly **single-threaded**. A comment in `save.c` states: *"If for some insane reason you need to save games in different threads, you'll need to protect your calls to SaveGame with a mutex."* State files have `open_count` tracking but no mutex protection.

### 9.2 Rust Threading Requirements

The Rust implementation must be memory-safe under Rust's ownership model, which requires addressing potential concurrent access even if the game is single-threaded in practice:

1. **Game State Bits:** A single global instance accessed from the game thread only. Interior mutability via `Mutex<Option<GameState>>` or equivalent. The mutex is uncontended in practice but satisfies Rust's `Sync` requirement for statics.

2. **State File Manager:** A single global instance managing three files. Interior mutability via `Mutex<Option<StateFileManager>>` or equivalent. Multiple state files may be "open" simultaneously (the C code opens RANDGRPINFO and DEFGRPINFO in the same function), but this is sequential access within one thread.

3. **Lock Granularity:** The lock protects the entire state file manager (all three files). This is sufficient because access is single-threaded. A per-file lock is not needed but would be acceptable.

4. **Deadlock Avoidance:** Functions must never attempt to acquire the same lock twice within one call stack. This is the root cause of the `rust_copy_game_state` blocker — it attempts to lock `GLOBAL_GAME_STATE` for reading the source and again for writing the destination. The fix is to acquire the lock once.

---

## 10. Error Handling

### 10.1 C Error Conventions

| Function | Success | Failure |
|----------|---------|---------|
| `OpenStateFile` | `GAME_STATE_FILE*` (non-NULL) | `NULL` |
| `CloseStateFile` | void (always succeeds) | — |
| `ReadStateFile` | Element count > 0 | 0 (EOF or error) |
| `WriteStateFile` | Element count > 0 | 0 (allocation failure) |
| `SeekStateFile` | 1 | 0 (clamped to 0) |
| `LengthStateFile` | File length | — |
| `DeleteStateFile` | void | — |
| `SaveGame` | `TRUE` | `FALSE` (I/O error) |
| `LoadGame` | `TRUE` | `FALSE` (corrupt file) |

### 10.2 Rust FFI Error Contract

At the FFI boundary, the Rust system **must** return exactly the same values as the C system for all error conditions:
- `rust_open_state_file`: returns 1 (success) or 0 (failure) — the C redirect layer translates this to `GAME_STATE_FILE*` or `NULL`.
- `rust_read_state_file`: returns element count (0 = EOF/error).
- `rust_write_state_file`: returns element count (0 = allocation failure).
- `rust_seek_state_file`: returns 1 (success) or 0 (clamped to 0).

### 10.3 Internal Rust Error Handling

Internally, Rust code uses `Result<T, StateFileError>` for all fallible operations. At the FFI boundary:
- `Result::Ok(value)` → return value.
- `Result::Err(_)` → return the appropriate failure sentinel (0 or NULL).
- **Panics must never cross the FFI boundary.** All FFI functions must catch panics (via `catch_unwind` or equivalent) and convert them to error return values.
- Poisoned mutexes (from a prior panic) must be handled gracefully — return error values, do not propagate the panic.

---

## 11. Compatibility Constraints

### 11.1 Save File Binary Compatibility

- Save files created by the C implementation must load correctly with the Rust implementation, and vice versa.
- The `GameState[]` byte array is the canonical format: it is written to the save file as raw bytes. Both implementations must produce **byte-for-byte identical** arrays for any given sequence of get/set operations.
- State file buffer contents must be identical for any given sequence of open/read/write/seek/close operations.
- All multi-byte values in the save file are little-endian.

### 11.2 Bit Layout Compatibility

- The Rust game state byte array must use the **same size** as the C array: 155 bytes (1238 bits).
- Bit positions must be identical: `SHOFIXTI_VISITS` is bit 0, `END_SAMATRA_GRPOFFS3` is bit 1237.
- The bit extraction/insertion algorithm must produce identical results for all possible inputs.
- The `NUM_GAME_STATE_BITS` constant in Rust must equal 1238. The existing Rust code uses 2048, which is incorrect (the extra bytes are harmless for bit operations but would be wrong for serialization size).

### 11.3 C Caller Compatibility

- All 1,964 macro call sites across 83 C source files must work without any source modifications.
- All ~103 state file API call sites across 6 C source files must work without modification.
- The `GAME_STATE_FILE*` opaque pointer type must be handled transparently — callers pass pointers returned by `OpenStateFile` to other functions. The redirect layer must translate between pointers and indices.
- The `sread_*`/`swrite_*` inline helpers in `state.h` must continue to work — they call `ReadStateFile`/`WriteStateFile` which redirect to Rust.

### 11.4 Legacy Save Compatibility

- Legacy saves (`starcon2.NN`) use `load_legacy.c`, which calls `getGameState`/`setGameState`/`copyGameState` on **local byte arrays** (not the global `GameState[]`). These C functions must remain available for legacy loading regardless of `USE_RUST_STATE`.
- After legacy loading transposes the bit layout, the final `GameState[]` bytes are synced to Rust via `rust_restore_game_state_from_bytes`.

---

## 12. Requirements (EARS Format)

### REQ-SFILE: State File I/O Requirements

#### Open/Close Semantics

**REQ-SFILE-001**: The state file system shall maintain exactly three in-memory state files: STARINFO (index 0), RANDGRPINFO (index 1), and DEFGRPINFO (index 2).

**REQ-SFILE-002**: When `OpenStateFile` is called with a valid index (0–2), the system shall return a handle to the corresponding state file and increment its open count.

**REQ-SFILE-003**: If `OpenStateFile` is called with an index outside the range [0, 2], then the system shall return a failure indicator (NULL pointer at the C API, 0 at the FFI).

**REQ-SFILE-004**: When `OpenStateFile` is called for a file that has no allocated buffer, the system shall allocate a buffer of `size_hint` bytes.

**REQ-SFILE-005**: If buffer allocation fails during `OpenStateFile`, then the system shall return a failure indicator.

**REQ-SFILE-006**: When `OpenStateFile` is called with a mode starting with `'w'`, the system shall reset the logical file size (`used`) to zero and set the cursor to zero.

**REQ-SFILE-007**: When `OpenStateFile` is called with a mode starting with `'r'`, the system shall preserve existing file content and set the cursor to zero.

**REQ-SFILE-008**: While a state file's open count exceeds 1 after an open, the system shall emit a warning log message.

**REQ-SFILE-009**: When `CloseStateFile` is called, the system shall decrement the open count and reset the cursor to zero without freeing the buffer.

**REQ-SFILE-010**: While a state file's open count is below 0 after a close, the system shall emit a warning log message.

#### Read/Write Operations

**REQ-SFILE-011**: When `ReadStateFile` is called with size and count, the system shall read up to `size × count` bytes from the current cursor position, copying data into the caller's buffer.

**REQ-SFILE-012**: When reading would exceed the physical buffer size, the system shall truncate to available bytes, rounded down to a whole number of elements.

**REQ-SFILE-013**: When the cursor is at or past the physical buffer size during a read, the system shall return 0 (EOF).

**REQ-SFILE-014**: The system shall return the number of complete elements read (`total_bytes_read / size`).

**REQ-SFILE-015**: When `WriteStateFile` is called, the system shall write `size × count` bytes at the current cursor position.

**REQ-SFILE-016**: When writing would exceed the physical buffer size, the system shall grow the buffer to at least `cursor + bytes`, preferring `current_physical_size × 3/2` if that is larger.

**REQ-SFILE-017**: If buffer reallocation fails during write, then the system shall return 0.

**REQ-SFILE-018**: When a write advances the cursor past the current logical file size (`used`), the system shall update `used` to the new cursor position.

**REQ-SFILE-019**: When the buffer grows beyond the original `size_hint`, the system shall update `size_hint` to the new size.

#### Seek Operations

**REQ-SFILE-020**: When `SeekStateFile` is called with `SEEK_SET`, the system shall set the cursor to the specified offset.

**REQ-SFILE-021**: When `SeekStateFile` is called with `SEEK_CUR`, the system shall add the offset to the current cursor position.

**REQ-SFILE-022**: When `SeekStateFile` is called with `SEEK_END`, the system shall set the cursor to `used + offset`.

**REQ-SFILE-023**: If a seek would result in a negative cursor position, then the system shall clamp the cursor to 0 and return 0.

**REQ-SFILE-024**: When a seek results in a non-negative cursor position, the system shall return 1.

**REQ-SFILE-R001**: The system shall allow the cursor to be positioned beyond both the logical size (`used`) and the physical buffer size without clamping. A seek-past-end is not an error.

**REQ-SFILE-R002**: When a write occurs at a cursor position beyond the current physical buffer size, the system shall grow the buffer to accommodate the write, zero-filling any gap between the old physical end and the new write position.

#### File Length

**REQ-SFILE-025**: The `LengthStateFile` function shall return the logical file size (high-water mark of bytes written), not the physical allocation size.

#### Delete

**REQ-SFILE-026**: When `DeleteStateFile` is called with a valid index, the system shall free the data buffer, reset `used` and `ptr` to 0, and release the buffer memory.

**REQ-SFILE-027**: If `DeleteStateFile` is called while the file's open count is non-zero, then the system shall emit a warning log message.

**REQ-SFILE-028**: If `DeleteStateFile` is called with an index outside [0, 2], then the system shall do nothing.

#### Rust-Specific State File Requirements

**REQ-SFILE-R003**: The state file manager shall use interior mutability (e.g., `Mutex`) to satisfy Rust's thread-safety requirements for global state.

**REQ-SFILE-R004**: All state file FFI functions shall catch panics at the boundary and convert them to appropriate error return values. No Rust panic shall propagate into C code.

**REQ-SFILE-R005**: All state file FFI functions shall validate pointer parameters (null checks) before dereferencing.

**REQ-SFILE-R006**: The state file FFI shall accept file indices (`c_int`) rather than opaque pointers. The C redirect layer in `state.c` shall translate between `GAME_STATE_FILE*` pointers and integer indices.

**REQ-SFILE-R007**: When a poisoned mutex is encountered (due to a prior panic), the FFI function shall return an error value rather than panicking.

**REQ-SFILE-R008**: The `ReadStateFile` Rust implementation shall track both logical size (`used`) and physical size separately, and shall check reads against the physical size to match C behavior.

### REQ-STATE: Game State Bits Requirements

#### Bit Get/Set

**REQ-STATE-001**: The `getGameState` equivalent shall extract up to 8 bits from the backing byte array at the specified bit range [startBit, endBit] inclusive and return them as a `u8`.

**REQ-STATE-002**: When a bit field is contained within a single byte (`startBit / 8 == endBit / 8`), the system shall extract bits by right-shifting and masking within that byte.

**REQ-STATE-003**: When a bit field spans two bytes, the system shall combine the high bits of the first byte with the low bits of the second byte, producing a result identical to the C `getGameState` function.

**REQ-STATE-004**: The `setGameState` equivalent shall write up to 8 bits into the backing byte array, clearing the target bits first, then setting the new value.

**REQ-STATE-005**: When setting a field that spans two bytes, the system shall update both bytes correctly, preserving all adjacent bits.

#### 32-bit Get/Set

**REQ-STATE-006**: The `getGameState32` equivalent shall read 32 bits as four consecutive 8-bit reads in little-endian order: `result = byte0 | (byte1 << 8) | (byte2 << 16) | (byte3 << 24)`.

**REQ-STATE-007**: The `setGameState32` equivalent shall write a 32-bit value as four consecutive 8-bit writes in little-endian order.

#### Bit Copy

**REQ-STATE-008**: The `copyGameState` equivalent shall copy a range of bits from a source state array to a destination bit position, processing up to 8 bits at a time.

**REQ-STATE-009**: When the remaining bits in a copy operation are fewer than 8, the system shall copy only the remaining bits without corrupting adjacent bits.

#### State Reset

**REQ-STATE-010**: When a game is loaded, the system shall zero-fill the entire backing byte array before populating it from save data.

#### Rust-Specific Game State Requirements

**REQ-STATE-R001**: The backing byte array shall be exactly 155 bytes (matching C's `(1238 + 7) >> 3`). The `NUM_GAME_STATE_BITS` constant shall equal 1238.

**REQ-STATE-R002**: The bit extraction and insertion algorithms shall produce byte-for-byte identical results to the C `getGameState`/`setGameState` functions for all valid inputs.

**REQ-STATE-R003**: The game state shall use `u8` array elements only (never larger types) to ensure endian-neutral bit manipulation.

**REQ-STATE-R004**: The game state global shall use interior mutability (e.g., `Mutex`) to satisfy Rust's thread-safety requirements.

**REQ-STATE-R005**: The `rust_copy_game_state` FFI function shall not deadlock when source and destination refer to the same global state. The implementation shall acquire the lock exactly once per call.

**REQ-STATE-R006**: All game state FFI functions shall catch panics at the boundary and convert them to appropriate error return values (0 for getters, no-op for setters).

**REQ-STATE-R007**: Bit manipulation operations shall not cause buffer overflows. All bit index parameters shall be validated against the array bounds before access.

**REQ-STATE-R008**: The system shall provide `rust_get_game_state_bytes()` to export the raw byte array for save serialization, and `rust_restore_game_state_from_bytes()` to import bytes after load deserialization.

**REQ-STATE-R009**: When `rust_restore_game_state_from_bytes` is called with a size smaller than 155 bytes, the system shall copy only the provided bytes and leave remaining bytes as zero.

**REQ-STATE-R010**: When `rust_restore_game_state_from_bytes` is called with a size larger than 155 bytes, the system shall copy only 155 bytes and ignore the remainder.

### REQ-SAVE: Save/Load Requirements

#### Serialization

**REQ-SAVE-001**: The save system shall write all multi-byte values in little-endian byte order.

**REQ-SAVE-002**: The save file shall begin with a 4-byte magic tag (`SAVEFILE_TAG` = 0x01534d55).

**REQ-SAVE-003**: Each data section after the magic tag shall be preceded by a 4-byte tag and a 4-byte size.

**REQ-SAVE-004**: The Summary chunk (tag `SUMMARY_TAG` = 0x6d6d7553) shall be the first chunk and shall contain the SIS_STATE, activity, date, Melnorme credits, ship list, device list, and variable-length save name.

**REQ-SAVE-005**: The Global State chunk (tag `GLOBAL_STATE_TAG` = 0x74536c47, fixed 75 bytes) shall be the second chunk and shall contain game configuration, clock state, ship position, and velocity.

**REQ-SAVE-006**: The Game State Bits chunk (tag `GAME_STATE_TAG` = 0x74536d47) shall be the third chunk and shall contain the raw `GameState[]` byte array.

**REQ-SAVE-007**: The system shall serialize the contents of all three state file buffers into appropriate tagged chunks (`SCAN_TAG`, `GROUP_LIST_TAG`, `BATTLE_GROUP_TAG`).

**REQ-SAVE-008**: The system shall only write queue chunks (RACE_Q, SHIP_Q, etc.) when the queue is non-empty.

#### Save Creation

**REQ-SAVE-009**: When `SaveGame` is called, the system shall call `SaveFlagshipState()` to snapshot the current ship position before serializing.

**REQ-SAVE-010**: When the player is in interplanetary space without an active encounter, the system shall call `PutGroupInfo(GROUPS_RANDOM, GROUP_SAVE_IP)` to flush IP group state before saving.

**REQ-SAVE-011**: The save file shall be named `uqmsave.NN` where NN is the zero-padded save slot number.

**REQ-SAVE-012**: If any write operation fails during save (`io_ok` becomes `FALSE`), then the system shall delete the partially written save file and return `FALSE`.

**REQ-SAVE-013**: The system shall track write success via a boolean flag, checked after every individual write operation.

#### Load Restoration

**REQ-SAVE-014**: When `LoadGame` is called, the system shall attempt to open `uqmsave.NN` first.

**REQ-SAVE-015**: If the new-format file does not exist or has an invalid header, then the system shall automatically fall back to `LoadLegacyGame` to try loading `starcon2.NN`.

**REQ-SAVE-016**: When loading a summary only (`SummPtr` non-NULL), the system shall read only the summary chunk and return without loading full game state.

**REQ-SAVE-017**: When loading full game state, the system shall reinitialize all queues before populating them.

**REQ-SAVE-018**: When a `GAME_STATE_TAG` chunk is larger than the `GameState[]` array, the system shall read only the array size bytes and skip the remainder.

**REQ-SAVE-019**: When a `GAME_STATE_TAG` chunk is smaller than the `GameState[]` array, the system shall read only the available bytes (remaining bytes stay zero from the prior memset).

**REQ-SAVE-020**: When an unknown chunk tag is encountered during loading, the system shall skip `chunkSize` bytes and continue processing.

#### State File Reconstruction on Load

**REQ-SAVE-021**: When a `SCAN_TAG` chunk is encountered, the system shall open STARINFO_FILE with mode "wb" and write the chunk data as DWORD values.

**REQ-SAVE-022**: When a `GROUP_LIST_TAG` chunk is encountered, the system shall read the existing RANDGRPINFO header, append the group list data, and update the header's GroupOffset[0].

**REQ-SAVE-023**: When a `BATTLE_GROUP_TAG` chunk with `encounter_id == 0` is encountered, the system shall write the battle group to RANDGRPINFO_FILE.

**REQ-SAVE-024**: When a `BATTLE_GROUP_TAG` chunk with `encounter_id > 0` is encountered, the system shall write the battle group to DEFGRPINFO_FILE and update the corresponding `*_GRPOFFS` game state bits with the file offset.

**REQ-SAVE-025**: When the first group-related chunk is encountered during loading, the system shall call `InitGroupInfo(TRUE)` and reset `BattleGroupRef` before processing.

**REQ-SAVE-026**: When a battle group chunk has `current == 1`, the system shall set `GLOBAL(BattleGroupRef)` to the written offset.

#### Legacy Detection

**REQ-SAVE-027**: The legacy loader shall validate `year_index` against the range `[START_YEAR, START_YEAR + YEARS_TO_KOHRAH_VICTORY + 27)` to detect endianness incompatibilities.

**REQ-SAVE-028**: When loading legacy saves, the system shall decompress data using the `cread`/`copen`/`cclose` decode library.

**REQ-SAVE-029**: When loading legacy game state bits, the system shall transpose bit positions from the legacy layout to the current layout using the `transpose[]` table and re-extract DEFGRP offsets from their old interleaved positions.

#### Error Handling

**REQ-SAVE-030**: If the save directory cannot be opened or the file cannot be created, then `SaveGame` shall return `FALSE`.

**REQ-SAVE-031**: If `LoadSummary` fails (corrupt header, unreadable fields), then `LoadGame` shall close the file and return `FALSE`.

**REQ-SAVE-032**: If `LoadGameState` fails (wrong tag or size for GLOBAL_STATE_TAG), then `LoadGame` shall close the file and return `FALSE`.

**REQ-SAVE-033**: If a chunk read fails mid-stream during loading, then `LoadGame` shall close the file and return `FALSE`.

**REQ-SAVE-034**: If `LoadLegacyGame` detects an endianness-incompatible save (`year_index` out of range), then the system shall log a warning and return `FALSE`.

#### Rust-Specific Save/Load Requirements

**REQ-SAVE-R001**: Before saving, the system shall call `rust_get_game_state_bytes()` to export the Rust state to the C byte array for serialization.

**REQ-SAVE-R002**: After loading (both new and legacy formats), the system shall call `rust_restore_game_state_from_bytes()` to synchronize the Rust state from the C byte array.

**REQ-SAVE-R003**: Save files produced by the Rust backend shall be byte-for-byte loadable by the C backend, and vice versa.

**REQ-SAVE-R004**: The Rust state system shall not interfere with legacy save loading. The `getGameState`/`setGameState`/`copyGameState` C functions shall remain available for `load_legacy.c` to operate on local byte arrays, independent of the `USE_RUST_STATE` flag.

---

## Appendix A: Existing Rust Implementation Status

### A.1 Files

| File | Status | Notes |
|------|--------|-------|
| `game_state.rs` | Functional | Bit manipulation works. `NUM_GAME_STATE_BITS` = 2048 (should be 1238). 11 tests pass. |
| `state_file.rs` | Partially broken | Seek clamps to `data.len()` (violates REQ-SFILE-R001). Size hints differ from C. Uses `Vec::len()` for both `used` and physical size (violates REQ-SFILE-R008). 21 tests pass. |
| `planet_info.rs` | Partial | `get_planet_info` doesn't account for moon counts when skipping. 15 tests pass. |
| `ffi.rs` | Partially broken | `rust_copy_game_state` deadlocks (violates REQ-STATE-R005). String-key get/set only handles 3 hardcoded names. `rust_open_state_file` returns success/failure, not pointer. 12 tests pass. |
| `mod.rs` | Complete | Module re-exports. |

### A.2 Known Blockers

| Blocker | Requirement | Description |
|---------|-------------|-------------|
| Seek-past-end clamping | REQ-SFILE-R001 | `StateFile::seek` clamps `ptr` to `data.len()`. Must allow unbounded positive cursor values. `grpinfo.c` depends on seeking past current data and writing to extend the buffer. |
| Copy deadlock | REQ-STATE-R005 | `rust_copy_game_state` acquires `GLOBAL_GAME_STATE` mutex twice (read + write). Must acquire once and operate within a single critical section. |

### A.3 Known Discrepancies

| Issue | Requirements | Description |
|-------|-------------|-------------|
| `NUM_GAME_STATE_BITS` = 2048 | REQ-STATE-R001 | Should be 1238. Extra bytes are harmless for bit operations but wrong for serialization. |
| Size hints 256KB/64KB/64KB | REQ-SFILE-004 | C uses computed `STAR_BUFSIZE`/4KB/10KB. Affects initial memory usage, not correctness. |
| No separate `used` vs physical size | REQ-SFILE-R008, REQ-SFILE-025 | `StateFile` uses `Vec::len()` as both logical and physical size. C tracks these separately: `used` (high-water mark), `size` (allocation), and `ptr` (cursor). |
| Read checks `data.len()` not physical size | REQ-SFILE-012 | Rust `StateFile::read` checks against `data.len()` which is the logical size. C checks against `size` (physical allocation). |

---

## Appendix B: Tagged Chunk Format Reference

| Tag Name | Hex Value | ASCII | Description |
|----------|-----------|-------|-------------|
| `SAVEFILE_TAG` | 0x01534d55 | UMS\x01 | File magic number |
| `SUMMARY_TAG` | 0x6d6d7553 | Summ | Summary metadata |
| `GLOBAL_STATE_TAG` | 0x74536c47 | GlSt | Global game configuration |
| `GAME_STATE_TAG` | 0x74536d47 | GmSt | Game state bit array |
| `EVENTS_TAG` | 0x73747645 | Evts | Timed game events |
| `ENCOUNTERS_TAG` | 0x74636e45 | Enct | Active encounters |
| `RACE_Q_TAG` | 0x51636152 | RacQ | Race/fleet info |
| `IP_GRP_Q_TAG` | 0x51704749 | IGpQ | IP encounter groups |
| `NPC_SHIP_Q_TAG` | 0x5163704e | NpcQ | NPC ships |
| `SHIP_Q_TAG` | 0x51706853 | ShpQ | Player fleet ships |
| `STAR_TAG` | 0x72617453 | Star | Star descriptor |
| `SCAN_TAG` | 0x6e616353 | Scan | Planet scan masks |
| `BATTLE_GROUP_TAG` | 0x70477442 | BtGp | Battle encounter group |
| `GROUP_LIST_TAG` | 0x73707247 | Grps | IP group list |

---

## Appendix C: C Type Mapping

| C Type | Size | Rust Equivalent |
|--------|------|----------------|
| `BYTE` | 1 byte | `u8` |
| `UWORD` / `COUNT` | 2 bytes | `u16` |
| `SWORD` | 2 bytes | `i16` |
| `DWORD` | 4 bytes | `u32` |
| `SDWORD` | 4 bytes | `i32` |
| `GAME_STATE_FILE*` | pointer | Translated to `c_int` (file index) at FFI boundary |
| `BOOLEAN` | int | `c_int` (0 = FALSE, non-zero = TRUE) |
