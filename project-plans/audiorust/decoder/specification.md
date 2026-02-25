# AIFF Audio Decoder — Rust Port Specification

## Purpose / Problem Statement

The UQM project contains a C AIFF/AIFC audio decoder (`sc2/src/libs/sound/decoders/aiffaud.c`) that handles `.aif` audio files used in the game's sound system. This specification defines the Rust replacement for that decoder, following the established two-file pattern (`aiff.rs` + `aiff_ffi.rs`) used by the WAV, Ogg, MOD, and DukAud Rust decoders.

The AIFF decoder supports two compression modes:
- **PCM** (uncompressed) — 8-bit and 16-bit, mono and stereo
- **SDX2 ADPCM** — a predictive codec used in AIFC files

The Rust port loads the entire audio data segment into memory during `open_from_bytes()` (matching the WAV decoder pattern), eliminating streaming file I/O from the pure Rust decoder and enabling full unit testability with synthetic byte arrays.

## Architectural Boundaries

### Module Scope

| File | Scope | Unsafe |
|------|-------|--------|
| `rust/src/sound/aiff.rs` | Pure Rust decoder: parsing, PCM decode, SDX2 decode, seeking | No `unsafe` |
| `rust/src/sound/aiff_ffi.rs` | C FFI bridge: vtable, UIO file I/O, Box lifecycle | `unsafe` (FFI only) |

### Module Boundary Rules

- `aiff.rs` has **no dependency** on FFI, raw pointers, or C types
- `aiff.rs` implements the `SoundDecoder` trait from `decoder.rs`
- `aiff_ffi.rs` depends on `aiff.rs` (creates/manages `AiffDecoder` instances)
- `aiff_ffi.rs` depends on `ffi.rs` (C types: `TFB_SoundDecoder`, `TFB_SoundDecoderFuncs`, etc.)
- `aiff_ffi.rs` exports exactly one symbol: `rust_aifa_DecoderVtbl`

### Integration Points

1. **`rust/src/sound/mod.rs`** — add `pub mod aiff;` and `pub mod aiff_ffi;`
2. **`sc2/src/libs/sound/decoders/decoder.c`** — conditional `USE_RUST_AIFF` vtable registration
3. **`sc2/src/libs/sound/decoders/rust_aiff.h`** — new header with `extern TFB_SoundDecoderFuncs rust_aifa_DecoderVtbl;`
4. **`sc2/src/config_unix.h.in`** — add `@SYMBOL_USE_RUST_AIFF_DEF@` placeholder
5. **`sc2/build.vars.in`** — add `USE_RUST_AIFF` build variable

## Data Contracts and Invariants

### Input Contract

`AiffDecoder::open_from_bytes(data: &[u8], name: &str)` accepts:
- A byte slice containing the complete AIFF or AIFC file
- The data must start with a valid 12-byte FORM header
- At least one COMM and one SSND chunk must be present

### Output Contract

`AiffDecoder::decode(buf: &mut [u8])` produces:
- PCM audio data in the output buffer
- 8-bit: unsigned (0–255), converted from AIFF signed (-128..127) via `wrapping_add(128)`
- 16-bit: signed, byte order controlled by `need_swap`
- Returns byte count written, or `Err(EndOfFile)` when exhausted

### Invariants

1. `data_pos == cur_pcm * file_block` — always synchronized
2. `cur_pcm <= max_pcm` — enforced by seek clamping and decode frame counting
3. `prev_val` is zeroed on `open_from_bytes()` and `seek()` — SDX2 predictor reset
4. `close()` is idempotent — safe to call multiple times
5. `last_error` is get-and-clear — `get_error()` reads then resets to 0

## Functional Requirements

### Format Parsing (FP)

| ID | Requirement |
|----|-------------|
| REQ-FP-1 | Read 12-byte FORM header (chunk_id, chunk_size, form_type) as big-endian |
| REQ-FP-2 | Reject non-FORM chunk ID with `InvalidData` |
| REQ-FP-3 | Reject non-AIFF/AIFC form types with `InvalidData` |
| REQ-FP-4 | Iterate chunks using remaining = chunk_size - 4 |
| REQ-FP-5 | Apply 1-byte alignment padding after odd-sized chunks |
| REQ-FP-6 | Calculate remaining data as `chunk_size - 4` |
| REQ-FP-7 | Skip unknown chunks via cursor seek |
| REQ-FP-8 | Parse COMM: channels(u16), sample_frames(u32), sample_size(u16), sample_rate(f80→i32) |
| REQ-FP-9 | Reject COMM chunks smaller than 18 bytes with last_error=-2 |
| REQ-FP-10 | Parse ext_type_id(u32) from extended COMM (size ≥ 22) |
| REQ-FP-11 | Skip remaining COMM data after parsed fields |
| REQ-FP-12 | Parse SSND: offset(u32), block_size(u32), compute data_start |
| REQ-FP-13 | Skip remaining SSND data after header |
| REQ-FP-14 | Implement IEEE 754 80-bit float to i32 conversion for sample rate |
| REQ-FP-15 | Default-initialize CommonChunk (ext_type_id defaults to 0) |

### Sample Format Validation (SV)

| ID | Requirement |
|----|-------------|
| REQ-SV-1 | Round bits_per_sample: `(sample_size + 7) & !7` |
| REQ-SV-2 | Reject bits_per_sample 0 or >16 with `UnsupportedFormat` |
| REQ-SV-3 | Reject channels not 1 or 2 with `UnsupportedFormat` |
| REQ-SV-4 | Reject sample_rate outside [300, 128000] with `UnsupportedFormat` |
| REQ-SV-5 | Reject sample_frames == 0 with `InvalidData` |
| REQ-SV-6 | Reject missing SSND chunk with `InvalidData` |
| REQ-SV-7 | Calculate block_align = (bits_per_sample / 8) * channels |
| REQ-SV-8 | PCM: file_block = block_align |
| REQ-SV-9 | SDX2: file_block = block_align / 2 |
| REQ-SV-10 | Extract audio data: data[data_start..data_start + sample_frames * file_block] |
| REQ-SV-11 | Map (channels, bits_per_sample) to AudioFormat enum |
| REQ-SV-12 | Set frequency = sample_rate as u32 |
| REQ-SV-13 | Calculate length = max_pcm as f32 / frequency as f32 |

### Compression Handling (CH)

| ID | Requirement |
|----|-------------|
| REQ-CH-1 | AIFF + ext_type_id==0 → CompressionType::None |
| REQ-CH-2 | AIFF + ext_type_id!=0 → UnsupportedFormat error |
| REQ-CH-3 | AIFC + ext_type_id==SDX2 → CompressionType::Sdx2 |
| REQ-CH-4 | AIFC + ext_type_id!=SDX2 → UnsupportedFormat error |
| REQ-CH-5 | SDX2 requires bits_per_sample == 16 |
| REQ-CH-6 | SDX2 requires channels ≤ MAX_CHANNELS (4), runtime check |
| REQ-CH-7 | SDX2: override need_swap = formats.big_endian != formats.want_big_endian (runtime, not compile-time) |

### PCM Decoding (DP)

| ID | Requirement |
|----|-------------|
| REQ-DP-1 | Frame count: min(bufsize / block_align, max_pcm - cur_pcm) |
| REQ-DP-2 | Copy dec_pcm * file_block bytes from self.data to output. Do NOT perform inline byte swapping for 16-bit PCM — the C framework's `SoundDecoder_Decode()` in `decoder.c` already handles byte swapping when `need_swap=true`. The Rust decoder just copies raw big-endian bytes. |
| REQ-DP-3 | Update cur_pcm and data_pos after decode |
| REQ-DP-4 | Return Ok(dec_pcm * block_align) bytes |
| REQ-DP-5 | 8-bit: wrapping_add(128) signed→unsigned conversion |
| REQ-DP-6 | Return Err(EndOfFile) when cur_pcm >= max_pcm |

### SDX2 Decoding (DS)

| ID | Requirement |
|----|-------------|
| REQ-DS-1 | Frame count: min(bufsize / block_align, max_pcm - cur_pcm) |
| REQ-DS-2 | Read compressed bytes from self.data (no in-place buffer trick) |
| REQ-DS-3 | Update cur_pcm and data_pos after decode |
| REQ-DS-4 | SDX2 algorithm: v = (sample * abs(sample)) << 1; odd-bit delta; clamp; store predictor |
| REQ-DS-5 | Interleaved channel iteration (ch=0,1,...,channels-1 per frame) |
| REQ-DS-6 | Return Ok(dec_pcm * block_align) bytes |
| REQ-DS-7 | Initialize prev_val to [0; MAX_CHANNELS] on open |
| REQ-DS-8 | Return Err(EndOfFile) when cur_pcm >= max_pcm |

### Seeking (SK)

| ID | Requirement |
|----|-------------|
| REQ-SK-1 | Clamp position: pcm_pos.min(max_pcm) |
| REQ-SK-2 | Update cur_pcm and data_pos = pcm_pos * file_block |
| REQ-SK-3 | Reset SDX2 predictor: prev_val = [0; MAX_CHANNELS] |
| REQ-SK-4 | Return Ok(clamped pcm_pos) |

### Error Handling (EH)

| ID | Requirement |
|----|-------------|
| REQ-EH-1 | get_error() returns current last_error and resets to 0 |
| REQ-EH-2 | Error codes: 0 (none), -1 (unknown), -2 (bad file), -3 (bad argument). Note: The C EH-2 spec also defines positive errno values for system-level file I/O errors; these are NOT applicable in the pure Rust decoder since it operates on in-memory byte slices (`&[u8]`) with no system file I/O. Positive errno mapping is only relevant in the FFI layer's `read_uio_file()` which uses C I/O. |
| REQ-EH-3 | open_from_bytes() calls close() on any failure before returning Err |
| REQ-EH-4 | close() is idempotent — clears data, positions, predictor |
| REQ-EH-5 | term() calls close() |
| REQ-EH-6 | decode() with unknown comp_type returns Err(DecoderError) |

### Lifecycle (LF)

| ID | Requirement |
|----|-------------|
| REQ-LF-1 | name() returns "AIFF" |
| REQ-LF-2 | init_module() stores DecoderFormats, returns true |
| REQ-LF-3 | init_module() ignores flags parameter |
| REQ-LF-4 | term_module() sets formats to None |
| REQ-LF-5 | init() sets need_swap = !want_big_endian, returns true |
| REQ-LF-6 | get_frame() returns 0 |
| REQ-LF-7 | open_from_bytes() resets all state before parsing |
| REQ-LF-8 | Successful open sets max_pcm, cur_pcm=0, data_pos=0, last_error=0 |
| REQ-LF-9 | is_null() returns false |
| REQ-LF-10 | needs_swap() returns self.need_swap |

### FFI Integration (FF)

| ID | Requirement |
|----|-------------|
| REQ-FF-1 | Define TFB_RustAiffDecoder with base TFB_SoundDecoder as first field |
| REQ-FF-2 | Export rust_aifa_DecoderVtbl with all 12 function pointers |
| REQ-FF-3 | Store DecoderFormats in static Mutex<Option<DecoderFormats>> |
| REQ-FF-4 | Init: allocate Box<AiffDecoder>, call `dec.init_module(0, &formats)` with formats from the global `RUST_AIFA_FORMATS` Mutex, call `dec.init()`, store as raw pointer, set `(*decoder).need_swap = false`. This matches the established `wav_ffi.rs` Init pattern — the C framework expects Init to prepare the decoder instance with formats so that `open_from_bytes()` can access `self.formats`. |
| REQ-FF-5 | Term: reconstruct Box from raw pointer, drop, null out |
| REQ-FF-6 | Open: read file via UIO into Vec<u8>, call open_from_bytes() |
| REQ-FF-7 | Open success: update base struct (frequency, format, length, is_null, need_swap) |
| REQ-FF-8 | Open failure: log error, return 0 |
| REQ-FF-9 | Decode: Ok(n)→n, EndOfFile→0, Err→0 (never negative) |
| REQ-FF-10 | All functions check for null decoder and null rust_decoder |
| REQ-FF-11 | GetStructSize returns size_of::<TFB_RustAiffDecoder>() |
| REQ-FF-12 | GetName returns pointer to "Rust AIFF\0" |
| REQ-FF-13 | Seek: call dec.seek(), return Ok value or pcm_pos on error |
| REQ-FF-14 | GetFrame: call dec.get_frame(), return result |
| REQ-FF-15 | Close: call dec.close() if pointer non-null |

## Error / Edge Cases

1. **Truncated file** — cursor read_exact fails → InvalidData
2. **Zero sample frames** — rejected during validation (REQ-SV-5)
3. **Odd chunk sizes** — 1-byte padding skip (REQ-FP-5)
4. **Duplicate COMM/SSND** — later overwrites earlier (matching C behavior)
5. **SDX2 predictor overflow** — clamped to i16 range [-32768, 32767]
6. **Seek past end** — clamped to max_pcm (REQ-SK-1)
7. **Decode after EOF** — returns Err(EndOfFile) (REQ-DP-6, REQ-DS-8)
8. **Double close** — idempotent, no error (REQ-EH-4)
9. **NULL FFI pointers** — all FFI functions return safe defaults (REQ-FF-10)
10. **Missing SSND chunk** — rejected (REQ-SV-6)
11. **Unknown compression in AIFC** — rejected (REQ-CH-4)
12. **80-bit float overflow** — mantissa clamped to 0x7FFF_FFFF (REQ-FP-14 step 9)

## Non-Functional Requirements

1. **Memory**: Audio data loaded fully into memory during open (Vec<u8>). No streaming file handle in pure Rust decoder.
2. **Thread Safety**: AiffDecoder is `Send` (only owned data, no Rc/raw pointers). FFI formats stored in `Mutex<Option<...>>`.
3. **Performance**: In-memory decode is O(n) in sample count. No allocation during decode/seek.
4. **Compatibility**: Produces identical audio output to C `aiffaud.c` for all valid inputs.
5. **Safety**: No `unsafe` in `aiff.rs`. All `unsafe` isolated to `aiff_ffi.rs` FFI boundary.

## Intentional Deviations from C

The following behavioral differences from the C `aiffaud.c` implementation are **intentional**:

1. **Validation order**: The Rust decoder checks SSND presence *before* block_align and compression type validation, while C validates *after* block_align. Both reject the same invalid inputs; only the error message for multi-error files may differ. This is an acceptable difference — the Rust ordering is more logical (validate data exists before processing it).

2. **TermModule clears formats**: The C `aifa_TermModule()` is effectively a no-op. The Rust version sets `formats = None` (matching the `wav.rs` pattern). This is a strict improvement — it prevents use of stale format data after module teardown and matches the established Rust decoder convention.

3. **SDX2 inline byte swap**: The C decoder writes native-endian i16 values via `*dst = v` and relies on the C framework's base `need_swap` field at the mixer level. The Rust decoder performs the byte swap *inline during SDX2 decode* (via `swap_bytes().to_ne_bytes()`). This is functionally equivalent — the output byte order is identical — but architecturally the swap happens at the decoder level rather than the mixer level. **Note:** For PCM mode, the Rust decoder does NOT perform inline byte swapping — it copies raw big-endian bytes and lets the C framework's `SoundDecoder_Decode()` handle the swap via the base struct's `need_swap` field, exactly matching the C AIFF decoder.

4. **f80 full 64-bit significand**: The C code discards the low 32 bits of the IEEE 754 80-bit float significand (31-bit effective precision). The Rust pseudocode uses the full 64-bit significand. Both produce identical results for all integer sample rates used in UQM (8000, 11025, 22050, 44100, 48000, 96000), but the Rust version is more mathematically correct for edge cases.

## Testability Requirements

### Unit Tests (aiff.rs)

1. Synthetic AIFF byte arrays for parsing (valid AIFF, valid AIFC, truncated, missing chunks)
2. IEEE 754 80-bit float round-trips for known sample rates (8000, 11025, 22050, 44100, 48000, 96000)
3. PCM decode: mono/stereo × 8-bit/16-bit, signed→unsigned conversion
4. SDX2 decode: known compressed sequences verified against C output, predictor state tracking
5. Seeking: position clamping, predictor reset verification, data_pos consistency
6. All validation error paths triggered independently
7. Edge cases: zero-length decode buffer, odd chunk sizes, duplicate chunks, boundary sample rates

### FFI Tests (aiff_ffi.rs)

1. Vtable static exists and has correct name string
2. Struct size is valid (>= base struct size)
3. Null pointer handling for all 12 vtable functions
4. InitModule/TermModule format storage lifecycle
