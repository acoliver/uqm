use crate::comm::host::Host;
use crate::comm::locdata::SessionData;
use crate::comm::races::slyland::descriptor::descriptor;
use crate::comm::races::slyland::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;

fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}

fn combat(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Combat, response as i32)
    });
}

fn callback(_: Handler) -> crate::comm::response::ResponseFunc {
    combat
}

pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |dialogue| dialogue.intro());
}
pub(crate) fn post(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
pub fn init_slyland_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> crate::comm::locdata::SessionData {
    let width = cx.screen_dimensions().0;
    let mut d = descriptor(width);
    d.alien_text_baseline.x = (1 + ((width - 80) >> 1)) as i16;
    d.alien_text_width = (width - 80 - 16) as u16;
    run(comm, cx, |s| s.initialize());
    SessionData {
        init_encounter_func: Some(intro),
        post_encounter_func: Some(post),
        uninit_encounter_func: Some(uninit),
        ..SessionData::new(d)
    }
}

mod api {

    use super::Handler;

    use crate::comm::races::slyland::api::*;

    #[cfg(not(test))]
    mod live {
        use super::*;
        use crate::comm::host::Host;

        // Like the original response references, these belong to the game thread.

        use crate::comm::commglue_abi::runtime::npc_number;

        extern "C" {

            static mut ScreenWidth: i32;
            static mut ScreenHeight: i32;
        }
        impl SlylandApi for Host<'_, '_, Handler> {
            fn choices(&self) -> [u16; 5] {
                self.comm.borrow().race_scratch.slyland_choices
            }
            fn remember_choices(&mut self, choices: [u16; 5]) {
                self.cx.borrow_mut().with_legacy(|| {
                    self.comm.borrow_mut().race_scratch.slyland_choices = choices;
                })
            }
            fn position(&self) -> (i32, i32, i32, i32) {
                let cx = self.cx.borrow();
                // SAFETY: display dimensions are stable on the game thread holding this campaign loan.
                unsafe {
                    (
                        cx.world.data.flagship.log_x,
                        cx.world.data.flagship.log_y,
                        ScreenWidth,
                        ScreenHeight,
                    )
                }
            }
            fn splice(&mut self, phrase: u32) {
                let comm = &mut **self.comm.borrow_mut();
                // SAFETY: the borrowed session retains its loaded phrase resources while splicing.
                unsafe {
                    crate::comm::ffi::rust_npc_phrase_splice(
                        self.cx.borrow().world,
                        comm,
                        phrase as i32,
                    );
                }
            }
            fn number(&mut self, number: i32, padded: bool) {
                let comm = &mut **self.comm.borrow_mut();
                // SAFETY: the session retains its number table, and the optional format is a static C string.
                self.cx.borrow_mut().with_legacy(|| unsafe {
                    npc_number(
                        comm,
                        number,
                        if padded {
                            c"%03d".as_ptr()
                        } else {
                            std::ptr::null()
                        },
                    );
                })
            }
        }
    }
}
