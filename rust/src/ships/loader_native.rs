//! Live descriptor/resource lifecycle translated from uqm/loadship.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::abi::{RACE_DESC, SPECIES_ID_NUM_SPECIES_ID};
    use std::ptr;

    use crate::ships::code_resources::engine::LoadCodeResInstance;

    use crate::ships::code_resources::engine::CaptureCodeRes;

    use crate::ships::code_resources::engine::ReleaseCodeRes;

    use crate::ships::code_resources::engine::DestroyCodeRes;

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

    use crate::game_init::space_resources::live::load_animation;

    use crate::game_init::space_resources::live::free_image;
    use crate::sound::heart_ffi::DestroyMusic;
    use crate::sound::heart_ffi::DestroySound;

    const CODE_RESOURCES: [Option<&std::ffi::CStr>; SPECIES_ID_NUM_SPECIES_ID as usize] = [
        None,
        Some(c"ship.arilou.code"),
        Some(c"ship.chmmr.code"),
        Some(c"ship.earthling.code"),
        Some(c"ship.orz.code"),
        Some(c"ship.pkunk.code"),
        Some(c"ship.shofixti.code"),
        Some(c"ship.spathi.code"),
        Some(c"ship.supox.code"),
        Some(c"ship.thraddash.code"),
        Some(c"ship.utwig.code"),
        Some(c"ship.vux.code"),
        Some(c"ship.yehat.code"),
        Some(c"ship.melnorme.code"),
        Some(c"ship.druuge.code"),
        Some(c"ship.ilwrath.code"),
        Some(c"ship.mycon.code"),
        Some(c"ship.slylandro.code"),
        Some(c"ship.umgah.code"),
        Some(c"ship.urquan.code"),
        Some(c"ship.zoqfotpik.code"),
        Some(c"ship.syreen.code"),
        Some(c"ship.kohrah.code"),
        Some(c"ship.androsynth.code"),
        Some(c"ship.chenjesu.code"),
        Some(c"ship.mmrnmhrm.code"),
        Some(c"ship.flagship.code"),
        Some(c"ship.samatra.code"),
        Some(c"ship.drone.code"),
    ];

    #[no_mangle]
    /// # Safety
    /// Call on the game thread with the ship catalog and resource system initialized.
    /// Species must identify a catalog entry; the returned descriptor must be freed once.
    pub unsafe extern "C" fn c_load_ship(species: u32, battle: u32) -> *mut RACE_DESC {
        let Some(key) = CODE_RESOURCES.get(species as usize) else {
            return ptr::null_mut();
        };
        let mut desc = ptr::null_mut();
        let code = CaptureCodeRes(
            LoadCodeResInstance(key.map_or(ptr::null(), |k| k.as_ptr())),
            (&raw mut crate::ships::native::abi::GlobData).cast(),
            &mut desc,
        );
        if code.is_null() {
            return ptr::null_mut();
        }
        let d = desc.cast::<RACE_DESC>();
        (*d).CodeRef = code;
        let info = &mut (*d).ship_info;
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
        if battle != 0 && !load_battle(d) {
            // Preserve the original failed-load contract: only CodeRef is freed.
            DestroyCodeRes(ReleaseCodeRes(code));
            return ptr::null_mut();
        }
        d
    }

    unsafe fn load_battle(d: *mut RACE_DESC) -> bool {
        let data = &mut (*d).ship_data;
        if load_animation(
            (&raw mut data.ship).cast(),
            data.ship_rsc[0],
            data.ship_rsc[1],
            data.ship_rsc[2],
        ) == 0
        {
            return false;
        }
        if !data.weapon_rsc[0].is_null()
            && load_animation(
                (&raw mut data.weapon).cast(),
                data.weapon_rsc[0],
                data.weapon_rsc[1],
                data.weapon_rsc[2],
            ) == 0
        {
            return false;
        }
        if !data.special_rsc[0].is_null()
            && load_animation(
                (&raw mut data.special).cast(),
                data.special_rsc[0],
                data.special_rsc[1],
                data.special_rsc[2],
            ) == 0
        {
            return false;
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

    #[no_mangle]
    /// # Safety
    /// Descriptor must be live and owned by the caller, with no outstanding borrows.
    /// Resource flags must match the parts loaded into this descriptor.
    pub unsafe extern "C" fn c_free_ship(d: *mut RACE_DESC, icons: u32, battle: u32) {
        if let Some(uninit) = (*d).uninit_func {
            uninit(d);
        }
        if battle != 0 {
            let data = &mut (*d).ship_data;
            free_image((&raw mut data.special).cast());
            free_image((&raw mut data.weapon).cast());
            free_image((&raw mut data.ship).cast());
            DestroyDrawable(ReleaseDrawable(data.captain_control.background.cast()));
            DestroyMusic(data.victory_ditty.cast());
            DestroySound((ReleaseStringTable(data.ship_sounds.cast())).cast());
        }
        if icons != 0 {
            let info = &mut (*d).ship_info;
            DestroyDrawable(ReleaseDrawable(info.melee_icon.cast()));
            DestroyDrawable(ReleaseDrawable(info.icons.cast()));
            DestroyStringTable(ReleaseStringTable(info.race_strings.cast()));
        }
        DestroyCodeRes(ReleaseCodeRes((*d).CodeRef));
    }

    #[no_mangle]
    /// # Safety
    /// Call on the game thread with the ship catalog and resource system initialized.
    /// Species must identify a catalog entry; the returned descriptor must be freed once.
    pub unsafe extern "C" fn load_ship(species: u32, battle: u32) -> *mut RACE_DESC {
        c_load_ship(species, battle)
    }
    #[no_mangle]
    /// # Safety
    /// Descriptor must be live and owned by the caller, with no outstanding borrows.
    /// Resource flags must match the parts loaded into this descriptor.
    pub unsafe extern "C" fn free_ship(d: *mut RACE_DESC, icons: u32, battle: u32) {
        c_free_ship(d, icons, battle);
    }
}

#[cfg(not(test))]
pub(crate) use engine::free_ship;
