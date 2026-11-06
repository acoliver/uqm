//! Resource ownership follows a descriptor through attachment and detachment.
use crate::engine::battle::{frames::FrameArray, record::*, RaceDesc};
use crate::graphics::native_abi::{FRAME, STRING};
#[cfg(not(test))]
use crate::graphics::{drawable_abi::DestroyDrawable, pixmap_abi::ReleaseDrawable};
#[cfg(not(test))]
use crate::strings::{
    sresins::LoadStringTableInstance,
    string_table::{
        CaptureStringTable, DestroyStringTable, GetStringAddress, ReleaseStringTable,
        SetAbsStringTableIndex,
    },
};

#[derive(Debug)]
pub(crate) struct ShipAssets {
    icons: FRAME,
    melee_icon: FRAME,
    strings: STRING,
    name: Option<Vec<u8>>,
    captain: FRAME,
    victory: Option<crate::sound::resource_types::LoadedMusic>,
    sounds: Option<crate::sound::owned_bank::LoadedSoundBank>,
    animations: [FrameArray; 3],
}

impl RaceDescriptor {
    #[cfg(not(test))]
    pub(crate) fn victory_music(&self) -> Option<&crate::sound::resource_types::LoadedMusic> {
        self.assets
            .as_ref()
            .and_then(|assets| assets.victory.as_ref())
    }
    pub(crate) fn take_captain_background(&mut self) -> FRAME {
        let assets = self
            .assets
            .as_mut()
            .unwrap_or_else(|| panic!("captain release requires loaded ship assets"));
        self.ship_data.captain_control.background = std::ptr::null_mut();
        std::mem::replace(&mut assets.captain, std::ptr::null_mut())
    }
}

#[cfg(not(test))]
fn load_names(key: &std::ffi::CStr) -> (STRING, Option<Vec<u8>>) {
    // SAFETY: the key is borrowed through loading. Any returned name points into
    // this newly captured table, which stays owned while its bytes are copied.
    unsafe {
        let strings = CaptureStringTable(LoadStringTableInstance(key.as_ptr()));
        let name = GetStringAddress(SetAbsStringTableIndex(strings, 2));
        let name = (!name.is_null()).then(|| std::ffi::CStr::from_ptr(name).to_bytes().to_vec());
        (strings.cast(), name)
    }
}

#[cfg(not(test))]
fn load_animation_set(keys: [Option<&'static std::ffi::CStr>; 3]) -> Option<FrameArray> {
    let mut raw = [std::ptr::null_mut(); 3];
    if load_animation(&mut raw, keys, "animation") == 0 {
        free_image(&mut raw, "animation");
        return None;
    }
    let array = FrameArray::from_native(raw);
    array.observe("animation");
    Some(array)
}

#[cfg(not(test))]
impl RaceDesc {
    pub(crate) fn initialize_assets(&mut self, battle: bool) -> bool {
        let name = self.initialize_icons();
        let loaded = !battle || self.initialize_battle_assets();
        let data = &self.raw.ship_data;
        let info = self.info();
        self.raw.assets = Some(Box::new(ShipAssets {
            icons: info.icons,
            melee_icon: info.melee_icon,
            strings: info.race_strings,
            name,
            captain: data.captain_control.background,
            victory: self.raw.loaded_victory.take(),
            sounds: self.raw.loaded_sounds.take(),
            animations: [data.special.clone(), data.weapon.clone(), data.ship.clone()],
        }));
        loaded
    }

    fn initialize_icons(&mut self) -> Option<Vec<u8>> {
        let info = &mut self.raw.ship_info;
        if let Some(key) = self.raw.resources.icons {
            info.icons = crate::engine::battle::frames::load_graphic(key);
        }
        if let Some(key) = self.raw.resources.melee_icon {
            info.melee_icon = crate::engine::battle::frames::load_graphic(key);
        }
        if let Some(key) = self.raw.resources.strings {
            let (strings, name) = load_names(key);
            info.race_strings = strings;
            name
        } else {
            None
        }
    }

    fn initialize_battle_assets(&mut self) -> bool {
        let data = &mut self.raw.ship_data;
        let Some(ship) = load_animation_set(self.raw.resources.ship) else {
            return false;
        };
        data.ship = ship;
        for (keys, frames) in [
            (self.raw.resources.weapon, &mut data.weapon),
            (self.raw.resources.special, &mut data.special),
        ] {
            if keys[0].is_some() {
                let Some(loaded) = load_animation_set(keys) else {
                    return false;
                };
                *frames = loaded;
            }
        }
        self.initialize_battle_audio_and_captain()
    }

    fn initialize_battle_audio_and_captain(&mut self) -> bool {
        let data = &mut self.raw.ship_data;
        if let Some(key) = self.raw.resources.captain {
            data.captain_control.background = crate::engine::battle::frames::load_graphic(key);
            if data.captain_control.background.is_null() {
                return false;
            }
        }
        if let Some(key) = self.raw.resources.victory {
            self.raw.loaded_victory = crate::sound::resource_types::LoadedMusic::load(key);
            data.victory_ditty = self
                .raw
                .loaded_victory
                .as_ref()
                .map_or(std::ptr::null_mut(), |music| music.as_raw().cast());
            if self.raw.loaded_victory.is_none() {
                return false;
            }
        }
        if let Some(key) = self.raw.resources.sounds {
            self.raw.loaded_sounds = crate::sound::owned_bank::LoadedSoundBank::load(key);
            data.ship_sounds = self
                .raw
                .loaded_sounds
                .as_ref()
                .map_or(std::ptr::null_mut(), |bank| bank.as_raw().cast());
            if self.raw.loaded_sounds.is_none() {
                return false;
            }
        }
        true
    }

    pub fn unload(mut self, world: &mut crate::engine::World, icons: bool, battle: bool) {
        self.cleanup(world);
        crate::collections::observation::record(
            crate::collections::observation::Event::ResourcesFree {
                descriptor: self.address(),
                icons,
                battle,
            },
        );
        if let Some(assets) = self.raw.assets.take() {
            assets.release(icons, battle);
        }
    }

    pub(crate) fn catalog_name(&self) -> &[u8] {
        let assets = self
            .raw
            .assets
            .as_ref()
            .unwrap_or_else(|| panic!("catalog descriptor has no loaded assets"));
        assert!(
            !assets.strings.is_null(),
            "catalog string table was not loaded"
        );
        assets
            .name
            .as_deref()
            .unwrap_or_else(|| panic!("catalog string table has no ship name"))
    }
}

#[cfg(not(test))]
impl ShipAssets {
    fn release(self, icons: bool, battle: bool) {
        if battle {
            for array in &self.animations {
                let mut raw = array.views().map(|frame| frame.raw());
                free_image(&mut raw, "animation");
                array.retire_views();
            }
        }
        if battle {
            // SAFETY: the matching loader transferred this captured captain's release authority.
            unsafe { DestroyDrawable(ReleaseDrawable(self.captain.cast())) };
            drop(self.victory);
            drop(self.sounds);
        }
        if icons {
            // SAFETY: these handles originate in their matching loaders and are released once.
            unsafe {
                DestroyDrawable(ReleaseDrawable(self.melee_icon.cast()));
                DestroyDrawable(ReleaseDrawable(self.icons.cast()));
                DestroyStringTable(ReleaseStringTable(self.strings.cast()));
            }
        }
    }
}

#[cfg(not(test))]
use crate::engine::battle::frames::{free_image, load_animation};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_captain_release_consumes_captured_authority_not_the_mutable_view() {
        let captain = std::ptr::dangling_mut();
        let mut descriptor = RaceDesc::empty();
        descriptor.raw.assets = Some(Box::new(ShipAssets {
            icons: std::ptr::null_mut(),
            melee_icon: std::ptr::null_mut(),
            strings: std::ptr::null_mut(),
            name: None,
            captain,
            victory: None,
            sounds: None,
            animations: std::array::from_fn(|_| FrameArray::EMPTY),
        }));
        descriptor.raw.ship_data.captain_control.background = captain.wrapping_add(1);
        assert_eq!(descriptor.raw.take_captain_background(), captain);
        assert!(descriptor
            .raw
            .ship_data
            .captain_control
            .background
            .is_null());
        let assets = descriptor.raw.assets.as_ref().unwrap();
        assert!(assets.captain.is_null());
        assert!(assets.icons.is_null());
        assert!(assets.melee_icon.is_null());
        assert!(assets.strings.is_null());
        assert!(assets.name.is_none());
        assert!(assets.victory.is_none());
        assert!(assets.sounds.is_none());
        assert!(assets.animations.iter().all(FrameArray::is_absent));
        assert!(descriptor.raw.take_captain_background().is_null());
    }
}
