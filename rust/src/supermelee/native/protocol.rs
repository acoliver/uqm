//! Protocol work borrows the input frame that admitted it.
#[cfg(not(test))]
use crate::engine::battle::net::ProtocolNoticeKind;
use crate::engine::{
    battle::{
        net::{ProtocolDriver, ProtocolNotice, ProtocolPoll},
        Cx,
    },
    observe::Io,
};
#[cfg(not(test))]
use std::ffi::CStr;

type NoticeHandler<'a> = dyn FnMut(&mut Cx<'_>, &mut Io<'_>, ProtocolNotice) + 'a;

pub(crate) struct Driver<'a, 'cx, 'io> {
    cx: &'a mut Cx<'cx>,
    io: &'a mut Io<'io>,
    notices: &'a mut NoticeHandler<'a>,
}

impl<'a, 'cx, 'io> Driver<'a, 'cx, 'io> {
    pub(crate) fn new(
        cx: &'a mut Cx<'cx>,
        io: &'a mut Io<'io>,
        notices: &'a mut NoticeHandler<'a>,
    ) -> Self {
        Self { cx, io, notices }
    }
}

impl ProtocolDriver for Driver<'_, '_, '_> {
    fn notice(&mut self, notice: ProtocolNotice) {
        (self.notices)(self.cx, self.io, notice);
    }

    fn wait(&mut self, poll: &mut ProtocolPoll<'_>) {
        let notices = &mut *self.notices;
        self.cx.run_input(self.io, false, |cx, io| {
            poll(&mut Driver::new(cx, io, notices))
        });
    }
}

#[cfg(not(test))]
pub(crate) fn with_battle_driver<R>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    action: impl FnOnce(&mut dyn ProtocolDriver) -> R,
) -> R {
    action(&mut Driver::new(cx, io, &mut battle_notice))
}

#[cfg(not(test))]
pub(crate) fn feedback(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    notice: ProtocolNotice,
    mut status: impl FnMut(&mut Cx<'_>, &CStr),
) {
    use ProtocolNoticeKind::*;
    let index = match notice.kind {
        Connected => Some(if notice.player == 0 { 8 } else { 9 }),
        Error(_) => Some(if notice.player == 0 { 10 } else { 11 }),
        Closed => Some(if notice.player == 0 { 12 } else { 13 }),
        Abort(reason) if reason < 4 => Some(25 + reason as usize),
        Abort(_) => None,
        Reset {
            reason: 2,
            remote: false,
        } => return,
        Reset { reason, .. } => (reason < 3).then_some(29 + reason as usize),
        ShipSelected => {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 1);
            return;
        }
    };
    if let Some(index) = index {
        let message = cx
            .kernel
            .game_text(crate::ships::native::abi::NETMELEE_STRING_BASE as usize + index)
            .to_owned();
        if notice.battle_active || matches!(notice.kind, Abort(_)) {
            crate::ui::confirm::do_popup_window(cx, io, &message);
        } else {
            status(cx, &message);
        }
    }
    if matches!(notice.kind, Connected) {
        crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 3);
    }
    if matches!(notice.kind, Reset { .. }) {
        cx.world
            .set_activity(cx.world.activity() | crate::ships::native::abi::CHECK_ABORT as u16);
    }
}

#[cfg(not(test))]
pub(crate) fn battle_notice(cx: &mut Cx<'_>, io: &mut Io<'_>, notice: ProtocolNotice) {
    feedback(cx, io, notice, |_, _| {
        panic!("a battle protocol notice requested menu status: {notice:?}")
    });
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
