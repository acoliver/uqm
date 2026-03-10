# How Logging and Observability Work in UQM (C + Rust)

> **Audience:** A fresh LLM entering a new context to help with the UQM Rust port.
> This doc covers how to read, capture, and filter logs for debugging. Read
> `howtorun.md` for build/run basics and `howtoconfigure.md` for the C/Rust
> boundary model first.

---

## Three Logging Channels

| Channel | Destination | Producer | Always On? |
|---|---|---|---|
| C `log_add()` | stderr (fd 2) | C code | Yes |
| Rust `eprintln!()` | stderr (fd 2) | Rust code | Yes |
| `rust-bridge.log` | File in working dir | `bridge_log.rs` | When Rust bridge is active |

### 1. C `log_add()` -> stderr

Defined in `libs/log/uqmlog.c`. Include: `#include "libs/log.h"`.

**Levels:** `log_Fatal` (=`log_User`), `log_Error`, `log_Warning`, `log_Info`,
`log_Debug`.

Buffered (up to 15 lines) until `log_init()` is called, then flushes
immediately.

### 2. Rust `eprintln!()` -> stderr

Primary Rust logging mechanism. Chosen over the `log` crate because:
- Zero-dependency, always available.
- Output interleaves chronologically with C's `log_add()` on the same fd.

> The `log` and `tracing` crates are in `Cargo.toml` but are **not the active
> runtime output path**. `eprintln!` is what you'll see in logs.

### 3. `rust-bridge.log` -> File

Created by `rust_bridge_init()` in `bridge_log.rs`. Located in the working
directory (typically `sc2/`). Contains:
- `RUST_BRIDGE_PHASE0_OK` marker
- Bridge lifecycle events
- Diagnostic breadcrumbs from `rust_bridge_log_msg()`

Independent of stderr. Exists whenever the Rust bridge is active regardless of
`--logfile`.

### Rust `logging.rs` -- Calling C's `log_add()` from Rust

`logging.rs` provides a Rust `LogLevel` enum matching the C enum and an
`unsafe fn log_add()` that calls C via FFI. Used primarily by the memory module
for fatal OOM messages:

```rust
use crate::logging::{log_add, LogLevel};
log_add(LogLevel::Fatal, "HMalloc() FATAL: out of memory.");
```

Most Rust modules use `eprintln!` instead.

---

## How C and Rust Share stderr

Both `log_add()` and `eprintln!()` write to fd 2. The `--logfile` CLI flag
redirects stderr via `freopen()` in `uqm.c`, which captures **both** C and Rust
output:

```c
// uqm.c main():
if (options.logFile != NULL)
    freopen(options.logFile, "w", stderr);
```

---

## Capturing Logs

```sh
cd sc2
./uqm 2>tmp/uqm_full.log            # capture all stderr
./uqm --logfile=tmp/uqm_full.log    # same via C flag
./uqm 2>&1 | tee tmp/uqm_live.log   # live + file
cat rust-bridge.log                  # bridge log (separate file in sc2/)
```

---

## `eprintln!` Prefix Conventions by Subsystem

### Audio Subsystem

| Prefix | Source | What |
|---|---|---|
| `[audio_heart]` | `heart_ffi.rs` | FFI entry/exit tracing |
| `[mixer_pump]` | `stream.rs` / mixer | cpal output thread lifecycle |
| `[mixer_pump_cb#N]` | mixer | Per-callback (~100/sec, very verbose) |
| `[stream]` | `stream.rs` | Decoder thread events |
| `[SpliceTrack]` | `heart_ffi.rs` | Track assembly tracing |
| `[PlayTrack]` | `heart_ffi.rs` | Playback start events |
| `[PlayChannel]` | `heart_ffi.rs` | SFX channel play events |
| `LoadSoundFile` | `heart_ffi.rs` | SFX bank loading |
| `LoadMusicFile` | `heart_ffi.rs` | Music file loading |
| `create_decoder_for_extension` | `heart_ffi.rs` | Decoder factory |

### PARITY Markers

```
[PARITY][STREAM_SEEK] source=6 pos_ms=1200
[PARITY][SEEK] request_ticks=840
[PARITY][FAST_FORWARD_SMOOTH] pos_ticks=1680 new_pos_ticks=1980
[PARITY][SUBTITLE] active=<none>
```

`[PARITY]` lines verify Rust output matches C behavior at key decision points.
Debugging aid -- eventual candidates for removal or feature-gating.

### Other Subsystems

| Pattern | Source | Channel |
|---|---|---|
| `Rust memory management initialized.` | `memory.rs` (via C `log_add()` FFI) | stderr |
| `HMalloc() FATAL:` | `memory.rs` (via C `log_add()` FFI) | stderr |
| `RUST_BRIDGE_PHASE0_OK` | `bridge_log.rs` | `rust-bridge.log` file |
| `rust_bridge_init:` | `bridge_log.rs` | stderr |

> **Note for new LLMs:** Not all subsystems have well-defined log prefixes yet.
> Graphics, resource, input, threading, video, comm, clock, and file I/O modules
> may emit `eprintln!` lines without a bracketed prefix. When adding logging to
> these subsystems, follow the `[subsystem_name]` convention shown above.

---

## Filtering Logs

```sh
# Rust lines (bracketed prefixes, loader lines):
grep -E '^\[|^Load|^create_decoder' tmp/uqm_full.log

# C log_add lines (typically no brackets):
grep -vE '^\[|^Load|^create_decoder' tmp/uqm_full.log

# PARITY markers only:
grep '\[PARITY\]' tmp/uqm_full.log

# Specific subsystem:
grep -E '\[audio_heart\]|\[mixer_pump\]|\[stream\]|LoadMusicFile|LoadSoundFile' tmp/uqm_full.log
grep -E '\[gfx\]|\[resource\]|\[thread\]' tmp/uqm_full.log
```

---

## Key Log Sequences for Diagnosis

### Healthy Startup (Rust Bridge Active)

```
initAudio: Using Rust mixer backend
Initializing sound decoders.
Sound decoders initialized.
[mixer_pump] thread started, opening cpal output stream...
[mixer_pump] using device: "Built-in Output"
[mixer_pump] started -- feeding mixer output to cpal
Rust memory management initialized.
```

If audio-heart is also active, you'll additionally see:

```
[audio_heart] InitSound called
[audio_heart] InitSound: success
```

### What Missing Lines Mean

| Expected But Missing | Meaning |
|---|---|
| No `RUST_BRIDGE_PHASE0_OK` in `rust-bridge.log` | Rust bridge not initialized -- `rust_bridge_init()` never called |
| No `[mixer_pump]` lines | Rust mixer not active -- still using SDL/OpenAL backend |
| `[mixer_pump] no output audio device found` | No audio output device available |
| No `LoadMusicFile` / `LoadSoundFile` | Audio-heart not active -- C heart files handling loading |
| No `[audio_heart] InitSound` | Audio-heart not compiled in -- check `config_unix.h` |
| `Rust memory management initialized.` missing | `USE_RUST_MEM` not set |

---

## Verbosity Control

There is currently **no runtime verbosity knob** for Rust `eprintln!` output.

To reduce noise:
- **Build-time:** Wrap verbose lines in `#[cfg(debug_assertions)]` or a custom
  feature flag.
- **Post-hoc:** Filter with `grep -v`.

The `[mixer_pump_cb#N]` lines (~100/sec) are the highest-volume output and first
candidates for gating.

---

## Quick Reference for Debugging

| I want to... | Do this |
|---|---|
| See all output live | `./uqm 2>&1 \| tee tmp/out.log` |
| Confirm Rust bridge is active | `cat rust-bridge.log` -- look for `RUST_BRIDGE_PHASE0_OK` |
| Confirm a specific Rust subsystem is active | `grep USE_RUST_FOO sc2/config_unix.h` and look for init log lines |
| Debug a crash | `./uqm 2>tmp/crash.log` then check last lines |
| See only audio-heart activity | `grep -E '\[audio_heart\]\|\[mixer_pump\]\|LoadMusicFile\|LoadSoundFile' tmp/out.log` |
| See only parity checks | `grep '\[PARITY\]' tmp/out.log` |
| Check bridge lifecycle | `cat rust-bridge.log` |
| Call C's `log_add()` from Rust | `use crate::logging::{log_add, LogLevel};` |
