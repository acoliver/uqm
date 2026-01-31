# UIO and Sound System - Test-First Implementation Plan

## Status Summary (UPDATED - Jan 29, 2026)

### Sound Module - COMPLETE
- [x] `decoder.rs` - SoundDecoder trait with full test coverage (2 tests)
- [x] `formats.rs` - AudioFormat/DecoderFormats with test coverage (6 tests)
- [x] `null.rs` - NullDecoder with comprehensive tests (11 tests)
- [x] `ogg.rs` - OggDecoder with unit tests (9 tests)
- [x] `ffi.rs` - FFI vtable with tests (4 tests)
- [x] Integration tests with real .ogg files (6 tests in tests/sound_integration.rs)
- [x] C wiring complete (decoder.c modified, rust_oggaud.h created)
- [x] Build config updated (USE_RUST_OGG flag added)

### UIO Module - Mostly Complete
- [x] `uio_bridge.rs` - Large file (2000+ lines) with many FFI functions
- [x] Syntax error fixed (uio_ferror was malformed)
- [x] Mount registry implementation
- [x] File/stream operations (uio_fopen, uio_fread, etc.)
- [x] Unit tests added (19 tests for uio_bridge.rs)
- [ ] Some functions are stubs (uio_vfprintf, uio_fputc, etc.) - OK for now
- [x] C code wired via USE_RUST_UIO flag in build.config

---

## Completed Work

### Tests Added

**UIO Unit Tests** (`rust/src/io/uio_bridge.rs`):
- test_mount_registry_basic
- test_resolve_mount_path_with_mount
- test_resolve_mount_path_no_mount
- test_resolve_mount_path_absolute_fs_path
- test_cstr_to_pathbuf_valid
- test_cstr_to_pathbuf_null
- test_resolve_path_relative
- test_resolve_path_absolute
- test_matches_pattern_literal
- test_matches_pattern_prefix
- test_matches_pattern_suffix
- test_matches_pattern_substring
- test_matches_pattern_regex_rmp
- test_matches_pattern_regex_zip_uqm
- test_matches_pattern_empty_pattern
- test_buffer_size_registry
- test_buffer_size_registry_null
- test_seek_constants
- test_open_flags_constants

**Sound Integration Tests** (`rust/tests/sound_integration.rs`):
- test_ogg_decoder_open_real_file
- test_ogg_decoder_decode_real_file
- test_ogg_decoder_multiple_decode_calls
- test_ogg_decoder_seek_to_start
- test_ogg_decoder_error_recovery
- test_ogg_decoder_format_detection

### C Integration

**New files created:**
- `sc2/src/libs/sound/decoders/rust_oggaud.h` - Header declaring Rust Ogg vtable

**Files modified:**
- `sc2/src/libs/sound/decoders/decoder.c`:
  - Added `#include "rust_oggaud.h"` when USE_RUST_OGG defined
  - Modified sd_decoders[] array to use `rust_ova_DecoderVtbl` when USE_RUST_OGG defined
- `sc2/build/unix/build.config`:
  - Added USE_RUST_OGG flag when rust_bridge is enabled
  - Added -DUSE_RUST_OGG to CCOMMONFLAGS

---

## Remaining Work

### Phase UIO-3: Additional C Wiring (LOW PRIORITY)
The UIO module is already wired via USE_RUST_UIO. The Rust uio_* functions
are exported as C symbols in the staticlib and will shadow the C library
symbols at link time.

### Runtime Verification
1. Build with rust_bridge enabled: `./build.sh uqm` (select enabled for rust_bridge)
2. Run the game
3. Check rust-bridge.log for:
   - RUST_OGG_INIT_MODULE
   - RUST_OGG_OPEN
   - RUST_OGG_DECODE
4. Verify audio plays correctly

---

## Test Commands

```bash
# Run all Rust tests
cd rust && cargo test --lib

# Run UIO tests only
cd rust && cargo test io::uio_bridge::tests --lib

# Run sound integration tests
cd rust && cargo test --test sound_integration

# Build release staticlib
cd rust && cargo build --release
```

---

## Summary

Total test count:
- 650 unit tests passing (cargo test --lib)
- 6 integration tests passing (sound_integration.rs)

Key accomplishments:
1. Fixed uio_bridge.rs syntax error
2. Added 19 unit tests for UIO module
3. Created sound integration tests with real .ogg files
4. Wired decoder.c to use Rust Ogg decoder when USE_RUST_OGG defined
5. Updated build.config to enable USE_RUST_OGG with rust_bridge
