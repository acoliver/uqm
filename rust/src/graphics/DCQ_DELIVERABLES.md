# DCQ (Draw Command Queue) Implementation - Phase 2, Scope #4

## Overview

This deliverable implements the Draw Command Queue (DCQ) subsystem as part of Phase 2: Graphics System Modernization of issue #4. The DCQ provides thread-safe, lock-stepped rendering with batching support for atomic frame rendering.

## Files Created

- `rust/src/graphics/dcqueue.rs` - Complete DCQ implementation (~1600 LOC)
- `rust/src/graphics/mod.rs` - Updated to export dcqueue module

## Core Types and Structures

### Configuration

- **`DcqConfig`** - Queue size and livelock configuration
  - `max_size`: Maximum queue slots (16,384 standard, 512 debug)
  - `force_slowdown_size`: Threshold for initial livelock slowdown (4,096 / 128)
  - `force_break_size`: Threshold for breaking batching (16,384 / 512)
  - `livelock_max`: Maximum commands before livelock deterrence (4,096 / 256)

### Draw Commands

All 14 command types from `sc2/src/libs/graphics/drawcmd.h` are implemented:

1. **`DrawCommand::Line`** - Line drawing with coordinates, color, mode, dest
2. **`DrawCommand::Rect`** - Rectangle with bounds, color, mode, dest
3. **`DrawCommand::Image`** - Image with position, scale, scale mode, colormap, mode, dest
4. **`DrawCommand::FilledImage`** - Tinted image with color fill
5. **`DrawCommand::FontChar`** - Font character rendering with optional backing
6. **`DrawCommand::Copy`** - Screen-to-screen rectangle copy
7. **`DrawCommand::CopyToImage`** - Screen-to-image rectangle copy
8. **`DrawCommand::ScissorEnable`** - Enable clipping region
9. **`DrawCommand::ScissorDisable`** - Disable clipping
10. **`DrawCommand::SetMipmap`** - Set image mipmap with hot spots
11. **`DrawCommand::DeleteImage`** - Delete image resource
12. **`DrawCommand::DeleteData`** - Delete raw data
13. **`DrawCommand::SendSignal`** - Signal a semaphore
14. **`DrawCommand::ReinitVideo`** - Reinitialize video subsystem
15. **`DrawCommand::Callback`** - Execute callback function

### Support Types

- **`Screen`** - Render destination (Main, Extra, Transition)
- **`DrawMode`** - Compositing operation (Normal, Blended)
- **`Color`** - RGBA color representation
- **`Point`** - 2D position (x, y)
- **`Extent`** - 2D dimensions (width, height)
- **`Rect`** - Screen rectangle with corner and extent
- **`ImageRef`** - Placeholder for TFB_Image (to be replaced in tfb_draw)
- **`ColorMapRef`** - Placeholder for ColorMap (to be replaced in cmap)
- **`FontCharRef`** - Placeholder for TFB_Char (to be replaced in font)

### Queue Implementation

- **`DrawCommandQueue`** - Thread-safe queue API (handles: Arc<Mutex<Inner>>)
  - Ring buffer management with front/back/insertion_point indices
  - Batching support via `BatchGuard` RAII
  - Thread-safe operations with condition variable for waiting
  - Queue size limits and overflow protection
  - Statistics tracking (`DcqStats`)

- **`DcqStats`** - Statistics snapshot
  - `size`: Visible commands available for processing
  - `full_size`: Total commands including batched
  - `max_size`: Queue capacity
  - `batching_depth`: Current batch nesting level
  - `utilization()`: Capacity percentage

- **`BatchGuard`** - RAII guard for automatic batch cleanup
  - Ensures `unbatch()` is called even on panic/early return

### Errors

- **`DcqError`** - Error types
  - `QueueFull` - Cannot add more commands
  - `InvalidConfig` - Bad configuration parameters
  - `WouldBlock` - Non-blocking operation would block

## Key Features

### Batching for Atomic Frame Rendering

Commands can be batched using:
- `batch()` / `unbatch()` - Manual batch control
- `batch_reset()` - Cancel all batching
- `scoped_batch()` - RAII-scoped batching

During batching, `FullSize` tracks all queued commands but `Size` only counts commands up to `back`. This prevents partial frame rendering.

### Thread Safety

- `Mutex<T>` protects all mutable state
- `Condvar` enables waiting for queue space
- Non-recursive mutex design (simpler than C's recursive mutex)
- `Arc` enables cloning of queue handles across threads

### Livelock Mitigation

Configuration constants control livelock deterrence:
- If queue starts with > `force_slowdown_size`, slow down production
- If processing > `livelock_max` commands, enable deterrence
- If batched pending > `force_break_size`, force batch break

### FIFO Ordering

Commands are processed in strict FIFO order using a ring buffer.

## Public APIs (for other modules)

```rust
// Queue creation
let dcq = DrawCommandQueue::new();
let dcq = DrawCommandQueue::with_config(DcqConfig::debug());

// Command operations
dcq.push(cmd)?;                    // Blocking push
dcq.try_push(cmd)?;                // Non-blocking push
let cmd = dcq.pop();               // Pop next command (returns None if empty)
dcq.clear();                       // Clear all commands

// Queue state
let len = dcq.len();              // Visible command count
let full = dcq.full_size();       // Total command count
let empty = dcq.is_empty();       // Check if empty
let full = dcq.is_full();         // Check if full
let stats = dcq.stats();          // Get detailed stats

// Batching
let _guard = dcq.batch();         // Scoped batch (RAII)
dcq.unbatch();                    // Manual batch end
dcq.batch_reset();                // Cancel batching
let result = scoped_batch(&dcq, || { /* ... */ });

// Space reservation (for livelock control)
dcq.lock_wait_space(10)?;         // Wait for N slots
dcq.unlock();                      // Release lock (no-op in Rust)
```

## Architecture Comparison with C

### Similarities
- Ring buffer with Front/Back/InsertionPoint indices
- Batching for atomic frame rendering
- Livelock deterrence thresholds
- All 14 command types
- FIFO ordering

### Differences
- **Rust**: Non-recursive mutex + RAII guards
- **C**: Recursive mutex with explicit lock/unlock
- **Rust**: Type-safe enums instead of union + type field
- **Rust**: Arc<Mutex<>> instead of global variables
- **Rust**: Condvar built into mutex type
- **Rust**: Scope-based resource management

## Test Coverage

All 51 unit tests pass, including:
- Configuration (standard, debug, default)
- All command type creation and equality
- Push/pop FIFO ordering
- Ring buffer wraparound behavior
- Batching (simple and nested)
- Scoped batches with RAII guards
- Queue limits and overflow handling
- Statistics and utilization calculations
- Thread-compatible operations (locks, condition variables)
- Resource reference types (ImageRef, ColorMapRef, FontCharRef)

## Integration Points

The DCQ module is designed to integrate with other Phase 2 subsystems:

1. **tfb_draw** - Will replace `ImageRef` with actual `TFB_Image`
2. **cmap** - Will replace `ColorMapRef` with actual `ColorMap`
3. **font** - Will replace `FontCharRef` with actual `TFB_Char`
4. **SDL drivers** - Will consume commands via `pop()` loop in render thread
5. **FFI layer** - Will expose queue operations to C code

## Performance Characteristics

- Lock contention: Standard `Mutex<T>` with `Condvar` for blocking
- Memory overhead: ~16K slots * sizeof(Option<DrawCommand>) for standard config
- Batching overhead: Minimal (single integer depth counter)
- Zero-copy command passing: Enum moved into ring buffer

## Future Work

The DCQ implementation is complete per issue #4 requirements. Future work may include:
- Integration with actual TFB_Image from tfb_draw module
- Integration with actual ColorMap from cmap module
- Integration with actual TFB_Char from font module
- Rendering loop in SDL driver that processes commands via `pop()`
- FFI bindings for C interop
- Performance benchmarking against C implementation
- Lock-free implementation (if needed for high-contention scenarios)

## Compliance with Issue Requirements

[OK] Rust module design at `rust/src/graphics/dcqueue.rs`
[OK] Core types/structs/enums mirroring C commands
[OK] Batching support for atomic frame rendering
[OK] Queue limits (configurable, with standard and debug presets)
[OK] Livelock mitigation (force_slowdown, force_break, livelock_max)
[OK] Minimal APIs required by other modules
[OK] Unit tests for queue behavior (51 tests, all passing)
[OK] Did not touch TFBDraw, cmap, fonts, scaling, SDL drivers, or FFI (as instructed)

## References

- C implementation: `sc2/src/libs/graphics/dcqueue.c`
- C header: `sc2/src/libs/graphics/dcqueue.h`
- Command definitions: `sc2/src/libs/graphics/drawcmd.h`
- Issue #4: Phase 2: Graphics System Modernization
