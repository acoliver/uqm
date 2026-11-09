use crate::comm::descriptors::urquan::descriptor;
use crate::comm::live::script::LiveScript;
use crate::comm::races::urquan::*;
use crate::comm::{locdata::CLocData, Comm};
use crate::engine::battle::Cx;

fn run(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    f: impl FnOnce(&mut dialogue::Dialogue<'_, LiveScript<'_, '_, Handler>>),
) {
    let mut api = LiveScript::new(comm, cx, callback);
    f(&mut dialogue::Dialogue { api: &mut api });
}

fn combat(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Combat, response as i32)
    });
}
fn normal(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Normal, response as i32)
    });
}
fn hypno(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Hypno, response as i32)
    });
}
fn mention_compulsion(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::MentionCompulsion, response as i32)
    });
}
fn describe_pet(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::DescribePet, response as i32)
    });
}
fn describe_compulsion(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::DescribeCompulsion, response as i32)
    });
}
fn loser(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Loser, response as i32)
    });
}

fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
    match handler {
        Handler::Combat => combat,
        Handler::Normal => normal,
        Handler::Hypno => hypno,
        Handler::MentionCompulsion => mention_compulsion,
        Handler::DescribePet => describe_pet,
        Handler::DescribeCompulsion => describe_compulsion,
        Handler::Loser => loser,
    }
}

pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |dialogue| dialogue.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |dialogue| dialogue.post());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}

pub unsafe fn init_urquan_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> CLocData {
    extern "C" {
        static mut ScreenWidth: i32;
    }
    // SAFETY: the caller initializes encounters on the game thread with stable display dimensions.
    let width = unsafe { ScreenWidth };
    let mut descriptor = descriptor(width);
    (descriptor.alien_text_baseline, descriptor.alien_text_width) = text_layout(width);
    run(comm, cx, |dialogue| dialogue.initialize());
    descriptor
}

mod api {

    use super::Handler;

    use crate::comm::races::urquan::api::*;
    use crate::engine::StoryKey;

    #[cfg(not(test))]
    impl UrquanApi for crate::comm::live::script::LiveScript<'_, '_, Handler> {
        fn group_battle(&self, state_key: StoryKey) -> bool {
            let cx = self.cx.borrow();
            let offset =
                crate::state::ffi::rust_get_game_state_32(cx.world, state_key.start() as i32);
            let group = cx.world.data.game.battle_group_ref;
            group != 0 && group == offset
        }

        fn intro_mode(&mut self, mode: i32, duration: u32) {
            let comm = &mut **self.comm.borrow_mut();
            self.cx.borrow_mut().with_legacy(|| {
                use crate::comm::comm_abi::set_comm_intro_mode;
                set_comm_intro_mode(comm, mode as u32, duration)
            })
        }
    }
}
