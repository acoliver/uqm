use super::Handler;
use crate::comm::races::encounter::EncounterApi;

pub(super) trait SlylandApi: EncounterApi<Handler> {
    fn choices(&self) -> [u16; 5];
    fn remember_choices(&mut self, choices: [u16; 5]);
    fn position(&self) -> (i32, i32, i32, i32);
    fn splice(&mut self, phrase: u32);
    fn number(&mut self, number: i32, padded: bool);
}

#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;

    // Like the original response references, these belong to the game thread.
    static mut CHOICES: [u16; 5] = [0; 5];

    use crate::comm::commglue_abi::runtime::NPCNumber;

    extern "C" {
        static mut GlobData: crate::ships::native::abi::GLOBDATA;
        static mut ScreenWidth: i32;
        static mut ScreenHeight: i32;
    }
    impl SlylandApi for LiveScript<Handler> {
        fn choices(&self) -> [u16; 5] {
            unsafe { CHOICES }
        }
        fn remember_choices(&mut self, choices: [u16; 5]) {
            unsafe {
                CHOICES = choices;
            }
        }
        fn position(&self) -> (i32, i32, i32, i32) {
            unsafe {
                (
                    GlobData.SIS_state.log_x,
                    GlobData.SIS_state.log_y,
                    ScreenWidth,
                    ScreenHeight,
                )
            }
        }
        fn splice(&mut self, phrase: u32) {
            unsafe {
                crate::comm::ffi::rust_npc_phrase_splice(phrase as i32);
            }
        }
        fn number(&mut self, number: i32, padded: bool) {
            unsafe {
                NPCNumber(
                    number,
                    if padded {
                        c"%03d".as_ptr()
                    } else {
                        std::ptr::null()
                    },
                );
            }
        }
    }
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Speech {
    Splice(u32),
    Number(i32, bool),
}
#[cfg(test)]
pub(super) struct World {
    pub choices: [u16; 5],
    pub position: (i32, i32, i32, i32),
    pub speech: Vec<(usize, Speech)>,
}
#[cfg(test)]
impl Default for World {
    fn default() -> Self {
        Self {
            choices: [0; 5],
            position: (0, 0, 320, 240),
            speech: Vec::new(),
        }
    }
}
#[cfg(test)]
impl SlylandApi for crate::comm::races::encounter::testing::Fake<Handler, World> {
    fn choices(&self) -> [u16; 5] {
        self.world.choices
    }
    fn remember_choices(&mut self, choices: [u16; 5]) {
        self.world.choices = choices;
    }
    fn position(&self) -> (i32, i32, i32, i32) {
        self.world.position
    }
    fn splice(&mut self, phrase: u32) {
        self.world
            .speech
            .push((self.effects.len(), Speech::Splice(phrase)));
    }
    fn number(&mut self, number: i32, padded: bool) {
        self.world
            .speech
            .push((self.effects.len(), Speech::Number(number, padded)));
    }
}
