# AIFF Decoder — Domain Model Analysis

## Entities

### AiffDecoder (Primary Entity)

The central decoder struct. Owns all state for parsing and decoding a single AIFF/AIFC file.

**Lifecycle States:**

```
Created → ModuleInitialized → InstanceInitialized → Opened → Decoding ↔ Seeking → Closed → Terminated
```

State transitions:

| From | Trigger | To | Side Effects |
|------|---------|----|--------------|
| Created | `new()` | Created | All fields zeroed/default |
| Created | `init_module(flags, formats)` | ModuleInitialized | Stores DecoderFormats |
| ModuleInitialized | `init()` | InstanceInitialized | Sets need_swap |
| InstanceInitialized | `open_from_bytes(data, name)` | Opened | Parses file, stores audio data, sets metadata |
| Opened | `decode(buf)` | Opened (Decoding) | Advances cur_pcm/data_pos |
| Opened | `seek(pos)` | Opened | Resets cur_pcm/data_pos/prev_val |
| Opened | `close()` | InstanceInitialized | Clears data, positions, predictor |
| Opened/InstanceInit | `term()` | Created | Calls close() |
| ModuleInitialized | `term_module()` | Created | Clears formats |

### CompressionType (Value Object)

```
enum CompressionType { None, Sdx2 }
```

Determined during `open_from_bytes()` based on form_type and ext_type_id. Immutable for the lifetime of an open file.

### CommonChunk (Value Object)

Parsed from the COMM chunk. Contains: channels, sample_frames, sample_size, sample_rate, ext_type_id.

### SoundDataHeader (Value Object)

Parsed from the SSND chunk header. Contains: offset, block_size.

### ChunkHeader (Value Object)

Parsed per-chunk during iteration. Contains: id (4CC), size.

### AudioFormat (External Enum)

From `formats.rs`: Mono8, Stereo8, Mono16, Stereo16. Selected based on (channels, bits_per_sample).

### DecoderFormats (External Struct)

From `formats.rs`. Stores C format codes and endianness preferences. Passed via `init_module()`.

## State Diagram

```
                      ┌─────────┐
                      │ Created │
                      └────┬────┘
                           │ init_module(flags, formats)
                      ┌────▼──────────────┐
                      │ ModuleInitialized  │
                      └────┬──────────────┘
                           │ init()
                      ┌────▼──────────────────┐
                      │ InstanceInitialized    │◄──────────────┐
                      └────┬──────────────────┘               │
                           │ open_from_bytes()                 │ close()
                      ┌────▼──────┐                           │
           ┌─────────►│  Opened   │───────────────────────────┘
           │          └──┬────┬───┘
           │             │    │
    seek() │   decode()  │    │ decode() returns EndOfFile
           │             │    │
           │          ┌──▼────▼──┐
           └──────────│ Decoding │
                      └──────────┘
```

## Error Handling Map

| Operation | Error Condition | DecodeError Variant | last_error |
|-----------|----------------|--------------------|----|
| open_from_bytes | Non-FORM chunk ID | InvalidData | -2 |
| open_from_bytes | Non-AIFF/AIFC form type | InvalidData | -2 |
| open_from_bytes | COMM chunk too small | InvalidData | -2 |
| open_from_bytes | sample_frames == 0 | InvalidData | -2 |
| open_from_bytes | No SSND chunk | InvalidData | -2 |
| open_from_bytes | bits_per_sample invalid | UnsupportedFormat | -2 |
| open_from_bytes | channels not 1 or 2 | UnsupportedFormat | -2 |
| open_from_bytes | sample_rate out of range | UnsupportedFormat | -2 |
| open_from_bytes | AIFF + ext_type_id != 0 | UnsupportedFormat | -2 |
| open_from_bytes | AIFC + unknown compression | UnsupportedFormat | -2 |
| open_from_bytes | SDX2 + bits != 16 | UnsupportedFormat | -2 |
| open_from_bytes | SDX2 + channels > 4 | UnsupportedFormat | -2 |
| decode | cur_pcm >= max_pcm | EndOfFile | 0 |
| decode | unknown comp_type | DecoderError | -1 |
| Any parse read | Cursor exhausted | InvalidData | -2 |

## Integration Touchpoints

### Rust-side Integration

1. **`rust/src/sound/mod.rs`** — Module registration: `pub mod aiff; pub mod aiff_ffi;`
2. **`rust/src/sound/decoder.rs`** — `SoundDecoder` trait (implemented by AiffDecoder)
3. **`rust/src/sound/formats.rs`** — `AudioFormat`, `DecoderFormats` (used by AiffDecoder)
4. **`rust/src/sound/ffi.rs`** — C types: `TFB_SoundDecoder`, `TFB_SoundDecoderFuncs`, `TFB_DecoderFormats`, `uio_DirHandle`
5. **`rust/src/bridge_log.rs`** — `rust_bridge_log_msg()` (used by FFI for diagnostics)

### C-side Integration

1. **`sc2/src/libs/sound/decoders/decoder.c`** — `sd_decoders[]` registration with `USE_RUST_AIFF` conditional
2. **`sc2/src/libs/sound/decoders/rust_aiff.h`** — New header declaring `extern TFB_SoundDecoderFuncs rust_aifa_DecoderVtbl`
3. **`sc2/src/config_unix.h.in`** — `@SYMBOL_USE_RUST_AIFF_DEF@` placeholder
4. **`sc2/build.vars.in`** — `USE_RUST_AIFF` / `SYMBOL_USE_RUST_AIFF_DEF` variables

### Old Code Replaced

When `USE_RUST_AIFF` is defined:
- `aifa_DecoderVtbl` (from `aiffaud.c`) is replaced by `rust_aifa_DecoderVtbl`
- The `"aif"` extension entry in `sd_decoders[]` points to the Rust vtable
- No C code is removed — the conditional compilation allows fallback

## Data Flow

```
1. C decoder.c calls rust_aifa_Init() via vtable
   → FFI allocates Box<AiffDecoder>, stores raw pointer

2. C decoder.c calls rust_aifa_Open(decoder, dir, filename) via vtable
   → FFI reads file via UIO into Vec<u8>
   → FFI calls dec.open_from_bytes(&data, filename)
   → AiffDecoder parses FORM header, iterates chunks
   → AiffDecoder parses COMM, SSND, validates, stores data
   → FFI updates base struct fields (frequency, format, length, need_swap)

3. C audio mixer calls rust_aifa_Decode(decoder, buf, bufsize) via vtable
   → FFI calls dec.decode(&mut buf_slice)
   → AiffDecoder dispatches to decode_pcm() or decode_sdx2()
   → PCM: copies from self.data, applies 8-bit conversion if needed
   → SDX2: reads compressed bytes, applies ADPCM algorithm
   → Returns byte count written

4. C audio mixer calls rust_aifa_Seek(decoder, pcm_pos) via vtable
   → FFI calls dec.seek(pcm_pos)
   → AiffDecoder clamps position, updates state, resets predictor

5. C decoder.c calls rust_aifa_Close(decoder) via vtable
   → FFI calls dec.close()
   → AiffDecoder clears data and state

6. C decoder.c calls rust_aifa_Term(decoder) via vtable
   → FFI reconstructs Box from raw pointer, drops it
```
