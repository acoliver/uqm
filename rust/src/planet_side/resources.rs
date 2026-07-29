//! RAII ownership for PlanetSide resources loaded through the Rust resource system.

use std::collections::HashMap;
use std::ffi::{c_void, CString};

use super::assets::LanderSoundTable;
use super::runtime::AdapterError;

const GRAPHIC_KEYS: [&str; 8] = [
    "graphics.lander",
    "graphics.quake",
    "graphics.lightning",
    "graphics.lavaspot",
    "graphics.landershield",
    "graphics.landerlaunch",
    "graphics.landerreturn",
    "graphics.orbview",
];
const SOUND_KEY: &str = "sounds.lander";

/// Typed index into the fixed PlanetSide graphic set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum LanderGraphic {
    Lander = 0,
    Earthquake = 1,
    Lightning = 2,
    Lava = 3,
    Shield = 4,
    Launch = 5,
    Return = 6,
    OrbitView = 7,
}

/// Resource-system boundary used for deterministic load/unload testing.
pub trait ResourcePort {
    fn load(&mut self, key: &'static str) -> Result<*mut c_void, AdapterError>;
    fn free(&mut self, key: &'static str);
}

/// Read-only access shared by owned test assets and production's globally
/// loaded lander asset set.
pub trait PlanetSideAssetAccess {
    fn graphic(&self, graphic: LanderGraphic) -> *mut c_void;
    fn sounds(&self) -> LanderSoundTable;
}

/// Scoped snapshot of the single production asset owner established by
/// `LoadLanderData` and released by `FreeLanderData`.
pub struct BorrowedPlanetSideAssets {
    pub(crate) graphics: [*mut c_void; 8],
    pub(crate) sounds: *mut c_void,
}

impl PlanetSideAssetAccess for BorrowedPlanetSideAssets {
    fn graphic(&self, graphic: LanderGraphic) -> *mut c_void {
        self.graphics[graphic as usize]
    }

    fn sounds(&self) -> LanderSoundTable {
        LanderSoundTable::from_raw(self.sounds)
    }
}

#[cfg(test)]
impl BorrowedPlanetSideAssets {
    pub(crate) fn for_test(graphics: [*mut c_void; 8], sounds: *mut c_void) -> Self {
        Self { graphics, sounds }
    }
}
/// Complete loaded PlanetSide asset set. Drop releases every successful load.
pub struct PlanetSideAssets<R: ResourcePort> {
    port: R,
    graphics: [*mut c_void; 8],
    sounds: *mut c_void,
    loaded_keys: Vec<&'static str>,
}

impl<R: ResourcePort> PlanetSideAssets<R> {
    pub fn load(mut port: R) -> Result<Self, AdapterError> {
        let mut graphics = [std::ptr::null_mut(); 8];
        let mut loaded_keys = Vec::with_capacity(9);
        for (index, key) in GRAPHIC_KEYS.into_iter().enumerate() {
            match port.load(key) {
                Ok(handle) if !handle.is_null() => {
                    graphics[index] = handle;
                    loaded_keys.push(key);
                }
                Ok(_) | Err(_) => {
                    for loaded in loaded_keys.iter().rev() {
                        port.free(loaded);
                    }
                    return Err(AdapterError::new("load_planet_side_graphic"));
                }
            }
        }
        let sounds = match port.load(SOUND_KEY) {
            Ok(handle) if !handle.is_null() => {
                loaded_keys.push(SOUND_KEY);
                handle
            }
            Ok(_) | Err(_) => {
                for loaded in loaded_keys.iter().rev() {
                    port.free(loaded);
                }
                return Err(AdapterError::new("load_planet_side_sound"));
            }
        };
        Ok(Self {
            port,
            graphics,
            sounds,
            loaded_keys,
        })
    }

    #[must_use]
    pub fn graphic(&self, graphic: LanderGraphic) -> *mut c_void {
        self.graphics[graphic as usize]
    }

    #[must_use]
    pub const fn sounds(&self) -> LanderSoundTable {
        LanderSoundTable::from_raw(self.sounds)
    }
}

impl<R: ResourcePort> PlanetSideAssetAccess for PlanetSideAssets<R> {
    fn graphic(&self, graphic: LanderGraphic) -> *mut c_void {
        self.graphic(graphic)
    }

    fn sounds(&self) -> LanderSoundTable {
        self.sounds()
    }
}

impl<R: ResourcePort> Drop for PlanetSideAssets<R> {
    fn drop(&mut self) {
        for key in self.loaded_keys.iter().rev() {
            self.port.free(key);
        }
    }
}

/// Production resource adapter. Loaded resources are detached, captured into
/// the handle type required by drawing/audio, and released symmetrically.
#[derive(Default)]
pub struct CffiResourcePort {
    handles: HashMap<&'static str, *mut c_void>,
}

#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn CaptureDrawable(drawable: *mut c_void) -> *mut c_void;
    fn ReleaseDrawable(frame: *mut c_void) -> *mut c_void;
    fn DestroyDrawable(drawable: *mut c_void);
    fn CaptureSound(sound: *mut c_void) -> *mut c_void;
    fn ReleaseSound(sound: *mut c_void) -> *mut c_void;
}

impl ResourcePort for CffiResourcePort {
    fn load(&mut self, key: &'static str) -> Result<*mut c_void, AdapterError> {
        let c_key = CString::new(key).map_err(|_| AdapterError::new("resource_key"))?;
        let raw = unsafe { crate::resource::ffi_bridge::res_GetResource(c_key.as_ptr()) };
        if raw.is_null() {
            return Err(AdapterError::new("load_resource"));
        }
        let detached = unsafe { crate::resource::ffi_bridge::res_DetachResource(c_key.as_ptr()) };
        if detached.is_null() {
            unsafe { crate::resource::ffi_bridge::res_FreeResource(c_key.as_ptr()) };
            return Err(AdapterError::new("detach_resource"));
        }

        #[cfg(feature = "linked_c_archive")]
        let captured = unsafe {
            if key == SOUND_KEY {
                CaptureSound(detached)
            } else {
                CaptureDrawable(detached)
            }
        };
        #[cfg(not(feature = "linked_c_archive"))]
        let captured = detached;

        if captured.is_null() {
            #[cfg(feature = "linked_c_archive")]
            unsafe {
                if key == SOUND_KEY {
                    crate::sound::heart_ffi::DestroySound(ReleaseSound(detached));
                } else {
                    DestroyDrawable(ReleaseDrawable(detached));
                }
            }
            return Err(AdapterError::new("capture_resource"));
        }
        self.handles.insert(key, captured);
        Ok(captured)
    }

    fn free(&mut self, key: &'static str) {
        let Some(handle) = self.handles.remove(key) else {
            return;
        };
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            if key == SOUND_KEY {
                crate::sound::heart_ffi::DestroySound(ReleaseSound(handle));
            } else {
                DestroyDrawable(ReleaseDrawable(handle));
            }
        }
        #[cfg(not(feature = "linked_c_archive"))]
        let _ = handle;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use super::*;

    struct Port {
        results: VecDeque<Result<usize, AdapterError>>,
        freed: Arc<Mutex<Vec<&'static str>>>,
    }

    impl ResourcePort for Port {
        fn load(&mut self, _key: &'static str) -> Result<*mut c_void, AdapterError> {
            self.results
                .pop_front()
                .unwrap_or(Ok(1))
                .map(|value| value as *mut c_void)
        }

        fn free(&mut self, key: &'static str) {
            self.freed.lock().unwrap().push(key);
        }
    }

    #[test]
    fn complete_asset_set_releases_in_reverse_order() {
        let freed = Arc::new(Mutex::new(Vec::new()));
        let assets = PlanetSideAssets::load(Port {
            results: VecDeque::new(),
            freed: Arc::clone(&freed),
        })
        .unwrap();
        assert!(!assets.graphic(LanderGraphic::Lander).is_null());
        assert!(assets.sounds().is_loaded());
        drop(assets);
        let mut expected = GRAPHIC_KEYS.to_vec();
        expected.push(SOUND_KEY);
        expected.reverse();
        assert_eq!(*freed.lock().unwrap(), expected);
    }

    #[test]
    fn partial_graphic_failure_rolls_back_successful_loads() {
        let freed = Arc::new(Mutex::new(Vec::new()));
        let result = PlanetSideAssets::load(Port {
            results: VecDeque::from([Ok(1), Ok(2), Err(AdapterError::new("injected"))]),
            freed: Arc::clone(&freed),
        });
        assert!(matches!(
            result,
            Err(AdapterError {
                operation: "load_planet_side_graphic"
            })
        ));
        assert_eq!(*freed.lock().unwrap(), [GRAPHIC_KEYS[1], GRAPHIC_KEYS[0]]);
    }

    #[test]
    fn sound_failure_releases_all_graphics() {
        let freed = Arc::new(Mutex::new(Vec::new()));
        let mut results = (0..8).map(|_| Ok(1)).collect::<VecDeque<_>>();
        results.push_back(Ok(0));
        let result = PlanetSideAssets::load(Port {
            results,
            freed: Arc::clone(&freed),
        });
        assert!(matches!(
            result,
            Err(AdapterError {
                operation: "load_planet_side_sound"
            })
        ));
        let mut expected = GRAPHIC_KEYS.to_vec();
        expected.reverse();
        assert_eq!(*freed.lock().unwrap(), expected);
    }
}
