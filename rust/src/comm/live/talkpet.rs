use crate::comm::host::Host;
use crate::comm::races::talkpet::descriptor::descriptor;
use crate::comm::races::talkpet::*;
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
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn mind_fuck_urquan(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::MindFuckUrquan,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn mind_control(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::MindControl,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn pet_info(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::PetInfo,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn pet_device(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::PetDevice,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn compel_player(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::CompelPlayer,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn kill_pet(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::KillPet,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn pet_deal(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::PetDeal,
            i32::try_from(r).expect("invalid response"),
        )
    });
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::ExitConversation => exit_conversation,
        Handler::MindFuckUrquan => mind_fuck_urquan,
        Handler::MindControl => mind_control,
        Handler::PetInfo => pet_info,
        Handler::PetDevice => pet_device,
        Handler::CompelPlayer => compel_player,
        Handler::KillPet => kill_pet,
        Handler::PetDeal => pet_deal,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_talkpet_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize the encounter on the game thread.
pub fn init_talkpet_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> SessionData {
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

    use crate::comm::races::talkpet::api::*;

    impl TalkpetApi for crate::comm::host::Host<'_, '_, Handler> {
        fn flags(&self) -> u8 {
            self.cx.borrow().world.data.game.glob_flags
        }
        fn npc_ships(&self) -> bool {
            self.cx
                .borrow()
                .world
                .npc_built_ships_ref()
                .head()
                .is_some()
        }
        fn umgah_alive(&mut self) -> bool {
            crate::engine::world::fleet::start_sphere_tracking(
                self.cx.borrow_mut().world,
                crate::engine::species::ship::UMGAH_SHIP as u16,
            ) != 0
        }
        fn strobe(&mut self) {
            let comm = &mut **self.comm.borrow_mut();

            self.cx.borrow_mut().with_legacy(|| {
                comm.data.definition.alien_ambient_array[16].anim_flags &=
                    !crate::comm::races::script::ANIM_DISABLED;
            })
        }
    }
}
