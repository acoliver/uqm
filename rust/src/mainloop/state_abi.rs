//! Shared state entry points formerly supplied by rust_bridge_mainloop.c.
use crate::io::uio_bridge::{uio_DirHandle, uio_MountHandle, uio_Repository};
use crate::navigation::globals::GlobData;
use crate::ships::native::abi as options;
use std::{ffi::c_char, ptr};
extern "C" {
    static mut contentMountHandle: *mut uio_MountHandle;
    #[link_name = "NextActivity"]
    static mut NEXT_ACTIVITY: u16;
}

pub(crate) use crate::navigation::{get as game_byte, set as set_game_byte};

macro_rules! accessor {
    ($name:ident() -> $ret:ty = $body:expr) => {
        /// # Safety
        /// Called on the owner game thread with live shared state.
        pub unsafe fn $name() -> $ret {
            $body
        }
    };
    ($name:ident($arg:ident: $ty:ty) = $body:expr) => {
        /// # Safety
        /// Called on the owner game thread; pointers obey the underlying provider contract.
        pub unsafe fn $name($arg: $ty) {
            $body
        }
    };
}
accessor!(set_current_activity(value: u16) = GlobData.Game_state.CurrentActivity = value);
accessor!(get_next_activity() -> u16 = NEXT_ACTIVITY);
accessor!(set_next_activity(value: u16) = NEXT_ACTIVITY = value);
accessor!(get_last_activity() -> u16 = crate::game_init::kernel_abi::LastActivity);
accessor!(set_last_activity(value: u16) = crate::game_init::kernel_abi::LastActivity = value);
accessor!(uqm_get_chmmr_bomb_state() -> u8 = game_byte("CHMMR_BOMB_STATE"));
accessor!(uqm_set_chmmr_bomb_state(value: u8) = set_game_byte("CHMMR_BOMB_STATE", value));
accessor!(uqm_get_starbase_available() -> u8 = game_byte("STARBASE_AVAILABLE"));
accessor!(uqm_get_global_flags_and_data() -> u8 = game_byte("GLOBAL_FLAGS_AND_DATA"));
accessor!(uqm_get_kohr_ah_killed_all() -> u8 = game_byte("KOHR_AH_KILLED_ALL"));
accessor!(uqm_get_crew_enlisted() -> u16 = GlobData.SIS_state.CrewEnlisted);
accessor!(uqm_zero_global_velocity() -> () = { GlobData.Game_state.velocity = std::mem::zeroed(); });
accessor!(uqm_set_flash_rect_null() -> () = crate::ui::sis::SetFlashRect(ptr::null()));
accessor!(uqm_set_player_input_all_or_explode() -> () = {
    if !crate::game_init::kernel_abi::SetPlayerInputAll() { log::error!("Could not set player input."); std::process::abort(); }
});
accessor!(set_main_exited(value: i32) = ptr::addr_of_mut!(super::lifecycle_abi::MainExited).write_volatile(value));
accessor!(uqm_init_audio() -> () = { crate::sound::audio_init::initAudio(crate::sound::volume_globals::snddriver, crate::sound::volume_globals::soundflags); });
accessor!(uqm_get_repository() -> *mut uio_Repository = crate::game_init::kernel_abi::repository);
accessor!(uqm_get_config_dir() -> *mut uio_DirHandle = options::configDir.cast());
accessor!(uqm_get_content_dir() -> *mut uio_DirHandle = options::contentDir.cast());
accessor!(uqm_get_content_mount_handle() -> *mut uio_MountHandle = contentMountHandle.cast());
accessor!(uqm_set_content_dir(value: *mut uio_DirHandle) = options::contentDir = value.cast());
accessor!(uqm_set_config_dir(value: *mut uio_DirHandle) = options::configDir = value.cast());
accessor!(uqm_set_save_dir(value: *mut uio_DirHandle) = options::saveDir = value.cast());
accessor!(uqm_set_melee_dir(value: *mut uio_DirHandle) = options::meleeDir = value.cast());
accessor!(uqm_set_content_mount_handle(value: *mut uio_MountHandle) = contentMountHandle = value.cast());
accessor!(uqm_set_base_content_path(path: *const c_char) = {
    let destination = ptr::addr_of_mut!(options::baseContentPath).cast::<c_char>();
    libc::strncpy(destination, path, super::dir_prep::PATH_MAX - 1);
    destination.add(super::dir_prep::PATH_MAX - 1).write(0);
});

type Queue = crate::navigation::abi::QUEUE;
accessor!(rust_get_avail_race_queue() -> *mut Queue = ptr::addr_of_mut!(GlobData.Game_state.avail_race_q));
accessor!(rust_get_npc_built_ship_queue() -> *mut Queue = ptr::addr_of_mut!(GlobData.Game_state.npc_built_ship_q));
accessor!(rust_get_encounter_queue() -> *mut Queue = ptr::addr_of_mut!(GlobData.Game_state.encounter_q));
accessor!(rust_get_built_ship_queue() -> *mut Queue = ptr::addr_of_mut!(GlobData.Game_state.built_ship_q));
accessor!(rust_get_ip_group_queue() -> *mut Queue = ptr::addr_of_mut!(GlobData.Game_state.ip_group_q));
accessor!(rust_set_cur_star_desc_ptr_null() -> () = crate::planets::starmap::CurStarDescPtr = ptr::null_mut());

/// # Safety
/// Data is null or points to a complete live LOCDATA on the game thread.
pub unsafe fn rust_copy_locdata_to_comm_data(data: *const std::ffi::c_void) {
    if !data.is_null() {
        crate::comm::locdata::COMM_DATA = data.cast::<crate::comm::locdata::CLocData>().read();
    }
}
