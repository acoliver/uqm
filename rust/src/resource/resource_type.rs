// Resource Type Definitions
// Defines strongly-typed resources for safe loading

use std::fmt;

/// Resource types supported by the game
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    String,
    Integer,
    Boolean,
    Color,
    Binary,
    Unknown,
}

impl ResourceType {
    /// Parse resource type from string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "string" => ResourceType::String,
            "integer" => ResourceType::Integer,
            "boolean" => ResourceType::Boolean,
            "color" => ResourceType::Color,
            "binary" => ResourceType::Binary,
            _ => ResourceType::Unknown,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::String => "STRING",
            ResourceType::Integer => "INTEGER",
            ResourceType::Boolean => "BOOLEAN",
            ResourceType::Color => "COLOR",
            ResourceType::Binary => "BINARY",
            ResourceType::Unknown => "UNKNOWN",
        }
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Color resource (RGBA)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorResource {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl ColorResource {
    /// Create a new color
    pub fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        ColorResource {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Create RGB color (alpha = 255)
    pub fn rgb(red: u8, green: u8, blue: u8) -> Self {
        ColorResource::new(red, green, blue, 255)
    }

    /// Parse from hex string (#RRGGBB or #RRGGBBAA)
    pub fn from_hex(hex: &str) -> Result<Self, ResourceError> {
        let hex = hex.trim_start_matches('#');

        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| ResourceError::InvalidFormat)?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| ResourceError::InvalidFormat)?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| ResourceError::InvalidFormat)?;
            Ok(ColorResource::rgb(r, g, b))
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| ResourceError::InvalidFormat)?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| ResourceError::InvalidFormat)?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| ResourceError::InvalidFormat)?;
            let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| ResourceError::InvalidFormat)?;
            Ok(ColorResource::new(r, g, b, a))
        } else {
            Err(ResourceError::InvalidFormat)
        }
    }

    /// Convert to hex string (#RRGGBBAA)
    pub fn to_hex(&self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            self.red, self.green, self.blue, self.alpha
        )
    }

    /// Convert to RGB hex string (#RRGGBB)
    pub fn to_rgb_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }
}

/// Generic resource value
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceValue {
    String(String),
    Integer(i32),
    Boolean(bool),
    Color(ColorResource),
    Binary(Vec<u8>),
}

impl ResourceValue {
    /// Get as string
    pub fn as_string(&self) -> Option<String> {
        match self {
            ResourceValue::String(s) => Some(s.clone()),
            ResourceValue::Integer(i) => Some(i.to_string()),
            ResourceValue::Boolean(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// Get as integer
    pub fn as_integer(&self) -> Option<i32> {
        match self {
            ResourceValue::Integer(i) => Some(*i),
            ResourceValue::Boolean(true) => Some(1),
            ResourceValue::Boolean(false) => Some(0),
            ResourceValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Get as boolean
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            ResourceValue::Boolean(b) => Some(*b),
            ResourceValue::Integer(i) => Some(*i != 0),
            ResourceValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" => Some(true),
                "false" | "no" | "0" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Get as color
    pub fn as_color(&self) -> Option<ColorResource> {
        match self {
            ResourceValue::Color(c) => Some(*c),
            ResourceValue::String(s) => ColorResource::from_hex(s).ok(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResourceError {
    NotFound,
    InvalidFormat,
    InvalidType,
    LoadFailed,
    CacheError,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceError::NotFound => write!(f, "Resource not found"),
            ResourceError::InvalidFormat => write!(f, "Invalid resource format"),
            ResourceError::InvalidType => write!(f, "Invalid resource type"),
            ResourceError::LoadFailed => write!(f, "Failed to load resource"),
            ResourceError::CacheError => write!(f, "Resource cache error"),
        }
    }
}

impl std::error::Error for ResourceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_from_str() {
        assert_eq!(ResourceType::from_str("string"), ResourceType::String);
        assert_eq!(ResourceType::from_str("INTEGER"), ResourceType::Integer);
        assert_eq!(ResourceType::from_str("unknown"), ResourceType::Unknown);
    }

    #[test]
    fn test_color_rgb() {
        let color = ColorResource::rgb(255, 128, 64);
        assert_eq!(color.red, 255);
        assert_eq!(color.green, 128);
        assert_eq!(color.blue, 64);
        assert_eq!(color.alpha, 255);
    }

    #[test]
    fn test_color_from_hex_rgb() {
        let color = ColorResource::from_hex("#FF8040").unwrap();
        assert_eq!(color, ColorResource::rgb(255, 128, 64));
    }

    #[test]
    fn test_color_from_hex_rgba() {
        let color = ColorResource::from_hex("#FF804080").unwrap();
        assert_eq!(color.red, 255);
        assert_eq!(color.green, 128);
        assert_eq!(color.blue, 64);
        assert_eq!(color.alpha, 128);
    }

    #[test]
    fn test_color_to_hex() {
        let color = ColorResource::rgb(255, 128, 64);
        assert_eq!(color.to_hex(), "#FF8040FF");
    }

    #[test]
    fn test_color_to_rgb_hex() {
        let color = ColorResource::rgb(255, 128, 64);
        assert_eq!(color.to_rgb_hex(), "#FF8040");
    }

    #[test]
    fn test_resource_value_as_string() {
        let value = ResourceValue::String("test".to_string());
        assert_eq!(value.as_string(), Some("test".to_string()));

        let value = ResourceValue::Integer(42);
        assert_eq!(value.as_string(), Some("42".to_string()));

        let value = ResourceValue::Boolean(true);
        assert_eq!(value.as_string(), Some("true".to_string()));
    }

    #[test]
    fn test_resource_value_as_integer() {
        let value = ResourceValue::Integer(42);
        assert_eq!(value.as_integer(), Some(42));

        let value = ResourceValue::Boolean(true);
        assert_eq!(value.as_integer(), Some(1));

        let value = ResourceValue::Boolean(false);
        assert_eq!(value.as_integer(), Some(0));

        let value = ResourceValue::String("123".to_string());
        assert_eq!(value.as_integer(), Some(123));
    }

    #[test]
    fn test_resource_value_as_boolean() {
        let value = ResourceValue::Boolean(true);
        assert_eq!(value.as_boolean(), Some(true));

        let value = ResourceValue::Integer(1);
        assert_eq!(value.as_boolean(), Some(true));

        let value = ResourceValue::Integer(0);
        assert_eq!(value.as_boolean(), Some(false));

        let value = ResourceValue::String("yes".to_string());
        assert_eq!(value.as_boolean(), Some(true));

        let value = ResourceValue::String("no".to_string());
        assert_eq!(value.as_boolean(), Some(false));
    }

    #[test]
    fn test_resource_value_as_color() {
        let color = ColorResource::rgb(255, 128, 64);
        let value = ResourceValue::Color(color);
        assert_eq!(value.as_color(), Some(color));

        let value = ResourceValue::String("#FF8040".to_string());
        assert_eq!(value.as_color(), Some(ColorResource::rgb(255, 128, 64)));
    }

    #[test]
    fn test_resource_type_display() {
        assert_eq!(format!("{}", ResourceType::String), "STRING");
        assert_eq!(format!("{}", ResourceType::Integer), "INTEGER");
    }

    #[test]
    fn test_color_partial_eq() {
        let c1 = ColorResource::rgb(255, 128, 64);
        let c2 = ColorResource::rgb(255, 128, 64);
        let c3 = ColorResource::rgb(255, 128, 65);

        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_resource_error_display() {
        let err = ResourceError::NotFound;
        assert!(format!("{}", err).contains("not found"));
    }
}
