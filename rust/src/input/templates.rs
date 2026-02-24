//! Control template save/load
//!
//! Handles saving and loading control configurations.

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// A control template containing all bindings for a control scheme
#[derive(Debug, Clone, Default)]
pub struct ControlTemplate {
    /// Template name
    pub name: String,
    /// Keyboard bindings: key name → target name
    pub key_bindings: HashMap<String, String>,
    /// Joystick button bindings: (joy, button) → target name
    pub joy_button_bindings: HashMap<(u32, i32), String>,
    /// Joystick axis bindings: (joy, axis, polarity) → target name
    pub joy_axis_bindings: HashMap<(u32, i32, i32), String>,
    /// Joystick hat bindings: (joy, hat, direction) → target name
    pub joy_hat_bindings: HashMap<(u32, i32, u8), String>,
}

impl ControlTemplate {
    /// Create a new empty template
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Add a keyboard binding
    pub fn add_key(&mut self, key_name: &str, target: &str) {
        self.key_bindings
            .insert(key_name.to_string(), target.to_string());
    }

    /// Add a joystick button binding
    pub fn add_joy_button(&mut self, joy: u32, button: i32, target: &str) {
        self.joy_button_bindings
            .insert((joy, button), target.to_string());
    }

    /// Add a joystick axis binding
    pub fn add_joy_axis(&mut self, joy: u32, axis: i32, polarity: i32, target: &str) {
        self.joy_axis_bindings
            .insert((joy, axis, polarity), target.to_string());
    }

    /// Add a joystick hat binding
    pub fn add_joy_hat(&mut self, joy: u32, hat: i32, direction: u8, target: &str) {
        self.joy_hat_bindings
            .insert((joy, hat, direction), target.to_string());
    }

    /// Save template to a file
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut file = fs::File::create(path)?;

        writeln!(file, "# Control template: {}", self.name)?;
        writeln!(file)?;

        // Keyboard bindings
        if !self.key_bindings.is_empty() {
            writeln!(file, "[keyboard]")?;
            for (key, target) in &self.key_bindings {
                writeln!(file, "{}={}", key, target)?;
            }
            writeln!(file)?;
        }

        // Joystick button bindings
        if !self.joy_button_bindings.is_empty() {
            writeln!(file, "[joystick.buttons]")?;
            for ((joy, button), target) in &self.joy_button_bindings {
                writeln!(file, "{}.{}={}", joy, button, target)?;
            }
            writeln!(file)?;
        }

        // Joystick axis bindings
        if !self.joy_axis_bindings.is_empty() {
            writeln!(file, "[joystick.axes]")?;
            for ((joy, axis, polarity), target) in &self.joy_axis_bindings {
                let pol_str = if *polarity < 0 { "-" } else { "+" };
                writeln!(file, "{}.{}{}={}", joy, axis, pol_str, target)?;
            }
            writeln!(file)?;
        }

        // Joystick hat bindings
        if !self.joy_hat_bindings.is_empty() {
            writeln!(file, "[joystick.hats]")?;
            for ((joy, hat, direction), target) in &self.joy_hat_bindings {
                let dir_str = match *direction {
                    1 => "up",
                    2 => "right",
                    4 => "down",
                    8 => "left",
                    _ => continue,
                };
                writeln!(file, "{}.{}.{}={}", joy, hat, dir_str, target)?;
            }
            writeln!(file)?;
        }

        Ok(())
    }

    /// Load template from a file
    pub fn load(path: &Path) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);

        let mut template = ControlTemplate::default();
        let mut section = String::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Section header
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].to_string();
                continue;
            }

            // Key=value pair
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match section.as_str() {
                    "keyboard" => {
                        template.add_key(key, value);
                    }
                    "joystick.buttons" => {
                        if let Some((joy_str, button_str)) = key.split_once('.') {
                            if let (Ok(joy), Ok(button)) =
                                (joy_str.parse::<u32>(), button_str.parse::<i32>())
                            {
                                template.add_joy_button(joy, button, value);
                            }
                        }
                    }
                    "joystick.axes" => {
                        // Format: joy.axis+/- = target
                        if let Some((joy_str, rest)) = key.split_once('.') {
                            if let Ok(joy) = joy_str.parse::<u32>() {
                                let polarity = if rest.ends_with('-') { -1 } else { 1 };
                                let axis_str = rest.trim_end_matches(['+', '-']);
                                if let Ok(axis) = axis_str.parse::<i32>() {
                                    template.add_joy_axis(joy, axis, polarity, value);
                                }
                            }
                        }
                    }
                    "joystick.hats" => {
                        // Format: joy.hat.direction = target
                        let parts: Vec<&str> = key.split('.').collect();
                        if parts.len() == 3 {
                            if let (Ok(joy), Ok(hat)) =
                                (parts[0].parse::<u32>(), parts[1].parse::<i32>())
                            {
                                let direction = match parts[2].to_lowercase().as_str() {
                                    "up" => 1,
                                    "right" => 2,
                                    "down" => 4,
                                    "left" => 8,
                                    _ => continue,
                                };
                                template.add_joy_hat(joy, hat, direction, value);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Extract name from filename
        if let Some(stem) = path.file_stem() {
            template.name = stem.to_string_lossy().to_string();
        }

        Ok(template)
    }

    /// Get the number of total bindings
    pub fn binding_count(&self) -> usize {
        self.key_bindings.len()
            + self.joy_button_bindings.len()
            + self.joy_axis_bindings.len()
            + self.joy_hat_bindings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_new_template() {
        let template = ControlTemplate::new("test");
        assert_eq!(template.name, "test");
        assert_eq!(template.binding_count(), 0);
    }

    #[test]
    fn test_add_key_binding() {
        let mut template = ControlTemplate::new("test");
        template.add_key("Space", "fire");
        template.add_key("Up", "thrust");

        assert_eq!(template.key_bindings.len(), 2);
        assert_eq!(
            template.key_bindings.get("Space"),
            Some(&"fire".to_string())
        );
    }

    #[test]
    fn test_add_joy_button_binding() {
        let mut template = ControlTemplate::new("test");
        template.add_joy_button(0, 0, "fire");
        template.add_joy_button(0, 1, "special");

        assert_eq!(template.joy_button_bindings.len(), 2);
        assert_eq!(
            template.joy_button_bindings.get(&(0, 0)),
            Some(&"fire".to_string())
        );
    }

    #[test]
    fn test_add_joy_axis_binding() {
        let mut template = ControlTemplate::new("test");
        template.add_joy_axis(0, 0, -1, "left");
        template.add_joy_axis(0, 0, 1, "right");

        assert_eq!(template.joy_axis_bindings.len(), 2);
    }

    #[test]
    fn test_add_joy_hat_binding() {
        let mut template = ControlTemplate::new("test");
        template.add_joy_hat(0, 0, 1, "up");
        template.add_joy_hat(0, 0, 4, "down");

        assert_eq!(template.joy_hat_bindings.len(), 2);
    }

    #[test]
    fn test_save_and_load() {
        let mut template = ControlTemplate::new("test_template");
        template.add_key("Space", "fire");
        template.add_key("Up", "thrust");
        template.add_joy_button(0, 0, "fire");
        template.add_joy_axis(0, 0, -1, "left");
        template.add_joy_hat(0, 0, 1, "up");

        // Save to temp file
        let file = NamedTempFile::new().unwrap();
        template.save(file.path()).unwrap();

        // Load back
        let loaded = ControlTemplate::load(file.path()).unwrap();

        assert_eq!(loaded.key_bindings.len(), 2);
        assert_eq!(loaded.key_bindings.get("Space"), Some(&"fire".to_string()));
        assert_eq!(loaded.joy_button_bindings.len(), 1);
        assert_eq!(loaded.joy_axis_bindings.len(), 1);
        assert_eq!(loaded.joy_hat_bindings.len(), 1);
    }

    #[test]
    fn test_load_with_comments() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# This is a comment").unwrap();
        writeln!(file, "[keyboard]").unwrap();
        writeln!(file, "# Another comment").unwrap();
        writeln!(file, "Space=fire").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "Up=thrust").unwrap();
        file.flush().unwrap();

        let template = ControlTemplate::load(file.path()).unwrap();
        assert_eq!(template.key_bindings.len(), 2);
    }

    #[test]
    fn test_binding_count() {
        let mut template = ControlTemplate::new("test");
        assert_eq!(template.binding_count(), 0);

        template.add_key("Space", "fire");
        assert_eq!(template.binding_count(), 1);

        template.add_joy_button(0, 0, "fire");
        assert_eq!(template.binding_count(), 2);

        template.add_joy_axis(0, 0, -1, "left");
        assert_eq!(template.binding_count(), 3);

        template.add_joy_hat(0, 0, 1, "up");
        assert_eq!(template.binding_count(), 4);
    }
}
