use crate::comm::host::Host;
use crate::comm::races::umgah::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn combat_is_inevitable(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::CombatIsInevitable,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn zombies(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::Zombies,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn normal_umgah(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::NormalUmgah,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn umgah_reward(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::UmgahReward,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::CombatIsInevitable => combat_is_inevitable,
        Handler::Zombies => zombies,
        Handler::NormalUmgah => normal_umgah,
        Handler::UmgahReward => umgah_reward,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_umgah_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize the encounter on the game thread.
pub(crate) fn initialize(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    _desc: &mut crate::comm::races::descriptor::CommDescriptor,
) {
    run(comm, cx, |s| s.initialize());
}

mod api {

    use super::Handler;

    use crate::comm::races::umgah::api::*;

    impl UmgahApi for crate::comm::host::Host<'_, '_, Handler> {
        fn add_biomass(&mut self, amount: u16) {
            let mut cx = self.cx.borrow_mut();
            let sis = &mut cx.world.data.flagship;
            sis.total_bio_mass = sis.total_bio_mass.wrapping_add(amount);
        }
    }
}
