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
    #[deprecated(note = "Use parse_c_color() for .rmp color format (rgb/rgba/rgb15)")]
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

/// Parse a C-style color descriptor string into RGBA components.
///
/// Supports formats matching the C `DescriptorToColor` function:
/// - `rgb(r, g, b)` — 8-bit components, alpha defaults to 0xFF
/// - `rgba(r, g, b, a)` — 8-bit components
/// - `rgb15(r, g, b)` — 5-bit components converted via CC5TO8: `(x << 3) | (x >> 2)`
///
/// Components accept C `%i` format: decimal, hex (`0x`), or octal (`0` prefix).
/// Values are clamped: 0–255 for 8-bit, 0–31 for 5-bit.
///
/// @plan PLAN-20260224-RES-SWAP.P06
/// @requirement REQ-RES-066-074
pub fn parse_c_color(descriptor: &str) -> Result<(u8, u8, u8, u8), String> {
    todo!("P06 stub: implement C-style color parsing")
}

/// Serialize RGBA components to a C-style color descriptor string.
///
/// Opaque colors (alpha == 0xFF) serialize as `rgb(0xRR, 0xGG, 0xBB)`.
/// Transparent colors serialize as `rgba(0xRR, 0xGG, 0xBB, 0xAA)`.
///
/// Matches the C `ColorToString` function output format.
///
/// @plan PLAN-20260224-RES-SWAP.P06
/// @requirement REQ-RES-066-074
pub fn serialize_color(r: u8, g: u8, b: u8, a: u8) -> String {
    todo!("P06 stub: implement C-style color serialization")
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
            #[allow(deprecated)]
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
    #[allow(deprecated)]
    fn test_color_from_hex_rgb() {
        let color = ColorResource::from_hex("#FF8040").unwrap();
        assert_eq!(color, ColorResource::rgb(255, 128, 64));
    }

    #[test]
    #[allow(deprecated)]
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
    #[allow(deprecated)]
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

    // --- P07: Color Parser TDD Tests (RED) ---
    // @plan PLAN-20260224-RES-SWAP.P07
    // @requirement REQ-RES-066-074
    //
    // All tests are #[ignore] because parse_c_color/serialize_color are todo!() stubs.
    // These will be un-ignored when the implementation is written.

    #[test]
    #[ignore]
    fn test_parse_c_color_rgb_decimal() {
        let (r, g, b, a) = parse_c_color("rgb(255, 128, 64)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_rgb_hex() {
        let (r, g, b, a) = parse_c_color("rgb(0xff, 0x80, 0x40)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_rgb_hex_uppercase() {
        let (r, g, b, a) = parse_c_color("rgb(0xFF, 0x80, 0x40)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_rgb_mixed() {
        let (r, g, b, a) = parse_c_color("rgb(0x1a, 0, 0x1a)").unwrap();
        assert_eq!((r, g, b, a), (26, 0, 26, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_rgba() {
        let (r, g, b, a) = parse_c_color("rgba(255, 128, 64, 200)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 200));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_rgba_hex() {
        let (r, g, b, a) = parse_c_color("rgba(0xff, 0x80, 0x40, 0xc8)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 200));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_rgb15() {
        // CC5TO8(31) = (31 << 3) | (31 >> 2) = 248 | 7 = 255
        // CC5TO8(16) = (16 << 3) | (16 >> 2) = 128 | 4 = 132
        // CC5TO8(0)  = (0 << 3)  | (0 >> 2)  = 0
        let (r, g, b, a) = parse_c_color("rgb15(31, 16, 0)").unwrap();
        assert_eq!((r, g, b, a), (255, 132, 0, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_rgb15_mid() {
        // CC5TO8(15) = (15 << 3) | (15 >> 2) = 120 | 3 = 123
        let (r, g, b, a) = parse_c_color("rgb15(15, 15, 15)").unwrap();
        assert_eq!((r, g, b, a), (123, 123, 123, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_rgb15_zero() {
        let (r, g, b, a) = parse_c_color("rgb15(0, 0, 0)").unwrap();
        assert_eq!((r, g, b, a), (0, 0, 0, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_clamp_high() {
        let (r, g, b, a) = parse_c_color("rgb(300, 128, 64)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_clamp_negative() {
        let (r, g, b, a) = parse_c_color("rgb(-1, 128, 64)").unwrap();
        assert_eq!((r, g, b, a), (0, 128, 64, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_clamp_rgb15() {
        // Clamped to 31, then CC5TO8(31) = 255
        // CC5TO8(16) = 132, CC5TO8(0) = 0
        let (r, g, b, a) = parse_c_color("rgb15(40, 16, 0)").unwrap();
        assert_eq!((r, g, b, a), (255, 132, 0, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_whitespace() {
        let (r, g, b, a) = parse_c_color("rgb( 255 , 128 , 64 )").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_octal() {
        // C %i: leading 0 = octal: 010=8, 020=16, 030=24
        let (r, g, b, a) = parse_c_color("rgb(010, 020, 030)").unwrap();
        assert_eq!((r, g, b, a), (8, 16, 24, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_rgb_octal_full() {
        // C %i: 0377 = 255, 0200 = 128, 0100 = 64
        let (r, g, b, a) = parse_c_color("rgb(0377, 0200, 0100)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_invalid() {
        assert!(parse_c_color("notacolor").is_err());
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_invalid_hex_format() {
        assert!(parse_c_color("#FF0000").is_err());
    }

    #[test]
    #[ignore]
    fn test_parse_c_color_empty() {
        assert!(parse_c_color("").is_err());
    }

    #[test]
    #[ignore]
    fn test_serialize_color_opaque() {
        assert_eq!(serialize_color(255, 128, 64, 255), "rgb(0xff, 0x80, 0x40)");
    }

    #[test]
    #[ignore]
    fn test_serialize_color_transparent() {
        assert_eq!(
            serialize_color(255, 128, 64, 200),
            "rgba(0xff, 0x80, 0x40, 0xc8)"
        );
    }

    #[test]
    #[ignore]
    fn test_serialize_color_zero_alpha() {
        assert_eq!(
            serialize_color(0, 0, 0, 0),
            "rgba(0x00, 0x00, 0x00, 0x00)"
        );
    }

    #[test]
    #[ignore]
    fn test_serialize_color_black_opaque() {
        assert_eq!(serialize_color(0, 0, 0, 255), "rgb(0x00, 0x00, 0x00)");
    }

    #[test]
    #[ignore]
    fn test_serialize_color_roundtrip_rgb() {
        let input = "rgb(0x1a, 0x00, 0x1a)";
        let (r, g, b, a) = parse_c_color(input).unwrap();
        assert_eq!(serialize_color(r, g, b, a), input);
    }

    #[test]
    #[ignore]
    fn test_serialize_color_roundtrip_rgba() {
        let input = "rgba(0xff, 0x00, 0x00, 0x80)";
        let (r, g, b, a) = parse_c_color(input).unwrap();
        assert_eq!(serialize_color(r, g, b, a), input);
    }
}
