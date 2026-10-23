use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Speaker {
    Zoq,
    Pik,
}
pub(super) trait ZoqApi: AllianceApi<Handler> {
    fn speak(&mut self, phrase: u16, speaker: Speaker);
    fn begin_talk(&mut self, track: u16);
    fn date(&self) -> (i16, u8);
}

pub(super) fn select_speaker(desc: &mut CLocData, last_alien: &mut u8, speaker: Speaker) {
    let (alien, transition, talk, frames, x) = match speaker {
        Speaker::Zoq => (0, 23, 18, 5, 61),
        Speaker::Pik => (2, 26, 29, 2, 181),
    };
    if *last_alien == alien {
        return;
    }
    if *last_alien != 1 {
        desc.alien_transition_desc.anim_flags |= 1 << 6;
    }
    *last_alien = alien;
    desc.alien_transition_desc.anim_flags |= 1 << 5;
    desc.alien_transition_desc.start_index = transition;
    desc.alien_talk_desc.start_index = talk;
    desc.alien_talk_desc.num_frames = frames;
    match speaker {
        Speaker::Zoq => desc.alien_ambient_array[1].anim_flags &= !(1 << 3),
        Speaker::Pik => desc.alien_ambient_array[1].anim_flags |= 1 << 3,
    }
    desc.alien_text_baseline = CPoint { x, y: 24 };
    desc.alien_text_fcolor = CColor {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    desc.alien_text_bcolor = CColor {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
}

// These callbacks run synchronously on the game thread, when the queued phrase
// begins. Selecting at queue time would apply the final speaker to every line.
#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::locdata::{CGlobData, COMM_DATA};
    use crate::comm::races::script::{live::LiveScript, ScriptApi};
    static mut LAST_ALIEN: u8 = 1;
    extern "C" {
        static mut GlobData: CGlobData;
    }
    unsafe fn select(speaker: Speaker) {
        select_speaker(
            &mut *std::ptr::addr_of_mut!(COMM_DATA),
            &mut *std::ptr::addr_of_mut!(LAST_ALIEN),
            speaker,
        );
    }
    unsafe extern "C" fn zoq() {
        select(Speaker::Zoq);
    }
    unsafe extern "C" fn pik() {
        select(Speaker::Pik);
    }
    impl ZoqApi for LiveScript<Handler> {
        fn speak(&mut self, phrase: u16, speaker: Speaker) {
            unsafe {
                crate::comm::ffi::rust_npc_phrase_cb(
                    i32::from(phrase),
                    Some(match speaker {
                        Speaker::Zoq => zoq,
                        Speaker::Pik => pik,
                    }),
                );
            }
        }
        fn begin_talk(&mut self, track: u16) {
            unsafe {
                LAST_ALIEN = 1;
                select(Speaker::Zoq);
            }
            self.talk_segue(track);
        }
        fn date(&self) -> (i16, u8) {
            unsafe {
                (
                    GlobData.game_state.game_clock.year_index as i16,
                    GlobData.game_state.game_clock.month_index,
                )
            }
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) struct Clock {
    pub year: i16,
    pub month: u8,
}
#[cfg(test)]
impl Default for Clock {
    fn default() -> Self {
        Self {
            year: 2155,
            month: 2,
        }
    }
}
#[cfg(test)]
impl ZoqApi for crate::comm::races::encounter::testing::Fake<Handler, Clock> {
    fn speak(&mut self, phrase: u16, speaker: Speaker) {
        self.effects
            .push(crate::comm::races::encounter::testing::Effect::Speaker(
                phrase,
                speaker == Speaker::Pik,
            ));
        crate::comm::races::script::ScriptApi::npc(self, i32::from(phrase));
    }
    fn begin_talk(&mut self, track: u16) {
        crate::comm::races::script::ScriptApi::talk_segue(self, track);
    }
    fn date(&self) -> (i16, u8) {
        (self.world.year, self.world.month)
    }
}
