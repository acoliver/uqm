//! Super Melee wire protocol and owned deterministic session runtime.
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    reason = "legacy scalar names shared with the remaining melee UI"
)]
pub mod abi {
    include!("../../abi/netplay_abi.rs");
}
mod battle;
pub mod checkbuf;
pub(crate) mod checksum;
pub(crate) mod crc;
pub mod netconnection;
pub(crate) mod netmelee;
mod netrcv;
mod notify;
pub mod packet;
mod packethandlers;
pub mod packetq;
mod ready;
pub(crate) mod session;
pub use netconnection::{Effect, Event};
pub use session::NetplaySession;
pub(crate) mod netinput;
use crate::platform::set_errno;
