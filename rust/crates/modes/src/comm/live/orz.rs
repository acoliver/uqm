use crate::comm::host::Host;
use crate::comm::races::orz::*;
use crate::comm::Comm;
use uqm_engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn exit_conversation(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::ExitConversation, r));
}
fn taalo_world(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::TaaloWorld, r));
}
fn orz_allied(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::OrzAllied, r));
}
fn offer_alliance(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::OfferAlliance, r));
}
fn orz_neutral(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::OrzNeutral, r));
}
fn orz_angry(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::OrzAngry, r));
}
pub(super) fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::ExitConversation => exit_conversation,
        Handler::TaaloWorld => taalo_world,
        Handler::OrzAllied => orz_allied,
        Handler::OfferAlliance => offer_alliance,
        Handler::OrzNeutral => orz_neutral,
        Handler::OrzAngry => orz_angry,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_orz_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize this encounter on the game thread.
pub(crate) fn initialize(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    _desc: &mut crate::comm::races::descriptor::CommDescriptor,
) {
    run(comm, cx, |s| s.initialize());
}

mod api {

    use super::Handler;

    use crate::comm::races::orz::api::*;

    impl OrzApi for crate::comm::host::Host<'_, '_, Handler> {
        fn remove_escorts(&mut self, race: u16) {
            use uqm_engine::world::fleet;
            let remaining = fleet::remove_escort_ships(self.cx.borrow_mut().world, race, u16::MAX);
            if remaining > 0 {
                // SAFETY: the campaign borrow ends before the game-thread display refresh.
                {
                    let cx = &mut *self.cx.borrow_mut();
                    crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, 32767, 32767, 32767)
                };
            }
        }
        fn talking(&mut self, enabled: bool) {
            self.comm.borrow_mut().set_talking_animation(enabled);
        }
    }
}
