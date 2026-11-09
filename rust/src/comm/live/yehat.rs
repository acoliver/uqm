use crate::comm::descriptors::yehat::descriptor;
use crate::comm::live::script::LiveScript;
use crate::comm::races::yehat::*;
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

fn exit(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Exit, response as i32)
    });
}
fn royalists(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Royalists, response as i32)
    });
}
fn start_revolt(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::StartRevolt, response as i32)
    });
}
fn home(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Home, response as i32)
    });
}
fn space(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Space, response as i32)
    });
}

fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
    match handler {
        Handler::Exit => exit,
        Handler::Royalists => royalists,
        Handler::StartRevolt => start_revolt,
        Handler::Home => home,
        Handler::Space => space,
    }
}

pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |dialogue| dialogue.intro());
}
pub(crate) fn post(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}

pub unsafe fn init_yehat_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> CLocData {
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
    use super::*;

    use super::Handler;
    use crate::comm::races::yehat::api::*;

    #[cfg(not(test))]
    impl YehatApi for LiveScript<'_, '_, Handler> {
        fn have_shofixti_escort(&self) -> bool {
            crate::engine::world::fleet::count_escort_ships(
                self.cx.borrow().world,
                u16::from(crate::comm::dispatch::ship::SHOFIXTI_SHIP),
            ) != 0
        }
        fn random_bit(&mut self) -> u8 {
            (crate::math::random() & 1) as u8
        }
        fn schedule_rebel_event(&mut self) {
            // SAFETY: the callback owns the initialized campaign clock on the game thread.
            unsafe {
                crate::time::clock_abi::live::add_event(self.cx.borrow_mut().world, 1, 0, 0, 0, 15)
            };
        }
    }
}
