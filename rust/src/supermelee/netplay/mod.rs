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
mod abi {
    include!(concat!(env!("OUT_DIR"), "/netplay_abi.rs"));
}
mod checkbuf;
mod checksum;
mod crc;
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
