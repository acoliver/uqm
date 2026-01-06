# Graphics FFI Layer - Phase 2 FFI Implementation

## Overview

This document describes the FFI (Foreign Function Interface) layer for the graphics subsystem, providing safe Rust bindings to C graphics functions.

## Files Created/Modified

### Created Files

1. **`rust/src/graphics/ffi.rs`** - Main FFI bindings module (490+ lines)
   - Raw unsafe FFI bindings to all C graphics functions
   - Comprehensive safety documentation
   - Type-safe enum representations
   - Unit tests for constants and type correctness

### Modified Files

1. **`rust/src/graphics/mod.rs`** - Added `ffi` module and re-exports
2. **`rust/src/graphics/cmap.rs`** - Fixed syntax error (extra closing brace)

## FFI Bindings Coverage

### Initialization/Shutdown Functions

| Function | Purpose | Safety Notes |
|----------|---------|--------------|
| `TFB_PreInit()` | Pre-initialization setup | Must be called first, main thread only |
| `TFB_InitGraphics()` | Initialize graphics driver | Must call after PreInit, main thread only |
| `TFB_ReInitGraphics()` | Reinitialize with new params | Requires initialized graphics, main thread |
| `TFB_UninitGraphics()` | Cleanup and shutdown | Must be called before program exit |

### Rendering Operations

| Function | Purpose | Safety Notes |
|----------|---------|--------------|
| `TFB_SwapBuffers()` | Swap display buffers | Requires initialized graphics |
| `TFB_ProcessEvents()` | Handle SDL events | Can be called in any state |

### Draw Command Queue (DCQ) Operations

| Function | Purpose | Safety Notes |
|----------|---------|--------------|
| `TFB_BatchGraphics()` | Start batching draw commands | Must be paired with Unbatch |
| `TFB_UnbatchGraphics()` | End batching and process | Must be called after Batch |
| `TFB_BatchReset()` | Reset batch state | Only for error recovery |

### Flush/Cleanup Operations

| Function | Purpose | Safety Notes |
|----------|---------|--------------|
| `TFB_FlushGraphics()` | Process pending commands | Main thread only, may block |
| `TFB_PurgeDanglingGraphics()` | Purge resources during shutdown | Main thread only |

### Display Configuration

| Function | Purpose | Safety Notes |
|----------|---------|--------------|
| `TFB_SetGamma()` | Set gamma correction | Requires initialized graphics |
| `TFB_UploadTransitionScreen()` | Upload transition image | Requires initialized graphics |
| `TFB_SupportsHardwareScaling()` | Check scaling support | Requires initialized graphics |

### Global State Variables

| Variable | Type | Safety Notes |
|----------|------|--------------|
| `GfxFlags` | `c_int` | Access after init only |
| `ScreenWidth` | `c_int` | Access after init only |
| `ScreenHeight` | `c_int` | Access after init only |
| `ScreenWidthActual` | `c_int` | Access after init only |
| `ScreenHeightActual` | `c_int` | Access after init only |
| `ScreenColorDepth` | `c_int` | Access after init only |
| `GraphicsDriver` | `c_int` | Access after init only |
| `FrameRate` | `libc::c_float` | Access after init only |
| `FrameRateTickBase` | `c_int` | Access after init only |

## Type Safety

### Enum Types

- **`TfbGfxDriver`** - Graphics driver IDs (SDL_OPENGL=0, SDL_PURE=1)
- **`TfbRedrawMode`** - Redraw flags (No=0, Fading=1, Expose=2, Yes=3)

### Flag Constants (module `gfx_flags`)

```rust
pub const FULLSCREEN: u32 = 1 << 0;
pub const SHOWFPS: u32 = 1 << 1;
pub const SCANLINES: u32 = 1 << 2;
pub const SCALE_BILINEAR: u32 = 1 << 3;
pub const SCALE_BIADAPT: u32 = 1 << 4;
pub const SCALE_BIADAPTADV: u32 = 1 << 5;
pub const SCALE_TRISCAN: u32 = 1 << 6;
pub const SCALE_HQXX: u32 = 1 << 7;
pub const SCALE_ANY: u32 = ...; // All scaling flags
pub const SCALE_SOFT_ONLY: u32 = ...; // Scaling without OpenGL
```

## Safety Documentation

The FFI module includes comprehensive safety documentation covering:

1. **Thread Safety** - Which functions must be called from main thread
2. **Initialization Sequence** - Proper order of operations
3. **DCQ Batching** - How to correctly batch draw commands
4. **Null Pointer Handling** - When NULL is acceptable
5. **Global State Access** - When it's safe to read globals

## Testing

### Test Coverage

The module includes unit tests that verify:

1. **Enum Value Correspondence** - Rust enums match C header constants
2. **Flag Constant Values** - Flag bit values correct
3. **Flag Compositions** - SCALE_ANY and SCALE_SOFT_ONLY computed correctly
4. **Null Pointer Handling** - NULL is a valid renderer parameter value
5. **Type Representations** - Types have correct size/alignment

### Running Tests

```bash
cd rust
cargo test --lib graphics::ffi
```

Note: Full integration testing requires the C graphics library to be built and linked.

## Building

### Prerequisites

To build and link the FFI layer:

1. **SDL2 Development Headers** - Required for graphics drivers
2. **C Compiler** - For compiling C graphics sources

### Build Configuration

The FFI bindings use a `#[link(name = "uqm_graphics", kind = "static")]` attribute.
The actual C library compilation will be configured in `build.rs` in a future phase.

### Current State

- [OK] Type declarations compile successfully
- [OK] Unit tests for constants pass
-  C library linking requires build.rs configuration (future phase)

## Public API

### Re-exports

The following types and constants are re-exported from `crate::graphics`:

```rust
pub use ffi::{
    gfx_flags,        // Flag constants module
    TfbGfxDriver,     // Driver enum
    TfbRedrawMode,    // Redraw mode enum
};
```

### Example Usage

```rust
use crate::graphics::{TfbGfxDriver, gfx_flags};

// Set up initialization parameters
let driver = TfbGfxDriver::SdlOpengl;
let flags = gfx_flags::FULLSCREEN | gfx_flags::SHOWFPS;

// Note: Actual FFI calls require unsafe block and proper initialization
// unsafe {
//     TFB_PreInit();
//     TFB_InitGraphics(driver as i32, flags, std::ptr::null(), 640, 480);
// }
```

## Scope Adherence

This FFI layer implementation strictly adheres to the Phase 2 scope:

[OK] **Included:**
- Init/uninit operations (TFB_PreInit, TFB_InitGraphics, TFB_ReInitGraphics, TFB_UninitGraphics)
- Swap operations (TFB_SwapBuffers)
- Queue operations (TFB_BatchGraphics, TFB_UnbatchGraphics, TFB_BatchReset, TFB_FlushGraphics)
- Safety documentation
- Minimal tests for type correctness and null pointers

[ERROR] **Excluded (as specified):**
- DCQ implementation logic
- tfb_draw logic
- Colormap implementation
- Font implementation
- Scaling implementation
- SDL driver implementation

## Next Steps

To complete the FFI integration:

1. **Configure build.rs** - Add C graphics library compilation
2. **Add bindgen** - Auto-generate bindings from C headers (optional, manual bindings provided)
3. **Integration Tests** - Add tests that call the actual C functions
4. **Safe Wrapper** - Create a safe Rust wrapper module (`graphics/wrapper.rs`)
