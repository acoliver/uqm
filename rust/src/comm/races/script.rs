//! Shared boundary for dialogue scripts while the surrounding engine still uses C.

use crate::comm::locdata::{CAnimationDesc, CLocData};

pub(super) const ANIM_DISABLED: u8 = 1 << 7;
pub(super) const ONE_SECOND: u16 = 840;
pub(super) const FUEL_TANK_SCALE: u32 = 100;
pub(super) const GLOBAL_PLAYER_NAME: i32 = -1_000_000;
pub(super) const GLOBAL_SHIP_NAME: i32 = -999_999;

#[derive(Clone, Copy, Default)]
pub(super) struct Supplies {
    pub landers: u8,
    pub fuel: u32,
    pub radioactives: u16,
}

pub(super) trait ScriptApi<H> {
    fn get(&self, key: &str) -> u8;
    fn set(&mut self, key: &str, value: u8);
    fn npc(&mut self, phrase: i32);
    fn response(&mut self, phrase: u16, handler: H, fragments: &[i32]);
    fn enabled(&self, phrase: u16) -> bool;
    fn disable(&mut self, phrase: u16);
    fn supplies(&self) -> Supplies;
    fn add_lander(&mut self);
    fn gauges(&mut self, crew: i16, fuel: i16, units: i32);
    fn palette(&mut self, index: u16);
    fn talk_segue(&mut self, track: u16);
    fn disable_animation(&mut self, index: usize);
    fn fade_palette(&mut self, index: u16, duration: i16);
}

pub(super) fn animation(
    start_index: u16,
    num_frames: u8,
    anim_flags: u8,
    frame_rate: (u16, u16),
    restart_rate: (u16, u16),
) -> CAnimationDesc {
    CAnimationDesc {
        start_index,
        num_frames,
        anim_flags,
        base_frame_rate: frame_rate.0,
        random_frame_rate: frame_rate.1,
        base_restart_rate: restart_rate.0,
        random_restart_rate: restart_rate.1,
        block_mask: 0,
    }
}

#[cfg(not(test))]
pub(super) mod live {
    use super::*;
    use crate::comm::locdata::{CGlobData, COMM_DATA};
    use std::ffi::{c_char, c_void, CStr};

    extern "C" {
        static mut GlobData: CGlobData;
        static mut shared_phrase_buf: [c_char; 2048];
        fn SetAbsStringTableIndex(table: *mut c_void, index: u16) -> *mut c_void;
        fn GetStringAddress(entry: *mut c_void) -> *const c_char;
        fn GetColorMapAddress(map: *mut c_void) -> *mut c_void;
        fn XFormColorMap(map: *mut c_void, duration: i16) -> u32;
        fn DrawLanders();
        fn DeltaSISGauges(crew: i16, fuel: i16, units: i32);
        fn AlienTalkSegue(track: u16);
    }

    // Callbacks and live globals belong to the game thread, as in the C scripts.
    pub(in crate::comm::races) struct LiveScript<H> {
        pub callback: fn(H) -> extern "C" fn(u32),
    }

    impl<H> ScriptApi<H> for LiveScript<H> {
        fn get(&self, key: &str) -> u8 {
            let (start, end) = crate::state::game_state_keys::bit_range(key)
                .expect("dialogue uses an unknown game-state key");
            unsafe { crate::state::ffi::rust_get_game_state_bits(start as i32, end as i32) }
        }

        fn set(&mut self, key: &str, value: u8) {
            let (start, end) = crate::state::game_state_keys::bit_range(key)
                .expect("dialogue uses an unknown game-state key");
            unsafe { crate::state::ffi::rust_set_game_state_bits(start as i32, end as i32, value) }
        }

        fn npc(&mut self, phrase: i32) {
            unsafe { crate::comm::ffi::rust_NPCPhrase_cb(phrase, None) }
        }

        fn response(&mut self, phrase: u16, handler: H, fragments: &[i32]) {
            unsafe {
                let text = if fragments.is_empty() {
                    GetStringAddress(SetAbsStringTableIndex(
                        COMM_DATA.conversation_phrases,
                        phrase - 1,
                    ))
                } else {
                    // Concatenate bytes exactly as construct_response does, including
                    // the spaces already present in the translated phrase fragments.
                    let mut text = Vec::new();
                    for &fragment in fragments {
                        let ptr = match fragment {
                            GLOBAL_PLAYER_NAME => {
                                std::ptr::addr_of!(GlobData.sis_state.commander_name).cast()
                            }
                            GLOBAL_SHIP_NAME => {
                                std::ptr::addr_of!(GlobData.sis_state.ship_name).cast()
                            }
                            _ => {
                                let index =
                                    u16::try_from(fragment - 1).expect("invalid response fragment");
                                GetStringAddress(SetAbsStringTableIndex(
                                    COMM_DATA.conversation_phrases,
                                    index,
                                ))
                            }
                        };
                        text.extend_from_slice(CStr::from_ptr(ptr).to_bytes());
                    }
                    assert!(
                        text.len() < 2048,
                        "constructed dialogue response exceeds shared buffer"
                    );
                    let buf = std::ptr::addr_of_mut!(shared_phrase_buf).cast::<u8>();
                    std::ptr::copy_nonoverlapping(text.as_ptr(), buf, text.len());
                    buf.add(text.len()).write(0);
                    buf.cast()
                };
                crate::comm::ffi::rust_DoResponsePhrase(
                    u32::from(phrase),
                    text,
                    Some((self.callback)(handler)),
                );
            }
        }

        fn enabled(&self, phrase: u16) -> bool {
            unsafe { crate::comm::ffi::rust_PhraseEnabled(i32::from(phrase)) != 0 }
        }

        fn disable(&mut self, phrase: u16) {
            unsafe { crate::comm::ffi::rust_DisablePhrase(i32::from(phrase)) }
        }

        fn supplies(&self) -> Supplies {
            unsafe {
                Supplies {
                    landers: GlobData.sis_state.num_landers,
                    fuel: GlobData.sis_state.fuel_on_board,
                    radioactives: GlobData.sis_state.element_amounts[6],
                }
            }
        }

        fn add_lander(&mut self) {
            unsafe {
                GlobData.sis_state.num_landers += 1;
                DrawLanders();
            }
        }

        fn gauges(&mut self, crew: i16, fuel: i16, units: i32) {
            unsafe { DeltaSISGauges(crew, fuel, units) }
        }

        fn palette(&mut self, index: u16) {
            unsafe {
                COMM_DATA.alien_colormap = SetAbsStringTableIndex(COMM_DATA.alien_colormap, index)
            }
        }

        fn talk_segue(&mut self, track: u16) {
            unsafe { AlienTalkSegue(track) }
        }

        fn disable_animation(&mut self, index: usize) {
            unsafe { COMM_DATA.alien_ambient_array[index].anim_flags |= ANIM_DISABLED }
        }

        fn fade_palette(&mut self, index: u16, duration: i16) {
            unsafe {
                let map = SetAbsStringTableIndex(COMM_DATA.alien_colormap, index);
                XFormColorMap(GetColorMapAddress(map), duration);
            }
        }
    }
}

pub(super) fn as_comm_data(desc: &CLocData) -> crate::comm::types::CommData {
    // SAFETY: the descriptor is a live, aligned CLocData for this call.
    unsafe { crate::comm::locdata::read_locdata_from_c(std::ptr::from_ref(desc).cast()) }
}
