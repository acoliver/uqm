# AIFF Audio Decoder — Technical Analysis & EARS Requirements

## Source Files

| File | Lines | Role |
|------|-------|------|
| `sc2/src/libs/sound/decoders/aiffaud.c` | 651 | AIFF/AIFF-C decoder implementation |
| `sc2/src/libs/sound/decoders/aiffaud.h` | 37 | Error enum, vtable extern |
| `sc2/src/libs/sound/decoders/decoder.h` | 130 | `TFB_SoundDecoderFuncs` vtable, `TFB_SoundDecoder` base struct |
| `sc2/src/libs/sound/decoders/decoder.c` | ~960 | Decoder registry, `SoundDecoder_Load`, byte-swap, looping |

---

# Part 1: Technical Analysis

## 1. AIFF / AIFF-C Format Overview

AIFF (Audio Interchange File Format) is an Apple-originated uncompressed audio container. AIFF-C is its compressed variant. Both use big-endian byte order throughout.

### File Structure

```
FORM <size:u32> AIFF   ← AIFF (uncompressed)
FORM <size:u32> AIFC   ← AIFF-C (compressed)
  ├─ COMM <size:u32> <common-chunk-data>
  ├─ SSND <size:u32> <sound-data-chunk>
  ├─ MARK <size:u32> <marker-data>       (optional, NOT parsed by aiffaud.c)
  ├─ INST <size:u32> <instrument-data>   (optional, NOT parsed by aiffaud.c)
  ├─ FVER <size:u32> <version-data>      (optional, NOT parsed by aiffaud.c)
  └─ ... other chunks skipped
```

### Chunk IDs (4-byte big-endian tags)

| Constant | Value | ASCII |
|----------|-------|-------|
| `aiff_FormID` | `0x464F524D` | `FORM` |
| `aiff_FormVersionID` | `0x46564552` | `FVER` |
| `aiff_CommonID` | `0x434F4D4D` | `COMM` |
| `aiff_SoundDataID` | `0x53534E44` | `SSND` |
| `aiff_FormTypeAIFF` | `0x41494646` | `AIFF` |
| `aiff_FormTypeAIFC` | `0x41494643` | `AIFC` |
| `aiff_CompressionTypeSDX2` | `0x53445832` | `SDX2` |

These are constructed via the macro:
```c
#define aiff_MAKE_ID(x1, x2, x3, x4) \
    (((x1) << 24) | ((x2) << 16) | ((x3) << 8) | (x4))
```

### Chunk Structures

**Chunk Header** (8 bytes, all fields big-endian):
```
[4 bytes] id   — four-character chunk identifier
[4 bytes] size — byte count of chunk data (excludes header)
```
Chunks are 2-byte aligned: if `size` is odd, one padding byte follows.

**File Header** (12 bytes):
```
[4 bytes] chunk.id   — must be 'FORM'
[4 bytes] chunk.size — remaining file size
[4 bytes] type       — 'AIFF' or 'AIFC'
```

**Common Chunk (COMM)** — minimum 18 bytes (`AIFF_COMM_SIZE = 2+4+2+10`):
```
[2 bytes]  channels       — 1 (mono) or 2 (stereo)
[4 bytes]  sampleFrames   — total number of sample frames
[2 bytes]  sampleSize     — bits per sample (actual, not rounded)
[10 bytes] sampleRate     — IEEE 754 80-bit extended float
```

**Extended Common Chunk (AIFC)** — adds 4 bytes (`AIFF_EXT_COMM_SIZE = AIFF_COMM_SIZE + 4`):
```
[4 bytes]  extTypeID      — compression type (e.g., 'SDX2')
[N bytes]  extName        — compression name (pstring, skipped by seek)
```

**Sound Data Chunk (SSND)** — 8-byte header (`AIFF_SSND_SIZE = 4+4`):
```
[4 bytes]  offset     — offset to first sample within data block
[4 bytes]  blockSize  — alignment block size (typically 0)
```
The actual audio data begins at `file_position_after_SSND_header + offset`.

## 2. The `TFB_AiffSoundDecoder` Struct

```c
typedef struct tfb_wavesounddecoder
{
    TFB_SoundDecoder decoder;           // base — MUST be first member
    sint32 last_error;                  // sticky error code (errno or aifa_Error)
    uio_Stream *fp;                     // file handle for streaming reads
    aiff_ExtCommonChunk fmtHdr;         // parsed COMM chunk data
    aiff_CompressionType comp_type;     // aifc_None or aifc_Sdx2
    unsigned bits_per_sample;           // 8 or 16 (rounded up to multiple of 8)
    unsigned block_align;               // bytes per output sample frame
    unsigned file_block;                // bytes per file sample frame (may differ for SDX2)
    uint32 data_ofs;                    // absolute file offset of first audio sample
    uint32 data_size;                   // total bytes of encoded audio data
    uint32 max_pcm;                     // total number of sample frames
    uint32 cur_pcm;                     // current decoded sample frame position
    sint32 prev_val[MAX_CHANNELS];      // SDX2 predictor state per channel (MAX_CHANNELS=4)
} TFB_AiffSoundDecoder;
```

The struct name `tfb_wavesounddecoder` is a historical artifact — it is reused from the WAV decoder naming.

### Error Codes (`aifa_Error` enum from `aiffaud.h`)

| Name | Value | Meaning |
|------|-------|---------|
| `aifae_None` | 0 | No error |
| `aifae_Unknown` | -1 | Unknown error |
| `aifae_BadFile` | -2 | Malformed AIFF file |
| `aifae_BadArg` | -3 | Bad argument |
| `aifae_Other` | -1000 | Other error |

Positive error values correspond to `errno` values from I/O failures.

## 3. Vtable Implementation

The decoder exposes `aifa_DecoderVtbl` of type `TFB_SoundDecoderFuncs`:

```c
TFB_SoundDecoderFuncs aifa_DecoderVtbl = {
    aifa_GetName,       // → "AIFF"
    aifa_InitModule,    // stores TFB_DecoderFormats*, returns true
    aifa_TermModule,    // no-op
    aifa_GetStructSize, // → sizeof(TFB_AiffSoundDecoder)
    aifa_GetError,      // returns and clears last_error
    aifa_Init,          // sets need_swap based on endianness
    aifa_Term,          // calls Close
    aifa_Open,          // parses AIFF, validates, positions to data
    aifa_Close,         // closes file handle
    aifa_Decode,        // dispatches to DecodePCM or DecodeSDX2
    aifa_Seek,          // seeks to PCM position, resets SDX2 state
    aifa_GetFrame,      // always returns 0
};
```

### Function-by-Function Analysis

#### `aifa_GetName()` → `"AIFF"`
Returns a static string identifying the decoder. Trivial.

#### `aifa_InitModule(flags, fmts)` → `true`
Stores the `TFB_DecoderFormats*` pointer in the static `aifa_formats`. The `flags` parameter is ignored (explicitly suppressed with `(void)flags`). Always returns `true`.

#### `aifa_TermModule()`
No-op. No module-level resources to free.

#### `aifa_GetStructSize()` → `sizeof(TFB_AiffSoundDecoder)`
Returns the size of the extended struct so the C framework can allocate via `HCalloc`.

#### `aifa_GetError(This)` → `int`
Reads `last_error`, resets it to 0, returns the old value. Classic "get-and-clear" pattern.

#### `aifa_Init(This)` → `true`
Sets `This->need_swap` based on whether the system endianness matches the desired output endianness:
```c
This->need_swap = !aifa_formats->want_big_endian;
```
AIFF data is big-endian, so if the consumer does NOT want big-endian, swapping is needed.
Note: for SDX2 compressed data, `need_swap` is overridden during `Open()`.

#### `aifa_Term(This)`
Calls `aifa_Close(This)` to ensure file resources are freed.

#### `aifa_Open(This, dir, filename)` → `bool`

This is the most complex function (~175 lines). It performs:

1. **File open**: Opens the file via `uio_fopen(dir, filename, "rb")`.
2. **State reset**: Zeros `data_size`, `max_pcm`, `data_ofs`, `fmtHdr`, `prev_val`.
3. **File header validation**: Reads 12-byte FORM header; checks `id == aiff_FormID` and `type` is `aiff_FormTypeAIFF` or `aiff_FormTypeAIFC`.
4. **Chunk iteration loop**: Iterates over remaining bytes (`remSize = fileHdr.chunk.size - sizeof(aiff_ID)`), reading chunks:
   - **COMM chunk**: Calls `aifa_readCommonChunk()` to parse channels, sampleFrames, sampleSize (as IEEE-754 80-bit float → sint32), and optionally `extTypeID` for AIFC. Seeks past any remaining chunk bytes.
   - **SSND chunk**: Calls `aifa_readSoundDataChunk()` to read offset and blockSize. Computes `data_ofs = ftell() + offset`. Seeks past the chunk data.
   - **Other chunks**: Skipped via `fseek(size)`.
   - **Alignment**: After each chunk, seeks past the padding byte if `size` is odd: `fseek(size & 1)`.
5. **Validation**:
   - `sampleFrames == 0` → error (no sound data)
   - `bits_per_sample` is rounded up to multiple of 8: `(sampleSize + 7) & ~7`; must be 8 or 16 (24/32 not supported)
   - `channels` must be 1 or 2
   - `sampleRate` must be in range `[300, 128000]`
   - `data_ofs` must be non-zero (SSND chunk was found)
6. **Compression detection**:
   - **AIFF** (`FormTypeAIFF`): `extTypeID` must be 0; sets `comp_type = aifc_None`
   - **AIFC** (`FormTypeAIFC`): `extTypeID` must be `SDX2` (`0x53445832`); sets `comp_type = aifc_Sdx2`, halves `file_block` (`/= 2`), asserts `channels <= MAX_CHANNELS`, recalculates `need_swap` using `big_endian != want_big_endian` (because SDX2 decoding produces machine-endian output)
   - SDX2 with non-16-bit samples → error
7. **Final setup**:
   - `block_align = bits_per_sample / 8 * channels` (output bytes per frame)
   - `file_block = block_align` (for PCM), or `block_align / 2` (for SDX2)
   - `data_size = sampleFrames * file_block`
   - Format selection: maps `(channels, bits_per_sample)` → `aifa_formats->mono8/stereo8/mono16/stereo16`
   - `frequency = sampleRate`
   - Seeks to `data_ofs`
   - `max_pcm = sampleFrames`, `cur_pcm = 0`
   - `length = (float)max_pcm / sampleRate`

On any error, `aifa_Close(This)` is called before returning `false`.

**Behavioral notes:**
- **Duplicate chunks**: If multiple `COMM` or `SSND` chunks appear, the parser silently overwrites previous values with the last parsed chunk's data — no duplicate-chunk rejection.
- **remSize validation**: The chunk traversal loop uses `remSize` (a `sint32`) and does not validate individual chunk sizes against actual remaining file length before seeking — a malformed chunk size could cause seeks past end of file.
- **SDX2 channels assert**: The `assert(channels <= MAX_CHANNELS)` on the SDX2 path is debug-only — in release builds, channels > MAX_CHANNELS would proceed unchecked.

#### `aifa_Close(This)`
Closes `fp` via `uio_fclose()` and sets it to `NULL`. Safe to call multiple times.

#### `aifa_Decode(This, buf, bufsize)` → `int`

Dispatches based on `comp_type`:
- `aifc_None` → `aifa_DecodePCM()`
- `aifc_Sdx2` → `aifa_DecodeSDX2()`
- Default: assert failure

Returns bytes decoded (≥0). Returns 0 when `cur_pcm >= max_pcm` (no more frames) or when `uio_fread` returns 0. **Note:** The C implementation never returns a negative value — decode errors silently produce a 0-byte result without setting `last_error`.

#### `aifa_DecodePCM(aifa, buf, bufsize)` → `int`

1. Calculates frames that fit: `dec_pcm = bufsize / block_align`, clamped to `max_pcm - cur_pcm`.
2. Reads directly via `uio_fread(buf, file_block, dec_pcm, fp)`.
3. Updates `cur_pcm += dec_pcm`.
4. **8-bit fixup**: AIFF stores 8-bit samples as **signed** (-128..127). The engine expects **unsigned** (0..255). Adds 128 to every byte: `*ptr += 128`.
5. Returns `dec_pcm * block_align`.

#### `aifa_DecodeSDX2(aifa, buf, bufsize)` → `int`

SDX2 (Square-root Delta Exact) is a lossy audio compression where each encoded byte produces one 16-bit sample.

1. Calculates frames: `dec_pcm = bufsize / block_align`, clamped to remaining.
2. **In-place decode trick**: Reads compressed data into the END of the output buffer to allow in-place expansion: `src = (sint8*)buf + bufsize - (dec_pcm * file_block)`.
3. Reads `file_block` bytes per frame via `uio_fread(src, file_block, dec_pcm, fp)`.
4. **SDX2 algorithm** (per sample, per channel):
   ```
   v = (sample_byte * abs(sample_byte)) << 1    // square with sign preservation
   if (sample_byte & 1)                          // odd → add previous value (delta mode)
       v += prev_val[channel]
   v = clamp(v, -32768, 32767)                   // saturate to 16-bit range
   prev_val[channel] = v                         // update predictor
   output_sample = v
   ```
5. Returns `dec_pcm * block_align`.

Key characteristics:
- Each compressed byte expands to a 16-bit sample (2:1 ratio).
- The `file_block` is half of `block_align` (since 1 byte → 2 bytes).
- `prev_val[]` maintains per-channel delta state across calls.
- The odd-bit check (`*src & 1`) determines delta vs absolute mode.

#### `aifa_Seek(This, pcm_pos)` → `uint32`

1. Clamps `pcm_pos` to `max_pcm`.
2. Sets `cur_pcm = pcm_pos`.
3. Seeks file to `data_ofs + pcm_pos * file_block`. **Note:** the return value of `uio_fseek` is not checked — if the seek fails, `cur_pcm` is still updated and the clamped position is still returned.
4. **Resets SDX2 predictor state**: `memset(prev_val, 0, sizeof(prev_val))`. The comment notes "the delta will recover faster with reset" — i.e., seeking into SDX2 mid-stream is lossy but self-correcting.
5. Returns the clamped `pcm_pos`.

#### `aifa_GetFrame(This)` → `0`
Always returns 0. AIFF files are single-frame (unlike DukAud with video frames).

## 4. IEEE 754 80-bit Float Parsing

The `read_be_f80()` function deserializes the AIFF sample rate field:

```
[2 bytes]  sign (1 bit) + exponent (15 bits)
[4 bytes]  mantissa high (includes explicit integer bit for 80-bit format)
[4 bytes]  mantissa low  (ignored in the conversion)
```

The implementation:
1. Extracts sign bit: `(se >> 15) & 1`
2. Extracts exponent: `se & 0x7FFF`
3. Shifts mantissa right by 1 to make room for sign (working in sint32)
4. Unbias exponent: `exp -= 16383` (2^14 - 1)
5. Calculates shift: `exp - 31 + 1` (mantissa is treated as 31 bits before the decimal)
6. If shift > 0, value overflows sint32 → clamped to `0x7FFFFFFF`
7. If shift < 0, right-shifts mantissa
8. Applies sign

Note: The 80-bit IEEE 754 format used in AIFF has an **explicit** mantissa MS bit (unlike 64-bit double which has an implicit leading 1). The code accounts for this by commenting out the implied-bit logic.

This approach truncates precision to 31 bits and only handles integer-range sample rates, which is entirely appropriate since sample rates are always whole numbers (e.g., 44100, 22050).

## 5. AIFF-C SDX2 Compression

SDX2 is the only compression codec supported. Key details:

- **Identification**: `extTypeID == 0x53445832` in the AIFC extended COMM chunk.
- **Ratio**: 2:1 compression (1 byte → 16-bit sample).
- **Algorithm**: Squared-delta encoding with per-channel predictor state.
- **Channel constraint**: Asserts `channels <= MAX_CHANNELS` (4) at open time.
- **Bit depth**: Only 16-bit SDX2 is supported; other bit depths are rejected.
- **Endianness**: After SDX2 decode, samples are in machine byte order, so `need_swap` is recalculated as `big_endian != want_big_endian`.
- **Seek behavior**: Predictor state is zeroed on seek. This causes a brief audio glitch at the seek point that self-corrects within a few samples.

## 6. Loop Point Handling

**aiffaud.c does NOT implement loop point parsing.** The MARK (marker) and INST (instrument) chunks that carry loop start/end markers are not recognized — they fall into the "skip uninteresting chunk" else branch.

Loop support in UQM is handled at the `SoundDecoder_Decode()` wrapper level in `decoder.c` via the `decoder->looping` flag and `SoundDecoder_Rewind()`, which calls `Seek(0)`. This is a simple "rewind to beginning" loop, not the sustain-loop that MARK+INST would enable.

## 7. Seeking Implementation

`aifa_Seek()` performs a direct file seek:
```
file_position = data_ofs + pcm_pos * file_block
```

This works because:
- **PCM**: Constant-size frames, direct byte offset calculation.
- **SDX2**: Also constant-size compressed frames (1 byte per channel per frame), so direct calculation works.

The SDX2 predictor state is zeroed on seek, accepting a brief transient artifact for simplicity.

## 8. Error Handling Patterns

The C code uses several error handling patterns:

1. **errno capture**: Low-level read helpers (`aifa_readFileHeader`, `aifa_readChunkHeader`, `aifa_readCommonChunk`, `aifa_readSoundDataChunk`) store `errno` into `last_error` on I/O failure.
2. **Custom error codes**: Only `aifa_readCommonChunk` uses `aifae_BadFile` (-2), when `COMM` chunk size is too small.
3. **Get-and-clear**: `aifa_GetError()` returns the error and resets to 0.
4. **Cascade cleanup**: On failure during `Open()`, `aifa_Close()` is called before returning `false`.
5. **Log-only failures**: Many validation failures in `aifa_Open()` (invalid form ID, unsupported type/channels/sample rate/bps, missing SSND chunk, unsupported compression) only call `log_add(log_Warning, ...)` and return `false` **without** setting `last_error`. The error is communicated solely through the `false` return and the log message.
6. **Decode returns**: `aifa_DecodePCM` and `aifa_DecodeSDX2` never return negative values and never set `last_error` — a short/failed `uio_fread` silently produces a 0-byte result.
7. **No exceptions**: Pure return-value error signaling; assertions only for programming errors (unknown `comp_type`, `channels > MAX_CHANNELS` in debug-only).

## 9. Decoder Registry Integration

In `decoder.c`, the AIFF decoder is registered as a built-in decoder:

```c
static TFB_RegSoundDecoder sd_decoders[MAX_REG_DECODERS + 1] = {
    // ... wav, mod, ogg, duk ...
    {true, true, "aif", &aifa_DecoderVtbl},
    {false, false, NULL, NULL}, // null term
};
```

Key observations:
- File extension: `"aif"` (not `"aiff"` — only 3 characters).
- Always built-in (no `USE_RUST_AIFF` conditional yet).
- The decoder is auto-initialized during `SoundDecoder_Init()` which calls `InitModule()` on all registered decoders.
- File matching is done by `SoundDecoder_Load()` which extracts the extension from the filename and looks it up.

## 10. Comparison with Existing Rust Decoders

### Architecture Pattern (established in `wav.rs` / `wav_ffi.rs` / `dukaud.rs` / `dukaud_ffi.rs`)

The Rust decoders follow a consistent two-file pattern:

| Layer | File | Responsibility |
|-------|------|---------------|
| Pure Rust | `wav.rs`, `dukaud.rs` | Format parsing, decoding logic, `SoundDecoder` trait |
| FFI glue | `wav_ffi.rs`, `dukaud_ffi.rs` | C vtable, `extern "C"` functions, UIO I/O, format mapping |

#### Pure Rust Decoder Pattern

- Implements the `SoundDecoder` trait from `decoder.rs`
- Constructor: `new()` → uninitialized state
- `open_from_bytes(&[u8], &str)` → parses format from memory
- `decode(&mut [u8])` → writes PCM bytes, returns `DecodeResult<usize>`
- `seek(u32)` → returns `DecodeResult<u32>`
- Uses `DecodeError` enum: `NotFound`, `InvalidData`, `UnsupportedFormat`, `IoError`, `NotInitialized`, `EndOfFile`, `SeekFailed`, `DecoderError`
- Pure Rust, no unsafe, no FFI concerns
- Comprehensive `#[cfg(test)]` unit tests

#### FFI Glue Pattern

- `#[repr(C)]` wrapper struct with `TFB_SoundDecoder` as first field + `*mut c_void` for Rust decoder
- `static Mutex<Option<DecoderFormats>>` for module-level format storage
- `static NAME: &[u8] = b"Rust Something\0"` for C string name
- `extern "C" fn read_uio_file(...)` helper to read files through UIO virtual filesystem
- 12 `extern "C"` functions matching `TFB_SoundDecoderFuncs` slots
- `#[no_mangle] pub static rust_xxx_DecoderVtbl: TFB_SoundDecoderFuncs`
- All functions null-check decoder pointer before use
- `Init` creates `Box::new(Decoder::new())`, stores as `*mut c_void`
- `Term` reconstructs `Box::from_raw()` and drops it
- `Open` reads file via UIO into `Vec<u8>`, calls `open_from_bytes()`
- Format codes mapped via locked `DecoderFormats` (mono8/stereo8/mono16/stereo16)
- Logging via `rust_bridge_log_msg()`

### Key Reusable Patterns for AIFF

1. **Two-file structure**: `aiff.rs` (pure decoder) + `aiff_ffi.rs` (C glue).
2. **`SoundDecoder` trait**: Already defines all necessary methods.
3. **`DecoderFormats`/`AudioFormat`**: Format mapping infrastructure exists.
4. **UIO file reading**: The `read_uio_file()` helper in `dukaud_ffi.rs` can be reused or shared.
5. **FFI wrapper struct**: `TFB_RustAiffDecoder { base: TFB_SoundDecoder, rust_decoder: *mut c_void }`.
6. **Vtable export**: `#[no_mangle] pub static rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs`.
7. **Null safety**: Every FFI function checks for null pointers.
8. **Error mapping** (Rust FFI): `DecodeError::EndOfFile` → return 0; other errors → return 0 (matching C behavior where decode never returns negative).

### AIFF-Specific Differences from WAV

| Aspect | WAV | AIFF |
|--------|-----|------|
| Byte order | Little-endian | Big-endian |
| 8-bit samples | Unsigned | Signed (needs +128) |
| Compression | None (PCM only) | SDX2 supported |
| Sample rate field | u32 | IEEE 754 80-bit float |
| `need_swap` init | `want_big_endian` | `!want_big_endian` (reversed) |
| Streaming | WAV loads all data into memory | AIFF streams from file (uses `uio_Stream`) |
| Predictor state | N/A | SDX2 `prev_val[MAX_CHANNELS]` |

The streaming vs in-memory difference is significant: the WAV decoder calls `open_from_bytes()` and keeps all data in a `Vec<u8>`, while AIFF uses `uio_fread()` during `Decode()`. For the Rust port, loading the entire file into memory (like WAV) is simpler and avoids the need for a streaming file handle, but slightly increases memory usage.

## 11. Crate Evaluation

### Option A: Custom Decoder (Recommended)

The AIFF decoder is simple enough to implement from scratch:
- ~200 lines of chunk parsing
- ~50 lines of PCM decode
- ~40 lines of SDX2 decode
- ~20 lines of 80-bit float parsing
- Total: ~350-400 lines of Rust

Advantages:
- Exact behavioral match with `aiffaud.c` (critical for game compatibility)
- No external dependencies
- Full control over SDX2 decompression
- Consistent with existing Rust decoders (`wav.rs`, `dukaud.rs`)
- Handles UQM's subset of AIFF (not the full spec)

### Option B: `hound` crate

`hound` supports WAV but **not AIFF**. Not applicable.

### Option C: `symphonia` crate

`symphonia` supports AIFF/AIFF-C but:
- Very large dependency (~30 crates)
- AIFF codec is behind a feature flag (`aiff`)
- Does not guarantee SDX2 compatibility
- Overkill for UQM's subset needs
- Would not match the exact behavior of `aiffaud.c`

### Recommendation

**Option A: Custom decoder.** The format subset is tiny (PCM + SDX2 only, mono/stereo only, 8/16-bit only), and the SDX2 algorithm is ~20 lines. A custom decoder ensures bit-exact compatibility and avoids dependency bloat.

---

# Part 2: EARS Requirements

## Format Parsing

### FP-1 File Header Validation
**When** the decoder opens an AIFF file, **the AIFF decoder shall** read the first 12 bytes as a big-endian FORM file header consisting of a 4-byte chunk ID, a 4-byte chunk size, and a 4-byte form type.

### FP-2 FORM ID Check
**When** the file header chunk ID is not `0x464F524D` (`FORM`), **the AIFF decoder shall** log a warning containing the invalid ID value and return an error.

### FP-3 Form Type Check
**When** the file header form type is neither `0x41494646` (`AIFF`) nor `0x41494643` (`AIFC`), **the AIFF decoder shall** log a warning containing the unsupported type value and return an error.

### FP-4 Chunk Iteration
**When** the file header has been validated, **the AIFF decoder shall** iterate over all chunks in the file, reading each chunk's 8-byte big-endian header (4-byte ID + 4-byte size), until the remaining byte count from the FORM size is exhausted.

### FP-5 Chunk Alignment
**The AIFF decoder shall** align the file position to a 2-byte boundary after processing each chunk by advancing past one padding byte when the chunk's size is odd.

### FP-6 Remaining Size Calculation
**The AIFF decoder shall** calculate the remaining file size for chunk iteration as `fileHdr.chunk.size - 4` (subtracting the 4-byte form type field already consumed).

### FP-7 Unknown Chunk Skipping
**When** a chunk ID does not match `COMM` (`0x434F4D4D`) or `SSND` (`0x53534E44`), **the AIFF decoder shall** skip the chunk by seeking forward by the chunk's size value.

### FP-8 Common Chunk Parsing
**When** a `COMM` chunk is encountered, **the AIFF decoder shall** parse the following big-endian fields: channels (u16), sampleFrames (u32), sampleSize (u16), and sampleRate (80-bit IEEE 754 float converted to i32).

### FP-9 Common Chunk Minimum Size
**When** a `COMM` chunk's size is less than 18 bytes (`AIFF_COMM_SIZE = 2+4+2+10`), **the AIFF decoder shall** set `last_error` to `aifae_BadFile` (-2) and return an error.

### FP-10 Extended Common Chunk
**When** a `COMM` chunk's size is at least 22 bytes (`AIFF_EXT_COMM_SIZE = AIFF_COMM_SIZE + 4`), **the AIFF decoder shall** additionally read the 4-byte big-endian `extTypeID` compression identifier.

### FP-11 Common Chunk Extra Data
**When** the `COMM` chunk contains more data than was parsed, **the AIFF decoder shall** seek past the remaining bytes (`chunk.size - bytes_read`).

### FP-12 Sound Data Chunk Parsing
**When** an `SSND` chunk is encountered, **the AIFF decoder shall** read the 8-byte big-endian sound data header (offset: u32, blockSize: u32) and compute the data start position as `current_file_position + offset`.

### FP-13 Sound Data Chunk Skip
**When** the `SSND` chunk header has been read, **the AIFF decoder shall** seek past the remaining chunk data by `chunk.size - 8` bytes (`AIFF_SSND_SIZE`).

### FP-14 IEEE 754 80-bit Float Conversion
**The AIFF decoder shall** convert the 10-byte big-endian IEEE 754 80-bit extended precision sample rate value to a signed 32-bit integer using the following algorithm:
1. Read 2 bytes as big-endian u16 for sign+exponent
2. Read 4 bytes as big-endian u32 for mantissa high
3. Read 4 bytes as big-endian u32 for mantissa low (discarded)
4. Extract sign as bit 15 of the sign+exponent word
5. Extract exponent as bits 0-14
6. Right-shift mantissa by 1 bit to make room for sign
7. Unbias exponent by subtracting 16383 (2^14 - 1)
8. Calculate shift as `exponent - 31 + 1`
9. If shift > 0, clamp mantissa to `0x7FFFFFFF`
10. If shift < 0, right-shift mantissa by `-shift`
11. Apply sign to produce the final i32 value

### FP-15 Zero-Initialize COMM Data
**When** parsing a `COMM` chunk, **the AIFF decoder shall** zero-initialize the entire extended common chunk structure before populating fields, ensuring that `extTypeID` defaults to 0 for plain AIFF files.

## Sample Format Validation

### SV-1 Bits Per Sample Rounding
**The AIFF decoder shall** round the `sampleSize` field up to the nearest multiple of 8: `bits_per_sample = (sampleSize + 7) & ~7`.

### SV-2 Bits Per Sample Range
**When** the rounded `bits_per_sample` is 0 or greater than 16, **the AIFF decoder shall** log a warning with the unsupported value and return an error.

### SV-3 Channel Count Validation
**When** the `channels` field is not 1 (mono) or 2 (stereo), **the AIFF decoder shall** log a warning with the unsupported channel count and return an error.

### SV-4 Sample Rate Validation
**When** the `sampleRate` field is less than 300 or greater than 128000, **the AIFF decoder shall** log a warning with the unsupported rate and return an error.

### SV-5 Sample Frames Validation
**When** the `sampleFrames` field is 0, **the AIFF decoder shall** log a warning "aiff file has no sound data" and return an error.

### SV-6 SSND Chunk Required
**When** no `SSND` chunk was found during chunk iteration (i.e., `data_ofs` is 0), **the AIFF decoder shall** log a warning "no SSND chunk found" and return an error.

### SV-7 Block Align Calculation
**The AIFF decoder shall** calculate `block_align` as `bits_per_sample / 8 * channels` — the number of bytes per decoded sample frame.

### SV-8 File Block Calculation (PCM)
**While** the compression type is `None` (PCM), **the AIFF decoder shall** set `file_block` equal to `block_align`.

### SV-9 File Block Calculation (SDX2)
**While** the compression type is `Sdx2`, **the AIFF decoder shall** set `file_block` to `block_align / 2`, reflecting the 2:1 compression ratio.

### SV-10 Data Size Calculation
**The AIFF decoder shall** calculate `data_size` as `sampleFrames * file_block`.

### SV-11 Format Code Selection
**The AIFF decoder shall** set the output `format` field based on channels and bits_per_sample:
- 1 channel, 8-bit → `DecoderFormats.mono8`
- 1 channel, 16-bit → `DecoderFormats.mono16`
- 2 channels, 8-bit → `DecoderFormats.stereo8`
- 2 channels, 16-bit → `DecoderFormats.stereo16`

### SV-12 Frequency Assignment
**The AIFF decoder shall** set the output `frequency` field to the parsed `sampleRate` value.

### SV-13 Length Calculation
**The AIFF decoder shall** calculate the total audio length in seconds as `(float)max_pcm / sampleRate` and store it in the `length` field.

## Compression Handling

### CH-1 AIFF PCM Mode
**When** the form type is `AIFF` and `extTypeID` is 0, **the AIFF decoder shall** set the compression type to `None` (uncompressed PCM).

### CH-2 AIFF Extension Rejection
**When** the form type is `AIFF` and `extTypeID` is non-zero, **the AIFF decoder shall** log a warning with the unsupported extension ID and return an error.

### CH-3 AIFC SDX2 Detection
**When** the form type is `AIFC` and `extTypeID` is `0x53445832` (`SDX2`), **the AIFF decoder shall** set the compression type to `Sdx2`.

### CH-4 AIFC Unknown Compression
**When** the form type is `AIFC` and `extTypeID` is not `0x53445832`, **the AIFF decoder shall** log a warning with the unsupported compression ID and return an error.

### CH-5 SDX2 Bits Per Sample
**When** the compression type is `Sdx2` and `bits_per_sample` is not 16, **the AIFF decoder shall** log a warning with the unsupported sample size and return an error.

### CH-6 SDX2 Channel Limit
**When** the compression type is `Sdx2`, **the AIFF decoder shall** verify that `channels <= 4` (`MAX_CHANNELS`).

### CH-7 SDX2 Endianness Override
**When** the compression type is `Sdx2`, **the AIFF decoder shall** set `need_swap` to `(system_big_endian != want_big_endian)` instead of the default AIFF big-endian swap logic, because SDX2 decoding produces samples in machine byte order.

## Decoding — PCM

### DP-1 PCM Frame Count
**The AIFF decoder shall** calculate the number of frames to decode as `min(bufsize / block_align, max_pcm - cur_pcm)`.

### DP-2 PCM File Read
**The AIFF decoder shall** read `dec_pcm` frames of `file_block` bytes each from the file into the output buffer via `uio_fread()`.

### DP-3 PCM Position Update
**When** PCM frames have been read, **the AIFF decoder shall** advance `cur_pcm` by the number of frames actually read.

### DP-4 PCM Return Value
**The AIFF decoder shall** return `dec_pcm * block_align` as the number of bytes decoded.

### DP-5 8-bit Signed-to-Unsigned Conversion
**When** `bits_per_sample` is 8, **the AIFF decoder shall** add 128 to every byte in the decoded output buffer to convert AIFF's signed 8-bit samples to the unsigned 8-bit format expected by the audio system.

### DP-6 PCM EOF
**When** `cur_pcm >= max_pcm`, **the AIFF decoder shall** return 0 bytes, indicating end-of-file.

## Decoding — SDX2

### DS-1 SDX2 Frame Count
**The AIFF decoder shall** calculate the number of SDX2 frames to decode as `min(bufsize / block_align, max_pcm - cur_pcm)`.

### DS-2 SDX2 In-Place Read Strategy
**The AIFF decoder shall** read compressed SDX2 data into the tail end of the output buffer at offset `bufsize - (dec_pcm * file_block)` to enable in-place expansion from compressed bytes to 16-bit samples.

### DS-3 SDX2 File Read
**The AIFF decoder shall** read `dec_pcm` frames of `file_block` bytes each from the file into the tail portion of the output buffer.

### DS-4 SDX2 Position Update
**When** SDX2 frames have been read, **the AIFF decoder shall** advance `cur_pcm` by the number of frames actually read.

### DS-5 SDX2 Decode Algorithm
**For each** sample byte in the compressed data, **for each** channel, **the AIFF decoder shall** apply the SDX2 algorithm:
1. Compute `v = (sample_byte * abs(sample_byte)) << 1` (square with sign preservation, doubled)
2. If the least significant bit of `sample_byte` is 1, add the previous value for this channel: `v += prev_val[channel]`
3. Saturate `v` to the range `[-32768, 32767]`
4. Store `v` as `prev_val[channel]` for the next sample
5. Write `v` as a 16-bit signed sample to the output

### DS-6 SDX2 Channel Iteration
**The AIFF decoder shall** iterate through channels for each sample frame in order, incrementing through the `prev_val[]` array per-channel, producing interleaved channel output.

### DS-7 SDX2 Return Value
**The AIFF decoder shall** return `dec_pcm * block_align` as the number of bytes decoded.

### DS-8 SDX2 Predictor Initialization
**When** the decoder is opened, **the AIFF decoder shall** initialize all `prev_val[]` entries to 0.

## Seeking

### SK-1 Seek Position Clamping
**When** the requested `pcm_pos` exceeds `max_pcm`, **the AIFF decoder shall** clamp it to `max_pcm`.

### SK-2 Seek Position Update
**The AIFF decoder shall** set `cur_pcm` to the clamped `pcm_pos`.

### SK-3 Seek File Position
**The AIFF decoder shall** seek the file to absolute position `data_ofs + pcm_pos * file_block`.

### SK-4 SDX2 Predictor Reset on Seek
**When** seeking, **the AIFF decoder shall** zero all `prev_val[]` entries (SDX2 predictor state) to allow the delta decoder to recover from the mid-stream position.

### SK-5 Seek Return Value
**The AIFF decoder shall** return the actual (clamped) PCM position after seeking.

## Error Handling

### EH-1 Error Get-and-Clear
**When** `GetError()` is called, **the AIFF decoder shall** return the current `last_error` value and reset it to 0.

### EH-2 I/O Error Capture
**When** a file read operation fails, **the AIFF decoder shall** store the system `errno` value in `last_error`.

### EH-3 Format Error Codes
**The AIFF decoder shall** use the following error codes:
- `aifae_None` (0): no error
- `aifae_Unknown` (-1): unknown error
- `aifae_BadFile` (-2): malformed AIFF file
- `aifae_BadArg` (-3): bad argument
- Positive values: system `errno`

### EH-4 Open Failure Cleanup
**When** any validation fails during `Open()`, **the AIFF decoder shall** call `Close()` to release resources before returning the error.

### EH-5 Close Idempotency
**The AIFF decoder shall** support multiple calls to `Close()` without error by checking whether the file handle is non-null before closing.

### EH-6 Term Calls Close
**When** `Term()` is called, **the AIFF decoder shall** call `Close()` to ensure resources are released.

### EH-7 Decode Dispatch Assertion
**When** `Decode()` is called with an unknown compression type, **the AIFF decoder shall** assert failure (this is a programming error, not a runtime error).

## Initialization and Module Lifecycle

### LF-1 GetName
**The AIFF decoder shall** return the string `"AIFF"` from `GetName()`.

### LF-2 InitModule Format Storage
**When** `InitModule()` is called, **the AIFF decoder shall** store the `TFB_DecoderFormats` pointer for use during `Open()` and `Init()`, and return `true`.

### LF-3 InitModule Flags Ignored
**The AIFF decoder shall** ignore the `flags` parameter passed to `InitModule()`.

### LF-4 TermModule No-Op
**The AIFF decoder shall** perform no action in `TermModule()`.

### LF-5 GetStructSize
**The AIFF decoder shall** return `sizeof(TFB_AiffSoundDecoder)` from `GetStructSize()`, providing the total allocation size needed for an instance.

### LF-6 Init Endianness
**When** `Init()` is called, **the AIFF decoder shall** set `need_swap` to `!want_big_endian`, because AIFF stores samples in big-endian format.

### LF-7 GetFrame
**The AIFF decoder shall** return 0 from `GetFrame()`, as AIFF files contain a single audio stream without frame divisions.

### LF-8 Open State Reset
**When** `Open()` is called, **the AIFF decoder shall** zero-initialize `data_size`, `max_pcm`, `data_ofs`, the format header struct, and the `prev_val[]` predictor array before parsing.

### LF-9 Open File Seek to Data
**When** `Open()` completes successfully, **the AIFF decoder shall** seek the file to `data_ofs` and set `cur_pcm` to 0, ready for the first `Decode()` call.

### LF-10 Open Sets Total Length
**When** `Open()` completes successfully, **the AIFF decoder shall** set `max_pcm` to `sampleFrames`, `cur_pcm` to 0, and clear `last_error`.

## FFI Integration

### FF-1 FFI Wrapper Struct
**The AIFF FFI layer shall** define a `#[repr(C)]` struct `TFB_RustAiffDecoder` with `TFB_SoundDecoder` as its first field and a `*mut c_void` pointer to the Rust `AiffDecoder` as its second field.

### FF-2 Vtable Export
**The AIFF FFI layer shall** export a `#[no_mangle] pub static rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs` with all 12 function pointers populated.

### FF-3 Module Format Storage
**The AIFF FFI layer shall** store `DecoderFormats` in a `static Mutex<Option<DecoderFormats>>` during `InitModule()` and clear it during `TermModule()`.

### FF-4 Init Allocation
**When** `Init()` is called via FFI, **the AIFF FFI layer shall** allocate a `Box<AiffDecoder>` via `Box::new(AiffDecoder::new())`, convert it to a raw pointer, and store it in the wrapper struct's `rust_decoder` field.

### FF-5 Term Deallocation
**When** `Term()` is called via FFI, **the AIFF FFI layer shall** reconstruct the `Box<AiffDecoder>` from the raw pointer via `Box::from_raw()` and drop it, then set the pointer to null.

### FF-6 Open File Reading
**When** `Open()` is called via FFI, **the AIFF FFI layer shall** read the entire AIFF file into memory using the UIO virtual filesystem functions (`uio_open`, `uio_fstat`, `uio_read`, `uio_close`), then pass the byte slice to the Rust decoder's `open_from_bytes()` method.

> **Note:** This is an intentional deviation from the C implementation, which streams via `uio_fread` during decode. The Rust architecture choice simplifies decoder state management at the cost of higher memory usage during `Open()`.

### FF-7 Open Base Field Update
**When** `Open()` succeeds via FFI, **the AIFF FFI layer shall** update the `TFB_SoundDecoder` base struct fields: `frequency`, `format` (mapped via `DecoderFormats`), `length`, `is_null` (false),

## Review Notes

*Reviewed by cross-referencing against the actual C source files: aiffaud.c, decoder.h. Issues below are about whether the document accurately describes the C code.*

### Accuracy Issues (resolved in doc body above)

1. **`aifa_Decode` return values**: The doc previously stated `<0` on error per vtable contract. In reality, `aifa_DecodePCM` and `aifa_DecodeSDX2` never return negative values — they return decoded byte count or `0`. → *Fixed in §3 `aifa_Decode` description and §8 Error Handling Patterns.*

2. **Error handling pattern overgeneralization**: The doc previously implied all validation failures use `aifae_BadFile`. In practice, only `aifa_readCommonChunk` sets `aifae_BadFile`; most validation failures in `aifa_Open()` only log and return `false` without setting `last_error`. → *Fixed in §8 Error Handling Patterns (now lists 7 distinct patterns).*

3. **Registry source**: The doc states AIFF is registered with extension `"aif"` in `decoder.c`. This is verified against `decoder.c` line 164 in the project but is external to `aiffaud.c` itself. → *Acceptable; the doc's §9 now references the specific file and line.*

### Completeness Gaps (resolved in doc body above)

1. **remSize robustness**: Chunk traversal doesn't validate chunk sizes against file length. → *Added as behavioral note after §3 `aifa_Open`.*

2. **Duplicate chunks**: `aifa_Open()` silently overwrites with last-parsed `COMM`/`SSND` values. → *Added as behavioral note.*

3. **`aifa_Seek` ignores seek failure**: `uio_fseek` return not checked; `cur_pcm` updated regardless. → *Fixed in §3 `aifa_Seek` description.*

4. **SDX2 debug-only assert**: `assert(channels <= MAX_CHANNELS)` is debug-only; no runtime check in release. → *Added as behavioral note.*

### EARS Issues (noted, not all resolved)

1. **Design-prescriptive requirements**: Some requirements (e.g., FP-14 SDX2 algorithm steps) describe internal implementation rather than observable behavior. This is intentional — the C algorithm must be matched exactly for audio fidelity, so implementation lock is appropriate here.

2. **FF-6 read-into-memory**: The FFI section requires `Open()` to read the entire file into memory, but the C implementation streams via `uio_fread`. This is noted as an intentional Rust architecture choice (not a C behavior requirement) and should be read as such.

3. **Requirement granularity**: Some requirements bundle multiple outcomes (e.g., LF-10). These could be split for verification but are left as-is since they describe a single logical operation in the C code.