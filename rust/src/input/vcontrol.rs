//! VControl main state and operations
//!
//! The main VControl struct that manages all input bindings.

use parking_lot::RwLock;
use std::sync::LazyLock;

use super::joystick::Joystick;
use super::keyboard::KeyboardBindings;

/// Global VControl instance
pub static VCONTROL: LazyLock<RwLock<VControl>> = LazyLock::new(|| RwLock::new(VControl::new()));

/// Error type for VControl operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VControlError {
    /// VControl not initialized
    NotInitialized,
    /// Joystick not found
    JoystickNotFound(u32),
    /// Binding not found
    BindingNotFound,
    /// Invalid parameter
    InvalidParameter(String),
    /// SDL error
    SdlError(String),
}

impl std::fmt::Display for VControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VControlError::NotInitialized => write!(f, "VControl not initialized"),
            VControlError::JoystickNotFound(idx) => write!(f, "Joystick {} not found", idx),
            VControlError::BindingNotFound => write!(f, "Binding not found"),
            VControlError::InvalidParameter(s) => write!(f, "Invalid parameter: {}", s),
            VControlError::SdlError(s) => write!(f, "SDL error: {}", s),
        }
    }
}

impl std::error::Error for VControlError {}

/// Result type for VControl operations
pub type VControlResult<T> = Result<T, VControlError>;

/// Gesture types for input tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gesture {
    Key(i32),
    JoyAxis { port: u32, axis: i32, polarity: i32 },
    JoyButton { port: u32, button: i32 },
    JoyHat { port: u32, hat: i32, dir: u8 },
}

/// Main VControl state
#[derive(Debug)]
pub struct VControl {
    /// Is the system initialized?
    initialized: bool,
    /// Keyboard bindings
    keyboard: KeyboardBindings,
    /// Connected joysticks
    joysticks: Vec<Option<Joystick>>,
    /// Maximum number of joysticks to track
    max_joysticks: usize,
    /// Last gesture for input configuration
    last_gesture: Option<Gesture>,
}

impl Default for VControl {
    fn default() -> Self {
        Self::new()
    }
}

impl VControl {
    /// Maximum joysticks supported
    pub const MAX_JOYSTICKS: usize = 8;

    /// Create a new VControl instance
    pub fn new() -> Self {
        Self {
            initialized: false,
            keyboard: KeyboardBindings::new(),
            joysticks: vec![None; Self::MAX_JOYSTICKS],
            max_joysticks: Self::MAX_JOYSTICKS,
            last_gesture: None,
        }
    }

    /// Initialize the VControl system
    pub fn init(&mut self) -> VControlResult<()> {
        if self.initialized {
            return Ok(());
        }

        self.keyboard = KeyboardBindings::new();
        self.joysticks = vec![None; self.max_joysticks];
        self.initialized = true;

        Ok(())
    }

    /// Uninitialize the VControl system
    pub fn uninit(&mut self) {
        if !self.initialized {
            return;
        }

        // Clear all bindings
        self.keyboard.clear();
        for joy in &mut self.joysticks {
            *joy = None;
        }

        self.initialized = false;
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Reset all control states to 0
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn reset_states(&mut self) {
        self.keyboard.reset_all_states();
        for joy in self.joysticks.iter_mut().flatten() {
            joy.reset_all();
        }
    }

    // === Keyboard bindings ===

    /// Add a keyboard binding
    pub fn add_key_binding(&mut self, keycode: i32, target: usize) -> bool {
        self.keyboard.add_binding(keycode, target)
    }

    /// Remove a keyboard binding
    pub fn remove_key_binding(&mut self, keycode: i32, target: usize) -> bool {
        self.keyboard.remove_binding(keycode, target)
    }

    /// Clear all keyboard bindings
    pub fn clear_key_bindings(&mut self) {
        self.keyboard.clear();
    }

    /// Handle key down event
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_key_down(&self, keycode: i32) {
        self.keyboard.handle_key_down(keycode);
    }

    /// Handle key up event
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_key_up(&self, keycode: i32) {
        self.keyboard.handle_key_up(keycode);
    }

    /// Check if a key has bindings
    pub fn has_key_bindings(&self, keycode: i32) -> bool {
        self.keyboard.has_bindings(keycode)
    }

    // === Joystick management ===

    /// Initialize a joystick
    pub fn init_joystick(
        &mut self,
        index: u32,
        name: String,
        num_axes: i32,
        num_buttons: i32,
        num_hats: i32,
    ) -> VControlResult<()> {
        if index as usize >= self.max_joysticks {
            return Err(VControlError::InvalidParameter(format!(
                "Joystick index {} exceeds max {}",
                index, self.max_joysticks
            )));
        }

        let joy = Joystick::new(index, name, num_axes, num_buttons, num_hats);
        self.joysticks[index as usize] = Some(joy);

        Ok(())
    }

    /// Uninitialize a joystick
    pub fn uninit_joystick(&mut self, index: u32) -> VControlResult<()> {
        if index as usize >= self.max_joysticks {
            return Err(VControlError::JoystickNotFound(index));
        }

        self.joysticks[index as usize] = None;
        Ok(())
    }

    /// Get number of initialized joysticks
    pub fn num_joysticks(&self) -> u32 {
        self.joysticks.iter().filter(|j| j.is_some()).count() as u32
    }

    /// Get a joystick by index
    pub fn get_joystick(&self, index: u32) -> Option<&Joystick> {
        self.joysticks.get(index as usize).and_then(|j| j.as_ref())
    }

    /// Get a mutable joystick by index
    pub fn get_joystick_mut(&mut self, index: u32) -> Option<&mut Joystick> {
        self.joysticks
            .get_mut(index as usize)
            .and_then(|j| j.as_mut())
    }

    // === Joystick bindings ===

    /// Add a joystick button binding
    pub fn add_joy_button_binding(
        &mut self,
        joy: u32,
        button: i32,
        target: usize,
    ) -> VControlResult<bool> {
        let joystick = self
            .get_joystick_mut(joy)
            .ok_or(VControlError::JoystickNotFound(joy))?;
        Ok(joystick.add_button_binding(button, target))
    }

    /// Remove a joystick button binding
    pub fn remove_joy_button_binding(
        &mut self,
        joy: u32,
        button: i32,
        target: usize,
    ) -> VControlResult<bool> {
        let joystick = self
            .get_joystick_mut(joy)
            .ok_or(VControlError::JoystickNotFound(joy))?;
        Ok(joystick.remove_button_binding(button, target))
    }

    /// Add a joystick axis binding
    pub fn add_joy_axis_binding(
        &mut self,
        joy: u32,
        axis: i32,
        polarity: i32,
        target: usize,
    ) -> VControlResult<bool> {
        let joystick = self
            .get_joystick_mut(joy)
            .ok_or(VControlError::JoystickNotFound(joy))?;
        Ok(joystick.add_axis_binding(axis, polarity, target))
    }

    /// Remove a joystick axis binding
    pub fn remove_joy_axis_binding(
        &mut self,
        joy: u32,
        axis: i32,
        polarity: i32,
        target: usize,
    ) -> VControlResult<bool> {
        let joystick = self
            .get_joystick_mut(joy)
            .ok_or(VControlError::JoystickNotFound(joy))?;
        Ok(joystick.remove_axis_binding(axis, polarity, target))
    }

    /// Add a joystick hat binding
    pub fn add_joy_hat_binding(
        &mut self,
        joy: u32,
        hat: i32,
        direction: u8,
        target: usize,
    ) -> VControlResult<bool> {
        let joystick = self
            .get_joystick_mut(joy)
            .ok_or(VControlError::JoystickNotFound(joy))?;
        Ok(joystick.add_hat_binding(hat, direction, target))
    }

    /// Remove a joystick hat binding
    pub fn remove_joy_hat_binding(
        &mut self,
        joy: u32,
        hat: i32,
        direction: u8,
        target: usize,
    ) -> VControlResult<bool> {
        let joystick = self
            .get_joystick_mut(joy)
            .ok_or(VControlError::JoystickNotFound(joy))?;
        Ok(joystick.remove_hat_binding(hat, direction, target))
    }

    /// Set joystick axis threshold
    pub fn set_joy_threshold(&mut self, joy: u32, threshold: i32) -> VControlResult<()> {
        let joystick = self
            .get_joystick_mut(joy)
            .ok_or(VControlError::JoystickNotFound(joy))?;
        joystick.set_threshold(threshold);
        Ok(())
    }

    // === Joystick event handling ===

    /// Handle joystick button event
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_joy_button(&self, joy: u32, button: i32, pressed: bool) {
        if let Some(joystick) = self.get_joystick(joy) {
            joystick.handle_button(button, pressed);
        }
    }

    /// Handle joystick axis event
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_joy_axis(&mut self, joy: u32, axis: i32, value: i16) {
        if let Some(joystick) = self.get_joystick_mut(joy) {
            joystick.handle_axis(axis, value);
        }
    }

    /// Handle joystick hat event
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_joy_hat(&mut self, joy: u32, hat: i32, value: u8) {
        if let Some(joystick) = self.get_joystick_mut(joy) {
            joystick.handle_hat(hat, value);
        }
    }

    /// Clear all joystick bindings for a specific joystick
    pub fn clear_joy_bindings(&mut self, joy: u32) -> VControlResult<()> {
        let joystick = self
            .get_joystick_mut(joy)
            .ok_or(VControlError::JoystickNotFound(joy))?;
        joystick.clear_bindings();
        Ok(())
    }

    // === Frame management ===

    /// Begin a new input frame - clears start bits
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn begin_frame(&mut self) {
        self.keyboard.begin_frame();
        for joy in self.joysticks.iter_mut().flatten() {
            joy.begin_frame();
        }
    }

    // === Gesture tracking ===

    /// Clear the last gesture
    pub fn clear_gesture(&mut self) {
        self.last_gesture = None;
    }

    /// Get the last gesture
    pub fn get_last_gesture(&self) -> Option<&Gesture> {
        self.last_gesture.as_ref()
    }

    /// Set the last gesture (called internally on key/button events)
    pub fn set_last_gesture(&mut self, gesture: Gesture) {
        self.last_gesture = Some(gesture);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn cleanup() {
        let mut vc = VCONTROL.write();
        vc.uninit();
    }

    #[test]
    #[serial]
    fn test_vcontrol_new() {
        cleanup();
        let vc = VControl::new();
        assert!(!vc.is_initialized());
        assert_eq!(vc.num_joysticks(), 0);
    }

    #[test]
    #[serial]
    fn test_vcontrol_init_uninit() {
        cleanup();
        let mut vc = VControl::new();

        assert!(vc.init().is_ok());
        assert!(vc.is_initialized());

        vc.uninit();
        assert!(!vc.is_initialized());
    }

    #[test]
    #[serial]
    fn test_add_key_binding() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        assert!(vc.add_key_binding(32, 0x1000)); // Space
        assert!(vc.has_key_bindings(32));
    }

    #[test]
    #[serial]
    fn test_remove_key_binding() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        vc.add_key_binding(32, 0x1000);
        assert!(vc.remove_key_binding(32, 0x1000));
        assert!(!vc.has_key_bindings(32));
    }

    #[test]
    #[serial]
    fn test_clear_key_bindings() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        vc.add_key_binding(32, 0x1000);
        vc.add_key_binding(65, 0x2000);

        vc.clear_key_bindings();
        assert!(!vc.has_key_bindings(32));
        assert!(!vc.has_key_bindings(65));
    }

    #[test]
    #[serial]
    fn test_key_down_up() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        let mut target: i32 = 0;
        vc.add_key_binding(32, &mut target as *mut i32 as usize);

        unsafe {
            vc.handle_key_down(32);
            assert_eq!(target, 1);

            vc.handle_key_up(32);
            assert_eq!(target, 0);
        }
    }

    #[test]
    #[serial]
    fn test_init_joystick() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        assert!(vc
            .init_joystick(0, "Test Joy".to_string(), 2, 10, 1)
            .is_ok());
        assert_eq!(vc.num_joysticks(), 1);

        let joy = vc.get_joystick(0).unwrap();
        assert_eq!(joy.name, "Test Joy");
        assert_eq!(joy.num_axes, 2);
        assert_eq!(joy.num_buttons, 10);
        assert_eq!(joy.num_hats, 1);
    }

    #[test]
    #[serial]
    fn test_uninit_joystick() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        vc.init_joystick(0, "Test".to_string(), 0, 0, 0).unwrap();
        assert_eq!(vc.num_joysticks(), 1);

        vc.uninit_joystick(0).unwrap();
        assert_eq!(vc.num_joysticks(), 0);
    }

    #[test]
    #[serial]
    fn test_joy_button_binding() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        vc.init_joystick(0, "Test".to_string(), 0, 5, 0).unwrap();

        assert!(vc.add_joy_button_binding(0, 0, 0x1000).unwrap());

        let mut target: i32 = 0;
        vc.add_joy_button_binding(0, 1, &mut target as *mut i32 as usize)
            .unwrap();

        unsafe {
            vc.handle_joy_button(0, 1, true);
            assert_eq!(target, 1);

            vc.handle_joy_button(0, 1, false);
            assert_eq!(target, 0);
        }
    }

    #[test]
    #[serial]
    fn test_joy_axis_binding() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        vc.init_joystick(0, "Test".to_string(), 2, 0, 0).unwrap();

        let mut neg_target: i32 = 0;
        let mut pos_target: i32 = 0;

        vc.add_joy_axis_binding(0, 0, -1, &mut neg_target as *mut i32 as usize)
            .unwrap();
        vc.add_joy_axis_binding(0, 0, 1, &mut pos_target as *mut i32 as usize)
            .unwrap();

        unsafe {
            // Push axis negative
            vc.handle_joy_axis(0, 0, -20000);
            assert_eq!(neg_target, 1);
            assert_eq!(pos_target, 0);

            // Center
            vc.handle_joy_axis(0, 0, 0);
            assert_eq!(neg_target, 0);
            assert_eq!(pos_target, 0);

            // Push axis positive
            vc.handle_joy_axis(0, 0, 20000);
            assert_eq!(neg_target, 0);
            assert_eq!(pos_target, 1);
        }
    }

    #[test]
    #[serial]
    fn test_joy_threshold() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        vc.init_joystick(0, "Test".to_string(), 2, 0, 0).unwrap();
        vc.set_joy_threshold(0, 5000).unwrap();

        let joy = vc.get_joystick(0).unwrap();
        assert_eq!(joy.threshold, 5000);
    }

    #[test]
    #[serial]
    fn test_joy_hat_binding() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        vc.init_joystick(0, "Test".to_string(), 0, 0, 1).unwrap();

        let mut up_target: i32 = 0;

        vc.add_joy_hat_binding(0, 0, 1, &mut up_target as *mut i32 as usize)
            .unwrap(); // 1 = UP

        unsafe {
            vc.handle_joy_hat(0, 0, 1); // UP
            assert_eq!(up_target, 1);

            vc.handle_joy_hat(0, 0, 0); // CENTER
            assert_eq!(up_target, 0);
        }
    }

    #[test]
    #[serial]
    fn test_reset_states() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        let mut target1: i32 = 5;
        let mut target2: i32 = 10;

        vc.add_key_binding(32, &mut target1 as *mut i32 as usize);

        vc.init_joystick(0, "Test".to_string(), 0, 5, 0).unwrap();
        vc.add_joy_button_binding(0, 0, &mut target2 as *mut i32 as usize)
            .unwrap();

        unsafe {
            vc.reset_states();
            assert_eq!(target1, 0);
            assert_eq!(target2, 0);
        }
    }

    #[test]
    #[serial]
    fn test_joystick_not_found() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        let result = vc.add_joy_button_binding(99, 0, 0x1000);
        assert!(matches!(result, Err(VControlError::JoystickNotFound(99))));
    }

    #[test]
    #[serial]
    fn test_clear_joy_bindings() {
        cleanup();
        let mut vc = VControl::new();
        vc.init().unwrap();

        vc.init_joystick(0, "Test".to_string(), 2, 5, 1).unwrap();
        vc.add_joy_button_binding(0, 0, 0x1000).unwrap();
        vc.add_joy_axis_binding(0, 0, -1, 0x2000).unwrap();
        vc.add_joy_hat_binding(0, 0, 1, 0x3000).unwrap();

        vc.clear_joy_bindings(0).unwrap();

        let joy = vc.get_joystick(0).unwrap();
        assert!(joy.buttons[0].is_empty());
        assert!(joy.axes[0].neg.is_empty());
        assert!(joy.hats[0].up.is_empty());
    }

    #[test]
    #[serial]
    fn test_global_vcontrol() {
        cleanup();

        {
            let mut vc = VCONTROL.write();
            vc.init().unwrap();
            vc.add_key_binding(32, 0x1000);
        }

        {
            let vc = VCONTROL.read();
            assert!(vc.is_initialized());
            assert!(vc.has_key_bindings(32));
        }

        cleanup();
    }
}
