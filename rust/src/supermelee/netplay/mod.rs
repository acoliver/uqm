//! SuperMelee wire protocol and deterministic battle checksums.
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    unnecessary_transmutes,
    clippy::all,
    reason = "generated ABI declarations share engine pointer types with other bridges"
)]
pub mod abi {
    include!("../../abi/netplay_abi.rs");
}
mod battle;
mod checkbuf;
pub(crate) mod checksum;
pub(crate) mod crc;
pub mod packet;
pub mod packethandlers;
mod packetq;
mod packetsenders;
use crate::network::set_errno;

mod npconfirm;
mod ready;
mod reset;

mod netstate;

mod netoptions;
pub(crate) mod session;
pub use session::NetplaySession;

pub(crate) mod netinput;

pub(crate) mod netconnection;

mod netrcv;

mod netsend;

mod netmisc;

pub(crate) mod notify;

pub(crate) mod notifyall;

pub(crate) mod netmelee;

#[cfg(not(test))]
pub(crate) use netconnection::NetConnection_isConnected;

#[cfg(not(test))]
pub(crate) use netconnection::{NetConnection_getPeerOptions, NetConnection_getPlayerNr};
#[cfg(not(test))]
pub(crate) use notify::Netplay_Notify_seedRandom;

#[cfg(test)]
mod test_runtime;

#[cfg(not(test))]
pub(crate) use netmelee::NetMelee_getBattleState;
