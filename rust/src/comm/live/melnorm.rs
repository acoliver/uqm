use crate::comm::host::Host;
use crate::comm::locdata::CSisState;
use crate::comm::races::melnorm::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;

fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut session = comm
        .race_scratch
        .melnorm
        .take()
        .unwrap_or_else(Session::new);
    {
        let mut api = Host::new(comm, cx, callback);
        f(&mut Dialogue {
            api: &mut api,
            session: &mut session,
        });
    }
    comm.race_scratch.melnorm = Some(session);
}

fn exit_conversation(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ExitConversation,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn do_rescue(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::DoRescue,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn do_buy(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::DoBuy,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn do_sell(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::DoSell,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn nature_of_conversation(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::NatureOfConversation,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn do_bluster(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::DoBluster,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn do_first_meeting(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::DoFirstMeeting,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn do_melnorme_miffed(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::DoMelnormeMiffed,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}
fn do_melnorme_pissed(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::DoMelnormePissed,
            u16::try_from(r).expect("response exceeds COUNT"),
        )
    });
}

fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::ExitConversation => exit_conversation,
        Handler::DoRescue => do_rescue,
        Handler::DoBuy => do_buy,
        Handler::DoSell => do_sell,
        Handler::NatureOfConversation => nature_of_conversation,
        Handler::DoBluster => do_bluster,
        Handler::DoFirstMeeting => do_first_meeting,
        Handler::DoMelnormeMiffed => do_melnorme_miffed,
        Handler::DoMelnormePissed => do_melnorme_pissed,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_melnorme_enc());
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

    use super::{CSisState, Handler};
    use crate::comm::races::melnorm::api::*;

    mod live {
        use super::*;
        use crate::comm::host::Host;

        use crate::comm::commglue_abi::runtime::npc_number;

        use crate::planets::cargo::draw_cargo;

        use crate::threading::pacing::linked::sleep_thread as SleepThread;
        impl MelnormApi for Host<'_, '_, Handler> {
            fn ship(&self) -> CSisState {
                self.cx.borrow().world.data.flagship
            }
            fn set_ship(&mut self, ship: CSisState) {
                self.cx.borrow_mut().world.data.flagship = ship;
            }
            fn module_cost(&self, id: usize) -> u8 {
                self.cx.borrow().world.data.game.module_cost[id]
            }
            fn set_module_cost(&mut self, id: usize, cost: u8) {
                self.cx.borrow_mut().world.data.game.module_cost[id] = cost;
            }
            fn number(&mut self, number: i32) {
                let comm = &mut **self.comm.borrow_mut();
                // SAFETY: the session retains the loaded number phrases; a null format requests the default.
                self.cx.borrow_mut().with_legacy(|| unsafe {
                    npc_number(comm, number, std::ptr::null());
                })
            }
            fn status_mode(&mut self, mode: i32) -> i32 {
                self.cx
                    .borrow_mut()
                    .kernel
                    .set_status_message_mode(mode as u32) as i32
            }
            fn draw_status(&mut self) {
                let cx = &mut *self.cx.borrow_mut();
                crate::ui::sis::draw_status_message(cx.kernel, cx.world, None)
            }
            fn draw_cargo(&mut self, old: u8, new: u8) {
                let cx = self.cx.borrow();
                draw_cargo(cx.kernel, cx.world, old, new);
            }
            fn sleep(&mut self, ticks: u32) {
                // SAFETY: the exclusive Cx loan publishes live game-thread UI and pacing owners.
                self.cx.borrow_mut().with_legacy(|| unsafe {
                    SleepThread(ticks);
                })
            }
            fn now(&mut self) -> u32 {
                self.cx
                    .borrow_mut()
                    .with_legacy(|| crate::engine::time::counter_abi::GetTimeCounter())
            }
            fn skip_sale(&mut self) -> bool {
                let runtime = self.cx.borrow().kernel.input_runtime();
                let input = runtime.input();
                let mut cx = self.cx.borrow_mut();
                runtime.update(&mut cx);
                input.current().any_button_pressed()
            }
            fn clear_display(&mut self) {
                let cx = &mut *self.cx.borrow_mut();
                let dimensions = cx.screen_dimensions();
                crate::ui::sis::clear_sis_rect(cx.kernel, cx.world, dimensions, 8)
            }
        }
    }
}
