use crate::comm::host::Host;
use crate::comm::races::utwig::descriptor::descriptor;
use crate::comm::races::utwig::*;
use crate::comm::{locdata::SessionData, Comm};
use crate::engine::battle::Cx;

fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn allied_home(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AlliedHome,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn exit_conversation(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ExitConversation,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn before_kohr_ah(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::BeforeKohrAh,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn after_kohr_ah(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AfterKohrAh,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn neutral_utwig(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::NeutralUtwig,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn bomb_world(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::BombWorld,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}

fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::AlliedHome => allied_home,
        Handler::ExitConversation => exit_conversation,
        Handler::BeforeKohrAh => before_kohr_ah,
        Handler::AfterKohrAh => after_kohr_ah,
        Handler::NeutralUtwig => neutral_utwig,
        Handler::BombWorld => bomb_world,
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
pub fn init_utwig_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> SessionData {
    let mut desc = descriptor();
    run(comm, cx, |s| s.initialize(&mut desc));
    SessionData {
        init_encounter_func: Some(intro),
        post_encounter_func: Some(post),
        uninit_encounter_func: Some(uninit),
        ..SessionData::new(desc)
    }
}
