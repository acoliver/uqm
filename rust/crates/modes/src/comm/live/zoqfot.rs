use crate::comm::host::Host;
use crate::comm::races::zoqfot::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;

fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn exit_conversation(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::ExitConversation, r));
}
fn zoq_fot_intro(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::ZoqFotIntro, r));
}
fn form_alliance(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::FormAlliance, r));
}
fn aquaint_zoq_fot(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::AquaintZoqFot, r));
}
fn zoq_fot_info(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::ZoqFotInfo, r));
}
fn zoq_fot_home(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::ZoqFotHome, r));
}

pub(super) fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
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

/// The phrase callback that makes `speaker` the talking head when the
/// track player reaches the line.
pub(super) fn speaker_callback(speaker: crate::comm::races::zoqfot::api::Speaker) -> fn(&mut Comm) {
    use crate::comm::races::zoqfot::api::{select_speaker, Speaker};
    fn select(comm: &mut Comm, speaker: Speaker) {
        select_speaker(
            &mut comm.data.definition,
            &mut comm.race_scratch.last_alien,
            speaker,
        );
    }
    match speaker {
        Speaker::Zoq => |comm| select(comm, Speaker::Zoq),
        Speaker::Pik => |comm| select(comm, Speaker::Pik),
    }
}

mod api {
    use super::*;

    use crate::comm::races::zoqfot::api::*;

    mod live {
        use super::*;
        use crate::comm::host::Host;
        use crate::comm::races::script::DialogueHost;
        impl ZoqApi for Host<'_, '_, Handler> {
            fn speak(&mut self, phrase: u16, speaker: Speaker) {
                let comm = &mut **self.comm.borrow_mut();

                crate::comm::ffi::npc_phrase(
                    self.cx.borrow().world,
                    comm,
                    i32::from(phrase),
                    Some(super::super::speaker_callback(speaker)),
                );
            }
            fn begin_talk(&mut self, track: u16) {
                {
                    let comm = &mut **self.comm.borrow_mut();
                    comm.race_scratch.last_alien = 1;
                    super::super::speaker_callback(Speaker::Zoq)(comm);
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
