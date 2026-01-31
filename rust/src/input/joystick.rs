//! Joystick input handling
//!
//! Handles joystick button, axis, and hat bindings.

/// Hat direction constants (matching SDL)
pub mod HatDirection {
    pub const CENTERED: u8 = 0;
    pub const UP: u8 = 1;
    pub const RIGHT: u8 = 2;
    pub const DOWN: u8 = 4;
    pub const LEFT: u8 = 8;
    pub const RIGHTUP: u8 = RIGHT | UP;
    pub const RIGHTDOWN: u8 = RIGHT | DOWN;
    pub const LEFTUP: u8 = LEFT | UP;
    pub const LEFTDOWN: u8 = LEFT | DOWN;
}

/// A joystick binding that maps a button/axis/hat to a control state variable.
#[derive(Debug, Clone, Copy)]
pub struct JoyBinding {
    /// Target control state variable address (as usize for FFI)
    pub target: usize,
}

impl JoyBinding {
    pub fn new(target: usize) -> Self {
        Self { target }
    }

    /// Set the target to a value
    ///
    /// # Safety
    /// Caller must ensure target is a valid writable i32 pointer
    pub unsafe fn set(&self, value: i32) {
        let ptr = self.target as *mut i32;
        if !ptr.is_null() {
            *ptr = value;
        }
    }
}

/// Joystick axis with negative and positive direction bindings
#[derive(Debug, Default, Clone)]
pub struct JoystickAxis {
    /// Bindings for negative axis direction (left/up)
    pub neg: Vec<JoyBinding>,
    /// Bindings for positive axis direction (right/down)
    pub pos: Vec<JoyBinding>,
    /// Current polarity: -1 = negative, 0 = centered, 1 = positive
    pub polarity: i32,
}

impl JoystickAxis {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding for the negative direction
    pub fn add_neg(&mut self, target: usize) -> bool {
        if self.neg.iter().any(|b| b.target == target) {
            return false;
        }
        self.neg.push(JoyBinding::new(target));
        true
    }

    /// Add a binding for the positive direction
    pub fn add_pos(&mut self, target: usize) -> bool {
        if self.pos.iter().any(|b| b.target == target) {
            return false;
        }
        self.pos.push(JoyBinding::new(target));
        true
    }

    /// Remove a binding from the negative direction
    pub fn remove_neg(&mut self, target: usize) -> bool {
        let len = self.neg.len();
        self.neg.retain(|b| b.target != target);
        self.neg.len() != len
    }

    /// Remove a binding from the positive direction
    pub fn remove_pos(&mut self, target: usize) -> bool {
        let len = self.pos.len();
        self.pos.retain(|b| b.target != target);
        self.pos.len() != len
    }

    /// Handle axis value change
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_value(&mut self, value: i16, threshold: i32) {
        let thresh = threshold as i16;

        if value < -thresh {
            // Negative direction active
            if self.polarity >= 0 {
                // Transitioning from center or positive
                for binding in &self.pos {
                    binding.set(0);
                }
                for binding in &self.neg {
                    binding.set(1);
                }
            }
            self.polarity = -1;
        } else if value > thresh {
            // Positive direction active
            if self.polarity <= 0 {
                // Transitioning from center or negative
                for binding in &self.neg {
                    binding.set(0);
                }
                for binding in &self.pos {
                    binding.set(1);
                }
            }
            self.polarity = 1;
        } else {
            // Dead zone - release both
            if self.polarity != 0 {
                for binding in &self.neg {
                    binding.set(0);
                }
                for binding in &self.pos {
                    binding.set(0);
                }
            }
            self.polarity = 0;
        }
    }

    /// Reset axis state
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn reset(&mut self) {
        for binding in &self.neg {
            binding.set(0);
        }
        for binding in &self.pos {
            binding.set(0);
        }
        self.polarity = 0;
    }
}

/// Joystick hat (D-pad) with 4-direction bindings
#[derive(Debug, Default, Clone)]
pub struct JoystickHat {
    /// Bindings for up direction
    pub up: Vec<JoyBinding>,
    /// Bindings for down direction
    pub down: Vec<JoyBinding>,
    /// Bindings for left direction
    pub left: Vec<JoyBinding>,
    /// Bindings for right direction
    pub right: Vec<JoyBinding>,
    /// Last hat value
    pub last: u8,
}

impl JoystickHat {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding for a direction
    pub fn add_binding(&mut self, direction: u8, target: usize) -> bool {
        let bindings = match direction {
            HatDirection::UP => &mut self.up,
            HatDirection::DOWN => &mut self.down,
            HatDirection::LEFT => &mut self.left,
            HatDirection::RIGHT => &mut self.right,
            _ => return false,
        };

        if bindings.iter().any(|b| b.target == target) {
            return false;
        }
        bindings.push(JoyBinding::new(target));
        true
    }

    /// Remove a binding from a direction
    pub fn remove_binding(&mut self, direction: u8, target: usize) -> bool {
        let bindings = match direction {
            HatDirection::UP => &mut self.up,
            HatDirection::DOWN => &mut self.down,
            HatDirection::LEFT => &mut self.left,
            HatDirection::RIGHT => &mut self.right,
            _ => return false,
        };

        let len = bindings.len();
        bindings.retain(|b| b.target != target);
        bindings.len() != len
    }

    /// Handle hat value change
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_value(&mut self, value: u8) {
        // Update each direction based on new value
        let was_up = self.last & HatDirection::UP != 0;
        let was_down = self.last & HatDirection::DOWN != 0;
        let was_left = self.last & HatDirection::LEFT != 0;
        let was_right = self.last & HatDirection::RIGHT != 0;

        let is_up = value & HatDirection::UP != 0;
        let is_down = value & HatDirection::DOWN != 0;
        let is_left = value & HatDirection::LEFT != 0;
        let is_right = value & HatDirection::RIGHT != 0;

        // Handle up
        if is_up != was_up {
            for binding in &self.up {
                binding.set(if is_up { 1 } else { 0 });
            }
        }

        // Handle down
        if is_down != was_down {
            for binding in &self.down {
                binding.set(if is_down { 1 } else { 0 });
            }
        }

        // Handle left
        if is_left != was_left {
            for binding in &self.left {
                binding.set(if is_left { 1 } else { 0 });
            }
        }

        // Handle right
        if is_right != was_right {
            for binding in &self.right {
                binding.set(if is_right { 1 } else { 0 });
            }
        }

        self.last = value;
    }

    /// Reset hat state
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn reset(&mut self) {
        for binding in &self.up {
            binding.set(0);
        }
        for binding in &self.down {
            binding.set(0);
        }
        for binding in &self.left {
            binding.set(0);
        }
        for binding in &self.right {
            binding.set(0);
        }
        self.last = HatDirection::CENTERED;
    }
}

/// Represents a connected joystick with its bindings
#[derive(Debug, Clone)]
pub struct Joystick {
    /// Joystick index
    pub index: u32,
    /// Joystick name
    pub name: String,
    /// Number of axes
    pub num_axes: i32,
    /// Number of buttons
    pub num_buttons: i32,
    /// Number of hats
    pub num_hats: i32,
    /// Axis dead zone threshold (default 10000)
    pub threshold: i32,
    /// Axis bindings
    pub axes: Vec<JoystickAxis>,
    /// Button bindings (list per button)
    pub buttons: Vec<Vec<JoyBinding>>,
    /// Hat bindings
    pub hats: Vec<JoystickHat>,
    /// Is this joystick currently open?
    pub is_open: bool,
}

impl Joystick {
    /// Create a new joystick entry
    pub fn new(index: u32, name: String, num_axes: i32, num_buttons: i32, num_hats: i32) -> Self {
        Self {
            index,
            name,
            num_axes,
            num_buttons,
            num_hats,
            threshold: 10000, // Default dead zone
            axes: (0..num_axes).map(|_| JoystickAxis::new()).collect(),
            buttons: (0..num_buttons).map(|_| Vec::new()).collect(),
            hats: (0..num_hats).map(|_| JoystickHat::new()).collect(),
            is_open: true,
        }
    }

    /// Add a button binding
    pub fn add_button_binding(&mut self, button: i32, target: usize) -> bool {
        if button < 0 || button >= self.num_buttons {
            return false;
        }
        let bindings = &mut self.buttons[button as usize];
        if bindings.iter().any(|b| b.target == target) {
            return false;
        }
        bindings.push(JoyBinding::new(target));
        true
    }

    /// Remove a button binding
    pub fn remove_button_binding(&mut self, button: i32, target: usize) -> bool {
        if button < 0 || button >= self.num_buttons {
            return false;
        }
        let bindings = &mut self.buttons[button as usize];
        let len = bindings.len();
        bindings.retain(|b| b.target != target);
        bindings.len() != len
    }

    /// Add an axis binding
    pub fn add_axis_binding(&mut self, axis: i32, polarity: i32, target: usize) -> bool {
        if axis < 0 || axis >= self.num_axes {
            return false;
        }
        let ax = &mut self.axes[axis as usize];
        if polarity < 0 {
            ax.add_neg(target)
        } else {
            ax.add_pos(target)
        }
    }

    /// Remove an axis binding
    pub fn remove_axis_binding(&mut self, axis: i32, polarity: i32, target: usize) -> bool {
        if axis < 0 || axis >= self.num_axes {
            return false;
        }
        let ax = &mut self.axes[axis as usize];
        if polarity < 0 {
            ax.remove_neg(target)
        } else {
            ax.remove_pos(target)
        }
    }

    /// Add a hat binding
    pub fn add_hat_binding(&mut self, hat: i32, direction: u8, target: usize) -> bool {
        if hat < 0 || hat >= self.num_hats {
            return false;
        }
        self.hats[hat as usize].add_binding(direction, target)
    }

    /// Remove a hat binding
    pub fn remove_hat_binding(&mut self, hat: i32, direction: u8, target: usize) -> bool {
        if hat < 0 || hat >= self.num_hats {
            return false;
        }
        self.hats[hat as usize].remove_binding(direction, target)
    }

    /// Set axis threshold
    pub fn set_threshold(&mut self, threshold: i32) {
        self.threshold = threshold.max(0).min(32767);
    }

    /// Handle button press/release
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_button(&self, button: i32, pressed: bool) {
        if button < 0 || button >= self.num_buttons {
            return;
        }
        let value = if pressed { 1 } else { 0 };
        for binding in &self.buttons[button as usize] {
            binding.set(value);
        }
    }

    /// Handle axis movement
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_axis(&mut self, axis: i32, value: i16) {
        if axis < 0 || axis >= self.num_axes {
            return;
        }
        self.axes[axis as usize].handle_value(value, self.threshold);
    }

    /// Handle hat movement
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn handle_hat(&mut self, hat: i32, value: u8) {
        if hat < 0 || hat >= self.num_hats {
            return;
        }
        self.hats[hat as usize].handle_value(value);
    }

    /// Reset all control states
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn reset_all(&mut self) {
        for bindings in &self.buttons {
            for binding in bindings {
                binding.set(0);
            }
        }
        for axis in &mut self.axes {
            axis.reset();
        }
        for hat in &mut self.hats {
            hat.reset();
        }
    }

    /// Clear all bindings
    pub fn clear_bindings(&mut self) {
        for bindings in &mut self.buttons {
            bindings.clear();
        }
        for axis in &mut self.axes {
            axis.neg.clear();
            axis.pos.clear();
        }
        for hat in &mut self.hats {
            hat.up.clear();
            hat.down.clear();
            hat.left.clear();
            hat.right.clear();
        }
    }

    /// Begin a new input frame - clear start bits from all bound targets
    ///
    /// # Safety
    /// Caller must ensure all target addresses are valid writable i32 pointers
    pub unsafe fn begin_frame(&self) {
        const VCONTROL_MASK: i32 = 0xFF;
        for bindings in &self.buttons {
            for binding in bindings {
                let ptr = binding.target as *mut i32;
                if !ptr.is_null() {
                    *ptr &= VCONTROL_MASK;
                }
            }
        }
        for axis in &self.axes {
            for binding in &axis.neg {
                let ptr = binding.target as *mut i32;
                if !ptr.is_null() {
                    *ptr &= VCONTROL_MASK;
                }
            }
            for binding in &axis.pos {
                let ptr = binding.target as *mut i32;
                if !ptr.is_null() {
                    *ptr &= VCONTROL_MASK;
                }
            }
        }
        for hat in &self.hats {
            for binding in &hat.up {
                let ptr = binding.target as *mut i32;
                if !ptr.is_null() {
                    *ptr &= VCONTROL_MASK;
                }
            }
            for binding in &hat.down {
                let ptr = binding.target as *mut i32;
                if !ptr.is_null() {
                    *ptr &= VCONTROL_MASK;
                }
            }
            for binding in &hat.left {
                let ptr = binding.target as *mut i32;
                if !ptr.is_null() {
                    *ptr &= VCONTROL_MASK;
                }
            }
            for binding in &hat.right {
                let ptr = binding.target as *mut i32;
                if !ptr.is_null() {
                    *ptr &= VCONTROL_MASK;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joy_binding_new() {
        let binding = JoyBinding::new(0x1000);
        assert_eq!(binding.target, 0x1000);
    }

    #[test]
    fn test_axis_add_bindings() {
        let mut axis = JoystickAxis::new();

        assert!(axis.add_neg(0x1000));
        assert!(axis.add_pos(0x2000));
        assert_eq!(axis.neg.len(), 1);
        assert_eq!(axis.pos.len(), 1);

        // Duplicate should fail
        assert!(!axis.add_neg(0x1000));
        assert!(!axis.add_pos(0x2000));
    }

    #[test]
    fn test_axis_handle_value() {
        let mut axis = JoystickAxis::new();
        let mut neg_target: i32 = 0;
        let mut pos_target: i32 = 0;

        axis.add_neg(&mut neg_target as *mut i32 as usize);
        axis.add_pos(&mut pos_target as *mut i32 as usize);

        unsafe {
            // Move to negative
            axis.handle_value(-15000, 10000);
            assert_eq!(neg_target, 1);
            assert_eq!(pos_target, 0);
            assert_eq!(axis.polarity, -1);

            // Move to center
            axis.handle_value(0, 10000);
            assert_eq!(neg_target, 0);
            assert_eq!(pos_target, 0);
            assert_eq!(axis.polarity, 0);

            // Move to positive
            axis.handle_value(15000, 10000);
            assert_eq!(neg_target, 0);
            assert_eq!(pos_target, 1);
            assert_eq!(axis.polarity, 1);
        }
    }

    #[test]
    fn test_axis_dead_zone() {
        let mut axis = JoystickAxis::new();
        let mut target: i32 = 0;

        axis.add_neg(&mut target as *mut i32 as usize);

        unsafe {
            // Value within dead zone should not trigger
            axis.handle_value(-5000, 10000);
            assert_eq!(target, 0);
            assert_eq!(axis.polarity, 0);

            // Value outside dead zone should trigger
            axis.handle_value(-15000, 10000);
            assert_eq!(target, 1);
            assert_eq!(axis.polarity, -1);
        }
    }

    #[test]
    fn test_hat_bindings() {
        let mut hat = JoystickHat::new();

        assert!(hat.add_binding(HatDirection::UP, 0x1000));
        assert!(hat.add_binding(HatDirection::DOWN, 0x2000));
        assert!(hat.add_binding(HatDirection::LEFT, 0x3000));
        assert!(hat.add_binding(HatDirection::RIGHT, 0x4000));

        assert_eq!(hat.up.len(), 1);
        assert_eq!(hat.down.len(), 1);
        assert_eq!(hat.left.len(), 1);
        assert_eq!(hat.right.len(), 1);
    }

    #[test]
    fn test_hat_handle_value() {
        let mut hat = JoystickHat::new();
        let mut up_target: i32 = 0;
        let mut right_target: i32 = 0;

        hat.add_binding(HatDirection::UP, &mut up_target as *mut i32 as usize);
        hat.add_binding(HatDirection::RIGHT, &mut right_target as *mut i32 as usize);

        unsafe {
            // Press up
            hat.handle_value(HatDirection::UP);
            assert_eq!(up_target, 1);
            assert_eq!(right_target, 0);

            // Move to up-right diagonal
            hat.handle_value(HatDirection::RIGHTUP);
            assert_eq!(up_target, 1);
            assert_eq!(right_target, 1);

            // Release to center
            hat.handle_value(HatDirection::CENTERED);
            assert_eq!(up_target, 0);
            assert_eq!(right_target, 0);
        }
    }

    #[test]
    fn test_joystick_new() {
        let joy = Joystick::new(0, "Test Joystick".to_string(), 2, 10, 1);

        assert_eq!(joy.index, 0);
        assert_eq!(joy.name, "Test Joystick");
        assert_eq!(joy.num_axes, 2);
        assert_eq!(joy.num_buttons, 10);
        assert_eq!(joy.num_hats, 1);
        assert_eq!(joy.axes.len(), 2);
        assert_eq!(joy.buttons.len(), 10);
        assert_eq!(joy.hats.len(), 1);
        assert!(joy.is_open);
    }

    #[test]
    fn test_joystick_button_binding() {
        let mut joy = Joystick::new(0, "Test".to_string(), 0, 5, 0);

        assert!(joy.add_button_binding(0, 0x1000));
        assert!(joy.add_button_binding(0, 0x2000)); // Multiple bindings
        assert!(!joy.add_button_binding(0, 0x1000)); // Duplicate
        assert!(!joy.add_button_binding(10, 0x3000)); // Out of range
    }

    #[test]
    fn test_joystick_handle_button() {
        let mut joy = Joystick::new(0, "Test".to_string(), 0, 5, 0);
        let mut target: i32 = 0;

        joy.add_button_binding(2, &mut target as *mut i32 as usize);

        unsafe {
            joy.handle_button(2, true);
            assert_eq!(target, 1);

            joy.handle_button(2, false);
            assert_eq!(target, 0);
        }
    }

    #[test]
    fn test_joystick_set_threshold() {
        let mut joy = Joystick::new(0, "Test".to_string(), 2, 0, 0);

        joy.set_threshold(5000);
        assert_eq!(joy.threshold, 5000);

        joy.set_threshold(-100);
        assert_eq!(joy.threshold, 0);

        joy.set_threshold(50000);
        assert_eq!(joy.threshold, 32767);
    }

    #[test]
    fn test_joystick_clear_bindings() {
        let mut joy = Joystick::new(0, "Test".to_string(), 2, 5, 1);

        joy.add_button_binding(0, 0x1000);
        joy.add_axis_binding(0, -1, 0x2000);
        joy.add_hat_binding(0, HatDirection::UP, 0x3000);

        joy.clear_bindings();

        assert!(joy.buttons[0].is_empty());
        assert!(joy.axes[0].neg.is_empty());
        assert!(joy.hats[0].up.is_empty());
    }
}
