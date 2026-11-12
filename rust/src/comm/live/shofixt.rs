use crate::comm::host::Host;
use crate::comm::races::shofixt::descriptor::descriptor;
use crate::comm::races::shofixt::*;
use crate::comm::{locdata::SessionData, Comm};
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut a = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut a })
}
fn exit(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Exit, r as i32))
}
fn maidens(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::GiveMaidens, r as i32))
}
fn console(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Console, r as i32))
}
fn explain(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Explain, r as i32))
}
fn realize(comm: &mut Comm, cx: &mut Cx<'_>, _: u32) {
    run(comm, cx, |s| s.handle(Handler::Realize, 0))
}
fn friendly(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Friendly, r as i32))
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::Exit => exit,
        Handler::GiveMaidens => maidens,
        Handler::Console => console,
        Handler::Explain => explain,
        Handler::Realize => realize,
        Handler::Friendly => friendly,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro())
}
pub(crate) fn post(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
pub fn init_shofixti_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> SessionData {
    let width = cx.screen_dimensions().0;
    let mut d = descriptor(width);
    (d.alien_text_baseline, d.alien_text_width) = text_layout(width);
    run(comm, cx, |s| s.initialize());
    SessionData {
        init_encounter_func: Some(intro),
        post_encounter_func: Some(post),
        uninit_encounter_func: Some(uninit),
        ..SessionData::new(d)
    }
}

mod api {

    use super::Handler;

    use crate::comm::races::shofixt::api::*;

    impl ShofixtApi for crate::comm::host::Host<'_, '_, Handler> {
        fn schedule_return(&mut self) {
            crate::time::clock_abi::add_event(&mut self.cx.borrow_mut(), 1, 2, 0, 0, 8);
        }
    }
}
