# Phase P09a Verification (Remediated) - ZIP/UQM Archive Mount Support

## Verdict
REJECT

## Basis
Two of the four original rejection reasons are now clearly fixed, and the requested verification command now passes. However, the CRC validation requirement is still not satisfied on the actual streaming read path used for mounted ZIP entries, so Phase P09a cannot be accepted yet.

## Previously rejected items re-verified

### 1. `uio_fopen` was not ZIP-aware — FIXED
Verified in `rust/src/io/uio_bridge.rs`.

`uio_fopen()` now:
- normalizes the virtual path
- locks the mount registry
- finds an active mount matching the virtual path
- detects `UIO_FSTYPE_ZIP`
- rejects write/append modes for ZIP mounts
- opens the archive entry through `mount.zip_index.open_entry(...)`
- wraps the result in `uio_HandleInner::ZipEntry(reader)`

Relevant code:
- `uio_fopen`: lines 2981-3025
- ZIP open path: `zip_index.open_entry(&suffix_str)` at line 3013

This directly addresses the prior rejection that `uio_fopen` only opened host filesystem paths.

### 2. `uio_access(X_OK)` for ZIP directories — FIXED
Verified in `rust/src/io/uio_bridge.rs`.

`uio_access()` now has ZIP-specific `X_OK` handling:
- for ZIP mounts, it recomputes the suffix relative to the mount point
- calls `zip_index.is_directory(&suffix_str)`
- returns success for archive directories
- returns `EACCES` for non-directory ZIP entries

Relevant code:
- ZIP visibility check: lines 247-258
- ZIP `X_OK` handling: lines 307-323
- directory test: line 317

This addresses the prior problem where `X_OK` incorrectly depended on `mounted_root.is_dir()` for ZIP mounts.

### 3. Compilation/test failures — FIXED
Requested verification command:

    cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1 | tail -5

Observed result:

    test threading::tests::test_hibernate_thread ... ok
    test sound::stream::tests::test_uninit_no_thread_ok ... ok

    test result: ok. 1548 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 0.16s

So the previous compilation/test failure is resolved.

## Remaining blocking issue

### 4. CRC validation in the streaming ZIP read path — NOT FIXED
Verified in `rust/src/io/zip_reader.rs`.

There is CRC validation in the buffered whole-entry path:
- `ZipIndex::read_entry()` reads the full entry into memory
- computes `crc32fast::hash(&buffer)`
- compares it against `entry.crc32`
- errors on mismatch

Relevant code:
- CRC validation exists at lines 219-229

However, mounted ZIP reads used by `uio_fopen()` go through the streaming path instead:
- `uio_fopen()` calls `zip_index.open_entry(...)`
- `open_entry()` returns `ZipEntryReader`
- `ZipEntryReader` implements `Read`
- `ZipEntryReader::read()` reopens the entry and reads bytes, but does not compute or compare CRC

Relevant code:
- `open_entry()`: lines 234-249
- `ZipEntryReader` definition: lines 252-258
- `impl Read for ZipEntryReader`: lines 287-317
- actual streaming read: line 314

What is missing in the streaming path:
- no running CRC accumulator
- no comparison against expected CRC at end-of-stream
- no validation on partial/final read completion

Because `uio_fopen()` uses `open_entry()` rather than `read_entry()`, the mounted-file read path still lacks explicit CRC enforcement in the code being verified.

## Final assessment
The remediation successfully fixed:
- ZIP-aware `uio_fopen`
- ZIP directory handling for `uio_access(X_OK)`
- the prior build/test failure

But the CRC requirement remains unmet on the actual streaming read path used by mounted ZIP files. For that reason, Phase P09a remains:

## Verdict
REJECT
