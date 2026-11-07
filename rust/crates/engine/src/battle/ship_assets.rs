//! Resource ownership follows a descriptor through attachment and detachment.
use crate::battle::{frames::FrameArray, record::*, RaceDesc};
use crate::graphics::native_abi::FRAME;
#[cfg(not(test))]
use crate::graphics::{drawable_abi::DestroyDrawable, pixmap_abi::ReleaseDrawable};
use crate::strings::owned_table::LoadedTextTable;

#[derive(Debug)]
pub struct ShipAssets {
    icons: FRAME,
    melee_icon: FRAME,
    strings: Option<LoadedTextTable>,
    captain: FRAME,
    victory: Option<crate::sound::resource_types::LoadedMusic>,
    sounds: Option<crate::sound::owned_bank::LoadedSoundBank>,
    animations: [FrameArray; 3],
}

impl RaceDesc {
    pub fn label(&self, index: u16) -> (&std::ffi::CStr, u16) {
        self.raw.label(index)
    }

    pub fn picker_resources(&self) -> crate::battle::link_queue::ShipPickerResources {
        let assets = self
            .raw
            .assets
            .as_ref()
            .unwrap_or_else(|| panic!("picker resources require loaded assets"));
        crate::battle::link_queue::ShipPickerResources::new(
            assets.strings.as_ref().and_then(|table| table.entry(0)),
            crate::battle::frames::FrameSelection::retain(assets.icons),
        )
    }
}

impl RaceDescriptor {
    pub fn label(&self, index: u16) -> (&std::ffi::CStr, u16) {
        self.assets
            .as_ref()
            .unwrap_or_else(|| panic!("ship labels require loaded assets"))
            .strings
            .as_ref()
            .map_or((c"", 0), |table| table.label(index))
    }
    #[cfg(not(test))]
    pub fn sound_bank(&self) -> Option<&crate::sound::owned_bank::LoadedSoundBank> {
        self.assets
            .as_ref()
            .and_then(|assets| assets.sounds.as_ref())
    }

    #[cfg(not(test))]
    pub fn victory_music(&self) -> Option<&crate::sound::resource_types::LoadedMusic> {
        self.assets
            .as_ref()
            .and_then(|assets| assets.victory.as_ref())
    }
    pub fn take_captain_background(&mut self) -> FRAME {
        let assets = self
            .assets
            .as_mut()
            .unwrap_or_else(|| panic!("captain release requires loaded ship assets"));
        self.ship_data.captain_control.background = std::ptr::null_mut();
        std::mem::replace(&mut assets.captain, std::ptr::null_mut())
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
    pub fn initialize_assets(&mut self, battle: bool) -> bool {
        let strings = self.initialize_icons();
        let loaded = !battle || self.initialize_battle_assets();
        let data = &self.raw.ship_data;
        let info = self.info();
        self.raw.assets = Some(Box::new(ShipAssets {
            icons: info.icons,
            melee_icon: info.melee_icon,
            strings,
            captain: data.captain_control.background,
            victory: self.raw.loaded_victory.take(),
            sounds: self.raw.loaded_sounds.take(),
            animations: [data.special.clone(), data.weapon.clone(), data.ship.clone()],
        }));
        loaded
    }

    fn initialize_icons(&mut self) -> Option<LoadedTextTable> {
        let info = &mut self.raw.ship_info;
        if let Some(key) = self.raw.resources.icons {
            info.icons = crate::battle::frames::load_graphic(key);
        }
        if let Some(key) = self.raw.resources.melee_icon {
            info.melee_icon = crate::battle::frames::load_graphic(key);
        }
        let strings = self.raw.resources.strings.and_then(LoadedTextTable::load);
        info.race_strings = strings
            .as_ref()
            .map_or(std::ptr::null_mut(), |table| table.as_raw().cast());
        strings
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
            data.captain_control.background = crate::battle::frames::load_graphic(key);
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

    pub fn unload(mut self, world: &mut crate::World, icons: bool, battle: bool) {
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

    pub fn catalog_name(&self) -> &[u8] {
        let assets = self
            .raw
            .assets
            .as_ref()
            .unwrap_or_else(|| panic!("catalog descriptor has no loaded assets"));
        assets
            .strings
            .as_ref()
            .and_then(|table| table.text(2))
            .unwrap_or_else(|| panic!("catalog string table has no ship name"))
            .to_bytes()
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
            }
        } else if let Some(strings) = self.strings {
            // The legacy campaign teardown retains the probe's native release authority.
            strings.into_raw();
        }
    }
}

#[cfg(not(test))]
use crate::battle::frames::{free_image, load_animation};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_labels_borrow_the_owned_table_and_keep_character_bounds() {
        use crate::strings::string_table::*;
        let mut descriptor = RaceDesc::empty();
        // SAFETY: this fixture exclusively owns the captured table until the final release.
        unsafe {
            let table = AllocStringTable(2, 0);
            let entry = CaptureStringTable(table);
            let bytes = b"Z\xc3\xb6q\0extra\0";
            (*entry).data = crate::memory::rust_hmalloc(bytes.len()).cast();
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), (*entry).data.cast(), bytes.len());
            (*entry).length = bytes.len() as i32;
            descriptor.raw.assets = Some(Box::new(ShipAssets {
                icons: std::ptr::null_mut(),
                melee_icon: std::ptr::null_mut(),
                strings: entry.cast(),
                name: None,
                captain: std::ptr::null_mut(),
                victory: None,
                sounds: None,
                animations: std::array::from_fn(|_| FrameArray::EMPTY),
            }));
            let (text, count) = descriptor.raw.label(0);
            assert_eq!(text.to_bytes(), "Zöq".as_bytes());
            assert_eq!(count, 3);
            assert_eq!(text.as_ptr(), (*entry).data);
            assert_eq!(descriptor.raw.label(2), (text, count));
            assert_eq!(descriptor.raw.label(1), (c"", 0));
            DestroyStringTable(ReleaseStringTable(entry));
        }
    }

    #[test]
    fn early_captain_release_consumes_captured_authority_not_the_mutable_view() {
        let captain = std::ptr::dangling_mut();
        let mut descriptor = RaceDesc::empty();
        descriptor.raw.assets = Some(Box::new(ShipAssets {
            icons: std::ptr::null_mut(),
            melee_icon: std::ptr::null_mut(),
            strings: None,
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
        assert!(assets.strings.is_none());
        assert!(assets.victory.is_none());
        assert!(assets.sounds.is_none());
        assert!(assets.animations.iter().all(FrameArray::is_absent));
        assert!(descriptor.raw.take_captain_background().is_null());
    }
}
