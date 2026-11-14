use crate::comm::descriptors::mycon::descriptor;
use crate::comm::live::script::LiveScript;
use crate::comm::races::mycon::*;
use crate::comm::{locdata::CLocData, Comm};
use crate::engine::battle::Cx;
fn run(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    f: impl FnOnce(&mut Dialogue<'_, LiveScript<'_, '_, Handler>>),
) {
    let mut session = comm.race_scratch.mycon.take().unwrap_or_default();
    {
        let mut api = LiveScript::new(comm, cx, callback);
        f(&mut Dialogue {
            api: &mut api,
            session: &mut session,
        });
    }
    comm.race_scratch.mycon = Some(session);
}

fn combat_is_inevitable(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::CombatIsInevitable,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn sun_device(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SunDevice,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn trick_mycon(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::TrickMycon,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn normal_mycon(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::NormalMycon,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::CombatIsInevitable => combat_is_inevitable,
        Handler::SunDevice => sun_device,
        Handler::TrickMycon => trick_mycon,
        Handler::NormalMycon => normal_mycon,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_mycon_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize the encounter on the game thread.
pub fn init_mycon_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> CLocData {
    let desc = descriptor();
    run(comm, cx, |s| s.initialize());
    desc
}

mod api {

    use super::Handler;

    use crate::comm::races::mycon::api::*;

    impl MyconApi for crate::comm::live::script::LiveScript<'_, '_, Handler> {
        fn random(&mut self) -> u32 {
            self.cx.borrow_mut().with_legacy(crate::math::random)
        }
    }
}
