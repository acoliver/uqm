use crate::comm::descriptors::chmmr::descriptor;
use crate::comm::host::Host;
use crate::comm::races::chmmr::*;
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
pub fn init_chmmr_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> CLocData {
    let desc = descriptor();
    run(comm, cx, |s| s.initialize());
    desc
}

mod api {

    use super::Handler;

    use crate::comm::races::chmmr::api::*;
    use crate::engine::StoryKey;

    impl ChmmrApi for crate::comm::host::Host<'_, '_, Handler> {
        fn refit(&mut self) {
            use crate::comm::races::script::DialogueHost;
            let starbase = self.get(StoryKey::STARBASE_AVAILABLE) != 0;

            {
                refit(&mut self.cx.borrow_mut().world.data, starbase);
            }
        }
        fn intro_mode(&mut self, mode: i32, duration: i32) {
            let comm = &mut **self.comm.borrow_mut();
            self.cx.borrow_mut().with_legacy(|| {
                use crate::comm::comm_abi::set_comm_intro_mode;
                set_comm_intro_mode(comm, mode as u32, duration as u32);
            })
        }
    }
}
