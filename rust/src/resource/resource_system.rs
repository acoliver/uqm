// Resource System
// Type-safe resource loading with caching

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::propfile::PropertyFile;
use super::resource_type::{ResourceError, ResourceType, ResourceValue};

/// Resource descriptor
#[derive(Debug, Clone)]
struct ResourceDescriptor {
    path: PathBuf,
    resource_type: ResourceType,
    data: Option<Arc<ResourceValue>>,
    ref_count: usize,
}

/// Resource system for loading and caching resources
pub struct ResourceSystem {
    base_path: PathBuf,
    resources: HashMap<String, ResourceDescriptor>,
    alias_map: HashMap<String, String>,
    enabled: bool,
}

impl ResourceSystem {
    /// Create a new resource system
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        ResourceSystem {
            base_path: base_path.as_ref().to_path_buf(),
            resources: HashMap::new(),
            alias_map: HashMap::new(),
            enabled: true,
        }
    }

    /// Enable or disable the resource system
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if the resource system is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Load an index file
    pub fn load_index<P: AsRef<Path>>(&mut self, path: P) -> Result<(), ResourceError> {
        let index_path = self.base_path.join(path);
        let propfile = PropertyFile::load(&index_path).map_err(|_| ResourceError::LoadFailed)?;

        // Parse index file entries
        for (key, value) in propfile.iter() {
            // Index file format: RESOURCE = FILENAME,TYPE
            if let Some((filename, rtype_str)) = value.split_once(',') {
                let resource_type = ResourceType::from_str(rtype_str.trim());
                self.register_resource(key, filename.trim(), resource_type);
            }
        }

        Ok(())
    }

    /// Register a resource
    pub fn register_resource(&mut self, name: &str, filename: &str, resource_type: ResourceType) {
        let path = self.base_path.join(filename);
        self.resources.insert(
            name.to_string(),
            ResourceDescriptor {
                path,
                resource_type,
                data: None,
                ref_count: 0,
            },
        );
    }

    /// Add an alias for a resource
    pub fn add_alias(&mut self, alias: &str, target: &str) {
        self.alias_map.insert(alias.to_string(), target.to_string());
    }

    /// Resolve an alias to its target name
    fn resolve_name(&self, name: &str) -> String {
        let mut resolved = name.to_string();
        let mut visited = std::collections::HashSet::new();

        while let Some(target) = self.alias_map.get(&resolved) {
            if !visited.insert(resolved.clone()) {
                // Cycle detected
                return name.to_string();
            }
            resolved = target.clone();
        }

        resolved
    }

    /// Get a resource by name
    pub fn get_resource(&mut self, name: &str) -> Result<Arc<ResourceValue>, ResourceError> {
        if !self.enabled {
            return Err(ResourceError::NotFound);
        }

        let resolved_name = self.resolve_name(name);

        // Check if resource exists and get its path and type
        let (path, resource_type) = match self.resources.get(&resolved_name) {
            Some(desc) => (desc.path.clone(), desc.resource_type),
            None => return Err(ResourceError::NotFound),
        };

        // Load resource (can't hold mutable borrow while loading)
        let value = if let Some(desc) = self.resources.get(&resolved_name) {
            if desc.data.is_some() {
                // Already cached
                desc.data.clone().ok_or(ResourceError::LoadFailed)?
            } else {
                // Need to load
                let value = self.load_resource_file(&path, resource_type)?;
                Arc::new(value)
            }
        } else {
            return Err(ResourceError::NotFound);
        };

        // Now get mutable borrow to increment ref count and store data
        if let Some(desc) = self.resources.get_mut(&resolved_name) {
            if desc.data.is_none() {
                desc.data = Some(value.clone());
            }
            desc.ref_count += 1;
            desc.data.clone().ok_or(ResourceError::LoadFailed)
        } else {
            Err(ResourceError::NotFound)
        }
    }

    /// Get an integer resource
    pub fn get_int(&mut self, name: &str) -> Result<i32, ResourceError> {
        let resource = self.get_resource(name)?;
        resource.as_integer().ok_or(ResourceError::InvalidType)
    }

    /// Get a string resource
    pub fn get_string(&mut self, name: &str) -> Result<String, ResourceError> {
        let resource = self.get_resource(name)?;
        resource
            .as_string()
            .map(|s| s.to_string())
            .ok_or(ResourceError::InvalidType)
    }

    /// Get a boolean resource
    pub fn get_bool(&mut self, name: &str) -> Result<bool, ResourceError> {
        let resource = self.get_resource(name)?;
        resource.as_boolean().ok_or(ResourceError::InvalidType)
    }

    /// Get a color resource
    pub fn get_color(
        &mut self,
        name: &str,
    ) -> Result<super::resource_type::ColorResource, ResourceError> {
        let resource = self.get_resource(name)?;
        resource.as_color().ok_or(ResourceError::InvalidType)
    }

    /// Release a resource (decrement ref count)
    pub fn release_resource(&mut self, name: &str) -> Result<(), ResourceError> {
        let resolved_name = self.resolve_name(name);

        if let Some(desc) = self.resources.get_mut(&resolved_name) {
            if desc.ref_count > 0 {
                desc.ref_count -= 1;

                // Clear cache if no more references
                if desc.ref_count == 0 {
                    desc.data = None;
                }
            }
            Ok(())
        } else {
            Err(ResourceError::NotFound)
        }
    }

    /// Clear all cached resources
    pub fn clear_cache(&mut self) {
        for desc in self.resources.values_mut() {
            desc.ref_count = 0;
            desc.data = None;
        }
    }

    /// Check if a resource exists
    pub fn resource_exists(&self, name: &str) -> bool {
        let resolved_name = self.resolve_name(name);
        self.resources.contains_key(&resolved_name)
    }

    /// Get the number of cached resources
    pub fn cached_count(&self) -> usize {
        self.resources.values().filter(|r| r.data.is_some()).count()
    }

    /// Load a resource file
    fn load_resource_file(
        &self,
        path: &Path,
        resource_type: ResourceType,
    ) -> Result<ResourceValue, ResourceError> {
        if !path.exists() {
            return Err(ResourceError::NotFound);
        }

        match resource_type {
            ResourceType::String => {
                let content =
                    std::fs::read_to_string(path).map_err(|_| ResourceError::LoadFailed)?;
                Ok(ResourceValue::String(content.trim().to_string()))
            }
            ResourceType::Integer => {
                let content =
                    std::fs::read_to_string(path).map_err(|_| ResourceError::LoadFailed)?;
                let value = content
                    .trim()
                    .parse()
                    .map_err(|_| ResourceError::InvalidFormat)?;
                Ok(ResourceValue::Integer(value))
            }
            ResourceType::Boolean => {
                let content =
                    std::fs::read_to_string(path).map_err(|_| ResourceError::LoadFailed)?;
                let value = match content.trim().to_lowercase().as_str() {
                    "true" | "yes" | "1" => true,
                    "false" | "no" | "0" => false,
                    _ => return Err(ResourceError::InvalidFormat),
                };
                Ok(ResourceValue::Boolean(value))
            }
            ResourceType::Color => {
                let content =
                    std::fs::read_to_string(path).map_err(|_| ResourceError::LoadFailed)?;
                #[allow(deprecated)]
                let color = super::resource_type::ColorResource::from_hex(content.trim())
                    .map_err(|_| ResourceError::InvalidFormat)?;
                Ok(ResourceValue::Color(color))
            }
            ResourceType::Binary => {
                let data = std::fs::read(path).map_err(|_| ResourceError::LoadFailed)?;
                Ok(ResourceValue::Binary(data))
            }
            ResourceType::Unknown => Err(ResourceError::InvalidType),
        }
    }
}

impl Default for ResourceSystem {
    fn default() -> Self {
        Self::new(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ColorResource;

    fn create_test_resources() -> (ResourceSystem, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let resources_dir = temp_dir.path().to_path_buf();

        // Create test resource files
        std::fs::write(resources_dir.join("test_string.txt"), "Test String")
            .expect("Failed to write test string");
        std::fs::write(resources_dir.join("test_int.txt"), "42").expect("Failed to write test int");
        std::fs::write(resources_dir.join("test_bool.txt"), "true")
            .expect("Failed to write test bool");
        std::fs::write(resources_dir.join("test_color.txt"), "#FF8040")
            .expect("Failed to write test color");

        let mut system = ResourceSystem::new(&resources_dir);

        // Register resources
        system.register_resource("TEST_STRING", "test_string.txt", ResourceType::String);
        system.register_resource("TEST_INT", "test_int.txt", ResourceType::Integer);
        system.register_resource("TEST_BOOL", "test_bool.txt", ResourceType::Boolean);
        system.register_resource("TEST_COLOR", "test_color.txt", ResourceType::Color);

        (system, temp_dir)
    }

    #[test]
    fn test_new() {
        let system = ResourceSystem::new(".");
        assert!(system.is_enabled());
        assert_eq!(system.cached_count(), 0);
    }

    #[test]
    fn test_get_string() {
        let (mut system, _temp_dir) = create_test_resources();

        let result = system.get_string("TEST_STRING");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Test String");
    }

    #[test]
    fn test_get_int() {
        let (mut system, _temp_dir) = create_test_resources();

        let result = system.get_int("TEST_INT");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_get_bool() {
        let (mut system, _temp_dir) = create_test_resources();

        let result = system.get_bool("TEST_BOOL");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_get_color() {
        let (mut system, _temp_dir) = create_test_resources();

        let result = system.get_color("TEST_COLOR");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ColorResource::rgb(255, 128, 64));
    }

    #[test]
    fn test_resource_not_found() {
        let (mut system, _temp_dir) = create_test_resources();

        let result = system.get_string("NONEXISTENT");
        assert_eq!(result, Err(ResourceError::NotFound));
    }

    #[test]
    fn test_caching() {
        let (mut system, _temp_dir) = create_test_resources();

        // First get - loads from file
        let result1 = system.get_string("TEST_STRING");
        assert!(result1.is_ok());
        assert_eq!(system.cached_count(), 1);

        // Second get - uses cache
        let result2 = system.get_string("TEST_STRING");
        assert!(result2.is_ok());
        assert_eq!(system.cached_count(), 1);

        // Results should be identical (same Arc)
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_release_resource() {
        let (mut system, _temp_dir) = create_test_resources();

        system.get_string("TEST_STRING").unwrap();
        assert_eq!(system.cached_count(), 1);
        system.release_resource("TEST_STRING").unwrap();
        assert_eq!(system.cached_count(), 0);
    }

    #[test]
    fn test_clear_cache() {
        let (mut system, _temp_dir) = create_test_resources();

        system.get_string("TEST_STRING").unwrap();
        system.get_int("TEST_INT").unwrap();
        assert_eq!(system.cached_count(), 2);
        system.clear_cache();
        assert_eq!(system.cached_count(), 0);
    }

    #[test]
    fn test_add_alias() {
        let (mut system, _temp_dir) = create_test_resources();

        system.add_alias("ALIAS_STRING", "TEST_STRING");
        system.add_alias("ALIAS_ALIAS", "ALIAS_STRING");

        let result = system.get_string("ALIAS_ALIAS");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Test String");
    }

    #[test]
    fn test_set_enabled() {
        let (mut system, _temp_dir) = create_test_resources();

        system.set_enabled(false);
        assert!(!system.is_enabled());

        let result = system.get_string("TEST_STRING");
        assert_eq!(result, Err(ResourceError::NotFound));

        system.set_enabled(true);
        assert!(system.is_enabled());
    }

    #[test]
    fn test_resource_exists() {
        let (system, _temp_dir) = create_test_resources();

        assert!(system.resource_exists("TEST_STRING"));
        assert!(!system.resource_exists("NONEXISTENT"));
    }

    #[test]
    fn test_default() {
        let system: ResourceSystem = Default::default();
        assert!(system.is_enabled());
    }
}
