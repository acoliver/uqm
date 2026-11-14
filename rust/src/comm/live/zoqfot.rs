use crate::comm::host::Host;
use crate::comm::races::zoqfot::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;

fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn exit_conversation(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ExitConversation,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn zoq_fot_intro(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ZoqFotIntro,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn form_alliance(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::FormAlliance,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn aquaint_zoq_fot(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AquaintZoqFot,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn zoq_fot_info(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ZoqFotInfo,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn zoq_fot_home(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ZoqFotHome,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}

fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::ExitConversation => exit_conversation,
        Handler::ZoqFotIntro => zoq_fot_intro,
        Handler::FormAlliance => form_alliance,
        Handler::AquaintZoqFot => aquaint_zoq_fot,
        Handler::ZoqFotInfo => zoq_fot_info,
        Handler::ZoqFotHome => zoq_fot_home,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize the encounter on the game thread.
pub(crate) fn initialize(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    desc: &mut crate::comm::races::descriptor::CommDescriptor,
) {
    run(comm, cx, |s| s.initialize(desc));
}

mod api {
    use super::*;

    use crate::comm::races::zoqfot::api::*;
    use crate::comm::Comm;

    mod live {
        use super::*;
        use crate::comm::host::Host;
        use crate::comm::races::script::DialogueHost;
        fn select(comm: &mut Comm, speaker: Speaker) {
            select_speaker(
                &mut comm.data.definition,
                &mut comm.race_scratch.last_alien,
                speaker,
            );
        }
        fn zoq(comm: &mut Comm) {
            select(comm, Speaker::Zoq);
        }
        fn pik(comm: &mut Comm) {
            select(comm, Speaker::Pik);
        }
        impl ZoqApi for Host<'_, '_, Handler> {
            fn speak(&mut self, phrase: u16, speaker: Speaker) {
                let comm = &mut **self.comm.borrow_mut();

                // SAFETY: the session owns the phrase table and retains its speaker callback until playback ends.
                unsafe {
                    crate::comm::ffi::rust_npc_phrase_cb(
                        self.cx.borrow().world,
                        comm,
                        i32::from(phrase),
                        Some(match speaker {
                            Speaker::Zoq => zoq,
                            Speaker::Pik => pik,
                        }),
                    );
                }
            }
            fn begin_talk(&mut self, track: u16) {
                {
                    let comm = &mut **self.comm.borrow_mut();
                    comm.race_scratch.last_alien = 1;
                    select(comm, Speaker::Zoq);
                }
                self.talk_segue(track);
            }
            fn date(&self) -> (i16, u8) {
                let cx = self.cx.borrow();
                let clock = &cx.world.data.game.game_clock;
                (clock.year_index as i16, clock.month_index)
            }
        }
    }
}
