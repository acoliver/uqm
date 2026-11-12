use crate::comm::host::Host;
use crate::comm::locdata::SessionData;
use crate::comm::races::slyhome::descriptor::descriptor;
use crate::comm::races::slyhome::*;
use crate::comm::Comm;
use crate::engine::battle::Cx;
fn run(comm: &mut Comm, cx: &mut Cx<'_>, f: impl FnOnce(&mut Dialogue<'_, Host<'_, '_, Handler>>)) {
    let mut api = Host::new(comm, cx, callback);
    f(&mut Dialogue { api: &mut api });
}
fn exit(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Exit, r as i32));
}
fn home(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Home, r as i32));
}
fn human(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Human, r as i32));
}
fn alien(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Alien, r as i32));
}
fn probe(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Probe, r as i32));
}
fn function(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Function, r as i32));
}
fn bug(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Bug, r as i32));
}
fn fix(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| s.handle(Handler::Fix, r as i32));
}
fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
    match h {
        Handler::Exit => exit,
        Handler::Home => home,
        Handler::Human => human,
        Handler::Alien => alien,
        Handler::Probe => probe,
        Handler::Function => function,
        Handler::Bug => bug,
        Handler::Fix => fix,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
pub fn init_slylandro_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> crate::comm::locdata::SessionData {
    let width = cx.screen_dimensions().0;
    let mut d = descriptor(width);
    d.alien_text_baseline.x = (1 + ((width - 80) >> 1)) as i16;
    d.alien_text_width = (width - 80) as u16;
    run(comm, cx, |s| s.initialize());
    SessionData {
        init_encounter_func: Some(intro),
        post_encounter_func: Some(post),
        uninit_encounter_func: Some(uninit),
        ..SessionData::new(d)
    }
}
