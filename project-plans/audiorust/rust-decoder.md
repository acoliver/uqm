# AIFF Audio Decoder — Rust Functional & Technical Specification

## Source Files (to be created)

| File | Role |
|------|------|
| `rust/src/sound/aiff.rs` | Pure Rust AIFF/AIFC decoder — format parsing, PCM decode, SDX2 decode |
| `rust/src/sound/aiff_ffi.rs` | C FFI bridge — vtable, UIO file I/O, `TFB_SoundDecoder` base struct management |

## Reference Files (existing)

| File | Role |
|------|------|
| `rust/src/sound/decoder.rs` | `SoundDecoder` trait, `DecodeError` enum, `DecodeResult<T>` type alias |
| `rust/src/sound/formats.rs` | `AudioFormat` enum, `DecoderFormats` struct |
| `rust/src/sound/ffi.rs` | `TFB_SoundDecoder`, `TFB_SoundDecoderFuncs`, `TFB_DecoderFormats`, `uio_DirHandle` |
| `rust/src/sound/wav.rs` | WAV decoder (pattern reference — similar chunk-based format) |
| `rust/src/sound/wav_ffi.rs` | WAV FFI bridge (pattern reference) |
| `rust/src/sound/dukaud.rs` | DukAud decoder (pattern reference — ADPCM decompression) |
| `rust/src/sound/dukaud_ffi.rs` | DukAud FFI bridge (pattern reference) |
| `sc2/src/libs/sound/decoders/aiffaud.c` | C reference implementation |
| `sc2/src/libs/sound/decoders/aiffaud.h` | C error enum, vtable extern |

---

# Part 1: Architecture Overview

## Decoder Ecosystem

```
                      ┌─────────────────────────────────┐
                      │      decoder.rs                  │
                      │  trait SoundDecoder              │
                      │  enum DecodeError                │
                      │  type DecodeResult<T>            │
                      └───────────┬─────────────────────┘
                                  │ impl SoundDecoder
          ┌───────────┬───────────┼───────────┬──────────────┐
          │           │           │           │              │
      wav.rs      ogg.rs    dukaud.rs   mod_decoder.rs   aiff.rs ← NEW
          │           │           │           │              │
      wav_ffi.rs  ffi.rs    dukaud_ffi.rs mod_ffi.rs   aiff_ffi.rs ← NEW
          │           │           │           │              │
          └───────────┴───────────┴───────────┴──────────────┘
                                  │
                      ┌───────────▼───────────┐
                      │   C decoder.c         │
                      │   sd_decoders[] table  │
                      │   ext: "aif"           │
                      └───────────────────────┘
```

The AIFF decoder follows the established two-file pattern:

- **`aiff.rs`** — Pure Rust, no `unsafe`, implements `SoundDecoder` trait. Parses AIFF/AIFC from an in-memory byte slice. Handles PCM and SDX2 decoding. Fully unit-testable.
- **`aiff_ffi.rs`** — Thin FFI shim. Exports a `TFB_SoundDecoderFuncs` vtable. Reads files via UIO, manages `Box<AiffDecoder>` lifetime, maps `AudioFormat` to C format codes.

## Trait Implementation

`AiffDecoder` implements `SoundDecoder` from `decoder.rs`. The trait requires `Send`, which `AiffDecoder` satisfies because it holds only owned data (`Vec<u8>`, primitive fields) with no `Rc`, raw pointers, or non-`Send` types.

Key trait methods and their AIFF-specific behavior:

| Trait Method | AIFF Behavior |
|-------------|---------------|
| `name()` | Returns `"AIFF"` |
| `init_module(flags, formats)` | Stores `DecoderFormats`, ignores `flags`, returns `true` |
| `term_module()` | Clears stored formats |
| `get_error()` | Returns and clears `last_error: i32` |
| `init()` | Sets `need_swap = !formats.want_big_endian`, returns `true` |
| `term()` | Calls `close()` |
| `open(path)` | Reads file via `std::fs::read`, delegates to `open_from_bytes` |
| `open_from_bytes(data, name)` | Parses AIFF/AIFC, validates, sets up decode state |
| `close()` | Clears audio data and resets state |
| `decode(buf)` | Dispatches to PCM or SDX2 decode path |
| `seek(pcm_pos)` | Clamps position, resets SDX2 predictor state |
| `get_frame()` | Returns `0` (AIFF has no frame divisions) |
| `frequency()` | Returns parsed sample rate |
| `format()` | Returns `AudioFormat` variant |
| `length()` | Returns `max_pcm as f32 / frequency as f32` |
| `is_null()` | Returns `false` |
| `needs_swap()` | Returns `need_swap` |

---

# Part 2: Module Descriptions

## 2.1 `aiff.rs` — Pure Rust Decoder

### Constants

```rust
// Chunk IDs (big-endian 4CC)
const FORM_ID: u32 = 0x464F524D;        // "FORM"
const FORM_TYPE_AIFF: u32 = 0x41494646; // "AIFF"
const FORM_TYPE_AIFC: u32 = 0x41494643; // "AIFC"
const COMMON_ID: u32 = 0x434F4D4D;      // "COMM"
const SOUND_DATA_ID: u32 = 0x53534E44;  // "SSND"
const SDX2_COMPRESSION: u32 = 0x53445832; // "SDX2"

// Chunk sizes
const AIFF_COMM_SIZE: usize = 18;       // 2 + 4 + 2 + 10
const AIFF_EXT_COMM_SIZE: usize = 22;   // AIFF_COMM_SIZE + 4
const AIFF_SSND_SIZE: usize = 8;        // 4 + 4

// Limits
const MAX_CHANNELS: usize = 4;
const MIN_SAMPLE_RATE: i32 = 300;
const MAX_SAMPLE_RATE: i32 = 128_000;
```

### Types

```rust
/// AIFF compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompressionType {
    None,  // Uncompressed PCM
    Sdx2,  // SDX2 ADPCM
}

/// Parsed COMM chunk fields
#[derive(Debug, Clone, Default)]
struct CommonChunk {
    channels: u16,
    sample_frames: u32,
    sample_size: u16,        // Raw bits-per-sample from file
    sample_rate: i32,        // Converted from 80-bit float
    ext_type_id: u32,        // Compression ID (0 for plain AIFF)
}

/// Parsed SSND chunk header
#[derive(Debug, Clone, Default)]
struct SoundDataHeader {
    offset: u32,
    block_size: u32,
}

/// Chunk header (8 bytes)
#[derive(Debug, Clone)]
struct ChunkHeader {
    id: u32,
    size: u32,
}
```

### `AiffDecoder` Struct

```rust
pub struct AiffDecoder {
    // Format metadata
    frequency: u32,
    format: AudioFormat,
    length: f32,
    need_swap: bool,

    // Error state
    last_error: i32,

    // Stored formats
    formats: Option<DecoderFormats>,

    // Initialization state
    initialized: bool,

    // Parsed header data
    common: CommonChunk,
    comp_type: CompressionType,
    bits_per_sample: u32,     // Rounded up to multiple of 8
    block_align: u32,         // Bytes per output sample frame
    file_block: u32,          // Bytes per file sample frame

    // Audio data (loaded into memory)
    data: Vec<u8>,            // Raw encoded audio data (PCM or SDX2)
    data_pos: usize,          // Current byte offset within data

    // PCM tracking
    max_pcm: u32,             // Total sample frames
    cur_pcm: u32,             // Current sample frame position

    // SDX2 predictor state
    prev_val: [i32; MAX_CHANNELS],
}
```

### Public API

```rust
impl AiffDecoder {
    pub fn new() -> Self;
}
```

The `SoundDecoder` trait provides the full public interface. There are no additional public methods beyond the trait.

### Internal Methods

```rust
impl AiffDecoder {
    // Parsing helpers (operate on Cursor<&[u8]>)
    fn read_be_u16(cursor: &mut Cursor<&[u8]>) -> DecodeResult<u16>;
    fn read_be_u32(cursor: &mut Cursor<&[u8]>) -> DecodeResult<u32>;
    fn read_be_i16(cursor: &mut Cursor<&[u8]>) -> DecodeResult<i16>;
    fn read_chunk_header(cursor: &mut Cursor<&[u8]>) -> DecodeResult<ChunkHeader>;
    fn read_common_chunk(cursor: &mut Cursor<&[u8]>, chunk_size: u32) -> DecodeResult<CommonChunk>;
    fn read_sound_data_header(cursor: &mut Cursor<&[u8]>) -> DecodeResult<SoundDataHeader>;
    fn read_be_f80(cursor: &mut Cursor<&[u8]>) -> DecodeResult<i32>;

    // Decode paths
    fn decode_pcm(&mut self, buf: &mut [u8]) -> DecodeResult<usize>;
    fn decode_sdx2(&mut self, buf: &mut [u8]) -> DecodeResult<usize>;
}
```

### Error Handling

`aiff.rs` uses `DecodeResult<T>` (i.e., `Result<T, DecodeError>`) from `decoder.rs`. The mapping from C error codes:

| C Error | Rust `DecodeError` Variant | Context |
|---------|---------------------------|---------|
| `aifae_BadFile` (-2) | `InvalidData(String)` | Malformed AIFF structure |
| `aifae_BadArg` (-3) | `DecoderError(String)` | Bad argument |
| `errno` (positive) | `IoError(String)` | I/O failure |
| validation log + `false` | `InvalidData(String)` or `UnsupportedFormat(String)` | Open-time validation |
| decode returns 0 | `DecodeError::EndOfFile` | No more frames |
| unknown comp_type | `DecoderError(String)` | Programming error (replaces C `assert`) |

The `last_error: i32` field retains C-compatible error codes for the FFI layer's `GetError()`. The pure Rust API uses `Result` types throughout.

### Architectural Choice: In-Memory vs Streaming

The C implementation streams audio data via `uio_fread()` during `Decode()`. The Rust implementation loads the entire audio data segment into `self.data: Vec<u8>` during `open_from_bytes()`, consistent with the WAV decoder pattern.

This means:
- `decode_pcm()` reads from `self.data[self.data_pos..]` instead of calling `uio_fread()`
- `decode_sdx2()` reads from `self.data[self.data_pos..]` instead of calling `uio_fread()`
- `seek()` repositions `self.data_pos` instead of calling `uio_fseek()`
- No file handle state in the pure Rust decoder

The trade-off: slightly higher memory usage during playback, but dramatically simpler state management, no `unsafe` file I/O, and full testability with synthetic byte arrays.

---

## 2.2 `aiff_ffi.rs` — C FFI Bridge

### FFI Wrapper Struct

```rust
#[repr(C)]
pub struct TFB_RustAiffDecoder {
    pub base: TFB_SoundDecoder,          // Must be first field
    pub rust_decoder: *mut c_void,       // Points to Box<AiffDecoder>
}
```

### Module-Level State

```rust
static RUST_AIFA_FORMATS: Mutex<Option<DecoderFormats>> = Mutex::new(None);
static RUST_AIFA_NAME: &[u8] = b"Rust AIFF\0";
```

### Exported Vtable

```rust
#[no_mangle]
pub static rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
    GetName: rust_aifa_GetName,
    InitModule: rust_aifa_InitModule,
    TermModule: rust_aifa_TermModule,
    GetStructSize: rust_aifa_GetStructSize,
    GetError: rust_aifa_GetError,
    Init: rust_aifa_Init,
    Term: rust_aifa_Term,
    Open: rust_aifa_Open,
    Close: rust_aifa_Close,
    Decode: rust_aifa_Decode,
    Seek: rust_aifa_Seek,
    GetFrame: rust_aifa_GetFrame,
};
```

### FFI Function Signatures

All 12 functions follow the pattern established in `dukaud_ffi.rs` and `wav_ffi.rs`:

| Function | Signature | Behavior |
|----------|-----------|----------|
| `rust_aifa_GetName` | `() -> *const c_char` | Returns `RUST_AIFA_NAME` pointer |
| `rust_aifa_InitModule` | `(flags: c_int, fmts: *const TFB_DecoderFormats) -> c_int` | Stores formats in `RUST_AIFA_FORMATS`, ignores `flags`, returns 1 |
| `rust_aifa_TermModule` | `()` | Clears `RUST_AIFA_FORMATS` |
| `rust_aifa_GetStructSize` | `() -> u32` | Returns `size_of::<TFB_RustAiffDecoder>()` |
| `rust_aifa_GetError` | `(*mut TFB_SoundDecoder) -> c_int` | Calls `dec.get_error()`, returns result |
| `rust_aifa_Init` | `(*mut TFB_SoundDecoder) -> c_int` | Creates `Box::new(AiffDecoder::new())`, stores raw pointer, returns 1 |
| `rust_aifa_Term` | `(*mut TFB_SoundDecoder)` | Reconstructs `Box::from_raw()`, drops it, sets pointer to null |
| `rust_aifa_Open` | `(*mut TFB_SoundDecoder, *mut uio_DirHandle, *const c_char) -> c_int` | Reads file via UIO, calls `open_from_bytes()`, updates base fields |
| `rust_aifa_Close` | `(*mut TFB_SoundDecoder)` | Calls `dec.close()` |
| `rust_aifa_Decode` | `(*mut TFB_SoundDecoder, *mut c_void, i32) -> c_int` | Calls `dec.decode()`, maps EndOfFile→0, errors→0 |
| `rust_aifa_Seek` | `(*mut TFB_SoundDecoder, u32) -> u32` | Calls `dec.seek()`, returns clamped position |
| `rust_aifa_GetFrame` | `(*mut TFB_SoundDecoder) -> u32` | Calls `dec.get_frame()`, returns 0 |

### Null Safety

Every FFI function checks for null `decoder` pointer before dereferencing. Every function checks for null `rust_decoder` before casting. This matches the pattern in all existing FFI modules.

### UIO File Reading

`rust_aifa_Open` reads the AIFF file into memory using the same UIO helper pattern as `dukaud_ffi.rs` and `wav_ffi.rs`:

```rust
unsafe fn read_uio_file(dir: *mut uio_DirHandle, path: *const c_char) -> Option<Vec<u8>>;
```

Steps: `uio_open` → `uio_fstat` (get size) → `uio_read` (loop until complete) → `uio_close`. Returns `None` on any failure.

### Format Mapping

`rust_aifa_Open` maps `AiffDecoder::format() -> AudioFormat` to C format codes via the locked `RUST_AIFA_FORMATS`:

```rust
match dec.format() {
    AudioFormat::Mono8   => formats.mono8,
    AudioFormat::Stereo8 => formats.stereo8,
    AudioFormat::Mono16  => formats.mono16,
    AudioFormat::Stereo16 => formats.stereo16,
}
```

### Decode Return Value Mapping

The C AIFF decoder's `Decode()` never returns negative values. The FFI layer matches this:

| Rust `DecodeResult` | FFI return |
|---------------------|------------|
| `Ok(n)` | `n as c_int` |
| `Err(DecodeError::EndOfFile)` | `0` |
| `Err(_)` | `0` |

### Logging

All significant FFI operations use `rust_bridge_log_msg()` for diagnostics, matching the existing decoders' logging patterns (e.g., `"RUST_AIFF_OPEN: filename"`, `"RUST_AIFF_OPEN: OK freq=X format=Y length=Zs"`).

---

# Part 3: Functional & Technical Specification

## 3.1 AIFF/AIFC File Parsing

### File Header (12 bytes)

The decoder reads the first 12 bytes from the input data as:
1. `chunk_id: u32` — big-endian, must equal `FORM_ID` (`0x464F524D`)
2. `chunk_size: u32` — big-endian, remaining data size after this field
3. `form_type: u32` — big-endian, must be `FORM_TYPE_AIFF` or `FORM_TYPE_AIFC`

Failure at any step produces `DecodeError::InvalidData` with a descriptive message.

### Chunk Iteration

After consuming the 12-byte file header, the decoder iterates through chunks using `remaining = chunk_size - 4` (the 4-byte form type already consumed). For each iteration:

1. Read 8-byte chunk header: `id: u32` (BE), `size: u32` (BE)
2. Subtract `8 + size` (plus 1 if `size` is odd, for alignment padding) from `remaining`
3. Dispatch on `id`:
   - `COMMON_ID` → parse Common Chunk
   - `SOUND_DATA_ID` → parse Sound Data Chunk
   - Anything else → skip `size` bytes
4. If `size` is odd, skip 1 additional padding byte

### Common Chunk (COMM) Parsing

Read from the chunk's data region:
1. `channels: u16` (BE)
2. `sample_frames: u32` (BE)
3. `sample_size: u16` (BE) — raw bits-per-sample
4. `sample_rate: i32` — converted from 10-byte IEEE 754 80-bit float via `read_be_f80()`

If `chunk_size >= AIFF_EXT_COMM_SIZE` (22):
5. `ext_type_id: u32` (BE) — compression identifier

If `chunk_size < AIFF_COMM_SIZE` (18):
- Set `last_error` to `-2` (BadFile equivalent), return `InvalidData`

If more data remains in the chunk after parsing, seek past it.

The `CommonChunk` struct is zero-initialized before populating, so `ext_type_id` defaults to `0` for plain AIFF files that lack the extended field.

### Sound Data Chunk (SSND) Parsing

Read from the chunk's data region:
1. `offset: u32` (BE) — offset to first sample within data block
2. `block_size: u32` (BE) — alignment block size (typically 0)

Compute `data_start = current_cursor_position + offset`.

The actual audio data is extracted as `data[data_start .. data_start + (chunk_size - AIFF_SSND_SIZE - offset)]`, or more precisely, the total data size is determined later from `sample_frames * file_block`.

### Duplicate Chunk Handling

If multiple `COMM` or `SSND` chunks appear, later values silently overwrite earlier ones. No duplicate-chunk rejection is performed, matching C behavior.

## 3.2 IEEE 754 80-bit Float Parsing

The `read_be_f80()` method converts the 10-byte AIFF sample rate field to `i32`:

1. Read `se: u16` (BE) — sign+exponent
2. Read `mantissa_hi: u32` (BE) — high 32 bits of mantissa
3. Read `_mantissa_lo: u32` (BE) — low 32 bits (discarded)
4. `sign = (se >> 15) & 1`
5. `exponent = se & 0x7FFF`
6. `mantissa = (mantissa_hi >> 1) as i32` — shift right by 1 to fit in signed 31 bits
7. `exponent -= 16383` — unbias (2^14 - 1)
8. `shift = exponent - 31 + 1`
9. If `shift > 0`: `mantissa = 0x7FFF_FFFF` (overflow clamp)
10. If `shift < 0`: `mantissa >>= -shift` (arithmetic right shift)
11. If `sign == 1`: `mantissa = -mantissa`
12. Return `mantissa`

The 80-bit IEEE 754 format has an explicit integer bit (bit 63 of the mantissa), unlike 64-bit double. The code handles this by not adding an implied `1` bit. Precision is truncated to 31 bits, which is sufficient for all valid sample rates (integer values 300–128000).

## 3.3 PCM Decoding

### Frame Calculation

```
dec_pcm = min(bufsize / block_align, max_pcm - cur_pcm)
```

If `cur_pcm >= max_pcm`, return `Err(DecodeError::EndOfFile)`.

### Data Read

Copy `dec_pcm * file_block` bytes from `self.data[self.data_pos..]` into the output buffer.

### Position Update

```
cur_pcm += dec_pcm
data_pos += dec_pcm * file_block
```

### 8-bit Signed-to-Unsigned Conversion

When `bits_per_sample == 8`, AIFF stores samples as signed (-128..127). The audio system expects unsigned (0..255). The decoder adds 128 (wrapping) to every byte in the output:

```rust
for byte in &mut buf[..decoded_bytes] {
    *byte = byte.wrapping_add(128);
}
```

### Return Value

Returns `Ok(dec_pcm as usize * block_align as usize)` — the number of bytes written to the output buffer.

## 3.4 SDX2 ADPCM Decompression

### Frame Calculation

```
dec_pcm = min(bufsize / block_align, max_pcm - cur_pcm)
```

### Data Read

Read `dec_pcm * file_block` bytes of compressed data from `self.data[self.data_pos..]`. For in-memory decoding, this is a slice reference — no in-place buffer trick is needed (the C code's in-place expansion from the tail of the output buffer is a file-streaming optimization that doesn't apply here).

### SDX2 Algorithm

For each sample frame, for each channel `ch` in `0..channels`:

```rust
let sample_byte = compressed[i] as i8;
let abs_val = (sample_byte as i32).abs();
let mut v = ((sample_byte as i32) * abs_val) << 1;  // square with sign, doubled

if (sample_byte & 1) != 0 {
    v += self.prev_val[ch];   // odd → delta mode (add predictor)
}

v = v.clamp(-32768, 32767);   // saturate to i16 range
self.prev_val[ch] = v;        // update predictor

// Write v as i16 to output (respecting endianness via need_swap)
```

Output samples are 16-bit signed. If `need_swap` is true, bytes are swapped when writing to the output buffer.

### Channel Iteration

Channels are interleaved: for a stereo file, each frame contains `[ch0_byte, ch1_byte]` in the compressed data and `[ch0_i16, ch1_i16]` in the output.

### Position Update

```
cur_pcm += dec_pcm
data_pos += dec_pcm * file_block
```

### Return Value

Returns `Ok(dec_pcm as usize * block_align as usize)`.

## 3.5 Seeking

### Position Clamping

```rust
let pcm_pos = pcm_pos.min(self.max_pcm);
```

### State Update

```rust
self.cur_pcm = pcm_pos;
self.data_pos = (pcm_pos as usize) * (self.file_block as usize);
```

### Predictor Reset

```rust
self.prev_val = [0i32; MAX_CHANNELS];
```

The predictor is always reset on seek, even for PCM mode (no-op since PCM doesn't use it). This ensures SDX2 delta recovery from any position.

### Return Value

Returns `Ok(pcm_pos)` — the clamped position.

## 3.6 Error Handling

### Error Enum (in `decoder.rs`, already exists)

```rust
pub enum DecodeError {
    NotFound(String),
    InvalidData(String),
    UnsupportedFormat(String),
    IoError(String),
    NotInitialized,
    EndOfFile,
    SeekFailed(String),
    DecoderError(String),
}
```

### `last_error` Field

The `AiffDecoder` maintains a `last_error: i32` field for C FFI compatibility:
- `0` = no error
- `-1` = unknown error
- `-2` = bad file (malformed AIFF)
- `-3` = bad argument
- Positive = system errno (not applicable in pure Rust, but retained for interface compatibility)

The `get_error()` method returns the current value and resets it to 0.

### Open Failure Cleanup

When any validation fails during `open_from_bytes()`, the method calls `self.close()` before returning `Err(...)`, ensuring no partial state persists.

### Close Idempotency

`close()` is safe to call multiple times. It clears `self.data`, resets positions, and clears predictor state. No file handle to manage.

## 3.7 Format Detection

| `channels` | `bits_per_sample` | `AudioFormat` |
|------------|-------------------|---------------|
| 1 | 8 | `AudioFormat::Mono8` |
| 1 | 16 | `AudioFormat::Mono16` |
| 2 | 8 | `AudioFormat::Stereo8` |
| 2 | 16 | `AudioFormat::Stereo16` |

## 3.8 Endianness Handling

### PCM Mode

AIFF stores samples in big-endian format. `need_swap` is set to `!formats.want_big_endian` during `init()`. If `need_swap` is `true` and `bits_per_sample` is 16, the decoder swaps each pair of bytes in the output.

### SDX2 Mode

SDX2 decoding produces samples in machine byte order (the arithmetic operations naturally produce native-endian values). `need_swap` is recalculated during `open_from_bytes()` as:

```rust
self.need_swap = cfg!(target_endian = "big") != self.formats.unwrap().want_big_endian;
```

This matches the C logic: `big_endian != want_big_endian`.

## 3.9 Validation (during `open_from_bytes`)

The following validations are performed in order after chunk iteration:

1. `sample_frames != 0` — "aiff file has no sound data"
2. `bits_per_sample = (sample_size + 7) & !7` — round up, must be 8 or 16
3. `channels` must be 1 or 2
4. `sample_rate` must be in `[300, 128000]`
5. `data_ofs != 0` — SSND chunk must have been found
6. Compression type validation:
   - AIFF: `ext_type_id` must be 0
   - AIFC: `ext_type_id` must be `SDX2_COMPRESSION`
7. SDX2: `bits_per_sample` must be 16
8. SDX2: `channels <= MAX_CHANNELS` (4)

Each validation failure returns `Err(DecodeError::InvalidData(...))` or `Err(DecodeError::UnsupportedFormat(...))` with a descriptive message.

---

# Part 4: EARS Requirements (Rust)

All requirement IDs match the C specification from `c-decoder.md` 1:1. Requirements are expressed in Rust terms.

## Format Parsing

### FP-1 File Header Validation
**When** the decoder's `open_from_bytes()` method is called, **the AIFF decoder shall** read the first 12 bytes as a big-endian FORM file header by parsing a 4-byte chunk ID (`u32`), a 4-byte chunk size (`u32`), and a 4-byte form type (`u32`) using `Cursor<&[u8]>` with big-endian byte order.

### FP-2 FORM ID Check
**When** the file header chunk ID is not `0x464F524D` (`FORM`), **the AIFF decoder shall** return `Err(DecodeError::InvalidData(...))` with a message containing the invalid ID value.

### FP-3 Form Type Check
**When** the file header form type is neither `0x41494646` (`AIFF`) nor `0x41494643` (`AIFC`), **the AIFF decoder shall** return `Err(DecodeError::InvalidData(...))` with a message containing the unsupported type value.

### FP-4 Chunk Iteration
**When** the file header has been validated, **the AIFF decoder shall** iterate over all chunks by reading each chunk's 8-byte big-endian header (`id: u32` + `size: u32`) from the `Cursor`, until `remaining` bytes (computed per FP-6) are exhausted.

### FP-5 Chunk Alignment
**The AIFF decoder shall** advance the `Cursor` position by 1 byte after processing each chunk whose `size` is odd, to maintain 2-byte alignment.

### FP-6 Remaining Size Calculation
**The AIFF decoder shall** calculate the remaining data for chunk iteration as `file_header.chunk_size - 4` (subtracting the 4-byte form type already consumed).

### FP-7 Unknown Chunk Skipping
**When** a chunk ID does not match `COMMON_ID` (`0x434F4D4D`) or `SOUND_DATA_ID` (`0x53534E44`), **the AIFF decoder shall** advance the `Cursor` position by the chunk's `size` value using `SeekFrom::Current`.

### FP-8 Common Chunk Parsing
**When** a `COMM` chunk is encountered, **the AIFF decoder shall** parse the following big-endian fields in order: `channels` (`u16`), `sample_frames` (`u32`), `sample_size` (`u16`), and `sample_rate` (10-byte IEEE 754 80-bit float converted to `i32` via `read_be_f80()`).

### FP-9 Common Chunk Minimum Size
**When** a `COMM` chunk's `size` field is less than 18 (`AIFF_COMM_SIZE`), **the AIFF decoder shall** set `self.last_error` to `-2` and return `Err(DecodeError::InvalidData("COMM chunk too small"))`.

### FP-10 Extended Common Chunk
**When** a `COMM` chunk's `size` field is at least 22 (`AIFF_EXT_COMM_SIZE`), **the AIFF decoder shall** additionally read a big-endian `u32` as the `ext_type_id` compression identifier.

### FP-11 Common Chunk Extra Data
**When** the `COMM` chunk contains more data than the fields parsed in FP-8/FP-10, **the AIFF decoder shall** advance the `Cursor` past the remaining bytes (`chunk_size - bytes_consumed`).

### FP-12 Sound Data Chunk Parsing
**When** an `SSND` chunk is encountered, **the AIFF decoder shall** read a big-endian `offset: u32` and `block_size: u32`, then compute `data_start` as `cursor.position() + offset`.

### FP-13 Sound Data Chunk Skip
**When** the `SSND` chunk header has been read, **the AIFF decoder shall** advance the `Cursor` past the remaining chunk data by `chunk_size - 8` (`AIFF_SSND_SIZE`) bytes.

### FP-14 IEEE 754 80-bit Float Conversion
**The AIFF decoder shall** implement `read_be_f80()` to convert the 10-byte big-endian IEEE 754 80-bit extended precision sample rate to an `i32` using the following algorithm:
1. Read 2 bytes as big-endian `u16` for sign+exponent (`se`)
2. Read 4 bytes as big-endian `u32` for mantissa high
3. Read 4 bytes as big-endian `u32` for mantissa low (discarded)
4. Extract sign: `(se >> 15) & 1`
5. Extract exponent: `se & 0x7FFF`
6. Shift mantissa right by 1: `mantissa = (mantissa_hi >> 1) as i32`
7. Unbias exponent: `exponent -= 16383`
8. Calculate shift: `shift = exponent - 31 + 1`
9. If `shift > 0`: clamp `mantissa` to `0x7FFF_FFFF_i32`
10. If `shift < 0`: arithmetic right-shift `mantissa` by `-shift`
11. If `sign == 1`: negate `mantissa`
12. Return the `i32` result

### FP-15 Zero-Initialize COMM Data
**When** parsing a `COMM` chunk, **the AIFF decoder shall** start with a default-initialized `CommonChunk` struct (all fields zero) before populating parsed fields, ensuring `ext_type_id` defaults to `0` for plain AIFF files.

## Sample Format Validation

### SV-1 Bits Per Sample Rounding
**The AIFF decoder shall** compute `bits_per_sample` as `(sample_size + 7) & !7`, rounding up the raw `sample_size` from the `COMM` chunk to the nearest multiple of 8.

### SV-2 Bits Per Sample Range
**When** the rounded `bits_per_sample` is 0 or greater than 16, **the AIFF decoder shall** return `Err(DecodeError::UnsupportedFormat(...))` with a message containing the unsupported value.

### SV-3 Channel Count Validation
**When** the `channels` field is not 1 or 2, **the AIFF decoder shall** return `Err(DecodeError::UnsupportedFormat(...))` with a message containing the unsupported channel count.

### SV-4 Sample Rate Validation
**When** the parsed `sample_rate` (as `i32`) is less than 300 or greater than 128000, **the AIFF decoder shall** return `Err(DecodeError::UnsupportedFormat(...))` with a message containing the unsupported rate.

### SV-5 Sample Frames Validation
**When** the `sample_frames` field is 0, **the AIFF decoder shall** return `Err(DecodeError::InvalidData("aiff file has no sound data"))`.

### SV-6 SSND Chunk Required
**When** no `SSND` chunk was found during chunk iteration (i.e., no `data_start` was computed), **the AIFF decoder shall** return `Err(DecodeError::InvalidData("no SSND chunk found"))`.

### SV-7 Block Align Calculation
**The AIFF decoder shall** calculate `block_align` as `(bits_per_sample / 8) * channels as u32` — the number of bytes per decoded output sample frame.

### SV-8 File Block Calculation (PCM)
**When** the compression type is `CompressionType::None`, **the AIFF decoder shall** set `file_block` equal to `block_align`.

### SV-9 File Block Calculation (SDX2)
**When** the compression type is `CompressionType::Sdx2`, **the AIFF decoder shall** set `file_block` to `block_align / 2`, reflecting the 2:1 compression ratio (1 byte → 2 bytes per sample).

### SV-10 Data Size Calculation
**The AIFF decoder shall** extract the audio data from the input byte slice as `data[data_start .. data_start + sample_frames * file_block]` and store it in `self.data: Vec<u8>`.

### SV-11 Format Code Selection
**The AIFF decoder shall** set `self.format: AudioFormat` based on `channels` and `bits_per_sample`:
- `(1, 8)` → `AudioFormat::Mono8`
- `(1, 16)` → `AudioFormat::Mono16`
- `(2, 8)` → `AudioFormat::Stereo8`
- `(2, 16)` → `AudioFormat::Stereo16`

### SV-12 Frequency Assignment
**The AIFF decoder shall** set `self.frequency` to the parsed `sample_rate` value cast as `u32`.

### SV-13 Length Calculation
**The AIFF decoder shall** calculate `self.length` as `max_pcm as f32 / frequency as f32`.

## Compression Handling

### CH-1 AIFF PCM Mode
**When** the form type is `FORM_TYPE_AIFF` and `ext_type_id == 0`, **the AIFF decoder shall** set `self.comp_type` to `CompressionType::None`.

### CH-2 AIFF Extension Rejection
**When** the form type is `FORM_TYPE_AIFF` and `ext_type_id != 0`, **the AIFF decoder shall** return `Err(DecodeError::UnsupportedFormat(...))` with a message containing the unexpected extension ID.

### CH-3 AIFC SDX2 Detection
**When** the form type is `FORM_TYPE_AIFC` and `ext_type_id == 0x53445832` (`SDX2`), **the AIFF decoder shall** set `self.comp_type` to `CompressionType::Sdx2`.

### CH-4 AIFC Unknown Compression
**When** the form type is `FORM_TYPE_AIFC` and `ext_type_id != 0x53445832`, **the AIFF decoder shall** return `Err(DecodeError::UnsupportedFormat(...))` with a message containing the unsupported compression ID.

### CH-5 SDX2 Bits Per Sample
**When** the compression type is `CompressionType::Sdx2` and `bits_per_sample != 16`, **the AIFF decoder shall** return `Err(DecodeError::UnsupportedFormat(...))` with a message containing the unsupported sample size.

### CH-6 SDX2 Channel Limit
**When** the compression type is `CompressionType::Sdx2`, **the AIFF decoder shall** verify that `channels as usize <= MAX_CHANNELS` (4) and return `Err(DecodeError::UnsupportedFormat(...))` if exceeded. Unlike the C implementation's debug-only `assert`, this is a runtime check in all build profiles.

### CH-7 SDX2 Endianness Override
**When** the compression type is `CompressionType::Sdx2`, **the AIFF decoder shall** override `self.need_swap` to `cfg!(target_endian = "big") != self.formats.unwrap().want_big_endian`, because SDX2 decoding produces samples in machine byte order rather than big-endian.

## Decoding — PCM

### DP-1 PCM Frame Count
**The AIFF decoder shall** calculate the number of PCM frames to decode as `min(buf.len() as u32 / block_align, max_pcm - cur_pcm)`.

### DP-2 PCM Data Read
**The AIFF decoder shall** copy `dec_pcm * file_block` bytes from `self.data[self.data_pos..]` into the output buffer `buf`.

### DP-3 PCM Position Update
**After** copying PCM data, **the AIFF decoder shall** advance `self.cur_pcm` by `dec_pcm` and `self.data_pos` by `dec_pcm as usize * file_block as usize`.

### DP-4 PCM Return Value
**The AIFF decoder shall** return `Ok(dec_pcm as usize * block_align as usize)` — the number of bytes written to the output buffer.

### DP-5 8-bit Signed-to-Unsigned Conversion
**When** `self.bits_per_sample == 8`, **the AIFF decoder shall** apply `byte.wrapping_add(128)` to every byte in the output buffer range `buf[..decoded_bytes]` to convert AIFF's signed 8-bit samples to unsigned 8-bit.

### DP-6 PCM EOF
**When** `self.cur_pcm >= self.max_pcm`, **the AIFF decoder shall** return `Err(DecodeError::EndOfFile)`.

## Decoding — SDX2

### DS-1 SDX2 Frame Count
**The AIFF decoder shall** calculate the number of SDX2 frames to decode as `min(buf.len() as u32 / block_align, max_pcm - cur_pcm)`.

### DS-2 SDX2 Data Read
**The AIFF decoder shall** read `dec_pcm * file_block` bytes of compressed data from `self.data[self.data_pos..]` as a byte slice. Since the Rust implementation operates on in-memory data, the C code's in-place tail-of-buffer read strategy is not required; the compressed data is read from `self.data` and decoded samples are written directly to the output buffer.

### DS-3 SDX2 Position Update
**After** decoding SDX2 data, **the AIFF decoder shall** advance `self.cur_pcm` by `dec_pcm` and `self.data_pos` by `dec_pcm as usize * file_block as usize`.

### DS-4 SDX2 Decode Algorithm
**For each** compressed byte, **for each** channel `ch` in `0..channels`, **the AIFF decoder shall** apply:
1. Cast the byte to `i8`, then to `i32` as `sample`
2. Compute `v = (sample * sample.abs()) << 1`
3. If `(sample as u8) & 1 != 0`: `v += self.prev_val[ch]` (delta mode)
4. Clamp: `v = v.clamp(-32768, 32767)`
5. Store: `self.prev_val[ch] = v`
6. Write `v as i16` to the output buffer in the appropriate byte order (native, with byte swap if `need_swap`)

### DS-5 SDX2 Channel Iteration
**The AIFF decoder shall** iterate through channels within each frame in order `ch = 0, 1, ..., channels-1`, reading one compressed byte per channel per frame and writing one `i16` per channel per frame to produce interleaved output.

### DS-6 SDX2 Return Value
**The AIFF decoder shall** return `Ok(dec_pcm as usize * block_align as usize)`.

### DS-7 SDX2 Predictor Initialization
**When** `open_from_bytes()` completes successfully, **the AIFF decoder shall** have `self.prev_val` initialized to `[0i32; MAX_CHANNELS]`.

### DS-8 SDX2 EOF
**When** `self.cur_pcm >= self.max_pcm`, **the AIFF decoder shall** return `Err(DecodeError::EndOfFile)`.

## Seeking

### SK-1 Seek Position Clamping
**When** the requested `pcm_pos` exceeds `self.max_pcm`, **the AIFF decoder shall** clamp it to `self.max_pcm` using `pcm_pos.min(self.max_pcm)`.

### SK-2 Seek Position Update
**The AIFF decoder shall** set `self.cur_pcm` to the clamped `pcm_pos` and `self.data_pos` to `pcm_pos as usize * self.file_block as usize`.

### SK-3 SDX2 Predictor Reset on Seek
**When** `seek()` is called, **the AIFF decoder shall** zero all `self.prev_val` entries (`self.prev_val = [0i32; MAX_CHANNELS]`) to reset SDX2 predictor state.

### SK-4 Seek Return Value
**The AIFF decoder shall** return `Ok(pcm_pos)` where `pcm_pos` is the clamped value.

## Error Handling

### EH-1 Error Get-and-Clear
**When** `get_error()` is called, **the AIFF decoder shall** return the current `self.last_error` value and reset it to `0`.

### EH-2 Error Code Semantics
**The AIFF decoder shall** use the following `last_error` values:
- `0`: no error
- `-1`: unknown error
- `-2`: malformed AIFF file (bad file)
- `-3`: bad argument

### EH-3 Open Failure Cleanup
**When** any validation or parsing fails during `open_from_bytes()`, **the AIFF decoder shall** call `self.close()` to release any partially-set state before returning `Err(...)`.

### EH-4 Close Idempotency
**The AIFF decoder shall** support multiple calls to `close()` without error. Each call clears `self.data`, resets `self.data_pos`, `self.cur_pcm`, `self.max_pcm`, and `self.prev_val`.

### EH-5 Term Calls Close
**When** `term()` is called, **the AIFF decoder shall** call `self.close()`.

### EH-6 Decode Unknown Compression
**When** `decode()` is called and `self.comp_type` is neither `None` nor `Sdx2`, **the AIFF decoder shall** return `Err(DecodeError::DecoderError("unknown compression type"))`. This replaces the C code's `assert(false)` with a recoverable error.

## Initialization and Module Lifecycle

### LF-1 GetName
**The AIFF decoder's** `name()` method **shall** return the static string `"AIFF"`.

### LF-2 InitModule Format Storage
**When** `init_module()` is called, **the AIFF decoder shall** store the provided `DecoderFormats` in `self.formats` and return `true`.

### LF-3 InitModule Flags Ignored
**The AIFF decoder shall** ignore the `flags` parameter in `init_module()` (use `let _ = flags;`).

### LF-4 TermModule
**When** `term_module()` is called, **the AIFF decoder shall** set `self.formats` to `None`.

### LF-5 Init Endianness
**When** `init()` is called, **the AIFF decoder shall** set `self.need_swap` to `!self.formats.unwrap().want_big_endian` (AIFF data is big-endian; swap if consumer does not want big-endian). Returns `true`.

### LF-6 GetFrame
**The AIFF decoder's** `get_frame()` method **shall** always return `0`.

### LF-7 Open State Reset
**When** `open_from_bytes()` is called, **the AIFF decoder shall** clear `self.data`, reset `self.data_pos` to 0, zero `self.prev_val`, reset `self.max_pcm` and `self.cur_pcm` to 0, and default-initialize the `CommonChunk` before parsing.

### LF-8 Open Completion
**When** `open_from_bytes()` completes successfully, **the AIFF decoder shall** have set `self.max_pcm` to `sample_frames`, `self.cur_pcm` to `0`, `self.data_pos` to `0`, and `self.last_error` to `0`.

### LF-9 is_null
**The AIFF decoder's** `is_null()` method **shall** always return `false`.

### LF-10 needs_swap
**The AIFF decoder's** `needs_swap()` method **shall** return `self.need_swap`.

## FFI Integration

### FF-1 FFI Wrapper Struct
**The AIFF FFI layer** (`aiff_ffi.rs`) **shall** define:
```rust
#[repr(C)]
pub struct TFB_RustAiffDecoder {
    pub base: TFB_SoundDecoder,
    pub rust_decoder: *mut c_void,
}
```
with `TFB_SoundDecoder` as the first field to ensure C struct layout compatibility.

### FF-2 Vtable Export
**The AIFF FFI layer shall** export:
```rust
#[no_mangle]
pub static rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs
```
with all 12 function pointers (`GetName`, `InitModule`, `TermModule`, `GetStructSize`, `GetError`, `Init`, `Term`, `Open`, `Close`, `Decode`, `Seek`, `GetFrame`) populated.

### FF-3 Module Format Storage
**The AIFF FFI layer shall** store `DecoderFormats` in:
```rust
static RUST_AIFA_FORMATS: Mutex<Option<DecoderFormats>> = Mutex::new(None);
```
Set during `rust_aifa_InitModule()`, cleared during `rust_aifa_TermModule()`.

### FF-4 Init Allocation
**When** `rust_aifa_Init()` is called, **the AIFF FFI layer shall** allocate a `Box::new(AiffDecoder::new())`, call `init_module()` with the stored formats and `init()`, convert to a raw `*mut c_void` via `Box::into_raw()`, and store it in `(*rd).rust_decoder`. The function shall set `(*decoder).need_swap = false` and return `1`.

### FF-5 Term Deallocation
**When** `rust_aifa_Term()` is called, **the AIFF FFI layer shall** reconstruct the `Box<AiffDecoder>` from `(*rd).rust_decoder` via `Box::from_raw()`, drop it, and set `(*rd).rust_decoder` to `ptr::null_mut()`.

### FF-6 Open File Reading
**When** `rust_aifa_Open()` is called, **the AIFF FFI layer shall** read the entire AIFF file into a `Vec<u8>` using the UIO virtual filesystem (`uio_open`, `uio_fstat`, `uio_read`, `uio_close`), then call `dec.open_from_bytes(&data, filename)`.

### FF-7 Open Base Field Update
**When** `rust_aifa_Open()` succeeds, **the AIFF FFI layer shall** update:
- `(*decoder).frequency = dec.frequency()`
- `(*decoder).format` = format code from locked `RUST_AIFA_FORMATS` mapped via `dec.format()`
- `(*decoder).length = dec.length()`
- `(*decoder).is_null = false`
- `(*decoder).need_swap = dec.needs_swap()`

### FF-8 Open Failure Return
**When** `rust_aifa_Open()` fails (Rust returns `Err`), **the AIFF FFI layer shall** log the error via `rust_bridge_log_msg()` and return `0`.

### FF-9 Decode Return Mapping
**The AIFF FFI layer's** `rust_aifa_Decode()` **shall** map Rust results to C return values:
- `Ok(n)` → `n as c_int`
- `Err(DecodeError::EndOfFile)` → `0`
- `Err(_)` → `0`

This matches the C behavior where `aifa_Decode` never returns negative values.

### FF-10 Null Pointer Safety
**Every** `extern "C"` function in `aiff_ffi.rs` **shall** check for null `decoder` pointer and null `(*rd).rust_decoder` pointer before dereferencing, returning a safe default value (0, -1, or void) on null.

### FF-11 GetStructSize
**The AIFF FFI layer's** `rust_aifa_GetStructSize()` **shall** return `std::mem::size_of::<TFB_RustAiffDecoder>() as u32`.

### FF-12 GetName
**The AIFF FFI layer's** `rust_aifa_GetName()` **shall** return a pointer to the null-terminated byte string `b"Rust AIFF\0"`.

### FF-13 Seek FFI
**The AIFF FFI layer's** `rust_aifa_Seek()` **shall** call `dec.seek(pcm_pos)` and return the `Ok` value, or return `pcm_pos` on error.

### FF-14 GetFrame FFI
**The AIFF FFI layer's** `rust_aifa_GetFrame()` **shall** call `dec.get_frame()` and return the result (always `0`).

### FF-15 Close FFI
**The AIFF FFI layer's** `rust_aifa_Close()` **shall** call `dec.close()` on the Rust decoder if the pointer is non-null.

---

# Part 5: FFI Boundary — C Integration

## Vtable Registration

The C `decoder.c` file's `sd_decoders[]` table must be updated to reference the Rust vtable. Under a `USE_RUST_AIFF` conditional:

```c
#ifdef USE_RUST_AIFF
extern TFB_SoundDecoderFuncs rust_aifa_DecoderVtbl;
#endif

static TFB_RegSoundDecoder sd_decoders[MAX_REG_DECODERS + 1] = {
    // ... other decoders ...
#ifdef USE_RUST_AIFF
    {true, true, "aif", &rust_aifa_DecoderVtbl},
#else
    {true, true, "aif", &aifa_DecoderVtbl},
#endif
    {false, false, NULL, NULL},
};
```

## Exported Symbol

The Rust library exports exactly one symbol for this decoder:

```rust
#[no_mangle]
pub static rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs = /* ... */;
```

The symbol name `rust_aifa_DecoderVtbl` follows the convention: `rust_` prefix + C decoder prefix (`aifa`) + `_DecoderVtbl`.

## FFI Function Signatures (Complete)

```rust
extern "C" fn rust_aifa_GetName() -> *const c_char;
extern "C" fn rust_aifa_InitModule(flags: c_int, fmts: *const TFB_DecoderFormats) -> c_int;
extern "C" fn rust_aifa_TermModule();
extern "C" fn rust_aifa_GetStructSize() -> u32;
extern "C" fn rust_aifa_GetError(decoder: *mut TFB_SoundDecoder) -> c_int;
extern "C" fn rust_aifa_Init(decoder: *mut TFB_SoundDecoder) -> c_int;
extern "C" fn rust_aifa_Term(decoder: *mut TFB_SoundDecoder);
extern "C" fn rust_aifa_Open(
    decoder: *mut TFB_SoundDecoder,
    dir: *mut uio_DirHandle,
    filename: *const c_char,
) -> c_int;
extern "C" fn rust_aifa_Close(decoder: *mut TFB_SoundDecoder);
extern "C" fn rust_aifa_Decode(
    decoder: *mut TFB_SoundDecoder,
    buf: *mut c_void,
    bufsize: i32,
) -> c_int;
extern "C" fn rust_aifa_Seek(decoder: *mut TFB_SoundDecoder, pcm_pos: u32) -> u32;
extern "C" fn rust_aifa_GetFrame(decoder: *mut TFB_SoundDecoder) -> u32;
```

## UIO External Functions

The FFI module declares (matching existing FFI modules):

```rust
extern "C" {
    fn uio_open(dir: *mut uio_DirHandle, path: *const c_char, flags: c_int, mode: c_int) -> *mut c_void;
    fn uio_read(handle: *mut c_void, buf: *mut u8, count: usize) -> isize;
    fn uio_close(handle: *mut c_void) -> c_int;
    fn uio_fstat(handle: *mut c_void, stat_buf: *mut libc::stat) -> c_int;
}
```

## Module Registration

`aiff.rs` and `aiff_ffi.rs` must be added to `rust/src/sound/mod.rs`:

```rust
pub mod aiff;
pub mod aiff_ffi;
```

---

# Appendix A: Requirement Traceability Matrix

| C Req ID | Rust Req ID | C Behavior | Rust Equivalent |
|----------|-------------|------------|-----------------|
| FP-1 | FP-1 | Read 12-byte FORM header | Same, via `Cursor<&[u8]>` |
| FP-2 | FP-2 | Log + return false | `Err(InvalidData)` |
| FP-3 | FP-3 | Log + return false | `Err(InvalidData)` |
| FP-4 | FP-4 | `while(remSize > 0)` loop | Same, cursor-based |
| FP-5 | FP-5 | `fseek(size & 1)` | `cursor.seek(Current(1))` |
| FP-6 | FP-6 | `fileHdr.chunk.size - sizeof(aiff_ID)` | `chunk_size - 4` |
| FP-7 | FP-7 | `fseek(chunk.size)` | `cursor.seek(Current(size))` |
| FP-8 | FP-8 | Parse COMM fields | Same fields, BE byte reads |
| FP-9 | FP-9 | `last_error = aifae_BadFile` | `last_error = -2` + `Err(InvalidData)` |
| FP-10 | FP-10 | Read extTypeID if size ≥ 22 | Same |
| FP-11 | FP-11 | `fseek(remaining)` | `cursor.seek(Current(remaining))` |
| FP-12 | FP-12 | Parse SSND offset/blockSize | Same |
| FP-13 | FP-13 | `fseek(chunk.size - AIFF_SSND_SIZE)` | `cursor.seek(Current(size - 8))` |
| FP-14 | FP-14 | `read_be_f80()` → sint32 | `read_be_f80()` → `i32` |
| FP-15 | FP-15 | `memset(fmtHdr, 0, ...)` | `CommonChunk::default()` |
| SV-1 | SV-1 | `(sampleSize + 7) & ~7` | `(sample_size + 7) & !7` |
| SV-2 | SV-2 | Log + return false | `Err(UnsupportedFormat)` |
| SV-3 | SV-3 | Log + return false | `Err(UnsupportedFormat)` |
| SV-4 | SV-4 | Log + return false | `Err(UnsupportedFormat)` |
| SV-5 | SV-5 | Log + return false | `Err(InvalidData)` |
| SV-6 | SV-6 | Log + return false | `Err(InvalidData)` |
| SV-7 | SV-7 | `bps/8 * channels` | Same |
| SV-8 | SV-8 | `file_block = block_align` | Same |
| SV-9 | SV-9 | `file_block = block_align / 2` | Same |
| SV-10 | SV-10 | `data_size = sampleFrames * file_block` | `data[data_start..data_start + frames*fb]` |
| SV-11 | SV-11 | `formats->mono8` etc. | `AudioFormat::Mono8` etc. |
| SV-12 | SV-12 | `frequency = sampleRate` | `self.frequency = sample_rate as u32` |
| SV-13 | SV-13 | `length = (float)max_pcm / sampleRate` | `max_pcm as f32 / frequency as f32` |
| CH-1 | CH-1 | `comp_type = aifc_None` | `CompressionType::None` |
| CH-2 | CH-2 | Log + return false | `Err(UnsupportedFormat)` |
| CH-3 | CH-3 | `comp_type = aifc_Sdx2` | `CompressionType::Sdx2` |
| CH-4 | CH-4 | Log + return false | `Err(UnsupportedFormat)` |
| CH-5 | CH-5 | Log + return false | `Err(UnsupportedFormat)` |
| CH-6 | CH-6 | `assert(channels <= MAX_CHANNELS)` (debug) | Runtime `Err(UnsupportedFormat)` (all builds) |
| CH-7 | CH-7 | `need_swap = big_endian != want_big_endian` | `cfg!(target_endian = "big") != want_big_endian` |
| DP-1 | DP-1 | `bufsize / block_align` clamped | Same |
| DP-2 | DP-2 | `uio_fread(buf, file_block, dec_pcm)` | `buf.copy_from_slice(data[pos..])` |
| DP-3 | DP-3 | `cur_pcm += dec_pcm` | Same + `data_pos` advance |
| DP-4 | DP-4 | `return dec_pcm * block_align` | `Ok(dec_pcm * block_align)` |
| DP-5 | DP-5 | `*ptr += 128` per byte | `byte.wrapping_add(128)` per byte |
| DP-6 | DP-6 | Return 0 | `Err(EndOfFile)` |
| DS-1 | DS-1 | `bufsize / block_align` clamped | Same |
| DS-2 | DS-2 | Read into tail of buf | Read from `self.data` (no in-place trick needed) |
| DS-3 | DS-3 | `cur_pcm += dec_pcm` | Same + `data_pos` advance |
| DS-4 → DS-5 | DS-4 | SDX2 algorithm | Same algorithm in Rust |
| DS-6 | DS-5 | Channel interleaving | Same |
| DS-7 | DS-6 | `return dec_pcm * block_align` | `Ok(dec_pcm * block_align)` |
| DS-8 | DS-7 | `memset(prev_val, 0, ...)` at open | `prev_val = [0; MAX_CHANNELS]` |
| — | DS-8 | (implicit in C: return 0) | `Err(EndOfFile)` |
| SK-1 | SK-1 | `if (pcm_pos > max_pcm) pcm_pos = max_pcm` | `pcm_pos.min(max_pcm)` |
| SK-2 | SK-2 | `cur_pcm = pcm_pos` | Same + `data_pos` update |
| SK-3 | SK-3 (removed) | `fseek(data_ofs + pcm_pos * file_block)` | `data_pos = pcm_pos * file_block` (no file seek) |
| SK-4 | SK-3 | `memset(prev_val, 0, ...)` | `prev_val = [0; MAX_CHANNELS]` |
| SK-5 | SK-4 | `return pcm_pos` | `Ok(pcm_pos)` |
| EH-1 | EH-1 | Get + clear last_error | Same |
| EH-2 | EH-2 (merged) | errno capture | Not applicable (no file I/O in pure Rust) |
| EH-3 | EH-2 | Error code values | Same numeric values |
| EH-4 | EH-3 | `aifa_Close()` on failure | `self.close()` on failure |
| EH-5 | EH-4 | Check null before close | Clear state, safe to call multiple times |
| EH-6 | EH-5 | `Term()` calls `Close()` | `term()` calls `close()` |
| EH-7 | EH-6 | `assert(false)` | `Err(DecoderError)` |
| LF-1 | LF-1 | `"AIFF"` | `"AIFF"` |
| LF-2 | LF-2 | Store formats pointer | `self.formats = Some(*formats)` |
| LF-3 | LF-3 | `(void)flags` | `let _ = flags;` |
| LF-4 | LF-4 | No-op | `self.formats = None` |
| LF-5 | (merged into FF-11) | `sizeof(struct)` | `size_of::<TFB_RustAiffDecoder>()` |
| LF-6 | LF-5 | `need_swap = !want_big_endian` | Same |
| LF-7 | LF-6 | Return 0 | Return 0 |
| LF-8 | LF-7 | Zero state before parse | `close()` + default init |
| LF-9 | LF-8 (merged) | Seek to data_ofs, cur_pcm=0 | `data_pos=0`, `cur_pcm=0` (no file seek) |
| LF-10 | LF-8 | Set max_pcm, cur_pcm, clear error | Same |
| FF-1 | FF-1 | — | `TFB_RustAiffDecoder` |
| FF-2 | FF-2 | — | `rust_aifa_DecoderVtbl` |
| FF-3 | FF-3 | — | `Mutex<Option<DecoderFormats>>` |
| FF-4 | FF-4 | — | `Box::new()` + `Box::into_raw()` |
| FF-5 | FF-5 | — | `Box::from_raw()` + drop |
| FF-6 | FF-6 | — | UIO read into `Vec<u8>` |
| FF-7 | FF-7 | — | Update base struct fields |

---

# Appendix B: Test Strategy

Unit tests in `aiff.rs` should cover:

1. **Parsing**: Synthetic AIFF/AIFC byte arrays exercising all chunk types, alignment padding, extended COMM
2. **IEEE 754 80-bit float**: Known sample rates (8000, 11025, 22050, 44100, 48000, 96000) round-tripped through `read_be_f80()`
3. **PCM decode**: Mono/stereo, 8-bit (with signed→unsigned), 16-bit
4. **SDX2 decode**: Known compressed sequences verified against C output, predictor state, channel interleaving
5. **Seeking**: Position clamping, predictor reset, data_pos update
6. **Validation failures**: Each SV/CH error path triggered independently
7. **Edge cases**: Zero-length buffers, odd chunk sizes, duplicate chunks, minimum/maximum sample rates

FFI tests in `aiff_ffi.rs` should cover:
1. Vtable existence and name string
2. Struct size validity
3. Null pointer handling for all 12 functions
4. `InitModule` / `TermModule` format storage lifecycle

## Review Notes

*Reviewed by cross-referencing c-decoder.md (78 EARS requirements) against this Rust spec, and verifying patterns against existing Rust decoders: `decoder.rs` (SoundDecoder trait), `wav.rs`/`wav_ffi.rs`, `dukaud.rs`/`dukaud_ffi.rs`.*

### 1. Requirement Coverage (C → Rust)

All 78 C EARS requirement IDs from `c-decoder.md` are accounted for in the Rust spec. The Rust spec renumbers several due to merges and adds 8 new FFI-specific requirements (FF-8 through FF-15). Detailed status:

**Fully covered (no issues):** FP-1 through FP-15, SV-1 through SV-13, CH-1 through CH-7, DP-1 through DP-6, DS-1, DS-4/DS-5 (merged into Rust DS-4), DS-6 (→ Rust DS-5), DS-7 (→ Rust DS-6), DS-8 (→ Rust DS-7), SK-1, SK-2, SK-4 (→ Rust SK-3), SK-5 (→ Rust SK-4), EH-1, EH-4 (→ Rust EH-3), EH-5 (→ Rust EH-4), EH-6 (→ Rust EH-5), EH-7 (→ Rust EH-6), LF-1 through LF-4, LF-6 (→ Rust LF-5), LF-7 (→ Rust LF-6), LF-8 (→ Rust LF-7), LF-9/LF-10 (→ Rust LF-8), FF-1 through FF-7.

**Issues found:**

| # | C Req | Rust Req | Issue | Severity |
|---|-------|----------|-------|----------|
| 1 | DS-2 | DS-2 | **Changed behavior, correctly justified.** C requires reading compressed data into the tail of the output buffer for in-place expansion. Rust DS-2 explicitly drops this in favor of reading from `self.data`. This is correct since the in-place trick is a streaming optimization unnecessary with in-memory data. The traceability matrix (Appendix A) correctly documents this. | Info |
| 2 | DS-3 | DS-3 | **C has DS-3 (file read) and DS-4 (position update) as separate items; Rust merges DS-3 (file read) into DS-2 and reassigns DS-3 to position update.** The traceability matrix row "DS-3" maps to Rust's DS-3 with `cur_pcm += dec_pcm` which is actually the C's DS-4. The C DS-3 (file read step) is subsumed into Rust DS-2. No behavioral gap, just confusing numbering. | Minor |
| 3 | EH-2 | EH-2 | **Semantic gap.** C EH-2 says "store system `errno` in `last_error`" on I/O failure. Rust EH-2 only lists numeric error code values (0, -1, -2, -3) and omits positive errno values entirely. The spec body (§3.6) says "Positive = system errno (not applicable in pure Rust, but retained for interface compatibility)" — this is correct since pure Rust has no file I/O, but the EH-2 requirement text itself should mention that positive errno values are not used (rather than silently omitting them). The traceability matrix notes "Not applicable (no file I/O in pure Rust)" which is accurate. | Minor |
| 4 | EH-3 | EH-2 | **Merged.** C EH-3 (error code table) is merged into Rust EH-2. The C error code `aifae_Other` (-1000) from `aiffaud.h` is absent from both the C EARS and Rust EARS — not a Rust spec issue per se, but worth noting that `-1000` is never used in the C implementation. | Info |
| 5 | SK-3 | SK-2 | **Changed behavior, correctly justified.** C SK-3 requires `fseek(data_ofs + pcm_pos * file_block)`. Rust SK-2 replaces this with `data_pos = pcm_pos * file_block` (no file seek needed for in-memory data). The traceability matrix marks SK-3 as "removed" and explains the in-memory equivalent. | Info |
| 6 | LF-4 | LF-4 | **Behavioral difference.** C LF-4 says `TermModule()` is a no-op. Rust LF-4 says it sets `self.formats = None`. This is actually an intentional Rust improvement (clean up stored state), and follows the `wav.rs` pattern where `term_module()` sets `self.formats = None` (line 218). But it is a behavioral difference from C — the C code never clears the static `aifa_formats` pointer. | Minor |
| 7 | LF-5 | FF-11 | **C LF-5 (`GetStructSize`) has no standalone Rust lifecycle requirement.** It's covered by FF-11 in the FFI section. The traceability matrix notes this merge. No gap, just relocated. | Info |
| 8 | FF-7 | FF-7 | **C spec FF-7 is truncated** — it cuts off mid-sentence in c-decoder.md at line 729: "...`is_null` (false),". The Rust spec FF-7 provides the complete version including `need_swap = dec.needs_swap()`. The C spec should be fixed but this is a C spec issue, not a Rust spec issue. | Info (C spec) |

**New Rust-only requirements (no C equivalent):** DS-8 (SDX2 EOF — implicit in C as returning 0, Rust makes it explicit as `Err(EndOfFile)`), LF-9 (`is_null()` returns false), LF-10 (`needs_swap()` accessor), FF-8 through FF-15 (8 additional FFI requirements covering failure logging, decode return mapping, null safety, struct size, name, seek/getframe/close FFI). All are appropriate Rust-specific concerns.

### 2. Technical Soundness

#### Verified Against Existing Code

**SoundDecoder trait conformance (`decoder.rs`):** The Rust spec's `AiffDecoder` trait table (Part 1) maps correctly to all 15 methods in the `SoundDecoder` trait (lines 55-137 of `decoder.rs`): `name()`, `init_module()`, `term_module()`, `get_error()`, `init()`, `term()`, `open()`, `open_from_bytes()`, `close()`, `decode()`, `seek()`, `get_frame()`, `frequency()`, `format()`, `length()`, `is_null()`, `needs_swap()`. All signatures match.

**Pattern inconsistency with `dukaud.rs`:** The spec says `AiffDecoder` implements the `SoundDecoder` trait. This matches `wav.rs`, `ogg.rs`, `null.rs`, and `mod_decoder.rs` — but NOT `dukaud.rs`. `DukAudDecoder` does NOT implement `SoundDecoder`; it has standalone methods with slightly different signatures (e.g., `open_from_data(&mut self, duk_data: &[u8], frm_data: &[u8])` instead of `open_from_bytes(&mut self, data: &[u8], name: &str)`). The spec's choice to implement `SoundDecoder` is correct — AIFF is a standard file format like WAV, not a multi-file format like DukAud. The spec's Part 1 architecture diagram correctly shows this.

**FFI pattern conformance (`dukaud_ffi.rs`):**
- `#[repr(C)]` wrapper struct with `TFB_SoundDecoder` as first field + `*mut c_void` — matches (line 33-37 of `dukaud_ffi.rs`).
- `static Mutex<Option<DecoderFormats>>` — matches (line 39).
- `static NAME: &[u8] = b"...\0"` — matches (line 41).
- `read_uio_file` helper using `uio_open/uio_fstat/uio_read/uio_close` — matches (lines 47-76).
- 12 vtable functions with null checks — matches (lines 82-340).
- `Box::new()` in Init, `Box::from_raw()` in Term — matches (lines 131-156).

**FF-4 concern:** The Rust spec FF-4 says Init should call `init_module()` with stored formats AND `init()` — this is NOT how `dukaud_ffi.rs` works. In `dukaud_ffi.rs`, `rust_duka_Init()` (line 131) just creates the `Box::new(DukAudDecoder::new())` and stores it. It does NOT call `init_module()` or `init()` on the Rust decoder. Similarly, in `wav_ffi.rs`, Init doesn't call `init_module()`. The C framework calls `InitModule()` once globally and `Init()` per-instance — these are separate operations. The spec's FF-4 conflates them, which could cause double-initialization bugs. **Recommendation:** FF-4 should just allocate the `Box<AiffDecoder>` and store it, set `need_swap`, and return 1 — matching the existing FFI pattern. The `init_module()` and `init()` calls should be triggered by the C framework through the vtable, not duplicated in the Rust FFI's `Init()`.

**Decode return value mapping:** The spec's FF-9 says `Err(_) → 0`. This matches the C behavior (AIFF decode never returns negative). However, `dukaud_ffi.rs` (line 286) returns `-1` for non-EOF errors. The AIFF spec is correct per the C reference (`aifa_DecodePCM` and `aifa_DecodeSDX2` never return negative), but this is an intentional difference from the DukAud FFI pattern worth noting during implementation.

**`need_swap` initialization location:** The spec says `init()` sets `need_swap = !want_big_endian` (LF-5). In `wav.rs`, `need_swap` is set in `init_module()` (line 213), not `init()`. The C `aifa_Init()` sets it in `Init()`. The spec follows the C pattern here, which is correct for AIFF, but differs from the WAV Rust pattern. This is fine since the C behavior is the source of truth.

#### Algorithm Correctness

**SDX2 algorithm (DS-4):** The spec's formula `v = (sample * sample.abs()) << 1` correctly reproduces the C code's `v = (sample_byte * abs(sample_byte)) << 1`. The sign preservation via multiplication with absolute value, odd-bit delta mode, clamping, and per-channel predictor state all match.

**IEEE 754 80-bit float (FP-14):** The 12-step algorithm matches the C `read_be_f80()` implementation exactly, including the explicit integer bit handling (no implicit leading 1) and the 31-bit precision truncation.

**8-bit signed→unsigned (DP-5):** `wrapping_add(128)` correctly reproduces the C `*ptr += 128` for converting AIFF's signed 8-bit to unsigned 8-bit.

### 3. Scope Completeness

**When `aiff.rs` and `aiff_ffi.rs` are implemented per this spec, `aiffaud.c` will be fully replaceable.** All required functionality is covered:

| Capability | Status |
|-----------|--------|
| AIFF file header parsing | [OK] Specified (FP-1 through FP-7) |
| COMM chunk parsing (plain + extended) | [OK] Specified (FP-8 through FP-11, FP-15) |
| SSND chunk parsing | [OK] Specified (FP-12, FP-13) |
| IEEE 754 80-bit float | [OK] Specified (FP-14) |
| Sample format validation | [OK] Specified (SV-1 through SV-13) |
| Uncompressed PCM decode (8-bit, 16-bit) | [OK] Specified (DP-1 through DP-6) |
| SDX2 ADPCM decode | [OK] Specified (DS-1 through DS-8) |
| 8-bit signed→unsigned conversion | [OK] Specified (DP-5) |
| Seeking with predictor reset | [OK] Specified (SK-1 through SK-4) |
| Endianness handling (PCM + SDX2) | [OK] Specified (LF-5, CH-7) |
| Error handling (get-and-clear) | [OK] Specified (EH-1 through EH-6) |
| Lifecycle (init/term/open/close) | [OK] Specified (LF-1 through LF-10) |
| FFI vtable + C integration | [OK] Specified (FF-1 through FF-15) |
| C `decoder.c` registration | [OK] Specified (Part 5, with `USE_RUST_AIFF` conditional) |
| Duplicate chunk handling | [OK] Specified (§3.1 "Duplicate Chunk Handling") |
| Chunk alignment padding | [OK] Specified (FP-5) |
| Unknown chunk skipping | [OK] Specified (FP-7) |

**No gaps remain** — no AIFF/AIFC features used by UQM are missing. MARK/INST chunks (loop points) are intentionally excluded, matching the C implementation. The spec correctly notes this in the C reference analysis.

### 4. Actionable Findings

1. **Fix FF-4** (Medium): Remove the `init_module()` and `init()` calls from the `rust_aifa_Init()` description. The FFI Init should only allocate the `Box<AiffDecoder>`, store it, and set base fields — matching `dukaud_ffi.rs` and `wav_ffi.rs`. The C framework calls InitModule and Init via the vtable separately.

2. **Clarify EH-2** (Low): Add a note that positive errno values from C EH-2 are not applicable since the pure Rust decoder has no system-level file I/O. The `last_error` field only uses values 0, -1, -2, -3.

3. **Fix C spec FF-7** (Low, C spec issue): The C spec's FF-7 is truncated mid-sentence. The Rust spec's FF-7 has the complete version and is correct.

4. **Traceability matrix DS row numbering** (Low): The DS-2/DS-3/DS-4/DS-5 renumbering in the traceability matrix is confusing. Consider adding a note explaining that the C's DS-2 (in-place buffer trick) is intentionally dropped and the subsequent IDs shift.

**Overall verdict:** The Rust spec is thorough, technically sound, and sufficient to fully replace `aiffaud.c`. The one medium-priority issue (FF-4 double-initialization) should be fixed before implementation to avoid bugs. All other findings are low-severity clarifications.

