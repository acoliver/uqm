//! SuperMelee wire protocol and deterministic battle checksums.
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    unnecessary_transmutes,
    clippy::all,
    clashing_extern_declarations,
    reason = "generated ABI declarations share engine pointer types with other bridges"
)]
pub mod abi {
    include!("../../abi/netplay_abi.rs");
}
mod checkbuf;
pub(crate) mod checksum;
pub(crate) mod crc;
mod packet;
mod packethandlers;
mod packetq;
mod packetsenders;
use crate::network::set_errno;

mod npconfirm;
mod ready;
mod reset;

mod netstate;

mod netoptions;

mod netinput;

mod netconnection;

mod netrcv;

mod netsend;

mod netmisc;

mod notify;

mod notifyall;

mod netmelee;

#[cfg(not(test))]
pub(crate) use netconnection::NetConnection_isConnected;
#[cfg(not(test))]
pub(crate) use notify::Netplay_Notify_shipSelected;

#[cfg(not(test))]
pub(crate) use netconnection::{NetConnection_getPeerOptions, NetConnection_getPlayerNr};
#[cfg(not(test))]
pub(crate) use netmelee::{
    cancelConfirmations, closeAllConnections, closeDisconnectedConnections,
    closePlayerNetworkConnection, confirmConnections, openPlayerNetworkConnection, setupInputDelay,
    NetMelee_getBattleState,
};
#[cfg(not(test))]
pub(crate) use notify::Netplay_Notify_seedRandom;
#[cfg(not(test))]
pub(crate) use notifyall::{
    Netplay_NotifyAll_inputDelay, Netplay_NotifyAll_setFleet, Netplay_NotifyAll_setShip,
    Netplay_NotifyAll_setTeamName,
};
