#[cfg(not(test))]
use crate::comm::live::vux as live;
use crate::comm::locdata::session_view;
#[cfg(test)]
use crate::comm::locdata::SessionData;

#[cfg(test)]
use crate::comm::races::vux::descriptor::descriptor;
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;

pub(crate) fn init(_comm: &mut Comm, _cx: &mut Cx<'_>) -> CommData {
    #[cfg(not(test))]
    {
        session_view(&live::init_vux_comm(_comm, _cx))
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

pub(crate) fn post_encounter(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::post(_comm, _cx);
}

pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u32 {
    #[cfg(not(test))]
    return live::uninit(_comm, _cx) as u32;
    #[cfg(test)]
    0
}
