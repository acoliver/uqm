use crate::comm::descriptors::syreen::descriptor;
use crate::comm::host::Host;
use crate::comm::races::syreen::*;
use crate::comm::{locdata::CLocData, Comm};
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn friendly_exit(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::FriendlyExit,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn sex(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(Handler::Sex, i32::try_from(r).expect("invalid response"))
    });
}
fn foreplay(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::Foreplay,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn after_ambush(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AfterAmbush,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn ambush_ready(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AmbushReady,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn syreen_shuttle(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SyreenShuttle,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn normal_syreen(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::NormalSyreen,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn initial_syreen(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::InitialSyreen,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn syreen_vault(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SyreenVault,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::FriendlyExit => friendly_exit,
        Handler::Sex => sex,
        Handler::Foreplay => foreplay,
        Handler::AfterAmbush => after_ambush,
        Handler::AmbushReady => ambush_ready,
        Handler::SyreenShuttle => syreen_shuttle,
        Handler::NormalSyreen => normal_syreen,
        Handler::InitialSyreen => initial_syreen,
        Handler::SyreenVault => syreen_vault,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_syreen_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize the encounter on the game thread.
pub fn init_syreen_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> CLocData {
    let desc = descriptor();
    run(comm, cx, |s| s.initialize());
    desc
}

mod api {

    use super::Handler;

    use crate::comm::races::syreen::api::*;

    impl SyreenApi for crate::comm::host::Host<'_, '_, Handler> {
        fn using_speech(&self) -> bool {
            self.cx.borrow().kernel.using_speech != 0
        }
    }
}
