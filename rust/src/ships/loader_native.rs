//! Ship resource loading and explicit descriptor teardown.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::RaceDesc;
    use crate::game_init::space_resources::{free_image, load_animation};
    use crate::graphics::{pixmap_abi::CaptureDrawable, resgfx::LoadGraphicInstance};
    use crate::sound::resource_types::{LoadMusicInstance, LoadSoundInstance};
    use crate::strings::string_table::CaptureStringTable;

    pub(crate) unsafe fn load_battle(d: &mut RaceDesc) -> bool {
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
        let data = d.data_mut();
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
}
