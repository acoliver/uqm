# State File I/O — Domain Model

## 1. State File Types

Three in-memory buffers serve as virtual files for runtime game data.
They are **not** disk files — they are `Vec<u8>` (Rust) / `BYTE*` (C)
buffers with a read/write cursor.

| Index | Constant | Symbolic Name | Initial Size | Purpose |
|-------|----------|---------------|-------------|---------|
| 0 | `STARINFO_FILE` | `"STARINFO"` | `STAR_BUFSIZE` (~182 KB) | Planet scan masks (bio/mineral/energy per planet/moon) |
| 1 | `RANDGRPINFO_FILE` | `"RANDGRPINFO"` | 4,096 bytes | Transient IP encounter groups (appear/expire per system visit) |
| 2 | `DEFGRPINFO_FILE` | `"DEFGRPINFO"` | 10,240 bytes | Scripted encounter groups (permanent story battle groups) |

## 2. In-Memory Buffer Model

### 2.1 Fields

```
StateFile {
    name: &str,          // "STARINFO", "RANDGRPINFO", "DEFGRPINFO"
    size_hint: usize,    // Initial allocation size, updated on growth
    open_count: i32,     // Reference count (warning-only)
    data: Vec<u8>,       // Heap-allocated buffer (physical_size = capacity)
    used: usize,         // Logical size (high-water mark of bytes written)
    ptr: usize,          // Current cursor position (may exceed used and capacity)
}
```

### 2.2 Key Distinction: used vs physical_size vs ptr

- `used` — highest byte ever written. This is what `LengthStateFile` returns.
- `physical_size` — allocated capacity. Reads check against this.
- `ptr` — cursor. May exceed both `used` and `physical_size` without error.

The **current Rust implementation** conflates `used` and `physical_size`
by using `Vec::len()` for both. This must be fixed: `StateFile` needs
a separate `used` field distinct from `data.len()`.

### 2.3 Rust vs C Field Mapping

| C Field | Rust Current | Rust Required |
|---------|-------------|---------------|
| `fp->data` | `data: Vec<u8>` | Same |
| `fp->size` (physical) | `data.len()` | `data.len()` or `data.capacity()` — see note |
| `fp->used` (logical) | `data.len()` (conflated!) | Separate `used: usize` field |
| `fp->ptr` (cursor) | `ptr: usize` (clamped!) | `ptr: usize` (unclamped) |
| `fp->size_hint` | `size_hint: usize` | Same |
| `fp->open_count` | `open_count: u32` | Change to `i32` (can go negative per C) |

**Note on physical size**: The C code uses `fp->size` as the physical
allocation size and checks reads against it. In Rust, we need `data.len()`
to represent the physical size — meaning we must ensure `data` is always
filled/resized to the allocated capacity. Alternatively, we track `used`
separately and always keep `data.len() >= used`, using `data.len()` as the
physical size for read boundary checks.

### 2.4 Proposed Fix: Separate `used` and `physical_size`

```rust
struct StateFile {
    name: &'static str,
    size_hint: usize,
    open_count: i32,       // Changed from u32 to i32
    data: Vec<u8>,         // data.len() == physical allocation size
    used: usize,           // Logical size (high-water mark)
    ptr: usize,            // Cursor (NO upper clamp)
}
```

- On open with "w": `used = 0`, `ptr = 0`, `data` filled to `size_hint`.
- On write: `data` grows if needed. `used = max(used, ptr)` after write.
- On read: check `ptr < data.len()` (physical size).
- On seek: `ptr = new_value` with NO upper clamp. Only negative → 0.
- `LengthStateFile` returns `used`, not `data.len()`.

## 3. Buffer Lifecycle

```
 ┌─────────────────────┐
 │    Not Allocated     │  data.is_empty(), used=0, ptr=0
 └──────────┬──────────┘
            │ OpenStateFile (first time)
            │ data = vec![0; size_hint]
            ▼
 ┌─────────────────────┐
 │     Allocated        │  data.len() >= size_hint
 │  open_count tracks   │  used <= data.len()
 │  concurrent opens    │  ptr may be anything >= 0
 └──────────┬──────────┘
            │
    Open/Close cycles     data persists
            │
            │ DeleteStateFile
            ▼
 ┌─────────────────────┐
 │   Freed / Reset      │  data.clear(), used=0, ptr=0
 └─────────────────────┘
```

## 4. Seek-Past-End Behavior (CRITICAL)

The C implementation allows:

```c
SeekStateFile(fp, 1000000, SEEK_SET);  // ptr = 1000000, buffer may be 100 bytes
// Subsequent read at ptr=1000000 → returns 0 (EOF)
// Subsequent write at ptr=1000000 → grows buffer, zero-fills gap
```

**Why this matters**: `FlushGroupInfo` in `grpinfo.c` does:
1. `offset = LengthStateFile(fp)` — gets current end
2. `SeekStateFile(fp, offset, SEEK_SET)` — seeks to end
3. `swrite_*` — writes at end, extending the buffer

This works because the seek position equals `used`, and the write extends.
But `grpinfo.c` also computes offsets that may temporarily exceed current
data length (e.g., when building group lists). The cursor must be allowed
to go anywhere non-negative.

**Current Rust bug**: `StateFile::seek` clamps to `data.len()`:
```rust
// BROKEN — clamps to data.len()
if result > self.data.len() as i64 {
    self.data.len() as i64
}
```

**Fix**: Remove the upper clamp. Only clamp negative to 0.

## 5. Save/Load Flow

### Save (C stays in control)

```
SaveGame():
  1. SaveStarInfo():
     a. fp = OpenStateFile(STARINFO_FILE, "rb")
     b. len = LengthStateFile(fp)
     c. For each 4-byte DWORD in the buffer: sread_32 → write_32 (LE)
     d. CloseStateFile(fp)
  2. SaveGroups():
     a. Similar: open RANDGRPINFO/DEFGRPINFO, read via sread_*, serialize LE
  3. Save game state bits (separate subsystem, OUT OF SCOPE)
```

### Load (C stays in control)

```
LoadGame():
  1. LoadScanInfo():
     a. fp = OpenStateFile(STARINFO_FILE, "wb")  // clears buffer
     b. For each DWORD from save file: read_32 (LE) → swrite_32 to buffer
     c. CloseStateFile(fp)
  2. LoadGroupList() / LoadBattleGroup():
     a. Similar: open state file, write data from save file
  3. Load game state bits via rust_restore_game_state_from_bytes (OUT OF SCOPE)
```

**Key insight**: The save/load system reads/writes the state file buffers
through the same 7 API functions. When `USE_RUST_STATE` is active, these
calls route through Rust. The save file format is unchanged — only the
in-memory buffer backend changes.

## 6. Copy Deadlock Analysis

`rust_copy_game_state` in `ffi.rs` currently:

```rust
fn rust_copy_game_state(dest_bit, src_start_bit, src_end_bit) {
    guard_convert_value_mut(&GLOBAL_GAME_STATE, |state| {   // Lock #1
        guard_convert_value(&GLOBAL_GAME_STATE, |src_state| { // Lock #2 — DEADLOCK
            state.copy_state(dest_bit, src_state, src_start_bit, src_end_bit);
        });
    });
}
```

`std::sync::Mutex` is not reentrant. Lock #2 blocks forever waiting for
Lock #1 to release.

**Fix**: Acquire the lock once. Since source and destination are the same
`GameState`, we need to:
1. Lock `GLOBAL_GAME_STATE` once.
2. Snapshot the source bits (or use `GameState::copy_state` with `self`
   as both source and destination, which requires adjusting the API).

The simplest fix: `copy_state` already takes `&GameState` as source.
Change the FFI to lock once and call a self-copy variant:

```rust
fn rust_copy_game_state(dest_bit, src_start_bit, src_end_bit) {
    let mut guard = GLOBAL_GAME_STATE.lock().unwrap();
    if let Some(state) = guard.as_mut() {
        state.copy_state_self(dest_bit, src_start_bit, src_end_bit);
    }
}
```

Where `copy_state_self` snapshots the source bytes before mutating.

## 7. open_count Type

The C `open_count` is `int` and can go negative (on close without open).
The Rust implementation uses `u32`, which would underflow. Must change
to `i32` to match C semantics (warning on negative, not crash).
