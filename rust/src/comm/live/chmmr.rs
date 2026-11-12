use crate::comm::host::Host;
use crate::comm::races::chmmr::descriptor::descriptor;
use crate::comm::races::chmmr::*;
use crate::comm::{locdata::SessionData, Comm};
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn exit_conversation(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ExitConversation,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn not_ready(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::NotReady,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn improve_bomb(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ImproveBomb,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn chmmr_free(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ChmmrFree,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn chmmr_advice(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ChmmrAdvice,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn chmmr_shielded(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ChmmrShielded,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn after_bomb(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AfterBomb,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::ExitConversation => exit_conversation,
        Handler::NotReady => not_ready,
        Handler::ImproveBomb => improve_bomb,
        Handler::ChmmrFree => chmmr_free,
        Handler::ChmmrAdvice => chmmr_advice,
        Handler::ChmmrShielded => chmmr_shielded,
        Handler::AfterBomb => after_bomb,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_chmmr_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize the encounter on the game thread.
pub fn init_chmmr_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> SessionData {
    let desc = descriptor();
    run(comm, cx, |s| s.initialize());
    SessionData {
        init_encounter_func: Some(intro),
        post_encounter_func: Some(post),
        uninit_encounter_func: Some(uninit),
        ..SessionData::new(desc)
    }
}
