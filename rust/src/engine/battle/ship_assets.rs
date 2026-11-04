//! Resource ownership follows a descriptor through attachment and detachment.
use crate::engine::battle::{frames::FrameArray, record::*, RaceDesc};
use crate::graphics::native_abi::{FRAME, STRING, STRING as SOUND};
use crate::graphics::{
    drawable_abi::DestroyDrawable,
    pixmap_abi::{CaptureDrawable, ReleaseDrawable},
    resgfx::LoadGraphicInstance,
};
use crate::sound::heart_ffi::{DestroyMusic, DestroySound};
use crate::strings::{
    sresins::LoadStringTableInstance,
    string_table::{CaptureStringTable, DestroyStringTable, ReleaseStringTable},
};

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

impl RaceDesc {
    pub(crate) fn data_mut(&mut self) -> &mut DATA_STUFF {
        &mut self.raw.ship_data
    }

    pub fn load(world: &mut crate::engine::World, species: u32, battle: bool) -> Option<Self> {
        let race = crate::ships::races::race(species)?;
        let mut descriptor = (race.descriptor)(world);
        // SAFETY: descriptors come from the fixed race table, whose resource keys are static C strings.
        // Loaded handles are captured here, not recovered from caller-mutable descriptor fields at teardown.
        let loaded = unsafe {
            let info = &mut descriptor.raw.ship_info;
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
            !battle || crate::ships::loader_native::engine::load_battle(&mut descriptor)
        };
        let data = &descriptor.raw.ship_data;
        let info = descriptor.info();
        descriptor.raw.assets = Some(Box::new(ShipAssets {
            icons: info.icons,
            melee_icon: info.melee_icon,
            strings: info.race_strings,
            captain: data.captain_control.background,
            victory: data.victory_ditty,
            sounds: data.ship_sounds,
            animations: [data.special.clone(), data.weapon.clone(), data.ship.clone()],
        }));
        if !loaded {
            descriptor.unload(world, true, true);
            return None;
        }
        Some(descriptor)
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

impl ShipAssets {
    fn release(self, icons: bool, battle: bool) {
        // SAFETY: each handle originates in the matching loader and this consumes its sole release authority.
        unsafe {
            if battle {
                for array in &self.animations {
                    let mut raw = array.views().map(|frame| frame.raw());
                    crate::game_init::space_resources::free_image(&mut raw, "animation");
                    array.retire_views();
                }
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
