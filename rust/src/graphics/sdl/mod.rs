//!
//! SDL2/OpenGL driver wrapper for graphics subsystem.
//!
//! This module provides Rust wrappers for SDL2 graphics functionality,
//! following the design of the original SDL2 C code in `sc2/src/libs/graphics/sdl/`.
//!
//! # Scope (Phase 2 Task 2.8)
//!
//! This module now provides:
//! - Complete SDL2 pure software driver implementation
//! - Complete OpenGL driver implementation
//! - Common types and error handling
//! - Driver trait interface
//! - Screen management (MAIN, EXTRA, TRANSITION)
//! - Event handling
//! - Fullscreen toggle support
//! - Buffer swapping
//! - Gamma correction (OpenGL and SDL window brightness)
//!
//! # Architecture
//!
//! The module is organized into submodules:
//! - `common`: Shared traits, error types, and utility types
//! - `sdl2`: SDL2 pure software driver implementation
//! - `opengl`: OpenGL driver implementation

pub mod common;
pub mod opengl;
pub mod sdl2;

pub use common::{
    DriverConfig, DriverError, DriverResult, RedrawMode, GraphicsDriver, Screen,
    ScreenDims, UpdateRect, GraphicsEvent,
};
pub use opengl::OpenGlDriver;
pub use sdl2::SdlDriver;
