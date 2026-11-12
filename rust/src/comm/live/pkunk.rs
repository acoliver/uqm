use crate::comm::host::Host;
use crate::comm::races::pkunk::descriptor::descriptor;
use crate::comm::races::pkunk::*;
use crate::comm::{locdata::SessionData, Comm};
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
fn pkunk_angry(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::PkunkAngry,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn discuss_conquer(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::DiscussConquer,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn offer_alliance(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::OfferAlliance,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn about_pkunk(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AboutPkunk,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn about_ilwrath(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AboutIlwrath,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn pkunk_home(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::PkunkHome,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn pkunk_friendly_space(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::PkunkFriendlySpace,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn pkunk_neutral_space(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::PkunkNeutralSpace,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn pkunk_migrate(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::PkunkMigrate,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::ExitConversation => exit_conversation,
        Handler::PkunkAngry => pkunk_angry,
        Handler::DiscussConquer => discuss_conquer,
        Handler::OfferAlliance => offer_alliance,
        Handler::AboutPkunk => about_pkunk,
        Handler::AboutIlwrath => about_ilwrath,
        Handler::PkunkHome => pkunk_home,
        Handler::PkunkFriendlySpace => pkunk_friendly_space,
        Handler::PkunkNeutralSpace => pkunk_neutral_space,
        Handler::PkunkMigrate => pkunk_migrate,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_pkunk_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize this encounter on the game thread.
pub fn init_pkunk_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> SessionData {
    let desc = descriptor();
    run(comm, cx, |s| s.initialize());
    SessionData {
        init_encounter_func: Some(intro),
        post_encounter_func: Some(post),
        uninit_encounter_func: Some(uninit),
        ..SessionData::new(desc)
    }
}

mod api {

    use super::Handler;

    use crate::comm::races::pkunk::api::*;

    impl PkunkApi for crate::comm::host::Host<'_, '_, Handler> {
        fn date(&self) -> (u16, u16, u16) {
            let owner = self.cx.borrow();
            let date = &owner.world.data.game.game_clock;
            (
                date.year_index,
                u16::from(date.month_index),
                u16::from(date.day_index),
            )
        }
        fn gift_ships(&mut self) -> u16 {
            use crate::engine::world::fleet;
            let added = fleet::add_escort_ships(
                self.cx.borrow_mut().world,
                crate::engine::species::ship::PKUNK_SHIP as u16,
                4,
                fleet::captain_name,
            );
            match added {
                Some(count) => {
                    // SAFETY: the campaign borrow ends before the game-thread display refresh.
                    {
                        let cx = &mut *self.cx.borrow_mut();
                        crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, 32767, 32767, 32767)
                    };
                    count
                }
                None => 0,
            }
        }
    }
}
