#[cfg(not(test))]
use crate::comm::live::slyland as live;
use crate::comm::locdata::session_view;
#[cfg(test)]
use crate::comm::locdata::SessionData;

use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;

#[cfg(test)]
use crate::comm::races::slyland::descriptor::descriptor;

pub(crate) fn init(_comm: &mut Comm, _cx: &mut Cx<'_>) -> CommData {
    #[cfg(not(test))]
    {
        session_view(&live::init_slyland_comm(_comm, _cx))
    }
    #[cfg(test)]
    {
        session_view(&SessionData::new(descriptor(320)))
    }
}

pub(crate) fn intro(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::intro(_comm, _cx);
}

pub(crate) fn post_encounter(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
