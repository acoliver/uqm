//! Live descriptor/resource lifecycle translated from uqm/loadship.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::RaceDesc;
    use crate::engine::World as WorldOwner;

    use crate::graphics::resgfx::LoadGraphicInstance;

    use crate::graphics::pixmap_abi::CaptureDrawable;

    use crate::graphics::pixmap_abi::ReleaseDrawable;

    use crate::graphics::drawable_abi::DestroyDrawable;

    use crate::strings::sresins::LoadStringTableInstance;

    use crate::strings::string_table::CaptureStringTable;

    use crate::strings::string_table::ReleaseStringTable;

    use crate::strings::string_table::DestroyStringTable;

    use crate::sound::resource_types::LoadMusicInstance;

    use crate::sound::resource_types::LoadSoundInstance;

    use crate::game_init::space_resources::load_animation;

    use crate::game_init::space_resources::free_image;
    use crate::sound::heart_ffi::DestroyMusic;
    use crate::sound::heart_ffi::DestroySound;

    /// # Safety
    /// The caller owns the resource system on the game thread.
    pub unsafe fn load_ship(species: u32, battle: u32) -> Option<RaceDesc> {
        let race = crate::ships::races::race(species)?;
        let mut owned = (race.descriptor)(&mut **WorldOwner::current());
        let info = &mut *owned.info_pointer();
        if !info.icons_rsc.is_null() {
            info.icons = CaptureDrawable((LoadGraphicInstance(info.icons_rsc)).cast()).cast();
        }
        if !info.melee_icon_rsc.is_null() {
            info.melee_icon =
                CaptureDrawable((LoadGraphicInstance(info.melee_icon_rsc)).cast()).cast();
        }
        if !info.race_strings_rsc.is_null() {
            info.race_strings =
                CaptureStringTable(LoadStringTableInstance(info.race_strings_rsc)).cast();
        }
        if battle != 0 && !load_battle(&mut owned) {
            unload_ship(owned, 1, 1);
            return None;
        }
        Some(owned)
    }

    unsafe fn load_battle(d: &mut RaceDesc) -> bool {
        let load_animation_set = |keys: [_; 3]| {
            use crate::engine::battle::frames::FrameArray;
            let mut raw = [std::ptr::null_mut(); 3];
            if load_animation(raw.as_mut_ptr(), keys[0], keys[1], keys[2], "animation") == 0 {
                free_image(&raw mut raw, "animation");
                return None;
            }
            let array = FrameArray::from_native(raw);
            array.observe("animation");
            Some(array)
        };
        let data = &mut *d.data_pointer();
        let Some(ship) = load_animation_set(data.ship_rsc) else {
            return false;
        };
        data.ship = ship;
        if !data.weapon_rsc[0].is_null() {
            let Some(weapon) = load_animation_set(data.weapon_rsc) else {
                return false;
            };
            data.weapon = weapon;
        }
        if !data.special_rsc[0].is_null() {
            let Some(special) = load_animation_set(data.special_rsc) else {
                return false;
            };
            data.special = special;
        }
        if !data.captain_control.captain_rsc.is_null() {
            data.captain_control.background =
                CaptureDrawable((LoadGraphicInstance(data.captain_control.captain_rsc)).cast())
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
                CaptureStringTable((LoadSoundInstance(data.ship_sounds_rsc)).cast()).cast();
            if data.ship_sounds.is_null() {
                return false;
            }
        }
        true
    }

    /// # Safety
    /// The descriptor resources are exclusively owned by this value.
    pub unsafe fn unload_ship(mut owned: RaceDesc, icons: u32, battle: u32) {
        let release_animation_set = |array: &crate::engine::battle::frames::FrameArray| {
            let mut raw = array.views().map(|frame| frame.raw());
            free_image(&raw mut raw, "animation");
            array.retire_views();
        };

        let descriptor = &mut owned;
        descriptor.cleanup(&mut **WorldOwner::current());
        crate::collections::observation::record(
            crate::collections::observation::Event::ResourcesFree {
                descriptor: descriptor.address(),
                icons: icons != 0,
                battle: battle != 0,
            },
        );
        if battle != 0 {
            let data = &mut *descriptor.data_pointer();
            release_animation_set(&data.special);
            data.special = crate::engine::battle::frames::FrameArray::EMPTY;
            release_animation_set(&data.weapon);
            data.weapon = crate::engine::battle::frames::FrameArray::EMPTY;
            release_animation_set(&data.ship);
            data.ship = crate::engine::battle::frames::FrameArray::EMPTY;
            DestroyDrawable(ReleaseDrawable(data.captain_control.background.cast()));
            DestroyMusic(data.victory_ditty.cast());
            DestroySound((ReleaseStringTable(data.ship_sounds.cast())).cast());
        }
        if icons != 0 {
            let info = &mut *descriptor.info_pointer();
            DestroyDrawable(ReleaseDrawable(info.melee_icon.cast()));
            DestroyDrawable(ReleaseDrawable(info.icons.cast()));
            DestroyStringTable(ReleaseStringTable(info.race_strings.cast()));
        }
    }
}

#[cfg(not(test))]
pub use engine::unload_ship;

use crate::engine::World as WorldOwner;
