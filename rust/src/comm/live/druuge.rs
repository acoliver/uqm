use crate::comm::host::Host;
use crate::comm::races::druuge::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut session = comm.race_scratch.druuge.take().unwrap_or_default();
    {
        let mut api = Host::new(comm, cx, callback);
        f(&mut Dialogue {
            api: &mut api,
            session: &mut session,
        });
    }
    comm.race_scratch.druuge = Some(session);
}

fn exit_conversation(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ExitConversation,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn buy(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::Buy,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn trade(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::Trade,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn sell(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::Sell,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn explain_slave_trade(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ExplainSlaveTrade,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn trade_world(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::TradeWorld,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn bomb_ambush(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::BombAmbush,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn space(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::Space,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::ExitConversation => exit_conversation,
        Handler::Buy => buy,
        Handler::Trade => trade,
        Handler::Sell => sell,
        Handler::ExplainSlaveTrade => explain_slave_trade,
        Handler::TradeWorld => trade_world,
        Handler::BombAmbush => bomb_ambush,
        Handler::Space => space,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_druuge_enc());
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
    use crate::comm::locdata::CSisState;
    use crate::comm::races::druuge::api::*;

    impl DruugeApi for crate::comm::host::Host<'_, '_, Handler> {
        fn ship(&self) -> CSisState {
            self.cx.borrow().world.data.flagship
        }
        fn has_escorts(&self) -> bool {
            self.cx.borrow().world.built_ships_ref().head().is_some()
        }
        fn crew_cost(&self) -> u8 {
            self.cx.borrow().world.data.game.crew_cost
        }
        fn set_crew_cost(&mut self, value: u8) {
            self.cx.borrow_mut().world.data.game.crew_cost = value;
        }
        fn number(&mut self, value: i32) {
            let comm = &mut **self.comm.borrow_mut();
            self.cx.borrow_mut().with_legacy(|| {
                use crate::comm::commglue_abi::runtime::npc_number;
                // SAFETY: the session owns the loaded number phrases; a null format requests the default.
                unsafe {
                    npc_number(comm, value, std::ptr::null());
                }
            })
        }
    }
}
