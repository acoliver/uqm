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
mod stringbank_tests {
    use crate::resource::stringbank::StringBank;

    #[test]
    fn test_stringbank_load() {
        let mut bank = StringBank::new();
        let content = r#"
HELLO=Hello!
WORLD=World!
"#;
        bank.load_table_from_string("en", content).unwrap();

        assert!(bank.has_language("en"));
        assert_eq!(bank.len("en"), Some(2));
    }

    #[test]
    fn test_stringbank_get_string() {
        let mut bank = StringBank::new();
        bank.load_table_from_string("en", "GREETING=Hello, World!")
            .unwrap();

        assert_eq!(bank.get("GREETING"), Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_stringbank_get_string_by_language() {
        let mut bank = StringBank::new();
        bank.load_table_from_string("en", "HELLO=Hello!").unwrap();
        bank.load_table_from_string("fr", "HELLO=Bonjour!").unwrap();

        assert_eq!(
            bank.get_with_language("HELLO", "en"),
            Some("Hello!".to_string())
        );
        assert_eq!(
            bank.get_with_language("HELLO", "fr"),
            Some("Bonjour!".to_string())
        );
    }

    #[test]
    fn test_stringbank_out_of_bounds() {
        // StringBank uses key-based lookup, not index-based.
        // Test missing key behavior.
        let bank = StringBank::new();
        assert_eq!(bank.get("NONEXISTENT"), None);
    }

    #[test]
    fn test_stringbank_fallback_to_default() {
        let mut bank = StringBank::new();

        // English has both keys
        bank.load_table_from_string("en", "KEY1=English1\nKEY2=English2")
            .unwrap();

        // French only has KEY1
        bank.load_table_from_string("fr", "KEY1=French1").unwrap();

        // Getting KEY2 from French should fall back to English
        assert_eq!(
            bank.get_with_language("KEY2", "fr"),
            Some("English2".to_string())
        );
    }

    #[test]
    fn test_stringbank_fallback_chain() {
        let mut bank = StringBank::with_default("de"); // German as default

        // English has the key
        bank.load_table_from_string("en", "KEY=English").unwrap();

        // German (default) has the key
        bank.load_table_from_string("de", "KEY=Deutsch").unwrap();

        // French doesn't have the key
        bank.load_table_from_string("fr", "OTHER=Autre").unwrap();

        // French should fall back to German (default), not English
        assert_eq!(
            bank.get_with_language("KEY", "fr"),
            Some("Deutsch".to_string())
        );
    }

    #[test]
    fn test_stringbank_formatted_strings() {
        let mut bank = StringBank::new();
        bank.load_table_from_string("en", "TEMPLATE=Hello, {0}! You have {1} messages.")
            .unwrap();

        let result = bank.get_formatted("TEMPLATE", &["Alice", "5"]);
        assert_eq!(result, "Hello, Alice! You have 5 messages.");
    }

    #[test]
    fn test_stringbank_formatted_missing_key() {
        let bank = StringBank::new();
        let result = bank.get_formatted("MISSING", &["arg1"]);
        // Should return the key itself
        assert_eq!(result, "MISSING");
    }

    #[test]
    fn test_stringbank_available_languages() {
        let mut bank = StringBank::new();
        bank.load_table_from_string("en", "").unwrap();
        bank.load_table_from_string("fr", "").unwrap();
        bank.load_table_from_string("de", "").unwrap();

        let langs = bank.available_languages();
        assert_eq!(langs.len(), 3);
        assert!(langs.contains(&"en".to_string()));
        assert!(langs.contains(&"fr".to_string()));
        assert!(langs.contains(&"de".to_string()));
    }

    #[test]
    fn test_stringbank_remove_language() {
        let mut bank = StringBank::new();
        bank.load_table_from_string("en", "KEY=Value").unwrap();
        assert!(bank.has_language("en"));

        bank.remove_language("en");
        assert!(!bank.has_language("en"));
    }

    #[test]
    fn test_stringbank_merge() {
        let mut bank1 = StringBank::new();
        let mut bank2 = StringBank::new();

        bank1.load_table_from_string("en", "KEY=English").unwrap();
        bank2.load_table_from_string("fr", "KEY=Français").unwrap();

        bank1.merge(&bank2);

        assert!(bank1.has_language("en"));
        assert!(bank1.has_language("fr"));
    }

    #[test]
    fn test_stringbank_case_insensitive_language() {
        let mut bank = StringBank::new();
        bank.load_table_from_string("EN", "KEY=Value").unwrap();

        // Language lookup should be case-insensitive
        assert!(bank.has_language("en"));
        assert!(bank.has_language("EN"));
        assert!(bank.has_language("En"));
    }

    #[test]
    fn test_stringbank_get_or_default() {
        let mut bank = StringBank::new();
        bank.load_table_from_string("en", "KEY=Value").unwrap();

        assert_eq!(bank.get_or("KEY", "Default"), "Value");
        assert_eq!(bank.get_or("MISSING", "Default"), "Default");
    }
}

#[cfg(test)]
mod resource_index_tests {
    use crate::resource::{ResourceSystem, ResourceType};
    use std::env;

    /// Create a test .rmp-style index file
    fn create_test_index() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let index_path = temp_dir.path().join("test.rmp");

        // Format: RESOURCE_NAME = FILENAME,TYPE
        let content = r#"
# Test resource index
STRING_RES = string.txt,STRING
INT_RES = int.txt,INTEGER
BOOL_RES = bool.txt,BOOLEAN
COLOR_RES = color.txt,COLOR
"#;
        std::fs::write(&index_path, content).unwrap();

        // Create the resource files
        std::fs::write(temp_dir.path().join("string.txt"), "Test String").unwrap();
        std::fs::write(temp_dir.path().join("int.txt"), "42").unwrap();
        std::fs::write(temp_dir.path().join("bool.txt"), "true").unwrap();
        std::fs::write(temp_dir.path().join("color.txt"), "#FF8040").unwrap();

        (temp_dir, index_path)
    }

    #[test]
    fn test_resource_index_parse() {
        let (temp_dir, index_path) = create_test_index();
        let mut system = ResourceSystem::new(temp_dir.path());

        let result = system.load_index(index_path.file_name().unwrap());
        assert!(result.is_ok());

        // Verify resources were registered
        assert!(system.resource_exists("STRING_RES"));
        assert!(system.resource_exists("INT_RES"));
        assert!(system.resource_exists("BOOL_RES"));
        assert!(system.resource_exists("COLOR_RES"));
    }

    #[test]
    fn test_resource_index_lookup() {
        let (temp_dir, index_path) = create_test_index();
        let mut system = ResourceSystem::new(temp_dir.path());
        system.load_index(index_path.file_name().unwrap()).unwrap();

        // Look up resources by name
        let string_val = system.get_string("STRING_RES").unwrap();
        assert_eq!(string_val, "Test String");

        let int_val = system.get_int("INT_RES").unwrap();
        assert_eq!(int_val, 42);

        let bool_val = system.get_bool("BOOL_RES").unwrap();
        assert!(bool_val);
    }

    #[test]
    fn test_resource_index_missing() {
        let (temp_dir, index_path) = create_test_index();
        let mut system = ResourceSystem::new(temp_dir.path());
        system.load_index(index_path.file_name().unwrap()).unwrap();

        // Try to look up a non-existent resource
        assert!(!system.resource_exists("NONEXISTENT"));

        let result = system.get_string("NONEXISTENT");
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_index_file_not_found() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut system = ResourceSystem::new(temp_dir.path());

        let result = system.load_index("nonexistent.rmp");
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_index_invalid_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("invalid.rmp");

        // Create an index with invalid format (no comma separator)
        let content = "INVALID_RES = file.txt";
        std::fs::write(&index_path, content).unwrap();

        let mut system = ResourceSystem::new(temp_dir.path());
        let result = system.load_index("invalid.rmp");
        // Should succeed but not register the malformed entry
        assert!(result.is_ok());
        assert!(!system.resource_exists("INVALID_RES"));
    }
}

#[cfg(test)]
mod resource_loading_tests {
    use crate::resource::{ResourceError, ResourceSystem, ResourceType};

    fn create_test_resources() -> (ResourceSystem, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let resources_dir = temp_dir.path().to_path_buf();

        // Create test resource files
        std::fs::write(resources_dir.join("hello.txt"), "Hello, World!").unwrap();
        std::fs::write(resources_dir.join("number.txt"), "12345").unwrap();
        std::fs::write(resources_dir.join("flag.txt"), "yes").unwrap();
        std::fs::write(resources_dir.join("binary.bin"), &[0x00, 0x01, 0x02, 0x03]).unwrap();

        let mut system = ResourceSystem::new(&resources_dir);

        system.register_resource("HELLO", "hello.txt", ResourceType::String);
        system.register_resource("NUMBER", "number.txt", ResourceType::Integer);
        system.register_resource("FLAG", "flag.txt", ResourceType::Boolean);
        system.register_resource("BINARY", "binary.bin", ResourceType::Binary);

        (system, temp_dir)
    }

    #[test]
    fn test_resource_load_file() {
        let (mut system, _temp_dir) = create_test_resources();

        let result = system.get_string("HELLO");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn test_resource_load_int() {
        let (mut system, _temp_dir) = create_test_resources();

        let result = system.get_int("NUMBER");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12345);
    }

    #[test]
    fn test_resource_load_bool() {
        let (mut system, _temp_dir) = create_test_resources();

        let result = system.get_bool("FLAG");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_resource_load_cached() {
        let (mut system, _temp_dir) = create_test_resources();

        // First load
        let result1 = system.get_string("HELLO");
        assert!(result1.is_ok());
        assert_eq!(system.cached_count(), 1);

        // Second load should use cache
        let result2 = system.get_string("HELLO");
        assert!(result2.is_ok());
        assert_eq!(system.cached_count(), 1); // Still 1, not 2

        // Values should be the same
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_resource_handle_refcount() {
        let (mut system, _temp_dir) = create_test_resources();

        // Get resource twice - ref count should be 2
        let _r1 = system.get_string("HELLO").unwrap();
        let _r2 = system.get_string("HELLO").unwrap();
        assert_eq!(system.cached_count(), 1);

        // Release once - should still be cached (ref count 1)
        system.release_resource("HELLO").unwrap();
        assert_eq!(system.cached_count(), 1);

        // Release again - should be freed (ref count 0)
        system.release_resource("HELLO").unwrap();
        assert_eq!(system.cached_count(), 0);
    }

    #[test]
    fn test_resource_not_found() {
        let (mut system, _temp_dir) = create_test_resources();

        let result = system.get_string("NONEXISTENT");
        assert_eq!(result, Err(ResourceError::NotFound));
    }

    #[test]
    fn test_resource_type_mismatch() {
        let (mut system, _temp_dir) = create_test_resources();

        // Try to get a string resource as an int
        let result = system.get_int("HELLO");
        assert_eq!(result, Err(ResourceError::InvalidType));
    }

    #[test]
    fn test_resource_disabled_system() {
        let (mut system, _temp_dir) = create_test_resources();

        system.set_enabled(false);
        let result = system.get_string("HELLO");
        assert_eq!(result, Err(ResourceError::NotFound));

        system.set_enabled(true);
        let result = system.get_string("HELLO");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_alias() {
        let (mut system, _temp_dir) = create_test_resources();

        system.add_alias("GREETING", "HELLO");
        system.add_alias("INDIRECT", "GREETING");

        // Should resolve alias chain
        let result = system.get_string("INDIRECT");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }
}

#[cfg(test)]
mod cache_tests {
    use crate::resource::{ResourceSystem, ResourceType};

    fn create_cache_test_system() -> (ResourceSystem, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let resources_dir = temp_dir.path().to_path_buf();

        // Create multiple test files
        for i in 0..10 {
            std::fs::write(resources_dir.join(format!("res_{}.txt", i)), format!("Value {}", i))
                .unwrap();
        }

        let mut system = ResourceSystem::new(&resources_dir);

        for i in 0..10 {
            system.register_resource(
                &format!("RES_{}", i),
                &format!("res_{}.txt", i),
                ResourceType::String,
            );
        }

        (system, temp_dir)
    }

    #[test]
    fn test_cache_insert_get() {
        let (mut system, _temp_dir) = create_cache_test_system();

        assert_eq!(system.cached_count(), 0);

        // Load a resource - should be cached
        let val = system.get_string("RES_0").unwrap();
        assert_eq!(val, "Value 0");
        assert_eq!(system.cached_count(), 1);

        // Get same resource again - should still be 1
        let val2 = system.get_string("RES_0").unwrap();
        assert_eq!(val2, "Value 0");
        assert_eq!(system.cached_count(), 1);
    }

    #[test]
    fn test_cache_multiple_resources() {
        let (mut system, _temp_dir) = create_cache_test_system();

        // Load multiple resources
        for i in 0..5 {
            system.get_string(&format!("RES_{}", i)).unwrap();
        }

        assert_eq!(system.cached_count(), 5);
    }

    #[test]
    fn test_cache_clear() {
        let (mut system, _temp_dir) = create_cache_test_system();

        // Load some resources
        system.get_string("RES_0").unwrap();
        system.get_string("RES_1").unwrap();
        system.get_string("RES_2").unwrap();
        assert_eq!(system.cached_count(), 3);

        // Clear cache
        system.clear_cache();
        assert_eq!(system.cached_count(), 0);
    }

    #[test]
    fn test_cache_lru_eviction() {
        // Note: The current Rust implementation doesn't have explicit LRU eviction
        // with a max capacity. This test documents expected behavior if/when
        // LRU eviction is implemented.
        //
        // For now, we test that resources can be manually released.
        let (mut system, _temp_dir) = create_cache_test_system();

        // Load a resource
        system.get_string("RES_0").unwrap();
        assert_eq!(system.cached_count(), 1);

        // Release it
        system.release_resource("RES_0").unwrap();
        assert_eq!(system.cached_count(), 0);
    }

    #[test]
    fn test_cache_capacity() {
        // Note: The current Rust implementation doesn't have a max capacity limit.
        // This test documents expected behavior and current unlimited behavior.
        let (mut system, _temp_dir) = create_cache_test_system();

        // Load all 10 resources
        for i in 0..10 {
            system.get_string(&format!("RES_{}", i)).unwrap();
        }

        // All should be cached (no eviction since no limit)
        assert_eq!(system.cached_count(), 10);
    }

    #[test]
    fn test_cache_release_nonexistent() {
        let (mut system, _temp_dir) = create_cache_test_system();

        let result = system.release_resource("NONEXISTENT");
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_double_release() {
        let (mut system, _temp_dir) = create_cache_test_system();

        // Load and get ref count to 1
        system.get_string("RES_0").unwrap();

        // Release once - clears cache
        system.release_resource("RES_0").unwrap();
        assert_eq!(system.cached_count(), 0);

        // Release again - should be safe (ref count already 0)
        let result = system.release_resource("RES_0");
        assert!(result.is_ok()); // Should not error, just no-op
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

#[cfg(test)]
mod ffi_tests {
    use crate::resource::ffi::*;
    use std::env;
    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_ffi_init_resource_system() {
        let temp_dir = env::temp_dir();
        let c_path = CString::new(temp_dir.to_str().unwrap()).unwrap();

        unsafe {
            let result = rust_init_resource_system(c_path.as_ptr());
            assert_eq!(result, 1);
        }
    }

    #[test]
    fn test_ffi_null_pointer_handling() {
        unsafe {
            // All functions should handle null pointers gracefully
            assert_eq!(rust_init_resource_system(ptr::null()), 0);
            assert!(rust_get_string_resource(ptr::null()).is_null());
            assert_eq!(rust_get_int_resource(ptr::null()), 0);
            assert_eq!(rust_get_bool_resource(ptr::null()), 0);
        }
    }

    #[test]
    fn test_ffi_resources_enabled() {
        let temp_dir = env::temp_dir();
        let c_path = CString::new(temp_dir.to_str().unwrap()).unwrap();

        unsafe {
            rust_init_resource_system(c_path.as_ptr());

            // Initially enabled
            assert_eq!(rust_resources_enabled(), 1);

            // Disable
            rust_set_resources_enabled(0);
            assert_eq!(rust_resources_enabled(), 0);

            // Re-enable
            rust_set_resources_enabled(1);
            assert_eq!(rust_resources_enabled(), 1);
        }
    }

    #[test]
    fn test_ffi_free_null_string() {
        // Should not crash when freeing null
        unsafe {
            rust_free_string(ptr::null_mut());
        }
    }
}
