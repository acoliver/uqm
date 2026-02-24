//! Resource cache - LRU cache for loaded resources
//!
//! This module provides a thread-safe LRU cache for loaded resource data
//! to prevent redundant disk I/O. Resources are cached by key and evicted
//! when the cache exceeds its size limit.
//!
//! # Example
//! ```
//! use uqm::resource::cache::ResourceCache;
//!
//! let cache = ResourceCache::new(1024 * 1024); // 1MB cache
//! cache.insert("my.resource", vec![1, 2, 3, 4]);
//!
//! if let Some(data) = cache.get("my.resource") {
//!     println!("Got {} bytes", data.size);
//! }
//! ```
//!
//! # Reference
//! See `sc2/src/libs/resource/resinit.c` for the C resource management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// A cached resource entry
///
/// Contains the loaded resource data along with metadata for LRU tracking
/// and reference counting for pinning.
#[derive(Debug)]
pub struct CachedResource {
    /// The raw resource data
    pub data: Vec<u8>,
    /// Size in bytes (same as data.len())
    pub size: usize,
    /// Last access time for LRU tracking
    last_access: RwLock<Instant>,
    /// Reference count for pinning (prevents eviction when > 0)
    ref_count: AtomicUsize,
}

impl CachedResource {
    /// Create a new cached resource from raw data
    pub fn new(data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            data,
            size,
            last_access: RwLock::new(Instant::now()),
            ref_count: AtomicUsize::new(0),
        }
    }

    /// Get the last access time
    pub fn last_access(&self) -> Instant {
        *self.last_access.read().unwrap()
    }

    /// Update the last access time to now
    pub fn touch(&self) {
        *self.last_access.write().unwrap() = Instant::now();
    }

    /// Get the current reference count
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
    }

    /// Increment the reference count (pins the resource)
    pub fn add_ref(&self) -> usize {
        self.ref_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement the reference count (unpins the resource)
    pub fn release(&self) -> usize {
        let prev = self.ref_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 0 {
            // Underflow - restore to 0
            self.ref_count.store(0, Ordering::SeqCst);
            return 0;
        }
        prev - 1
    }

    /// Check if the resource is pinned (ref_count > 0)
    pub fn is_pinned(&self) -> bool {
        self.ref_count.load(Ordering::SeqCst) > 0
    }
}

/// Internal cache state protected by RwLock
struct CacheInner {
    /// Map of resource keys to cached entries
    entries: HashMap<String, Arc<CachedResource>>,
    /// Current total size of all cached entries
    current_size_bytes: usize,
}

impl CacheInner {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            current_size_bytes: 0,
        }
    }
}

/// Thread-safe LRU cache for loaded resources
///
/// The cache maintains a size limit and evicts least recently used entries
/// when the limit is exceeded. Entries with a reference count > 0 are
/// considered "pinned" and will not be evicted.
pub struct ResourceCache {
    /// Internal state protected by RwLock
    inner: RwLock<CacheInner>,
    /// Maximum cache size in bytes
    max_size_bytes: usize,
}

impl ResourceCache {
    /// Create a new resource cache with the given size limit
    ///
    /// # Arguments
    /// * `max_size_bytes` - Maximum total size of cached data in bytes
    ///
    /// # Example
    /// ```
    /// use uqm::resource::cache::ResourceCache;
    ///
    /// let cache = ResourceCache::new(10 * 1024 * 1024); // 10MB cache
    /// ```
    pub fn new(max_size_bytes: usize) -> Self {
        Self {
            inner: RwLock::new(CacheInner::new()),
            max_size_bytes,
        }
    }

    /// Get a cached resource by key
    ///
    /// Returns the cached resource if found, updating its last access time.
    /// Returns None if the resource is not in the cache.
    ///
    /// # Arguments
    /// * `key` - The resource key to look up
    ///
    /// # Example
    /// ```
    /// use uqm::resource::cache::ResourceCache;
    ///
    /// let cache = ResourceCache::new(1024);
    /// cache.insert("test", vec![1, 2, 3]);
    ///
    /// if let Some(resource) = cache.get("test") {
    ///     assert_eq!(resource.data, vec![1, 2, 3]);
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<Arc<CachedResource>> {
        let inner = self.inner.read().unwrap();
        if let Some(entry) = inner.entries.get(key) {
            entry.touch();
            Some(Arc::clone(entry))
        } else {
            None
        }
    }

    /// Insert a resource into the cache
    ///
    /// If the cache would exceed its size limit, LRU entries are evicted
    /// until there is room. Entries with ref_count > 0 are not evicted.
    ///
    /// If the resource already exists, it is replaced.
    ///
    /// # Arguments
    /// * `key` - The resource key
    /// * `data` - The raw resource data
    ///
    /// # Example
    /// ```
    /// use uqm::resource::cache::ResourceCache;
    ///
    /// let cache = ResourceCache::new(1024);
    /// cache.insert("my.resource", vec![1, 2, 3, 4]);
    /// ```
    pub fn insert(&self, key: &str, data: Vec<u8>) {
        let new_size = data.len();
        let resource = Arc::new(CachedResource::new(data));

        let mut inner = self.inner.write().unwrap();

        // Remove existing entry if present (to update size correctly)
        if let Some(old) = inner.entries.remove(key) {
            inner.current_size_bytes = inner.current_size_bytes.saturating_sub(old.size);
        }

        // Evict LRU entries until we have room
        while inner.current_size_bytes + new_size > self.max_size_bytes {
            if !self.evict_lru_internal(&mut inner) {
                // No more evictable entries - break to avoid infinite loop
                break;
            }
        }

        // Insert the new entry
        inner.current_size_bytes += new_size;
        inner.entries.insert(key.to_string(), resource);
    }

    /// Remove a resource from the cache
    ///
    /// # Arguments
    /// * `key` - The resource key to remove
    ///
    /// # Returns
    /// `true` if the resource was removed, `false` if it wasn't in the cache
    pub fn remove(&self, key: &str) -> bool {
        let mut inner = self.inner.write().unwrap();
        if let Some(old) = inner.entries.remove(key) {
            inner.current_size_bytes = inner.current_size_bytes.saturating_sub(old.size);
            true
        } else {
            false
        }
    }

    /// Clear the entire cache
    ///
    /// Removes all entries regardless of their reference count.
    pub fn clear(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.entries.clear();
        inner.current_size_bytes = 0;
    }

    /// Get the current cache size in bytes
    pub fn size_bytes(&self) -> usize {
        self.inner.read().unwrap().current_size_bytes
    }

    /// Get the maximum cache size in bytes
    pub fn max_size_bytes(&self) -> usize {
        self.max_size_bytes
    }

    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        self.inner.read().unwrap().entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().entries.is_empty()
    }

    /// Evict the least recently used entry
    ///
    /// Finds the entry with the oldest last_access time that is not pinned
    /// (ref_count == 0) and removes it.
    ///
    /// # Returns
    /// `true` if an entry was evicted, `false` if no evictable entries exist
    pub fn evict_lru(&self) -> bool {
        let mut inner = self.inner.write().unwrap();
        self.evict_lru_internal(&mut inner)
    }

    /// Internal LRU eviction (caller must hold write lock)
    fn evict_lru_internal(&self, inner: &mut CacheInner) -> bool {
        // Find the LRU entry that is not pinned
        let mut oldest_key: Option<String> = None;
        let mut oldest_time: Option<Instant> = None;

        for (key, entry) in inner.entries.iter() {
            // Skip pinned entries
            if entry.is_pinned() {
                continue;
            }

            let access_time = entry.last_access();
            if oldest_time.is_none() || access_time < oldest_time.unwrap() {
                oldest_key = Some(key.clone());
                oldest_time = Some(access_time);
            }
        }

        // Evict the oldest entry if found
        if let Some(key) = oldest_key {
            if let Some(old) = inner.entries.remove(&key) {
                inner.current_size_bytes = inner.current_size_bytes.saturating_sub(old.size);
                return true;
            }
        }

        false
    }

    /// Check if a key exists in the cache
    pub fn contains(&self, key: &str) -> bool {
        self.inner.read().unwrap().entries.contains_key(key)
    }

    /// Get all keys currently in the cache
    pub fn keys(&self) -> Vec<String> {
        self.inner.read().unwrap().entries.keys().cloned().collect()
    }
}

impl std::fmt::Debug for ResourceCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read().unwrap();
        f.debug_struct("ResourceCache")
            .field("max_size_bytes", &self.max_size_bytes)
            .field("current_size_bytes", &inner.current_size_bytes)
            .field("entries", &inner.entries.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_cache_new() {
        let cache = ResourceCache::new(1024);
        assert_eq!(cache.max_size_bytes(), 1024);
        assert_eq!(cache.size_bytes(), 0);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_insert_get() {
        let cache = ResourceCache::new(1024);
        let data = vec![1, 2, 3, 4, 5];

        cache.insert("test.resource", data.clone());

        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
        assert_eq!(cache.size_bytes(), 5);
        assert!(cache.contains("test.resource"));

        let retrieved = cache
            .get("test.resource")
            .expect("Should get cached resource");
        assert_eq!(retrieved.data, data);
        assert_eq!(retrieved.size, 5);
    }

    #[test]
    fn test_cache_miss() {
        let cache = ResourceCache::new(1024);
        cache.insert("exists", vec![1, 2, 3]);

        // Resource not in cache
        assert!(cache.get("not.exists").is_none());
        assert!(!cache.contains("not.exists"));

        // Resource exists
        assert!(cache.get("exists").is_some());
        assert!(cache.contains("exists"));
    }

    #[test]
    fn test_cache_eviction_lru() {
        let cache = ResourceCache::new(100);

        // Insert first entry
        cache.insert("first", vec![0; 40]);
        thread::sleep(Duration::from_millis(10));

        // Insert second entry
        cache.insert("second", vec![0; 40]);
        thread::sleep(Duration::from_millis(10));

        // Access first to make it more recent
        let _ = cache.get("first");
        thread::sleep(Duration::from_millis(10));

        // Insert third entry - should evict "second" (older access time)
        cache.insert("third", vec![0; 40]);

        // "second" should have been evicted (LRU)
        assert!(cache.contains("first"), "first should still exist");
        assert!(!cache.contains("second"), "second should be evicted");
        assert!(cache.contains("third"), "third should exist");
    }

    #[test]
    fn test_cache_size_limit() {
        let cache = ResourceCache::new(100);

        // Insert entries up to the limit
        cache.insert("a", vec![0; 30]);
        cache.insert("b", vec![0; 30]);
        cache.insert("c", vec![0; 30]);

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.size_bytes(), 90);

        // Insert one more that pushes over the limit
        cache.insert("d", vec![0; 30]);

        // Should have evicted oldest entries to make room
        assert!(cache.size_bytes() <= 100);
        assert!(cache.contains("d"));
    }

    #[test]
    fn test_cache_clear() {
        let cache = ResourceCache::new(1024);

        cache.insert("a", vec![1, 2, 3]);
        cache.insert("b", vec![4, 5, 6]);
        cache.insert("c", vec![7, 8, 9]);

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.size_bytes(), 9);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert_eq!(cache.size_bytes(), 0);
        assert!(cache.is_empty());
        assert!(!cache.contains("a"));
        assert!(!cache.contains("b"));
        assert!(!cache.contains("c"));
    }

    #[test]
    fn test_cache_remove() {
        let cache = ResourceCache::new(1024);

        cache.insert("keep", vec![1, 2, 3]);
        cache.insert("remove", vec![4, 5, 6, 7]);

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.size_bytes(), 7);

        let removed = cache.remove("remove");
        assert!(removed);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.size_bytes(), 3);
        assert!(!cache.contains("remove"));
        assert!(cache.contains("keep"));

        // Removing non-existent key returns false
        let removed2 = cache.remove("not.exists");
        assert!(!removed2);
    }

    #[test]
    fn test_cache_thread_safety() {
        let cache = Arc::new(ResourceCache::new(10000));
        let mut handles = vec![];

        // Spawn multiple threads that insert and read
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                let key = format!("resource.{}", i);
                let data = vec![i as u8; 100];

                // Insert
                cache_clone.insert(&key, data.clone());

                // Read back
                for _ in 0..100 {
                    if let Some(resource) = cache_clone.get(&key) {
                        assert_eq!(resource.data.len(), 100);
                    }
                    thread::yield_now();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().expect("Thread should complete");
        }

        // Cache should have entries
        assert!(cache.len() > 0);
    }

    #[test]
    fn test_cache_pinned_no_evict() {
        let cache = ResourceCache::new(100);

        // Insert and pin first entry
        cache.insert("pinned", vec![0; 40]);
        let pinned = cache.get("pinned").expect("Should get pinned");
        pinned.add_ref();

        thread::sleep(Duration::from_millis(10));

        // Insert second entry
        cache.insert("unpinned", vec![0; 40]);

        thread::sleep(Duration::from_millis(10));

        // Insert third entry - should evict "unpinned" not "pinned"
        cache.insert("third", vec![0; 40]);

        // "pinned" should NOT be evicted even though it's older
        assert!(cache.contains("pinned"), "pinned should not be evicted");
        assert!(!cache.contains("unpinned"), "unpinned should be evicted");
        assert!(cache.contains("third"), "third should exist");

        // Cleanup
        pinned.release();
    }

    #[test]
    fn test_cached_resource_ref_count() {
        let resource = CachedResource::new(vec![1, 2, 3]);

        assert_eq!(resource.ref_count(), 0);
        assert!(!resource.is_pinned());

        // Add reference
        assert_eq!(resource.add_ref(), 1);
        assert_eq!(resource.ref_count(), 1);
        assert!(resource.is_pinned());

        // Add another
        assert_eq!(resource.add_ref(), 2);
        assert_eq!(resource.ref_count(), 2);

        // Release one
        assert_eq!(resource.release(), 1);
        assert_eq!(resource.ref_count(), 1);
        assert!(resource.is_pinned());

        // Release last
        assert_eq!(resource.release(), 0);
        assert_eq!(resource.ref_count(), 0);
        assert!(!resource.is_pinned());
    }

    #[test]
    fn test_cached_resource_touch() {
        let resource = CachedResource::new(vec![1, 2, 3]);
        let initial_access = resource.last_access();

        thread::sleep(Duration::from_millis(10));
        resource.touch();

        let new_access = resource.last_access();
        assert!(new_access > initial_access);
    }

    #[test]
    fn test_cache_replace_existing() {
        let cache = ResourceCache::new(1024);

        cache.insert("key", vec![1, 2, 3]);
        assert_eq!(cache.size_bytes(), 3);

        // Replace with larger data
        cache.insert("key", vec![1, 2, 3, 4, 5]);
        assert_eq!(cache.len(), 1); // Still one entry
        assert_eq!(cache.size_bytes(), 5); // Size updated

        let resource = cache.get("key").expect("Should get");
        assert_eq!(resource.data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_cache_keys() {
        let cache = ResourceCache::new(1024);

        cache.insert("alpha", vec![1]);
        cache.insert("beta", vec![2]);
        cache.insert("gamma", vec![3]);

        let mut keys = cache.keys();
        keys.sort();

        assert_eq!(keys, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn test_cache_debug() {
        let cache = ResourceCache::new(1024);
        cache.insert("test", vec![1, 2, 3, 4, 5]);

        let debug_str = format!("{:?}", cache);
        assert!(debug_str.contains("ResourceCache"));
        assert!(debug_str.contains("max_size_bytes"));
        assert!(debug_str.contains("1024"));
    }

    #[test]
    fn test_evict_lru_manual() {
        let cache = ResourceCache::new(1000);

        cache.insert("a", vec![0; 100]);
        thread::sleep(Duration::from_millis(10));
        cache.insert("b", vec![0; 100]);
        thread::sleep(Duration::from_millis(10));
        cache.insert("c", vec![0; 100]);

        assert_eq!(cache.len(), 3);

        // Manually evict LRU
        let evicted = cache.evict_lru();
        assert!(evicted);
        assert_eq!(cache.len(), 2);
        assert!(!cache.contains("a")); // "a" was oldest
        assert!(cache.contains("b"));
        assert!(cache.contains("c"));
    }

    #[test]
    fn test_evict_lru_empty_cache() {
        let cache = ResourceCache::new(1000);

        // Evicting from empty cache should return false
        let evicted = cache.evict_lru();
        assert!(!evicted);
    }

    #[test]
    fn test_evict_lru_all_pinned() {
        let cache = ResourceCache::new(1000);

        cache.insert("pinned1", vec![0; 100]);
        cache.insert("pinned2", vec![0; 100]);

        // Pin both entries
        let p1 = cache.get("pinned1").unwrap();
        let p2 = cache.get("pinned2").unwrap();
        p1.add_ref();
        p2.add_ref();

        // Should not be able to evict anything
        let evicted = cache.evict_lru();
        assert!(!evicted);
        assert_eq!(cache.len(), 2);

        // Cleanup
        p1.release();
        p2.release();
    }
}
