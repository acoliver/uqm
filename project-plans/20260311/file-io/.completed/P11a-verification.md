# P11a Verification — Phase P11 (Lifecycle, Init/Uninit, Resource Cleanup)

## Verdict
REJECT

## Reviewed Inputs
1. `.completed/P11.md`
2. `plan/11-lifecycle-cleanup.md`
3. `rust/src/io/uio_bridge.rs`
4. Requested verification command:
   - `cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1 | tail -5`

## What passes
- `UIO_INITIALIZED` exists as an `AtomicBool`.
  - `rust/src/io/uio_bridge.rs:102`
- `uio_init()` is not a log-only stub.
  - It swaps the init flag and clears mount/buffer registries.
  - `rust/src/io/uio_bridge.rs:2727-2750`
- `uio_unInit()` resets the init flag, clears registries, and resets mount IDs.
  - `rust/src/io/uio_bridge.rs:2760-2787`
- `uio_closeRepository()` does unmount all repository mounts before freeing the repository.
  - `rust/src/io/uio_bridge.rs:2806-2811`
  - via `uio_unmountAllDirs()` / `remove_repository_mounts()`
- Post-unmount cleanup paths are structurally independent of mount registry state:
  - `uio_close()` drops the boxed handle directly.
    - `rust/src/io/uio_bridge.rs:3230-3236`
  - `uio_fclose()` frees the stream buffer from the side registry, drops the handle, then drops the stream.
    - `rust/src/io/uio_bridge.rs:3557-3578`
  - `uio_closeDir()` decrements refcount and frees independently of mount state.
    - `rust/src/io/uio_bridge.rs:2894-2906`
  - `uio_releaseStdioAccess()` frees owned C string / temp file resources independently of mount state.
    - `rust/src/io/uio_bridge.rs:2356-2386`
- Requested test command passes:

```text
test threading::tests::test_condvar_wait_timeout ... ok
test threading::tests::test_semaphore_zero_blocks ... ok

test result: ok. 1571 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 0.12s
```

## Why this phase is rejected
The plan explicitly requires:
- `uio_DirList_free` to "eliminate fragile side-channel free dependency, or prove the final allocation strategy is self-contained and leak-free"
- structural verification item: "uio_DirList_free uses a self-contained or otherwise safe free path"
- success criterion: "no memory leaks in stream/dirlist/stdio-temp paths"

The actual implementation does **not** meet that bar.

### Failing issue: `uio_DirList_free` still depends on a fragile side-channel registry
Relevant code:
- `rust/src/io/uio_bridge.rs:3981-4042`
- `rust/src/io/uio_bridge.rs:4054-4068+`

Key observations:
- `uio_DirList_free()` attempts to look up the backing buffer size in `BUFFER_SIZE_REGISTRY`.
- If the size is not found, it intentionally skips deallocation:
  - comment says: `If size not found in registry, we have a leak - but better than double-free!`
  - `rust/src/io/uio_bridge.rs:4020-4026`
- The function comments still describe this as a workaround / side-channel solution rather than a self-contained ownership model.
- `uio_unInit()` clears `BUFFER_SIZE_REGISTRY` wholesale:
  - `rust/src/io/uio_bridge.rs:2778-2781`

That combination means an outstanding `uio_DirList*` freed after `uio_unInit()` can lose the metadata required to free its buffer correctly, violating the plan’s leak-free cleanup expectation and undermining the claim that cleanup remains safe after lifecycle shutdown.

## Additional mismatch vs completion report
`P11.md` claims:
- `uio_DirList_free()` "uses self-contained allocation strategy with side-channel registry"

That is internally inconsistent with the code. A side-channel registry is not self-contained, and the implementation explicitly documents a leak fallback when metadata is missing.

## Conclusion
Phase P11 is close, but the current `uio_DirList_free()` implementation does not satisfy the plan’s required standard for resource cleanup safety and leak freedom. Because of that, Phase P11 cannot be accepted as complete.
