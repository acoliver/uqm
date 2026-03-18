// Resource System Tests
// Comprehensive tests for property files, string banks, resource system,
// resource index parsing, caching, and reference counting.

#[cfg(test)]
#[allow(deprecated)]
mod propfile_tests {
    use crate::resource::propfile::{PropertyError, PropertyFile};
    use std::env;

    #[test]
    fn test_propfile_parse_simple() {
        // Basic key=value parsing
        let content = "KEY=VALUE";
        let pf = PropertyFile::from_string(content).unwrap();
        assert_eq!(pf.get("KEY"), Some(&"VALUE".to_string()));
    }

    #[test]
    fn test_propfile_parse_with_spaces() {
        // Whitespace around key and value should be trimmed
        let content = "  KEY  =  VALUE  ";
        let pf = PropertyFile::from_string(content).unwrap();
        assert_eq!(pf.get("KEY"), Some(&"VALUE".to_string()));
    }

    #[test]
    fn test_propfile_parse_multiple_entries() {
        let content = r#"
KEY1=VALUE1
KEY2=VALUE2
KEY3=VALUE3
"#;
        let pf = PropertyFile::from_string(content).unwrap();
        assert_eq!(pf.len(), 3);
        assert_eq!(pf.get("KEY1"), Some(&"VALUE1".to_string()));
        assert_eq!(pf.get("KEY2"), Some(&"VALUE2".to_string()));
        assert_eq!(pf.get("KEY3"), Some(&"VALUE3".to_string()));
    }

    #[test]
    fn test_propfile_parse_comments() {
        // Lines starting with # are comments and should be ignored
        let content = r#"
# This is a comment
KEY1=VALUE1
# Another comment
KEY2=VALUE2
"#;
        let pf = PropertyFile::from_string(content).unwrap();
        assert_eq!(pf.len(), 2);
        assert_eq!(pf.get("KEY1"), Some(&"VALUE1".to_string()));
        assert_eq!(pf.get("KEY2"), Some(&"VALUE2".to_string()));
    }

    #[test]
    fn test_propfile_parse_empty_lines() {
        // Empty lines should be ignored
        let content = r#"
KEY1=VALUE1

KEY2=VALUE2


KEY3=VALUE3
"#;
        let pf = PropertyFile::from_string(content).unwrap();
        assert_eq!(pf.len(), 3);
    }

    #[test]
    fn test_propfile_parse_sections() {
        // The Rust implementation doesn't have native section support like INI files,
        // but keys with prefixes can simulate sections (e.g., SECTION_KEY)
        let content = r#"
MENU_TITLE=Main Menu
MENU_START=Start Game
MENU_QUIT=Quit Game
SETTINGS_VOLUME=80
SETTINGS_FULLSCREEN=true
"#;
        let pf = PropertyFile::from_string(content).unwrap();

        // Test that section-prefixed keys work
        let menu_keys = pf.get_keys_with_prefix("MENU_");
        assert_eq!(menu_keys.len(), 3);

        let settings_keys = pf.get_keys_with_prefix("SETTINGS_");
        assert_eq!(settings_keys.len(), 2);
    }

    #[test]
    fn test_propfile_get_value() {
        let mut pf = PropertyFile::default();
        pf.set("KEY", "VALUE");

        assert_eq!(pf.get("KEY"), Some(&"VALUE".to_string()));
    }

    #[test]
    fn test_propfile_get_value_case_insensitive() {
        // Keys should be case-insensitive
        let mut pf = PropertyFile::default();
        pf.set("MyKey", "VALUE");

        assert_eq!(pf.get("mykey"), Some(&"VALUE".to_string()));
        assert_eq!(pf.get("MYKEY"), Some(&"VALUE".to_string()));
        assert_eq!(pf.get("MyKey"), Some(&"VALUE".to_string()));
    }

    #[test]
    fn test_propfile_missing_key() {
        let pf = PropertyFile::default();

        assert_eq!(pf.get("NONEXISTENT"), None);
    }

    #[test]
    fn test_propfile_get_or_default() {
        let mut pf = PropertyFile::default();
        pf.set("KEY", "VALUE");

        assert_eq!(pf.get_or("KEY", "DEFAULT"), "VALUE");
        assert_eq!(pf.get_or("MISSING", "DEFAULT"), "DEFAULT");
    }

    #[test]
    fn test_propfile_get_int() {
        let mut pf = PropertyFile::default();
        pf.set("NUM", "42");
        pf.set("NEGATIVE", "-100");
        pf.set("INVALID", "not_a_number");

        assert_eq!(pf.get_int("NUM"), Some(42));
        assert_eq!(pf.get_int("NEGATIVE"), Some(-100));
        assert_eq!(pf.get_int("INVALID"), None);
        assert_eq!(pf.get_int("MISSING"), None);
    }

    #[test]
    fn test_propfile_get_bool() {
        let mut pf = PropertyFile::default();
        pf.set("TRUE_VAL", "true");
        pf.set("FALSE_VAL", "false");
        pf.set("INVALID", "maybe");

        assert_eq!(pf.get_bool("TRUE_VAL"), Some(true));
        assert_eq!(pf.get_bool("FALSE_VAL"), Some(false));
        assert_eq!(pf.get_bool("INVALID"), None);
        assert_eq!(pf.get_bool("MISSING"), None);
    }

    #[test]
    fn test_propfile_value_with_equals() {
        // Values can contain = characters
        let content = "FORMULA=a=b+c";
        let pf = PropertyFile::from_string(content).unwrap();
        assert_eq!(pf.get("FORMULA"), Some(&"a=b+c".to_string()));
    }

    #[test]
    fn test_propfile_special_characters_in_value() {
        // Values can contain special characters
        let content = r#"
MESSAGE=Hello, World!
PATH=/usr/local/bin
PERCENT=100%
"#;
        let pf = PropertyFile::from_string(content).unwrap();
        assert_eq!(pf.get("MESSAGE"), Some(&"Hello, World!".to_string()));
        assert_eq!(pf.get("PATH"), Some(&"/usr/local/bin".to_string()));
        assert_eq!(pf.get("PERCENT"), Some(&"100%".to_string()));
    }

    #[test]
    fn test_propfile_load_file_not_found() {
        let result = PropertyFile::load("/nonexistent/path/to/file.txt");
        assert_eq!(result, Err(PropertyError::FileNotFound));
    }

    #[test]
    fn test_propfile_save_and_load() {
        let mut pf = PropertyFile::default();
        pf.set("KEY1", "VALUE1");
        pf.set("KEY2", "VALUE2");

        let path = env::temp_dir().join("test_propfile_roundtrip.txt");
        pf.save(&path).unwrap();

        let loaded = PropertyFile::load(&path).unwrap();
        assert_eq!(loaded.get("KEY1"), Some(&"VALUE1".to_string()));
        assert_eq!(loaded.get("KEY2"), Some(&"VALUE2".to_string()));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_propfile_merge() {
        let mut pf1 = PropertyFile::default();
        let mut pf2 = PropertyFile::default();

        pf1.set("KEY1", "VALUE1");
        pf2.set("KEY2", "VALUE2");
        pf2.set("KEY1", "OVERWRITTEN"); // This should overwrite KEY1 in pf1

        pf1.merge(&pf2);

        assert_eq!(pf1.len(), 2);
        assert_eq!(pf1.get("KEY1"), Some(&"OVERWRITTEN".to_string()));
        assert_eq!(pf1.get("KEY2"), Some(&"VALUE2".to_string()));
    }

    #[test]
    fn test_propfile_remove() {
        let mut pf = PropertyFile::default();
        pf.set("KEY", "VALUE");
        assert!(pf.contains("KEY"));

        let removed = pf.remove("KEY");
        assert_eq!(removed, Some("VALUE".to_string()));
        assert!(!pf.contains("KEY"));
    }

    #[test]
    fn test_propfile_clear() {
        let mut pf = PropertyFile::default();
        pf.set("KEY1", "VALUE1");
        pf.set("KEY2", "VALUE2");
        assert_eq!(pf.len(), 2);

        pf.clear();
        assert!(pf.is_empty());
    }

    #[test]
    fn test_propfile_error_display() {
        let err = PropertyError::FileNotFound;
        let msg = format!("{}", err);
        assert!(msg.contains("not found"));

        let err = PropertyError::InvalidFormat;
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid"));

        let err = PropertyError::IoError;
        let msg = format!("{}", err);
        assert!(msg.contains("I/O"));
    }
}

#[cfg(test)]
mod resource_type_tests {
    use crate::resource::{ColorResource, ResourceError, ResourceType, ResourceValue};

    #[test]
    fn test_resource_type_from_str() {
        assert_eq!(ResourceType::from_str("string"), ResourceType::String);
        assert_eq!(ResourceType::from_str("STRING"), ResourceType::String);
        assert_eq!(ResourceType::from_str("integer"), ResourceType::Integer);
        assert_eq!(ResourceType::from_str("INTEGER"), ResourceType::Integer);
        assert_eq!(ResourceType::from_str("boolean"), ResourceType::Boolean);
        assert_eq!(ResourceType::from_str("color"), ResourceType::Color);
        assert_eq!(ResourceType::from_str("binary"), ResourceType::Binary);
        assert_eq!(ResourceType::from_str("unknown_type"), ResourceType::Unknown);
    }

    #[test]
    fn test_resource_type_as_str() {
        assert_eq!(ResourceType::String.as_str(), "STRING");
        assert_eq!(ResourceType::Integer.as_str(), "INTEGER");
        assert_eq!(ResourceType::Boolean.as_str(), "BOOLEAN");
        assert_eq!(ResourceType::Color.as_str(), "COLOR");
        assert_eq!(ResourceType::Binary.as_str(), "BINARY");
        assert_eq!(ResourceType::Unknown.as_str(), "UNKNOWN");
    }

    #[test]
    fn test_color_resource_rgb() {
        let color = ColorResource::rgb(255, 128, 64);
        assert_eq!(color.red, 255);
        assert_eq!(color.green, 128);
        assert_eq!(color.blue, 64);
        assert_eq!(color.alpha, 255); // Default alpha
    }

    #[test]
    fn test_color_resource_rgba() {
        let color = ColorResource::new(255, 128, 64, 128);
        assert_eq!(color.red, 255);
        assert_eq!(color.green, 128);
        assert_eq!(color.blue, 64);
        assert_eq!(color.alpha, 128);
    }

    #[test]
    #[allow(deprecated)]
    fn test_color_from_hex_rgb() {
        let color = ColorResource::from_hex("#FF8040").unwrap();
        assert_eq!(color, ColorResource::rgb(255, 128, 64));

        // Without # prefix
        let color = ColorResource::from_hex("FF8040").unwrap();
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
    #[allow(deprecated)]
    fn test_color_from_hex_invalid() {
        assert!(ColorResource::from_hex("GG8040").is_err());
        assert!(ColorResource::from_hex("FF80").is_err()); // Too short
        assert!(ColorResource::from_hex("FF8040FF00").is_err()); // Too long
    }

    #[test]
    fn test_color_to_hex() {
        let color = ColorResource::rgb(255, 128, 64);
        assert_eq!(color.to_hex(), "#FF8040FF");

        let color = ColorResource::new(255, 128, 64, 128);
        assert_eq!(color.to_hex(), "#FF804080");
    }

    #[test]
    fn test_color_to_rgb_hex() {
        let color = ColorResource::rgb(255, 128, 64);
        assert_eq!(color.to_rgb_hex(), "#FF8040");
    }

    #[test]
    fn test_resource_value_conversions() {
        // String to string
        let val = ResourceValue::String("test".to_string());
        assert_eq!(val.as_string(), Some("test".to_string()));

        // Integer to string
        let val = ResourceValue::Integer(42);
        assert_eq!(val.as_string(), Some("42".to_string()));
        assert_eq!(val.as_integer(), Some(42));

        // Boolean to various
        let val = ResourceValue::Boolean(true);
        assert_eq!(val.as_boolean(), Some(true));
        assert_eq!(val.as_integer(), Some(1));
        assert_eq!(val.as_string(), Some("true".to_string()));

        // String "yes" to boolean
        let val = ResourceValue::String("yes".to_string());
        assert_eq!(val.as_boolean(), Some(true));

        // String "123" to integer
        let val = ResourceValue::String("123".to_string());
        assert_eq!(val.as_integer(), Some(123));
    }

    #[test]
    fn test_resource_error_display() {
        assert!(format!("{}", ResourceError::NotFound).contains("not found"));
        assert!(format!("{}", ResourceError::InvalidFormat).contains("format"));
        assert!(format!("{}", ResourceError::InvalidType).contains("type"));
        assert!(format!("{}", ResourceError::LoadFailed).contains("load"));
        assert!(format!("{}", ResourceError::CacheError).contains("cache"));
    }
}
