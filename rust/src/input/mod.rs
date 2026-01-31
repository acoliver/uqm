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
//! The main VControl state is protected by a RwLock for thread-safe access
//! from event handlers.

pub mod ffi;
pub mod joystick;
pub mod keyboard;
pub mod keynames;
pub mod templates;
pub mod vcontrol;

pub use joystick::{HatDirection, Joystick, JoystickAxis, JoystickHat};
pub use keyboard::KeyBinding;
pub use keynames::{key_from_name, key_name};
pub use templates::ControlTemplate;
pub use vcontrol::{VControl, VControlError, VCONTROL};
