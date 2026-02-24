// Property File Parser
// Parses simple key=value property files
//
// @plan PLAN-20260224-RES-SWAP.P03
// @requirement REQ-RES-018, REQ-RES-R007, REQ-RES-006-012

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

#[cfg(test)]
use std::env;

/// Parse a property file string, invoking `handler` for each key-value pair.
///
/// This is the replacement for `PropertyFile::from_string`, matching the C
/// `PropFile_from_string` behavior: preserves key case, handles inline `#`
/// comments, and supports an optional key prefix.
///
/// # Arguments
/// * `data` - The property file content to parse
/// * `handler` - Callback invoked with `(key, value)` for each entry
/// * `prefix` - Optional prefix to prepend to all keys (total key length capped at 255)
///
/// @plan PLAN-20260224-RES-SWAP.P03
/// @requirement REQ-RES-018, REQ-RES-R007, REQ-RES-006-012
pub fn parse_propfile(_data: &str, _handler: &mut dyn FnMut(&str, &str), _prefix: Option<&str>) {
    todo!("Parse propfile — see component-001.md")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyError {
    FileNotFound,
    InvalidFormat,
    IoError,
}

impl std::fmt::Display for PropertyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PropertyError::FileNotFound => write!(f, "Property file not found"),
            PropertyError::InvalidFormat => write!(f, "Invalid property file format"),
            PropertyError::IoError => write!(f, "I/O error reading property file"),
        }
    }
}

impl std::error::Error for PropertyError {}

impl From<io::Error> for PropertyError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => PropertyError::FileNotFound,
            _ => PropertyError::IoError,
        }
    }
}

/// Property file containing key-value pairs
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PropertyFile {
    properties: HashMap<String, String>,
}

impl PropertyFile {
    /// Load a property file from disk
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, PropertyError> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);

        let mut properties = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key=value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();

                if !key.is_empty() {
                    properties.insert(key.to_uppercase(), value);
                }
            }
        }

        Ok(PropertyFile { properties })
    }

    /// Load a property file from a string
    #[deprecated(note = "Use parse_propfile() instead")]
    pub fn from_string(content: &str) -> Result<Self, PropertyError> {
        let mut properties = HashMap::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key=value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();

                if !key.is_empty() {
                    properties.insert(key.to_uppercase(), value);
                }
            }
        }

        Ok(PropertyFile { properties })
    }

    /// Get a property value by key
    pub fn get(&self, key: &str) -> Option<&String> {
        self.properties.get(&key.to_uppercase())
    }

    /// Get a property value with a default
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// Get an integer property
    pub fn get_int(&self, key: &str) -> Option<i32> {
        self.get(key)?.parse().ok()
    }

    /// Get a boolean property
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key)?.parse().ok()
    }

    /// Set a property value
    pub fn set(&mut self, key: &str, value: &str) {
        self.properties
            .insert(key.to_uppercase(), value.to_string());
    }

    /// Remove a property
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.properties.remove(&key.to_uppercase())
    }

    /// Check if a property exists
    pub fn contains(&self, key: &str) -> bool {
        self.properties.contains_key(&key.to_uppercase())
    }

    /// Get the number of properties
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Check if the property file is empty
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Get an iterator over all properties
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.properties.iter()
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.properties.keys()
    }

    /// Get all values
    pub fn values(&self) -> impl Iterator<Item = &String> {
        self.properties.values()
    }

    /// Save the property file to disk
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), PropertyError> {
        let mut properties: Vec<_> = self.properties.iter().collect();
        properties.sort_by(|a, b| a.0.cmp(b.0));

        let mut content = String::new();
        for (key, value) in properties {
            content.push_str(key);
            content.push('=');
            content.push_str(value);
            content.push('\n');
        }

        fs::write(path, content)?;
        Ok(())
    }

    /// Clear all properties
    pub fn clear(&mut self) {
        self.properties.clear();
    }

    /// Merge another property file into this one
    pub fn merge(&mut self, other: &PropertyFile) {
        for (key, value) in other.iter() {
            self.properties.insert(key.clone(), value.clone());
        }
    }

    /// Get matching keys with a prefix
    pub fn get_keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        let prefix_upper = prefix.to_uppercase();
        self.keys()
            .filter(|k| k.starts_with(&prefix_upper))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let pf = PropertyFile::default();
        assert!(pf.is_empty());
    }

    #[test]
    fn test_from_string() {
        let content = r#"
# This is a comment
KEY1=VALUE1
KEY2 = VALUE2 
KEY3= VALUE3 
KEY4 = VALUE4

"#;
        let pf = PropertyFile::from_string(content).unwrap();

        assert_eq!(pf.get("KEY1"), Some(&"VALUE1".to_string()));
        assert_eq!(pf.get("KEY2"), Some(&"VALUE2".to_string()));
        assert_eq!(pf.get("KEY3"), Some(&"VALUE3".to_string()));
        assert_eq!(pf.get("KEY4"), Some(&"VALUE4".to_string()));
    }

    #[test]
    fn test_from_string_empty() {
        let pf = PropertyFile::from_string("").unwrap();
        assert!(pf.is_empty());
    }

    #[test]
    fn test_get_set() {
        let mut pf = PropertyFile::default();

        assert!(pf.get("KEY").is_none());

        pf.set("KEY", "VALUE");
        assert_eq!(pf.get("KEY"), Some(&"VALUE".to_string()));

        pf.set("key", "value2"); // Case-insensitive
        assert_eq!(pf.get("KEY"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_get_or() {
        let mut pf = PropertyFile::default();

        assert_eq!(pf.get_or("KEY", "DEFAULT"), "DEFAULT");

        pf.set("KEY", "VALUE");
        assert_eq!(pf.get_or("KEY", "DEFAULT"), "VALUE");
    }

    #[test]
    fn test_get_int() {
        let mut pf = PropertyFile::default();

        assert!(pf.get_int("NUM").is_none());

        pf.set("NUM", "42");
        assert_eq!(pf.get_int("NUM"), Some(42));

        pf.set("NUM", "INVALID");
        assert!(pf.get_int("NUM").is_none());
    }

    #[test]
    fn test_get_bool() {
        let mut pf = PropertyFile::default();

        assert!(pf.get_bool("FLAG").is_none());

        pf.set("FLAG", "true");
        assert_eq!(pf.get_bool("FLAG"), Some(true));

        pf.set("FLAG", "false");
        assert_eq!(pf.get_bool("FLAG"), Some(false));
    }

    #[test]
    fn test_remove() {
        let mut pf = PropertyFile::default();

        pf.set("KEY", "VALUE");
        assert!(pf.contains("KEY"));

        let removed = pf.remove("KEY");
        assert_eq!(removed, Some("VALUE".to_string()));
        assert!(!pf.contains("KEY"));
    }

    #[test]
    fn test_contains() {
        let mut pf = PropertyFile::default();

        assert!(!pf.contains("KEY"));

        pf.set("KEY", "VALUE");
        assert!(pf.contains("KEY"));
    }

    #[test]
    fn test_len() {
        let mut pf = PropertyFile::default();

        assert_eq!(pf.len(), 0);

        pf.set("KEY1", "VALUE1");
        assert_eq!(pf.len(), 1);

        pf.set("KEY2", "VALUE2");
        assert_eq!(pf.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut pf = PropertyFile::default();

        pf.set("KEY1", "VALUE1");
        pf.set("KEY2", "VALUE2");
        assert_eq!(pf.len(), 2);

        pf.clear();
        assert!(pf.is_empty());
    }

    #[test]
    fn test_iter() {
        let mut pf = PropertyFile::default();

        pf.set("KEY1", "VALUE1");
        pf.set("KEY2", "VALUE2");

        let keys: Vec<_> = pf.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"KEY1".to_string()));
        assert!(keys.contains(&"KEY2".to_string()));
    }

    #[test]
    fn test_merge() {
        let mut pf1 = PropertyFile::default();
        let mut pf2 = PropertyFile::default();

        pf1.set("KEY1", "VALUE1");
        pf2.set("KEY2", "VALUE2");

        pf1.merge(&pf2);

        assert_eq!(pf1.len(), 2);
        assert_eq!(pf1.get("KEY1"), Some(&"VALUE1".to_string()));
        assert_eq!(pf1.get("KEY2"), Some(&"VALUE2".to_string()));
    }

    #[test]
    fn test_get_keys_with_prefix() {
        let mut pf = PropertyFile::default();

        pf.set("SECTION_KEY1", "VALUE1");
        pf.set("SECTION_KEY2", "VALUE2");
        pf.set("OTHER_KEY", "VALUE3");

        let keys = pf.get_keys_with_prefix("SECTION_");
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_case_insensitive() {
        let mut pf = PropertyFile::default();

        pf.set("MyKey", "VALUE");

        assert_eq!(pf.get("mykey"), Some(&"VALUE".to_string()));
        assert_eq!(pf.get("MYKEY"), Some(&"VALUE".to_string()));
        assert_eq!(pf.get("MyKeY"), Some(&"VALUE".to_string()));
    }

    #[test]
    fn test_save_round_trip() {
        let mut pf = PropertyFile::default();

        pf.set("KEY1", "VALUE1");
        pf.set("KEY2", "VALUE2");

        let path = env::temp_dir().join("test_propfile.txt");
        pf.save(&path).unwrap();

        let loaded = PropertyFile::load(&path).unwrap();
        assert_eq!(loaded.get("KEY1"), Some(&"VALUE1".to_string()));
        assert_eq!(loaded.get("KEY2"), Some(&"VALUE2".to_string()));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_complex_formatting() {
        let content = r#"
# Header comment
KEY1=VALUE1
  KEY2  =  VALUE2  
# Another comment
KEY3 = VALUE3

"#;

        let pf = PropertyFile::from_string(content).unwrap();

        assert_eq!(pf.len(), 3);
        assert_eq!(pf.get("KEY1"), Some(&"VALUE1".to_string()));
        assert_eq!(pf.get("KEY2"), Some(&"VALUE2".to_string()));
        assert_eq!(pf.get("KEY3"), Some(&"VALUE3".to_string()));
    }

    #[test]
    fn test_property_error_file_not_found() {
        let result = PropertyFile::load("/nonexistent/file.txt");
        assert_eq!(result, Err(PropertyError::FileNotFound));
    }

    #[test]
    fn test_property_error_display() {
        let err = PropertyError::FileNotFound;
        assert!(format!("{}", err).contains("not found"));
    }
}
