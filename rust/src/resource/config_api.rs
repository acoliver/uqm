//! Config API for resource Put/Get operations and index serialization
//!
//! Provides typed Put/Get functions (string, integer, boolean, color) and
//! SaveResourceIndex serialization matching the C `resinit.c` config functions.
//!
//! @plan PLAN-20260224-RES-SWAP.P09
//! @requirement REQ-RES-047-065

use std::collections::HashMap;

use super::resource_type::serialize_color;

/// Descriptor for a resource entry in the config map.
///
/// Mirrors the C `ResourceDesc` conceptually, but uses safe Rust types.
/// The `#[repr(C)]` FFI representation comes in later phases.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-051-056
pub struct ResourceDesc {
    /// File path / value string (from TYPE:path)
    pub fname: String,
    /// Type name: "STRING", "INT32", "BOOLEAN", "COLOR"
    pub res_type: String,
    /// Numeric data (INT32, BOOLEAN, COLOR packed RGBA)
    pub data_num: u32,
    /// String data (STRING type)
    pub data_str: Option<String>,
}

/// Map of resource key → descriptor
pub type ResourceMap = HashMap<String, ResourceDesc>;

/// Create or update a STRING entry in the resource map.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-051
pub fn put_string(map: &mut ResourceMap, key: &str, value: &str) {
    let desc = ResourceDesc {
        fname: value.to_string(),
        res_type: "STRING".to_string(),
        data_num: 0,
        data_str: Some(value.to_string()),
    };
    map.insert(key.to_string(), desc);
}

/// Create or update an INT32 entry in the resource map.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-052
pub fn put_integer(map: &mut ResourceMap, key: &str, value: i32) {
    let desc = ResourceDesc {
        fname: value.to_string(),
        res_type: "INT32".to_string(),
        data_num: value as u32,
        data_str: None,
    };
    map.insert(key.to_string(), desc);
}

/// Create or update a BOOLEAN entry in the resource map.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-053
pub fn put_boolean(map: &mut ResourceMap, key: &str, value: bool) {
    let desc = ResourceDesc {
        fname: if value { "true" } else { "false" }.to_string(),
        res_type: "BOOLEAN".to_string(),
        data_num: if value { 1 } else { 0 },
        data_str: None,
    };
    map.insert(key.to_string(), desc);
}

/// Create or update a COLOR entry in the resource map.
///
/// Color is packed as `(r << 24) | (g << 16) | (b << 8) | a` in data_num,
/// matching the C representation.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-054
pub fn put_color(map: &mut ResourceMap, key: &str, r: u8, g: u8, b: u8, a: u8) {
    let packed = ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32);
    let desc = ResourceDesc {
        fname: serialize_color(r, g, b, a),
        res_type: "COLOR".to_string(),
        data_num: packed,
        data_str: None,
    };
    map.insert(key.to_string(), desc);
}

/// Get a STRING value from the resource map.
///
/// Returns `None` if the key doesn't exist or isn't a STRING type.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-047
pub fn get_string<'a>(map: &'a ResourceMap, key: &str) -> Option<&'a str> {
    let desc = map.get(key)?;
    if desc.res_type != "STRING" {
        return None;
    }
    desc.data_str.as_deref()
}

/// Get an INT32 value from the resource map.
///
/// Returns `None` if the key doesn't exist or isn't an INT32 type.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-048
pub fn get_integer(map: &ResourceMap, key: &str) -> Option<i32> {
    let desc = map.get(key)?;
    if desc.res_type != "INT32" {
        return None;
    }
    Some(desc.data_num as i32)
}

/// Get a BOOLEAN value from the resource map.
///
/// Returns `None` if the key doesn't exist or isn't a BOOLEAN type.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-049
pub fn get_boolean(map: &ResourceMap, key: &str) -> Option<bool> {
    let desc = map.get(key)?;
    if desc.res_type != "BOOLEAN" {
        return None;
    }
    Some(desc.data_num != 0)
}

/// Get a COLOR value from the resource map as (r, g, b, a).
///
/// Returns `None` if the key doesn't exist or isn't a COLOR type.
/// Unpacks from `(r << 24) | (g << 16) | (b << 8) | a`.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-050
pub fn get_color(map: &ResourceMap, key: &str) -> Option<(u8, u8, u8, u8)> {
    let desc = map.get(key)?;
    if desc.res_type != "COLOR" {
        return None;
    }
    let packed = desc.data_num;
    let r = ((packed >> 24) & 0xFF) as u8;
    let g = ((packed >> 16) & 0xFF) as u8;
    let b = ((packed >> 8) & 0xFF) as u8;
    let a = (packed & 0xFF) as u8;
    Some((r, g, b, a))
}

/// Serialize a resource descriptor to `TYPE:value` format.
///
/// Uses `serialize_color()` for COLOR entries; for other types, uses `fname`.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-060
pub fn serialize_entry(desc: &ResourceDesc) -> String {
    match desc.res_type.as_str() {
        "STRING" => {
            let val = desc.data_str.as_deref().unwrap_or(&desc.fname);
            format!("STRING:{}", val)
        }
        "INT32" => format!("INT32:{}", desc.data_num as i32),
        "BOOLEAN" => {
            let val = if desc.data_num != 0 { "true" } else { "false" };
            format!("BOOLEAN:{}", val)
        }
        "COLOR" => {
            let packed = desc.data_num;
            let r = ((packed >> 24) & 0xFF) as u8;
            let g = ((packed >> 16) & 0xFF) as u8;
            let b = ((packed >> 8) & 0xFF) as u8;
            let a = (packed & 0xFF) as u8;
            format!("COLOR:{}", serialize_color(r, g, b, a))
        }
        _ => format!("{}:{}", desc.res_type, desc.fname),
    }
}

/// Generate the content of a resource index file from the map.
///
/// Filters entries by `root` prefix (if given), optionally strips the prefix
/// from keys, and serializes each entry as `key = TYPE:value\n`.
///
/// Entries are sorted by key for deterministic output.
///
/// @plan PLAN-20260224-RES-SWAP.P09
/// @requirement REQ-RES-060-065
pub fn save_resource_index(map: &ResourceMap, root: Option<&str>, strip_root: bool) -> String {
    let mut lines: Vec<String> = Vec::new();

    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(prefix) = root {
            if !key.starts_with(prefix) {
                continue;
            }
        }

        let desc = &map[key];
        let serialized = serialize_entry(desc);

        let output_key = if strip_root {
            if let Some(prefix) = root {
                key.strip_prefix(prefix).unwrap_or(key)
            } else {
                key.as_str()
            }
        } else {
            key.as_str()
        };

        lines.push(format!("{} = {}", output_key, serialized));
    }

    if lines.is_empty() {
        String::new()
    } else {
        let mut result = lines.join("\n");
        result.push('\n');
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- P10: Put/Get roundtrip tests ---
    // @plan PLAN-20260224-RES-SWAP.P10

    #[test]
    fn test_put_get_string() {
        let mut map = ResourceMap::new();
        put_string(&mut map, "test.key", "hello world");
        assert_eq!(get_string(&map, "test.key"), Some("hello world"));
    }

    #[test]
    fn test_put_get_integer() {
        let mut map = ResourceMap::new();
        put_integer(&mut map, "test.num", 42);
        assert_eq!(get_integer(&map, "test.num"), Some(42));
    }

    #[test]
    fn test_put_get_boolean() {
        let mut map = ResourceMap::new();
        put_boolean(&mut map, "test.flag", true);
        assert_eq!(get_boolean(&map, "test.flag"), Some(true));
        put_boolean(&mut map, "test.flag2", false);
        assert_eq!(get_boolean(&map, "test.flag2"), Some(false));
    }

    #[test]
    fn test_put_get_color() {
        let mut map = ResourceMap::new();
        put_color(&mut map, "test.color", 0x1a, 0x00, 0x1a, 0xff);
        assert_eq!(
            get_color(&map, "test.color"),
            Some((0x1a, 0x00, 0x1a, 0xff))
        );
    }

    #[test]
    fn test_put_get_color_with_alpha() {
        let mut map = ResourceMap::new();
        put_color(&mut map, "test.color", 0xff, 0x00, 0x00, 0x80);
        assert_eq!(
            get_color(&map, "test.color"),
            Some((0xff, 0x00, 0x00, 0x80))
        );
    }

    #[test]
    fn test_put_overwrites() {
        let mut map = ResourceMap::new();
        put_string(&mut map, "key", "first");
        assert_eq!(get_string(&map, "key"), Some("first"));
        put_string(&mut map, "key", "second");
        assert_eq!(get_string(&map, "key"), Some("second"));
    }

    #[test]
    fn test_put_auto_creates() {
        let mut map = ResourceMap::new();
        assert_eq!(get_string(&map, "nonexistent"), None);
        put_string(&mut map, "nonexistent", "value");
        assert_eq!(get_string(&map, "nonexistent"), Some("value"));
    }

    #[test]
    fn test_put_type_change() {
        let mut map = ResourceMap::new();
        put_integer(&mut map, "key", 42);
        assert_eq!(get_integer(&map, "key"), Some(42));
        assert_eq!(get_string(&map, "key"), None);

        put_string(&mut map, "key", "now a string");
        assert_eq!(get_string(&map, "key"), Some("now a string"));
        assert_eq!(get_integer(&map, "key"), None);
    }

    #[test]
    fn test_get_missing() {
        let map = ResourceMap::new();
        assert_eq!(get_string(&map, "nope"), None);
        assert_eq!(get_integer(&map, "nope"), None);
        assert_eq!(get_boolean(&map, "nope"), None);
        assert_eq!(get_color(&map, "nope"), None);
    }

    // --- P10: Serialization tests ---

    #[test]
    fn test_serialize_string() {
        let mut map = ResourceMap::new();
        put_string(&mut map, "k", "value");
        let desc = &map["k"];
        assert_eq!(serialize_entry(desc), "STRING:value");
    }

    #[test]
    fn test_serialize_integer() {
        let mut map = ResourceMap::new();
        put_integer(&mut map, "k", 42);
        let desc = &map["k"];
        assert_eq!(serialize_entry(desc), "INT32:42");
    }

    #[test]
    fn test_serialize_boolean_true() {
        let mut map = ResourceMap::new();
        put_boolean(&mut map, "k", true);
        let desc = &map["k"];
        assert_eq!(serialize_entry(desc), "BOOLEAN:true");
    }

    #[test]
    fn test_serialize_boolean_false() {
        let mut map = ResourceMap::new();
        put_boolean(&mut map, "k", false);
        let desc = &map["k"];
        assert_eq!(serialize_entry(desc), "BOOLEAN:false");
    }

    #[test]
    fn test_serialize_color_opaque() {
        let mut map = ResourceMap::new();
        put_color(&mut map, "k", 0xff, 0x80, 0x40, 0xff);
        let desc = &map["k"];
        assert_eq!(serialize_entry(desc), "COLOR:rgb(0xff, 0x80, 0x40)");
    }

    #[test]
    fn test_serialize_color_transparent() {
        let mut map = ResourceMap::new();
        put_color(&mut map, "k", 0xff, 0x80, 0x40, 0xc8);
        let desc = &map["k"];
        assert_eq!(serialize_entry(desc), "COLOR:rgba(0xff, 0x80, 0x40, 0xc8)");
    }

    // --- P10: SaveResourceIndex tests ---

    #[test]
    fn test_save_resource_index_basic() {
        let mut map = ResourceMap::new();
        put_integer(&mut map, "config.sfxvol", 20);
        put_boolean(&mut map, "config.fullscreen", true);
        put_string(&mut map, "config.scaler", "no");

        let output = save_resource_index(&map, None, false);
        assert!(output.contains("config.fullscreen = BOOLEAN:true"));
        assert!(output.contains("config.scaler = STRING:no"));
        assert!(output.contains("config.sfxvol = INT32:20"));
    }

    #[test]
    fn test_save_resource_index_root_filter() {
        let mut map = ResourceMap::new();
        put_integer(&mut map, "config.a", 1);
        put_integer(&mut map, "keys.b", 2);

        let output = save_resource_index(&map, Some("config."), false);
        assert!(output.contains("config.a"));
        assert!(!output.contains("keys.b"));
    }

    #[test]
    fn test_save_resource_index_strip_root() {
        let mut map = ResourceMap::new();
        put_integer(&mut map, "config.sfxvol", 20);
        put_boolean(&mut map, "config.fullscreen", true);

        let output = save_resource_index(&map, Some("config."), true);
        assert!(output.contains("sfxvol = INT32:20"));
        assert!(output.contains("fullscreen = BOOLEAN:true"));
        assert!(!output.contains("config.sfxvol"));
        assert!(!output.contains("config.fullscreen"));
    }

    #[test]
    fn test_save_resource_index_empty() {
        let map = ResourceMap::new();
        let output = save_resource_index(&map, None, false);
        assert_eq!(output, "");
    }

    #[test]
    fn test_save_resource_index_sorted() {
        let mut map = ResourceMap::new();
        put_string(&mut map, "z.key", "z");
        put_string(&mut map, "a.key", "a");
        put_string(&mut map, "m.key", "m");

        let output = save_resource_index(&map, None, false);
        let lines: Vec<&str> = output.trim().lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("a.key"));
        assert!(lines[1].starts_with("m.key"));
        assert!(lines[2].starts_with("z.key"));
    }

    #[test]
    fn test_save_resource_index_color_format() {
        let mut map = ResourceMap::new();
        put_color(&mut map, "test.color", 0x1a, 0x00, 0x1a, 0xff);

        let output = save_resource_index(&map, None, false);
        assert!(output.contains("COLOR:rgb(0x1a, 0x00, 0x1a)"));
    }

    #[test]
    fn test_save_resource_index_color_alpha_format() {
        let mut map = ResourceMap::new();
        put_color(&mut map, "test.color", 0xff, 0x00, 0x00, 0x80);

        let output = save_resource_index(&map, None, false);
        assert!(output.contains("COLOR:rgba(0xff, 0x00, 0x00, 0x80)"));
    }

    #[test]
    fn test_put_negative_integer() {
        let mut map = ResourceMap::new();
        put_integer(&mut map, "key", -1);
        assert_eq!(get_integer(&map, "key"), Some(-1));
    }

    #[test]
    fn test_serialize_negative_integer() {
        let mut map = ResourceMap::new();
        put_integer(&mut map, "k", -42);
        let desc = &map["k"];
        assert_eq!(serialize_entry(desc), "INT32:-42");
    }

    #[test]
    fn test_get_wrong_type_returns_none() {
        let mut map = ResourceMap::new();
        put_integer(&mut map, "num", 42);
        assert_eq!(get_string(&map, "num"), None);
        assert_eq!(get_boolean(&map, "num"), None);
        assert_eq!(get_color(&map, "num"), None);

        put_string(&mut map, "str", "hello");
        assert_eq!(get_integer(&map, "str"), None);
        assert_eq!(get_boolean(&map, "str"), None);
        assert_eq!(get_color(&map, "str"), None);
    }
}
