use crate::comm::host::Host;
use crate::comm::races::rebel::descriptor::descriptor;
use crate::comm::races::rebel::*;
use crate::comm::{locdata::SessionData, Comm};
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut a = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut a })
}
fn exit(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Exit, r as i32))
}
fn info(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Info, r as i32))
}
fn rebels(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Rebels, r as i32))
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::Exit => exit,
        Handler::Info => info,
        Handler::Rebels => rebels,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro())
}
pub(crate) fn post(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
pub fn init_rebel_yehat_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> SessionData {
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

    use crate::comm::races::rebel::api::*;

    impl RebelApi for crate::comm::host::Host<'_, '_, Handler> {
        fn date(&self) -> Date {
            let cx = self.cx.borrow();
            let clock = &cx.world.data.game.game_clock;
            Date {
                year: clock.year_index,
                month: clock.month_index,
                day: clock.day_index,
            }
        }
    }
}
