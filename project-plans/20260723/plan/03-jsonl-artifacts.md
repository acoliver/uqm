# Phase 03: Trace, Artifact, and Identity I/O Primitives

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P03`

Require `.completed/P02.md`. Own only `REQ-IO-001..003` and the serialization primitive of `REQ-TRACE-001`. This phase does not own `REQ-SHOT-*`, end-to-end identity, trace terminal integration, graphics observation/capture completion, activity mutation, FFI, teardown, or child supervision.

## Files

Create `rust/src/automation/{trace,artifact,identity}.rs`; modify automation module/error and justified manifest/lock entries. Prefer verified existing dependencies; if SHA-256 is absent, add one minimal direct production dependency and document why.

## TDD slices

1. Strict typed JSONL records with schema/run/record sequence/input/present/elapsed and semantic transition variants; each line independently parses.
2. Two-phase ordered primitive: reserve checked sequence/immutable payload, publish or RAII-cancel through a dedicated cursor, advance/notify exactly once, and never require/hold the runtime mutex while waiting/writing. Concurrent out-of-order completion publishes in sequence; panic/drop cannot leave a gap. First sink failure rejects later success while cancelled slots advance in memory.
3. Safe/exclusive artifact and temporary naming, collision behavior, root confinement, and cleanup ownership.
4. Durable file helper contract: temporary create_new -> write/encode closure -> `BufWriter::flush` -> recover `File` -> `sync_all` -> close -> exclusive no-replace final publication -> directory sync attempt; classify only OS `Unsupported` as recorded unsupported, all other errors fatal. Fault-inject every stage.
5. SHA-256 executable/file and sorted tree manifests (`relative path`, type, size, digest); reject symlink escape and unstable ordering; mutation changes digest.
6. Identity metadata never substitutes paths for digests and records directory-sync support.

Use pseudocode 003 ordered-I/O lines and pseudocode 004 capture-transaction lines only as pure I/O. Run focused tests and strict gates. No fake power-loss guarantee; claim only completed supported calls, and no callback/capture integration in this phase. Worker hands off; no marker.
