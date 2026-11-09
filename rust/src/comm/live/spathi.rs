use crate::comm::descriptors::spathi::descriptor;
use crate::comm::live::script::LiveScript;
use crate::comm::races::spathi::*;
use crate::comm::{locdata::CLocData, Comm};
use crate::engine::battle::Cx;
fn run(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    f: impl FnOnce(&mut Dialogue<'_, LiveScript<'_, '_, Handler>>),
) {
    let mut session = comm.race_scratch.spathi.take().unwrap_or_default();
    {
        let mut api = LiveScript::new(comm, cx, callback);
        f(&mut Dialogue {
            api: &mut api,
            session: &mut session,
        });
    }
    comm.race_scratch.spathi = Some(session);
}

fn exit_conversation(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ExitConversation,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn spathi_on_pluto(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SpathiOnPluto,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn spathi_must_grovel(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SpathiMustGrovel,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn spathi_allies(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SpathiAllies,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn spathi_friendly(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SpathiFriendly,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn spathi_befriend(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SpathiBefriend,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn spathi_antagonize(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SpathiAntagonize,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn spathi_neutral(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SpathiNeutral,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
    match handler {
        Handler::ExitConversation => exit_conversation,
        Handler::SpathiOnPluto => spathi_on_pluto,
        Handler::SpathiMustGrovel => spathi_must_grovel,
        Handler::SpathiAllies => spathi_allies,
        Handler::SpathiFriendly => spathi_friendly,
        Handler::SpathiBefriend => spathi_befriend,
        Handler::SpathiAntagonize => spathi_antagonize,
        Handler::SpathiNeutral => spathi_neutral,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_spathi_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize the encounter descriptor and initial battle segue.
pub fn init_spathi_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> CLocData {
    let mut desc = descriptor();
    run(comm, cx, |s| s.initialize(&mut desc));
    desc
}
