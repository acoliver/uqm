use crate::comm::host::Host;
use crate::comm::races::ilwrath::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn combat_is_inevitable(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::CombatIsInevitable, r));
}
fn ilwrath_gods(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::IlwrathGods, r));
}
fn ilwrath_info(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::IlwrathInfo, r));
}
fn ilwrath_home(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::IlwrathHome, r));
}
fn gods_order(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::GodsOrder, r));
}
fn gods_speak(comm: &mut Comm, cx: &mut Cx<'_>, r: u16) {
    run(comm, cx, |s| s.handle(Handler::GodsSpeak, r));
}
pub(super) fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
    match handler {
        Handler::CombatIsInevitable => combat_is_inevitable,
        Handler::IlwrathGods => ilwrath_gods,
        Handler::IlwrathInfo => ilwrath_info,
        Handler::IlwrathHome => ilwrath_home,
        Handler::GodsOrder => gods_order,
        Handler::GodsSpeak => gods_speak,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_ilwrath_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize the encounter descriptor and initial battle segue.
pub(crate) fn initialize(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    desc: &mut crate::comm::races::descriptor::CommDescriptor,
) {
    run(comm, cx, |s| s.initialize(desc));
}
