#[cfg(not(test))]
use crate::comm::live::comandr as live;
use crate::comm::locdata::{session_view, SessionData};
use crate::comm::races::comandr::configure;
use crate::engine::StoryKey;

use crate::comm::races::comandr::descriptor::descriptor;
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;

pub(crate) fn init(_comm: &mut Comm, cx: &mut Cx<'_>) -> CommData {
    let mut desc = descriptor();
    configure(
        &mut desc,
        cx.world.story_state(StoryKey::RADIOACTIVES_PROVIDED) != 0,
    );
    session_view(&SessionData::new(desc))
}

pub(crate) fn intro(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::intro(_comm, _cx);
}

pub(crate) fn post_encounter(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
