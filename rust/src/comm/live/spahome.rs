use crate::comm::descriptors::spahome::descriptor;
use crate::comm::host::Host;
use crate::comm::races::spahome::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn exit(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Exit, r as i32));
}
fn password(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Password, r as i32));
}
fn council(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Council, r as i32));
}
fn angry(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Angry, r as i32));
}
fn alliance(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Alliance, r as i32));
}
fn learn(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Learn, r as i32));
}
fn quest(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Quest, r as i32));
}
fn party(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Party, r as i32));
}
fn allies(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Allies, r as i32));
}
fn info(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Info, r as i32));
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::Exit => exit,
        Handler::Password => password,
        Handler::Council => council,
        Handler::Angry => angry,
        Handler::Alliance => alliance,
        Handler::Learn => learn,
        Handler::Quest => quest,
        Handler::Party => party,
        Handler::Allies => allies,
        Handler::Info => info,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
pub unsafe fn init_spahome_comm(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
) -> crate::comm::locdata::CLocData {
    extern "C" {
        static mut ScreenWidth: i32;
    }
    // SAFETY: the caller initializes encounters on the game thread with stable display dimensions.
    let width = unsafe { ScreenWidth };
    let mut d = descriptor(width);
    d.alien_text_baseline.x = (1 + ((width - 80) >> 1)) as i16;
    d.alien_text_width = (width - 80 - 16) as u16;
    run(comm, cx, |s| s.initialize());
    d
}

mod api {

    use super::Handler;

    use crate::comm::races::spahome::api::*;

    impl SpahomeApi for crate::comm::host::Host<'_, '_, Handler> {
        fn have_spathi_escort(&self) -> bool {
            crate::engine::world::fleet::count_escort_ships(
                self.cx.borrow().world,
                u16::from(crate::engine::species::ship::SPATHI_SHIP),
            ) != 0
        }
    }
}
