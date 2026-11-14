use crate::comm::descriptors::arilou::descriptor;
use crate::comm::host::Host;
use crate::comm::races::arilou::*;
use crate::comm::{locdata::CLocData, Comm};
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
fn arilou_home(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ArilouHome,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn angry_home_arilou(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AngryHomeArilou,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn angry_space_arilou(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AngrySpaceArilou,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn friendly_space_arilou(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::FriendlySpaceArilou,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::ExitConversation => exit_conversation,
        Handler::ArilouHome => arilou_home,
        Handler::AngryHomeArilou => angry_home_arilou,
        Handler::AngrySpaceArilou => angry_space_arilou,
        Handler::FriendlySpaceArilou => friendly_space_arilou,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_arilou_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize this encounter on the game thread.
pub fn init_arilou_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> CLocData {
    let desc = descriptor();
    run(comm, cx, |s| s.initialize());
    desc
}
