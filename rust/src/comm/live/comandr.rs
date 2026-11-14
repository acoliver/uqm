use crate::comm::descriptors::comandr::descriptor;
use crate::comm::host::Host;
use crate::comm::races::comandr::*;
use crate::comm::{locdata::CLocData, Comm};
use crate::engine::battle::Cx;

fn api<'a, 'b>(
    comm: &'a mut Comm,
    cx: &'a mut crate::engine::battle::Cx<'b>,
) -> Host<'a, 'b, Handler> {
    Host::new(comm, cx, callback)
}

macro_rules! callbacks {
    ($($handler:ident => $name:ident),* $(,)?) => {
        $(fn $name(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) { Commander(&mut api(comm, cx)).handle(Handler::$handler, u16::try_from(r).expect("response reference exceeds COUNT")); })*
        fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
            match handler { $(Handler::$handler => $name,)* }
        }
    };
}
callbacks! {
    ByeBye => bye_bye,
    NoRadioactives => no_radioactives,
    AskAfterRadios => ask_after_radios,
    BaseDestroyed => base_destroyed,
    TellMoonBase => tell_moon_base,
    TellProbe => tell_probe,
    RevealSelf => reveal_self,
    GiveRadios => give_radios,
}

pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    Commander(&mut api(comm, cx)).intro();
}
pub(crate) fn post(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}

/// Build the Earth Commander descriptor for this session's campaign state.
pub fn init_commander_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> CLocData {
    let mut desc = descriptor();
    configure(
        &mut desc,
        api(comm, cx).get(StoryKey::RADIOACTIVES_PROVIDED) != 0,
    );
    // setSegue(peace) also updates the legacy battle bit used by dispatch.
    api(comm, cx).set(StoryKey::BATTLE_SEGUE, 0);
    crate::comm::ffi::rust_set_segue(comm, 0);
    desc
}
