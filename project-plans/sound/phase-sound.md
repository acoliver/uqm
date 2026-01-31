# Sound System - Rust Integration Plan

## Overview
Replace C sound decoders with Rust implementations, starting with Ogg Vorbis support.

## Current C Architecture
- `decoder.c` - Main decoder dispatch, vtable pattern
- `oggaud.c` - Ogg Vorbis decoder using libvorbis/tremor
- `wav.c`, `modaud.c`, `aiffaud.c`, `dukaud.c` - Other format decoders
- All decoders implement `TFB_SoundDecoderFuncs` vtable

## C Decoder Interface (from decoder.h)
```c
typedef struct tfb_sounddecoderfuncs {
    const char* (*GetName)(void);
    bool (*InitModule)(int flags, const TFB_DecoderFormats*);
    void (*TermModule)(void);
    uint32 (*GetStructSize)(void);
    int (*GetError)(TFB_SoundDecoder*);
    bool (*Init)(TFB_SoundDecoder*);
    void (*Term)(TFB_SoundDecoder*);
    bool (*Open)(TFB_SoundDecoder*, uio_DirHandle*, const char*);
    void (*Close)(TFB_SoundDecoder*);
    int (*Decode)(TFB_SoundDecoder*, void* buf, sint32 bufsize);
    uint32 (*Seek)(TFB_SoundDecoder*, uint32 pcm_pos);
    uint32 (*GetFrame)(TFB_SoundDecoder*);
} TFB_SoundDecoderFuncs;
```

## Test-First Approach

### Phase S1: Rust Decoder Trait + Unit Tests
1. Define `SoundDecoder` trait in Rust matching C vtable
2. Create `DecoderFormats` struct matching C
3. Write unit tests for trait contract
4. Implement `NullDecoder` as reference implementation

### Phase S2: Ogg Vorbis Decoder + Tests
1. Add `lewton` or `ogg`+`vorbis` crates for pure Rust decoding
2. Implement `OggDecoder` implementing `SoundDecoder` trait
3. Unit tests with embedded test audio data
4. Integration test reading actual .ogg files

### Phase S3: FFI Bridge + Build Integration
1. Create C-compatible FFI wrapper for Rust decoders
2. Implement `rust_ova_DecoderVtbl` matching C vtable layout
3. Add `USE_RUST_OGG` build toggle
4. Conditionally register Rust decoder in `decoder.c`

### Phase S4: Runtime Verification
1. Build with `USE_RUST_OGG=1`
2. Run game, load .ogg music
3. Verify `RUST_OGG_DECODE` log markers
4. Compare audio output quality

## Rust Module Structure
```
rust/src/sound/
├── mod.rs           # Module exports
├── decoder.rs       # SoundDecoder trait
├── formats.rs       # DecoderFormats, AudioFormat enums
├── null.rs          # NullDecoder implementation
├── ogg.rs           # OggDecoder implementation
├── ffi.rs           # C FFI bindings
└── tests/
    ├── decoder_tests.rs
    └── ogg_tests.rs
```

## Dependencies
```toml
[dependencies]
lewton = "0.10"      # Pure Rust Ogg Vorbis decoder
ogg = "0.9"          # Ogg container parser (if needed)
```

## Success Criteria
- All Rust unit tests pass
- Game plays .ogg music through Rust decoder
- Log markers confirm Rust code path
- No audio quality degradation

## Estimated Effort
- Phase S1: 2-3 hours (trait + null decoder + tests)
- Phase S2: 3-4 hours (ogg decoder + tests)
- Phase S3: 2-3 hours (FFI + build integration)
- Phase S4: 1-2 hours (verification)
Total: ~10-12 hours
