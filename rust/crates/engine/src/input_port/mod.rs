//! Input/VControl system
//!
//! This module provides a virtual control abstraction layer that maps
//! keyboard keys and joystick inputs to game actions.
//!
//! # Architecture
//!
//! The VControl system maintains mappings from:
//! - Keyboard keys → control state variables
//! - Joystick buttons → control state variables
//! - Joystick axes → control state variables (with threshold/dead zone)
//! - Joystick hats → control state variables (4-way or 8-way)
//!
//! # Thread Safety
//!
//! Each `InputControls` owns its `VControl`, so bindings and the control words
//! they write live and die together.

pub mod controls;
pub mod frames;
pub mod gesture;
pub mod joystick;
pub mod keyboard;
pub mod keynames;
pub mod menu_binding;
pub mod sdl_input;
pub(crate) use sdl_input::process_input_event;
pub mod templates;
pub mod vcontrol;

pub use gesture::Gesture;
pub use joystick::Joystick;
pub use keynames::{key_from_name, key_name};
pub use menu_binding::{query_menu_binding, MenuBindingResult};
pub use vcontrol::{VControl, VControlError};

pub mod session;
