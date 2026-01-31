# Report: uio_bridge.rs Fix Attempt

## Summary
Attempted to fix `rust/src/io/uio_bridge.rs` to implement minimal functional stdio-backed UIO (not just log). Successfully implemented the required functions, but encountered linker errors indicating that the Rust implementation does not provide complete coverage of the C UIO API.

## Edits Made

### 1. Replaced entire `uio_bridge.rs` with functional implementation
- Replaced 87 lines of stub code with 563 lines of functional implementation
- All requested functions implemented:
  - `uio_init()` / `uio_unInit()`: Basic lifecycle with logging
  - `uio_openRepository()` / `uio_closeRepository()`: Repository handle management
  - `uio_openDir()` / `uio_closeDir()`: Directory handles with path resolution
  - `uio_mountDir()`: Returns non-null handle (fixed from null)
  - `uio_openDirRelative()`: Relative path resolution
  - `uio_open()` / `uio_close()`: File handle management with std::fs
  - `uio_read()` / `uio_write()`: File I/O with Mutex-protected std::fs::File
  - `uio_fstat()`: File metadata via std::fs::metadata
  - `uio_unlink()`: File deletion via std::fs::remove_file
  - `uio_fopen()` / `uio_fclose()`: Stream operations
  - `uio_fread()` / `uio_fseek()` / `uio_ftell()`: Stream I/O with seeking
  - `uio_getDirList()` / `uio_DirList_free()`: Directory enumeration with std::fs::read_dir

### 2. Key implementation details:
- **Handle modeling**: Used boxed structs with PathBuf/Mutex<std::fs::File>
- **Path handling**: Proper resolution of relative paths to current directory
- **Special case for '/'**: Allows opening root and config directories
- **Logging**: All operations write to `rust-uio.log`
- **Thread safety**: File handles protected by Mutex
- **Memory management**: Uses Box::leak for persistent handles

## Build Results

### Rust Build (SUCCESS)
```bash
cd /Users/acoliver/projects/uqm && cargo build --manifest-path rust/Cargo.toml --release
```
**Result**: Compiled successfully in 19.58s with only warnings (unused imports/constants, non_snake_case names)
- All 16 warnings are style-related, not functional
- Library built at: `rust/target/release/libuqm_rust.a`

### C Link (FAILURE)
```bash
rm sc2/uqm-debug && cd sc2 && sh build/unix/build.sh uqm
```
**Result**: Linker failed with 58 missing symbols

**Linker Error**: Undefined symbols for architecture arm64

Missing symbols include:
- `uio_GPDirHandle_delete`, `uio_GPDir_addFile`, `uio_GPDir_*` (10 GPDir functions)
- `uio_GPFileHandle_delete`, `uio_GPFile_*` (4 GPFile functions)
- `uio_GPRoot_*` (3 GPRoot functions)
- `uio_DirHandle_print` (debug function)
- `uio_accessFileBlock`, `uio_closeFileBlock`, `uio_copyFileBlock`, `uio_openFileBlock` (FileBlock functions)
- `uio_*` stream functions: `uio_clearerr`, `uio_feof`, `uio_ferror`, `uio_fgetc`, `uio_fgets`, `uio_fputc`, `uio_fputs`, `uio_fwrite`
- `uio_*` filesystem functions: `uio_mkdir`, `uio_rename`, `uio_rmdir`, `uio_stat`, `uio_streamHandle`
- `uio_*` mount functions: `uio_printMounts`, `uio_transplantDir`, `uio_unmountDir`
- `uio_*` internal functions: `uio_walkGPPath`, `uio_gPFileFlagsFromPRootFlags`, etc.

## Root Cause Analysis

The `USE_RUST_UIO` flag in `sc2/src/libs/uio/Makeinfo` excludes most C UIO sources:
```makefile
if [ -n "$USE_RUST_UIO" ]; then
    uqm_CFILES="charhashtable.c paths.c uioutils.c"
    uqm_HFILES="charhashtable.h paths.h uioutils.h"
fi
```

This means these C files are **NOT compiled**:
- `defaultfs.c`, `fileblock.c`, `fstypes.c`, `gphys.c`
- `io.c`, `ioaux.c`, `match.c`, `mount.c`, `mounttree.c`
- `physical.c`, `uiostream.c`, `utils.c`
- `debug.c` (still compiled)

The missing symbols are defined in these excluded C files, but the Rust implementation in `uio_bridge.rs` only implements the basic functions listed in the task. It does **NOT** implement the complex GPDir/GPFile/GPRoot APIs, fileblock API, or advanced UIO features.

## Runtime Test Result

Before rebuild, tested with old Rust library:
```bash
./sc2/uqm-debug --contentdir ./sc2/content
```
**Output** (after 5s, then killed):
```
The Ur-Quan Masters v0.8.0 (compiled Jan 25 2026 18:05:36)
...
Initializing base SDL functionality.
Using SDL version 2.32.10 (compiled with 2.32.10)
Could not open '/' dir.
Using config dir '/Users/acoliver/.uqm/'
Fatal error: Could not mount config dir: No such file or directory
```

This shows the original stub implementation failed at directory operations. The new implementation fixes the path resolution for '/' but cannot be tested due to linker errors.

## Conclusion

**Task Status**: INCOMPLETE

The implementation successfully provides all requested functions with std::fs backing, but the project requires **many more UIO functions** beyond those specified. The `USE_RUST_UIO` flag excludes too much C code without providing Rust equivalents.

**Recommendation**: Either:
1. Disable `USE_RUST_UIO` to use full C UIO implementation
2. Implement all missing GPDir/GPFile/GPRoot/fileblock APIs in Rust (major undertaking, ~2000+ lines)
3. Create hybrid approach where only specific functions are replaced (keep C for complex internals)
