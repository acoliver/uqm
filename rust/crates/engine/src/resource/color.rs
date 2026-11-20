//! Colour descriptors in resource files: `rgb()`, `rgba()` and `rgb15()`.

/// Parse a single integer component using C `sscanf %i` semantics:
/// `0x`/`0X` prefix → hex, `0` prefix → octal, otherwise decimal.
/// Returns the parsed value as i32 (may be negative or overflow u8 range).
fn parse_c_int(s: &str) -> Result<i32, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty component".to_string());
    }

    let (negative, s) = if let Some(rest) = s.strip_prefix('-') {
        (true, rest.trim())
    } else {
        (false, s)
    };

    let value = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).map_err(|e| format!("invalid hex: {e}"))?
    } else if s.starts_with('0') && s.len() > 1 && s[1..].chars().all(|c| c.is_ascii_digit()) {
        i64::from_str_radix(s, 8).map_err(|e| format!("invalid octal: {e}"))?
    } else {
        s.parse::<i64>()
            .map_err(|e| format!("invalid integer: {e}"))?
    };

    let value = if negative { -value } else { value };
    i32::try_from(value).map_err(|e| format!("integer overflow: {e}"))
}

/// Clamp a parsed integer to `[0, max]`, logging a warning on out-of-range values.
fn clamp_component(value: i32, max: i32, label: &str) -> u8 {
    if value < 0 {
        log::warn!("color component {} clamped from {} to 0", label, value);
        0
    } else if value > max {
        log::warn!(
            "color component {} clamped from {} to {}",
            label,
            value,
            max
        );
        max as u8
    } else {
        value as u8
    }
}

/// Convert a 5-bit color component to 8-bit using the CC5TO8 formula.
/// `CC5TO8(x) = (x << 3) | (x >> 2)`
fn cc5to8(val: u8) -> u8 {
    (val << 3) | (val >> 2)
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
/// @plan PLAN-20260224-RES-SWAP.P08
/// @requirement REQ-RES-066-074
pub fn parse_c_color(descriptor: &str) -> Result<(u8, u8, u8, u8), String> {
    let s = descriptor.trim();

    // Determine format and extract the content inside parentheses
    let (mode, inner) = if let Some(rest) = s.strip_prefix("rgba(") {
        ("rgba", rest)
    } else if let Some(rest) = s.strip_prefix("rgb15(") {
        ("rgb15", rest)
    } else if let Some(rest) = s.strip_prefix("rgb(") {
        ("rgb", rest)
    } else {
        return Err(format!("unrecognized color format: {}", s));
    };

    let inner = inner
        .strip_suffix(')')
        .ok_or_else(|| format!("missing closing parenthesis in: {}", s))?;

    let parts: Vec<&str> = inner.split(',').collect();

    match mode {
        "rgb" => {
            if parts.len() != 3 {
                return Err(format!("rgb() requires 3 components, got {}", parts.len()));
            }
            let r = clamp_component(parse_c_int(parts[0])?, 255, "r");
            let g = clamp_component(parse_c_int(parts[1])?, 255, "g");
            let b = clamp_component(parse_c_int(parts[2])?, 255, "b");
            Ok((r, g, b, 255))
        }
        "rgba" => {
            if parts.len() != 4 {
                return Err(format!("rgba() requires 4 components, got {}", parts.len()));
            }
            let r = clamp_component(parse_c_int(parts[0])?, 255, "r");
            let g = clamp_component(parse_c_int(parts[1])?, 255, "g");
            let b = clamp_component(parse_c_int(parts[2])?, 255, "b");
            let a = clamp_component(parse_c_int(parts[3])?, 255, "a");
            Ok((r, g, b, a))
        }
        "rgb15" => {
            if parts.len() != 3 {
                return Err(format!(
                    "rgb15() requires 3 components, got {}",
                    parts.len()
                ));
            }
            let r5 = clamp_component(parse_c_int(parts[0])?, 31, "r");
            let g5 = clamp_component(parse_c_int(parts[1])?, 31, "g");
            let b5 = clamp_component(parse_c_int(parts[2])?, 31, "b");
            Ok((cc5to8(r5), cc5to8(g5), cc5to8(b5), 255))
        }
        _ => unreachable!(),
    }
}

/// Serialize RGBA components to a C-style color descriptor string.
///
/// Opaque colors (alpha == 0xFF) serialize as `rgb(0xRR, 0xGG, 0xBB)`.
/// Transparent colors serialize as `rgba(0xRR, 0xGG, 0xBB, 0xAA)`.
///
/// Matches the C `ColorToString` function output format.
///
/// @plan PLAN-20260224-RES-SWAP.P08
/// @requirement REQ-RES-066-074
pub fn serialize_color(r: u8, g: u8, b: u8, a: u8) -> String {
    if a == 255 {
        format!("rgb(0x{:02x}, 0x{:02x}, 0x{:02x})", r, g, b)
    } else {
        format!("rgba(0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x})", r, g, b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_c_color_rgb_decimal() {
        let (r, g, b, a) = parse_c_color("rgb(255, 128, 64)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    fn test_parse_c_color_rgb_hex() {
        let (r, g, b, a) = parse_c_color("rgb(0xff, 0x80, 0x40)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    fn test_parse_c_color_rgb_hex_uppercase() {
        let (r, g, b, a) = parse_c_color("rgb(0xFF, 0x80, 0x40)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    fn test_parse_c_color_rgb_mixed() {
        let (r, g, b, a) = parse_c_color("rgb(0x1a, 0, 0x1a)").unwrap();
        assert_eq!((r, g, b, a), (26, 0, 26, 255));
    }

    #[test]
    fn test_parse_c_color_rgba() {
        let (r, g, b, a) = parse_c_color("rgba(255, 128, 64, 200)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 200));
    }

    #[test]
    fn test_parse_c_color_rgba_hex() {
        let (r, g, b, a) = parse_c_color("rgba(0xff, 0x80, 0x40, 0xc8)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 200));
    }

    #[test]
    fn test_parse_c_color_rgb15() {
        // CC5TO8(31) = (31 << 3) | (31 >> 2) = 248 | 7 = 255
        // CC5TO8(16) = (16 << 3) | (16 >> 2) = 128 | 4 = 132
        // CC5TO8(0)  = (0 << 3)  | (0 >> 2)  = 0
        let (r, g, b, a) = parse_c_color("rgb15(31, 16, 0)").unwrap();
        assert_eq!((r, g, b, a), (255, 132, 0, 255));
    }

    #[test]
    fn test_parse_c_color_rgb15_mid() {
        // CC5TO8(15) = (15 << 3) | (15 >> 2) = 120 | 3 = 123
        let (r, g, b, a) = parse_c_color("rgb15(15, 15, 15)").unwrap();
        assert_eq!((r, g, b, a), (123, 123, 123, 255));
    }

    #[test]
    fn test_parse_c_color_rgb15_zero() {
        let (r, g, b, a) = parse_c_color("rgb15(0, 0, 0)").unwrap();
        assert_eq!((r, g, b, a), (0, 0, 0, 255));
    }

    #[test]
    fn test_parse_c_color_clamp_high() {
        let (r, g, b, a) = parse_c_color("rgb(300, 128, 64)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    fn test_parse_c_color_clamp_negative() {
        let (r, g, b, a) = parse_c_color("rgb(-1, 128, 64)").unwrap();
        assert_eq!((r, g, b, a), (0, 128, 64, 255));
    }

    #[test]
    fn test_parse_c_color_clamp_rgb15() {
        // Clamped to 31, then CC5TO8(31) = 255
        // CC5TO8(16) = 132, CC5TO8(0) = 0
        let (r, g, b, a) = parse_c_color("rgb15(40, 16, 0)").unwrap();
        assert_eq!((r, g, b, a), (255, 132, 0, 255));
    }

    #[test]
    fn test_parse_c_color_whitespace() {
        let (r, g, b, a) = parse_c_color("rgb( 255 , 128 , 64 )").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    fn test_parse_c_color_octal() {
        // C %i: leading 0 = octal: 010=8, 020=16, 030=24
        let (r, g, b, a) = parse_c_color("rgb(010, 020, 030)").unwrap();
        assert_eq!((r, g, b, a), (8, 16, 24, 255));
    }

    #[test]
    fn test_parse_c_color_rgb_octal_full() {
        // C %i: 0377 = 255, 0200 = 128, 0100 = 64
        let (r, g, b, a) = parse_c_color("rgb(0377, 0200, 0100)").unwrap();
        assert_eq!((r, g, b, a), (255, 128, 64, 255));
    }

    #[test]
    fn test_parse_c_color_invalid() {
        assert!(parse_c_color("notacolor").is_err());
    }

    #[test]
    fn test_parse_c_color_invalid_hex_format() {
        assert!(parse_c_color("#FF0000").is_err());
    }

    #[test]
    fn test_parse_c_color_empty() {
        assert!(parse_c_color("").is_err());
    }

    #[test]
    fn test_serialize_color_opaque() {
        assert_eq!(serialize_color(255, 128, 64, 255), "rgb(0xff, 0x80, 0x40)");
    }

    #[test]
    fn test_serialize_color_transparent() {
        assert_eq!(
            serialize_color(255, 128, 64, 200),
            "rgba(0xff, 0x80, 0x40, 0xc8)"
        );
    }

    #[test]
    fn test_serialize_color_zero_alpha() {
        assert_eq!(serialize_color(0, 0, 0, 0), "rgba(0x00, 0x00, 0x00, 0x00)");
    }

    #[test]
    fn test_serialize_color_black_opaque() {
        assert_eq!(serialize_color(0, 0, 0, 255), "rgb(0x00, 0x00, 0x00)");
    }

    #[test]
    fn test_serialize_color_roundtrip_rgb() {
        let input = "rgb(0x1a, 0x00, 0x1a)";
        let (r, g, b, a) = parse_c_color(input).unwrap();
        assert_eq!(serialize_color(r, g, b, a), input);
    }

    #[test]
    fn test_serialize_color_roundtrip_rgba() {
        let input = "rgba(0xff, 0x00, 0x00, 0x80)";
        let (r, g, b, a) = parse_c_color(input).unwrap();
        assert_eq!(serialize_color(r, g, b, a), input);
    }
}
