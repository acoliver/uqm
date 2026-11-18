// Resource bridge for the two-tier ship loader.
// @plan PLAN-20260314-SHIPS.P05

use super::types::ShipsError;

// ---------------------------------------------------------------------------
// Opaque handle types (matching C library conventions)
// ---------------------------------------------------------------------------

/// Opaque handle to a C DRAWABLE (graphic/frame).
pub type DrawableHandle = usize;

/// Opaque handle to a C MUSIC object.
pub type MusicHandle = usize;

/// Opaque handle to a C SOUND object.
pub type SoundHandle = usize;

/// Opaque handle to a C STRING_TABLE.
pub type StringTableHandle = usize;

/// Sentinel value for NULL_RESOURCE (C: NULL_RESOURCE == 0).
pub const NULL_RESOURCE: u32 = 0;

/// Resource loading and release supplied to the loader by its caller.
///
/// `NULL_RESOURCE` loads to the zero handle and the zero handle is never
/// released, matching the C loader; implementations see only real requests.
pub trait ResourceBridge {
    fn graphic(&mut self, res_id: u32) -> Result<DrawableHandle, ShipsError>;
    fn music(&mut self, res_id: u32) -> Result<MusicHandle, ShipsError>;
    fn sound(&mut self, res_id: u32) -> Result<SoundHandle, ShipsError>;
    fn string_table(&mut self, res_id: u32) -> Result<StringTableHandle, ShipsError>;
    fn release(&mut self, handle: usize);

    fn load_graphic(&mut self, res_id: u32) -> Result<DrawableHandle, ShipsError> {
        load(res_id, |id| self.graphic(id))
    }
    fn load_music(&mut self, res_id: u32) -> Result<MusicHandle, ShipsError> {
        load(res_id, |id| self.music(id))
    }
    fn load_sound(&mut self, res_id: u32) -> Result<SoundHandle, ShipsError> {
        load(res_id, |id| self.sound(id))
    }
    fn load_string_table(&mut self, res_id: u32) -> Result<StringTableHandle, ShipsError> {
        load(res_id, |id| self.string_table(id))
    }
    fn free_graphic(&mut self, handle: DrawableHandle) {
        free(handle, |handle| self.release(handle));
    }
    fn free_music(&mut self, handle: MusicHandle) {
        free(handle, |handle| self.release(handle));
    }
    fn free_sound(&mut self, handle: SoundHandle) {
        free(handle, |handle| self.release(handle));
    }
    fn free_string_table(&mut self, handle: StringTableHandle) {
        free(handle, |handle| self.release(handle));
    }
}

fn load(res_id: u32, load: impl FnOnce(u32) -> Result<usize, ShipsError>) -> Result<usize, ShipsError> {
    if res_id == NULL_RESOURCE {
        Ok(0)
    } else {
        load(res_id)
    }
}

fn free(handle: usize, release: impl FnOnce(usize)) {
    if handle != 0 {
        release(handle);
    }
}

/// The registry's descriptors carry no resource ids: every `*_res` field is
/// `NULL_RESOURCE`, so the production loader never requests or releases one.
pub struct RegistryResources;

impl ResourceBridge for RegistryResources {
    fn graphic(&mut self, res_id: u32) -> Result<DrawableHandle, ShipsError> {
        panic!("registry descriptors carry no graphic resource ids (got {res_id})")
    }
    fn music(&mut self, res_id: u32) -> Result<MusicHandle, ShipsError> {
        panic!("registry descriptors carry no music resource ids (got {res_id})")
    }
    fn sound(&mut self, res_id: u32) -> Result<SoundHandle, ShipsError> {
        panic!("registry descriptors carry no sound resource ids (got {res_id})")
    }
    fn string_table(&mut self, res_id: u32) -> Result<StringTableHandle, ShipsError> {
        panic!("registry descriptors carry no string table resource ids (got {res_id})")
    }
    fn release(&mut self, handle: usize) {
        panic!("registry descriptors own no resource handles (got {handle:#x})")
    }
}

/// Recording resources for loader tests, with failure injection.
#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use std::collections::HashSet;

    #[derive(Default)]
    pub(crate) struct MockResources {
        next: usize,
        allocated: HashSet<usize>,
        freed: Vec<usize>,
        fail_on: u32,
    }

    impl MockResources {
        /// Make every later load of `res_id` fail; zero disables injection.
        pub(crate) fn fail_on(&mut self, res_id: u32) {
            self.fail_on = res_id;
        }
        pub(crate) fn allocated_count(&self) -> usize {
            self.allocated.len()
        }
        pub(crate) fn is_allocated(&self, handle: usize) -> bool {
            self.allocated.contains(&handle)
        }
        pub(crate) fn free_count(&self, handle: usize) -> usize {
            self.freed.iter().filter(|&&freed| freed == handle).count()
        }
        fn allocate(&mut self, kind: &str, res_id: u32) -> Result<usize, ShipsError> {
            if self.fail_on == res_id {
                return Err(ShipsError::LoadFailed(format!(
                    "Mock failure: load_{kind} for resource {res_id}"
                )));
            }
            self.next += 1;
            self.allocated.insert(self.next);
            Ok(self.next)
        }
    }

    impl ResourceBridge for MockResources {
        fn graphic(&mut self, res_id: u32) -> Result<DrawableHandle, ShipsError> {
            self.allocate("graphic", res_id)
        }
        fn music(&mut self, res_id: u32) -> Result<MusicHandle, ShipsError> {
            self.allocate("music", res_id)
        }
        fn sound(&mut self, res_id: u32) -> Result<SoundHandle, ShipsError> {
            self.allocate("sound", res_id)
        }
        fn string_table(&mut self, res_id: u32) -> Result<StringTableHandle, ShipsError> {
            self.allocate("string_table", res_id)
        }
        fn release(&mut self, handle: usize) {
            self.freed.push(handle);
            self.allocated.remove(&handle);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::mock::MockResources;
    use super::*;

    #[test]
    fn null_resource_returns_zero() {
        let mut res = RegistryResources;
        assert_eq!(res.load_graphic(NULL_RESOURCE).unwrap(), 0);
        assert_eq!(res.load_music(NULL_RESOURCE).unwrap(), 0);
        assert_eq!(res.load_sound(NULL_RESOURCE).unwrap(), 0);
        assert_eq!(res.load_string_table(NULL_RESOURCE).unwrap(), 0);
    }

    #[test]
    #[should_panic(expected = "registry descriptors carry no graphic resource ids")]
    fn registry_resources_reject_a_real_resource_id() {
        let _ = RegistryResources.load_graphic(7);
    }

    #[test]
    fn mock_allocates_unique_handles() {
        let mut res = MockResources::default();
        let h1 = res.load_graphic(1).unwrap();
        let h2 = res.load_graphic(2).unwrap();
        let h3 = res.load_music(3).unwrap();
        assert_ne!(h1, 0);
        assert_ne!(h2, 0);
        assert_ne!(h3, 0);
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_eq!(res.allocated_count(), 3);

        res.free_graphic(h1);
        assert!(!res.is_allocated(h1));
        assert_eq!(res.free_count(h1), 1);
        res.free_graphic(h2);
        res.free_music(h3);
        assert_eq!(res.allocated_count(), 0);
    }

    #[test]
    fn free_zero_handle_is_noop() {
        let mut res = RegistryResources;
        res.free_graphic(0);
        res.free_music(0);
        res.free_sound(0);
        res.free_string_table(0);
    }

    #[test]
    fn failure_injection_works() {
        let mut res = MockResources::default();
        res.fail_on(42);
        assert!(res.load_graphic(42).is_err());
        assert!(res.load_music(42).is_err());
        assert!(res.load_sound(42).is_err());
        assert!(res.load_string_table(42).is_err());
        // Other resource IDs still succeed
        let h = res.load_graphic(1).unwrap();
        assert_ne!(h, 0);
        res.free_graphic(h);
        assert_eq!(res.allocated_count(), 0);
    }
}
