//! Resource ownership follows a descriptor through attachment and detachment.
#[cfg(not(test))]
use crate::engine::battle::{frames::FrameArray, record::*, RaceDesc};
#[cfg(not(test))]
use crate::graphics::native_abi::{FRAME, STRING, STRING as SOUND};
#[cfg(not(test))]
use crate::graphics::{
    drawable_abi::DestroyDrawable,
    pixmap_abi::{CaptureDrawable, ReleaseDrawable},
    resgfx::LoadGraphicInstance,
};
#[cfg(not(test))]
use crate::sound::heart_ffi::{DestroyMusic, DestroySound};
#[cfg(not(test))]
use crate::strings::{
    sresins::LoadStringTableInstance,
    string_table::{CaptureStringTable, DestroyStringTable, ReleaseStringTable},
};

#[cfg(not(test))]
#[derive(Debug)]
pub(crate) struct ShipAssets {
    icons: FRAME,
    melee_icon: FRAME,
    strings: STRING,
    captain: FRAME,
    victory: MUSIC_REF,
    sounds: SOUND,
    animations: [FrameArray; 3],
}

#[cfg(not(test))]
fn load_animation_set(keys: [*const std::ffi::c_char; 3]) -> Option<FrameArray> {
    let mut raw = [std::ptr::null_mut(); 3];
    let keys = keys.map(|key| {
        // SAFETY: only the closed race factory supplies these static resource keys.
        (!key.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(key) })
    });
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
        self.initialize_icons();
        let loaded = !battle || self.initialize_battle_assets();
        let data = &self.raw.ship_data;
        let info = self.info();
        self.raw.assets = Some(Box::new(ShipAssets {
            icons: info.icons,
            melee_icon: info.melee_icon,
            strings: info.race_strings,
            captain: data.captain_control.background,
            victory: data.victory_ditty,
            sounds: data.ship_sounds,
            animations: [data.special.clone(), data.weapon.clone(), data.ship.clone()],
        }));
        loaded
    }

    fn initialize_icons(&mut self) {
        let info = &mut self.raw.ship_info;
        // SAFETY: the closed race factory supplies static resource keys. Each loader
        // transfers its handle to this descriptor's private release authority.
        unsafe {
            if !info.icons_rsc.is_null() {
                info.icons = CaptureDrawable(LoadGraphicInstance(info.icons_rsc).cast()).cast();
            }
            if !info.melee_icon_rsc.is_null() {
                info.melee_icon =
                    CaptureDrawable(LoadGraphicInstance(info.melee_icon_rsc).cast()).cast();
            }
            if !info.race_strings_rsc.is_null() {
                info.race_strings =
                    CaptureStringTable(LoadStringTableInstance(info.race_strings_rsc)).cast();
            }
        }
    }

    fn initialize_battle_assets(&mut self) -> bool {
        let data = &mut self.raw.ship_data;
        let Some(ship) = load_animation_set(data.ship_rsc) else {
            return false;
        };
        data.ship = ship;
        for (keys, frames) in [
            (data.weapon_rsc, &mut data.weapon),
            (data.special_rsc, &mut data.special),
        ] {
            if !keys[0].is_null() {
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
        // SAFETY: only the closed race factory constructs these static keys, and
        // captured handles remain privately owned even when a later load fails.
        unsafe {
            if !data.captain_control.captain_rsc.is_null() {
                data.captain_control.background =
                    CaptureDrawable(LoadGraphicInstance(data.captain_control.captain_rsc).cast())
                        .cast();
                if data.captain_control.background.is_null() {
                    return false;
                }
            }
            if !data.victory_ditty_rsc.is_null() {
                data.victory_ditty = LoadMusicInstance(data.victory_ditty_rsc).cast();
                if data.victory_ditty.is_null() {
                    return false;
                }
            }
            if !data.ship_sounds_rsc.is_null() {
                data.ship_sounds =
                    CaptureStringTable(LoadSoundInstance(data.ship_sounds_rsc).cast()).cast();
                if data.ship_sounds.is_null() {
                    return false;
                }
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

    pub(crate) fn catalog_name(&self) -> Vec<u8> {
        let assets = self
            .raw
            .assets
            .as_ref()
            .unwrap_or_else(|| panic!("catalog descriptor has no loaded assets"));
        assert!(
            !assets.strings.is_null(),
            "catalog string table was not loaded"
        );
        // SAFETY: this private handle was captured by load and cannot be changed through the legacy record.
        unsafe {
            use crate::strings::string_table::{GetStringAddress, SetAbsStringTableIndex};
            let name = GetStringAddress(SetAbsStringTableIndex(assets.strings.cast(), 2));
            assert!(!name.is_null(), "catalog string table has no ship name");
            std::ffi::CStr::from_ptr(name).to_bytes().to_vec()
        }
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
