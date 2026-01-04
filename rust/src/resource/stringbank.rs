// String Bank
// Manages collections of localized strings

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use super::propfile::PropertyFile;
use super::resource_type::ResourceError;

/// String bank for managing multiple string tables
#[derive(Debug, Clone)]
pub struct StringBank {
    tables: HashMap<String, Arc<PropertyFile>>,
    default_language: String,
}

impl StringBank {
    /// Create a new string bank
    pub fn new() -> Self {
        StringBank {
            tables: HashMap::new(),
            default_language: "en".to_string(),
        }
    }

    /// Create a new string bank with a default language
    pub fn with_default(default_language: &str) -> Self {
        let mut bank = Self::new();
        bank.default_language = default_language.to_string();
        bank
    }

    /// Load a string table from a property file
    pub fn load_table<P: AsRef<Path>>(
        &mut self,
        language: &str,
        path: P,
    ) -> Result<(), ResourceError> {
        let propfile = PropertyFile::load(path).map_err(|_| ResourceError::LoadFailed)?;

        self.tables
            .insert(language.to_lowercase(), Arc::new(propfile));

        Ok(())
    }

    /// Load a string table from a string
    pub fn load_table_from_string(
        &mut self,
        language: &str,
        content: &str,
    ) -> Result<(), ResourceError> {
        let propfile = PropertyFile::from_string(content).map_err(|_| ResourceError::LoadFailed)?;

        self.tables
            .insert(language.to_lowercase(), Arc::new(propfile));

        Ok(())
    }

    /// Set the default language
    pub fn set_default_language(&mut self, language: &str) {
        self.default_language = language.to_lowercase();
    }

    /// Get the current default language
    pub fn default_language(&self) -> &str {
        &self.default_language
    }

    /// Get a string from the default language table
    pub fn get(&self, key: &str) -> Option<String> {
        self.get_with_language(key, &self.default_language)
    }

    /// Get a string from a specific language table
    pub fn get_with_language(&self, key: &str, language: &str) -> Option<String> {
        let lang_lower = language.to_lowercase();

        // Try the requested language
        if let Some(table) = self.tables.get(&lang_lower) {
            if let Some(value) = table.get(key) {
                return Some(value.clone());
            }
        }

        // Fall back to default language
        if lang_lower != self.default_language {
            if let Some(table) = self.tables.get(&self.default_language) {
                if let Some(value) = table.get(key) {
                    return Some(value.clone());
                }
            }
        }

        // Fall back to English if available
        if self.default_language != "en" && lang_lower != "en" {
            if let Some(table) = self.tables.get("en") {
                if let Some(value) = table.get(key) {
                    return Some(value.clone());
                }
            }
        }

        None
    }

    /// Get a string with a default value if not found
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or_else(|| default.to_string())
    }

    /// Get a formatted string with placeholders
    ///
    /// Supports {0}, {1}, etc. as placeholders
    pub fn get_formatted(&self, key: &str, args: &[&str]) -> String {
        let template = self.get_or(key, key);
        let mut result = template;

        for (i, arg) in args.iter().enumerate() {
            let placeholder = format!("{{{}}}", i);
            result = result.replace(&placeholder, arg);
        }

        result
    }

    /// Check if a key exists in any table
    pub fn contains(&self, key: &str) -> bool {
        self.tables.values().any(|table| table.contains(key))
    }

    /// Check if a key exists in a specific language table
    pub fn contains_in_language(&self, key: &str, language: &str) -> bool {
        let lang_lower = language.to_lowercase();
        self.tables
            .get(&lang_lower)
            .map(|table| table.contains(key))
            .unwrap_or(false)
    }

    /// Get all available languages
    pub fn available_languages(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }

    /// Check if a language is available
    pub fn has_language(&self, language: &str) -> bool {
        self.tables.contains_key(&language.to_lowercase())
    }

    /// Remove a language table
    pub fn remove_language(&mut self, language: &str) -> Option<Arc<PropertyFile>> {
        self.tables.remove(&language.to_lowercase())
    }

    /// Get the number of strings in a specific language table
    pub fn len(&self, language: &str) -> Option<usize> {
        self.tables
            .get(&language.to_lowercase())
            .map(|table| table.len())
    }

    /// Check if any tables are loaded
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// Get all keys from a specific language table
    pub fn keys(&self, language: &str) -> Option<Vec<String>> {
        self.tables
            .get(&language.to_lowercase())
            .map(|table| table.keys().cloned().collect())
    }

    /// Clear all string tables
    pub fn clear(&mut self) {
        self.tables.clear();
    }

    /// Merge another string bank into this one
    pub fn merge(&mut self, other: &StringBank) {
        for (language, table) in &other.tables {
            self.tables
                .entry(language.clone())
                .or_insert_with(|| Arc::clone(table));
        }
    }

    /// Get shared reference to a language table
    pub fn get_table(&self, language: &str) -> Option<&Arc<PropertyFile>> {
        self.tables.get(&language.to_lowercase())
    }
}

impl Default for StringBank {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let bank = StringBank::new();
        assert!(bank.is_empty());
        assert_eq!(bank.default_language(), "en");
    }

    #[test]
    fn test_with_default() {
        let bank = StringBank::with_default("fr");
        assert_eq!(bank.default_language(), "fr");
    }

    #[test]
    fn test_load_table_from_string() {
        let mut bank = StringBank::new();

        let content = r#"
HELLO=Hello, World!
GOODBYE=Goodbye!
"#;

        bank.load_table_from_string("en", content).unwrap();

        assert_eq!(bank.get("HELLO"), Some("Hello, World!".to_string()));
        assert_eq!(bank.get("GOODBYE"), Some("Goodbye!".to_string()));
    }

    #[test]
    fn test_get_fallback() {
        let mut bank = StringBank::new();

        // Load English (default)
        let en_content = r#"
HELLO=Hello!
KEY1=English value
"#;
        bank.load_table_from_string("en", en_content).unwrap();

        // Load French (without KEY1)
        let fr_content = r#"
HELLO=Bonjour!
KEY2=French value
"#;
        bank.load_table_from_string("fr", fr_content).unwrap();

        // Get from French
        assert_eq!(
            bank.get_with_language("HELLO", "fr"),
            Some("Bonjour!".to_string())
        );

        // Fall back to English for missing key
        assert_eq!(
            bank.get_with_language("KEY1", "fr"),
            Some("English value".to_string())
        );

        // Key only in French
        assert_eq!(
            bank.get_with_language("KEY2", "fr"),
            Some("French value".to_string())
        );
    }

    #[test]
    fn test_get_or() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "KEY=Value").unwrap();

        assert_eq!(bank.get_or("KEY", "Default"), "Value");
        assert_eq!(bank.get_or("MISSING", "Default"), "Default");
    }

    #[test]
    fn test_get_formatted() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "GREETING=Hello, {0}! You have {1} messages.")
            .unwrap();

        let result = bank.get_formatted("GREETING", &["Alice", "5"]);
        assert_eq!(result, "Hello, Alice! You have 5 messages.");

        // Missing key returns key itself
        let result = bank.get_formatted("MISSING", &["arg1", "arg2"]);
        assert_eq!(result, "MISSING");
    }

    #[test]
    fn test_contains() {
        let mut bank = StringBank::new();

        assert!(!bank.contains("KEY"));

        bank.load_table_from_string("en", "KEY=Value").unwrap();

        assert!(bank.contains("KEY"));
        assert!(!bank.contains("MISSING"));
    }

    #[test]
    fn test_available_languages() {
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
    fn test_has_language() {
        let mut bank = StringBank::new();

        assert!(!bank.has_language("en"));

        bank.load_table_from_string("en", "").unwrap();

        assert!(bank.has_language("en"));
        assert!(!bank.has_language("fr"));
    }

    #[test]
    fn test_remove_language() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "KEY=Value").unwrap();

        assert!(bank.has_language("en"));

        bank.remove_language("en");

        assert!(!bank.has_language("en"));
        assert_eq!(bank.get("KEY"), None);
    }

    #[test]
    fn test_len() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "KEY1=Value1\nKEY2=Value2\nKEY3=Value3")
            .unwrap();

        assert_eq!(bank.len("en"), Some(3));
        assert_eq!(bank.len("fr"), None);
    }

    #[test]
    fn test_keys() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "KEY1=Value1\nKEY2=Value2")
            .unwrap();

        let keys = bank.keys("en").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"KEY1".to_string()));
        assert!(keys.contains(&"KEY2".to_string()));
    }

    #[test]
    fn test_clear() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "KEY=Value").unwrap();
        bank.load_table_from_string("fr", "KEY=Valeur").unwrap();

        assert_eq!(bank.available_languages().len(), 2);

        bank.clear();

        assert!(bank.is_empty());
    }

    #[test]
    fn test_merge() {
        let mut bank1 = StringBank::new();
        let mut bank2 = StringBank::new();

        bank1.load_table_from_string("en", "KEY1=Value1").unwrap();
        bank2.load_table_from_string("fr", "KEY2=Value2").unwrap();

        bank1.merge(&bank2);

        assert!(bank1.has_language("en"));
        assert!(bank1.has_language("fr"));
        assert_eq!(bank1.get("KEY1"), Some("Value1".to_string()));
        assert_eq!(
            bank1.get_with_language("KEY2", "fr"),
            Some("Value2".to_string())
        );
    }

    #[test]
    fn test_get_table() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "KEY=Value").unwrap();

        let table = bank.get_table("en");
        assert!(table.is_some());
        assert_eq!(table.unwrap().get("KEY"), Some(&"Value".to_string()));
    }

    #[test]
    fn test_case_insensitive_keys() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "MYKEY=MyValue").unwrap();

        // Property files are case-insensitive for keys
        assert_eq!(bank.get("mykey"), Some("MyValue".to_string()));
        assert_eq!(bank.get("MYKEY"), Some("MyValue".to_string()));
    }

    #[test]
    fn test_empty_tables() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "").unwrap();

        assert!(bank.has_language("en"));
        assert_eq!(bank.len("en"), Some(0));
    }

    #[test]
    fn test_comments_and_empty_lines() {
        let mut bank = StringBank::new();

        let content = r#"
# This is a comment

KEY1=Value1

# Another comment
KEY2=Value2
"#;

        bank.load_table_from_string("en", content).unwrap();

        assert_eq!(bank.len("en"), Some(2));
        assert_eq!(bank.get("KEY1"), Some("Value1".to_string()));
        assert_eq!(bank.get("KEY2"), Some("Value2".to_string()));
    }

    #[test]
    fn test_special_characters() {
        let mut bank = StringBank::new();

        bank.load_table_from_string("en", "KEY=Hello, World!\nNUM=Value: 42\nPAREN=(test)")
            .unwrap();

        assert_eq!(bank.get("KEY"), Some("Hello, World!".to_string()));
        assert_eq!(bank.get("NUM"), Some("Value: 42".to_string()));
        assert_eq!(bank.get("PAREN"), Some("(test)".to_string()));
    }
}
