use crate::comm::host::Host;
use crate::comm::races::vux::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;

fn run(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    f: impl FnOnce(&mut dialogue::Dialogue<'_, Host<'_, '_, Handler>>),
) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut dialogue::Dialogue { api: &mut api });
}

fn combat(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Combat, response as i32)
    });
}
fn menagerie(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Menagerie, response as i32)
    });
}
fn normal(comm: &mut Comm, cx: &mut Cx<'_>, response: u32) {
    run(comm, cx, |dialogue| {
        dialogue.handle(Handler::Normal, response as i32)
    });
}

fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
    match handler {
        Handler::Combat => combat,
        Handler::Menagerie => menagerie,
        Handler::Normal => normal,
    }
}

pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |dialogue| dialogue.intro());
}
pub(crate) fn post(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}

pub(crate) fn initialize(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    _desc: &mut crate::comm::races::descriptor::CommDescriptor,
) {
    run(comm, cx, |s| s.initialize());
}
