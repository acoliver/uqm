//! Resource ownership follows a descriptor through attachment and detachment.
#[cfg(not(test))]
use crate::engine::battle::{frames::FrameArray, record::*, RaceDesc};
#[cfg(not(test))]
use crate::graphics::native_abi::{FRAME, STRING, STRING as SOUND};
#[cfg(not(test))]
use crate::graphics::{drawable_abi::DestroyDrawable, pixmap_abi::ReleaseDrawable};
#[cfg(not(test))]
use crate::sound::heart_ffi::{DestroyMusic, DestroySound};
#[cfg(not(test))]
use crate::strings::{
    sresins::LoadStringTableInstance,
    string_table::{
        CaptureStringTable, DestroyStringTable, GetStringAddress, ReleaseStringTable,
        SetAbsStringTableIndex,
    },
};

#[cfg(not(test))]
#[derive(Debug)]
pub(crate) struct ShipAssets {
    icons: FRAME,
    melee_icon: FRAME,
    strings: STRING,
    name: Option<Vec<u8>>,
    captain: FRAME,
    victory: MUSIC_REF,
    sounds: SOUND,
    animations: [FrameArray; 3],
}

#[cfg(not(test))]
fn load_music(key: &std::ffi::CStr) -> MUSIC_REF {
    // SAFETY: the borrowed key stays live through decoding and the descriptor owns the result.
    unsafe { LoadMusicInstance(key.as_ptr()).cast() }
}

#[cfg(not(test))]
fn load_sounds(key: &std::ffi::CStr) -> SOUND {
    // SAFETY: the borrowed key stays live through decoding and the descriptor owns the captured bank.
    unsafe { CaptureStringTable(LoadSoundInstance(key.as_ptr()).cast()).cast() }
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
            victory: data.victory_ditty,
            sounds: data.ship_sounds,
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
            data.victory_ditty = load_music(key);
            if data.victory_ditty.is_null() {
                return false;
            }
        }
        if let Some(key) = self.raw.resources.sounds {
            data.ship_sounds = load_sounds(key);
            if data.ship_sounds.is_null() {
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
        // SAFETY: each handle originates in the matching loader and this consumes its sole release authority.
        unsafe {
            if battle {
                DestroyDrawable(ReleaseDrawable(self.captain.cast()));
                DestroyMusic(self.victory.cast());
                DestroySound(ReleaseStringTable(self.sounds.cast()).cast());
            }
            if icons {
                DestroyDrawable(ReleaseDrawable(self.melee_icon.cast()));
                DestroyDrawable(ReleaseDrawable(self.icons.cast()));
                DestroyStringTable(ReleaseStringTable(self.strings.cast()));
            }
        }
    }
}

#[cfg(not(test))]
use crate::engine::battle::frames::{free_image, load_animation};
#[cfg(not(test))]
use crate::sound::resource_types::{LoadMusicInstance, LoadSoundInstance};
