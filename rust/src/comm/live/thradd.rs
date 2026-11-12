use crate::comm::host::Host;
use crate::comm::races::thradd::descriptor::descriptor;
use crate::comm::races::thradd::*;
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
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn thradd_allies(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ThraddAllies,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn thradd_demeanor(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ThraddDemeanor,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn thradd_culture(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ThraddCulture,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn helix_world(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::HelixWorld,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn thradd_hostile(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ThraddHostile,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::ExitConversation => exit_conversation,
        Handler::ThraddAllies => thradd_allies,
        Handler::ThraddDemeanor => thradd_demeanor,
        Handler::ThraddCulture => thradd_culture,
        Handler::HelixWorld => helix_world,
        Handler::ThraddHostile => thradd_hostile,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_thradd_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize this encounter on the game thread.
pub fn init_thradd_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> SessionData {
    let desc = descriptor();
    run(comm, cx, |s| s.initialize());
    SessionData {
        init_encounter_func: Some(intro),
        post_encounter_func: Some(post),
        uninit_encounter_func: Some(uninit),
        ..SessionData::new(desc)
    }
}
