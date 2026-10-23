//! Live descriptor/resource lifecycle translated from uqm/loadship.c.
#[cfg(any(not(test), feature = "linked_c_archive"))]
mod engine {
    use crate::ships::native::abi::{RACE_DESC, SPECIES_ID_NUM_SPECIES_ID};
    use std::{
        ffi::{c_char, c_int, c_void},
        ptr,
    };

    extern "C" {
        fn LoadCodeResInstance(res: *const c_char) -> *mut c_void;
        fn CaptureCodeRes(
            code: *mut c_void,
            globals: *mut c_void,
            out: *mut *mut c_void,
        ) -> *mut c_void;
        fn ReleaseCodeRes(code: *mut c_void) -> *mut c_void;
        fn DestroyCodeRes(code: *mut c_void) -> u32;
        fn LoadGraphicInstance(res: *const c_char) -> *mut c_void;
        fn CaptureDrawable(drawable: *mut c_void) -> *mut c_void;
        fn ReleaseDrawable(frame: *mut c_void) -> *mut c_void;
        fn DestroyDrawable(drawable: *mut c_void) -> u32;
        fn LoadStringTableInstance(res: *const c_char) -> *mut c_void;
        fn CaptureStringTable(table: *mut c_void) -> *mut c_void;
        fn ReleaseStringTable(string: *mut c_void) -> *mut c_void;
        fn DestroyStringTable(table: *mut c_void) -> c_int;
        fn LoadMusicInstance(res: *const c_char) -> *mut c_void;
        fn DestroyMusic(music: *mut c_void);
        fn LoadSoundInstance(res: *const c_char) -> *mut c_void;
        fn DestroySound(sound: *mut c_void);
        fn load_animation(
            frames: *mut c_void,
            big: *const c_char,
            medium: *const c_char,
            small: *const c_char,
        ) -> u32;
        fn free_image(frames: *mut c_void) -> u32;
    }

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
            info.icons = CaptureDrawable(LoadGraphicInstance(info.icons_rsc)).cast();
        }
        if !info.melee_icon_rsc.is_null() {
            info.melee_icon = CaptureDrawable(LoadGraphicInstance(info.melee_icon_rsc)).cast();
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
                CaptureDrawable(LoadGraphicInstance(data.captain_control.captain_rsc)).cast();
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
            data.ship_sounds = CaptureStringTable(LoadSoundInstance(data.ship_sounds_rsc)).cast();
            if data.ship_sounds.is_null() {
                return false;
            }
        }
        true
    }

    #[no_mangle]
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
            DestroySound(ReleaseStringTable(data.ship_sounds.cast()));
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
    pub unsafe extern "C" fn load_ship(species: u32, battle: u32) -> *mut RACE_DESC {
        c_load_ship(species, battle)
    }
    #[no_mangle]
    pub unsafe extern "C" fn free_ship(d: *mut RACE_DESC, icons: u32, battle: u32) {
        c_free_ship(d, icons, battle);
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub(crate) use engine::free_ship;
