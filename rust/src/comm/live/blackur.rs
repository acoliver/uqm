use crate::comm::host::Host;
use crate::comm::races::blackur::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut a = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut a });
}
fn combat(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Combat, r as i32));
}
fn story(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Story, r as i32));
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::Combat => combat,
        Handler::Story => story,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
pub(crate) fn initialize(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    _desc: &mut crate::comm::races::descriptor::CommDescriptor,
) {
    run(comm, cx, |s| s.initialize());
}

mod api {

    use super::Handler;
    use crate::comm::races::blackur::api::*;

    use crate::engine::StoryKey;

    impl BlackurApi for crate::comm::host::Host<'_, '_, Handler> {
        fn battle_group(&self, key: StoryKey) -> (u32, u32) {
            let cx = self.cx.borrow();
            let offset = crate::state::ffi::rust_get_game_state_32(cx.world, key.start() as i32);
            (cx.world.data.game.battle_group_ref, offset)
        }
    }
}
