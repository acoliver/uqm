use super::abi::*;
use super::packet::*;
use super::packetq::queue_packet;
use std::ffi::c_char;

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendInit(conn: *mut NetConnection) {
    queue_packet(
        conn,
        Packet::Init {
            protocol: [
                NETPLAY_PROTOCOL_VERSION_MAJOR as u8,
                NETPLAY_PROTOCOL_VERSION_MINOR as u8,
            ],
            version: [
                UQM_MAJOR_VERSION as u8,
                UQM_MINOR_VERSION as u8,
                UQM_PATCH_VERSION as u8,
            ],
        },
    );
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendAck(conn: *mut NetConnection, id: u32) {
    queue_packet(conn, Packet::Ack(id));
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendReady(conn: *mut NetConnection) {
    queue_packet(conn, Packet::Ready);
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendHandshake0(conn: *mut NetConnection) {
    queue_packet(conn, Packet::Handshake0);
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendHandshake1(conn: *mut NetConnection) {
    queue_packet(conn, Packet::Handshake1);
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendHandshakeCancel(conn: *mut NetConnection) {
    queue_packet(conn, Packet::HandshakeCancel);
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendHandshakeCancelAck(conn: *mut NetConnection) {
    queue_packet(conn, Packet::HandshakeCancelAck);
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendSeedRandom(conn: *mut NetConnection, seed: u32) {
    queue_packet(conn, Packet::SeedRandom(seed));
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendInputDelay(conn: *mut NetConnection, delay: u32) {
    queue_packet(conn, Packet::InputDelay(delay));
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendSelectShip(conn: *mut NetConnection, index: FleetShipIndex) {
    queue_packet(conn, Packet::SelectShip(index));
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendBattleInput(conn: *mut NetConnection, input: BATTLE_INPUT_STATE) {
    queue_packet(conn, Packet::BattleInput(input));
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendFrameCount(conn: *mut NetConnection, frame: BattleFrameCounter) {
    queue_packet(conn, Packet::FrameCount(frame));
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendAbort(conn: *mut NetConnection, reason: NetplayAbortReason) {
    queue_packet(conn, Packet::Abort(reason as u16));
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendReset(conn: *mut NetConnection, reason: NetplayResetReason) {
    queue_packet(conn, Packet::Reset(reason as u16));
}

/// # Safety
/// `conn` must be connected; `name` must contain `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sendTeamName(
    conn: *mut NetConnection,
    side: NetplaySide,
    name: *const c_char,
    len: usize,
) {
    queue_packet(
        conn,
        Packet::TeamName {
            side: side as u8,
            name: std::slice::from_raw_parts(name.cast(), len).to_vec(),
        },
    );
}

/// # Safety
/// `conn` must be connected; `ships` must contain `count` readable ship IDs.
#[no_mangle]
pub unsafe extern "C" fn sendFleet(
    conn: *mut NetConnection,
    side: NetplaySide,
    ships: *const MeleeShip,
    count: usize,
) {
    let ships = std::slice::from_raw_parts(ships, count)
        .iter()
        .enumerate()
        .map(|(index, &ship)| (index as u8, ship as u8))
        .collect();
    queue_packet(
        conn,
        Packet::Fleet {
            side: side as u8,
            ships,
        },
    );
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendFleetShip(
    conn: *mut NetConnection,
    side: NetplaySide,
    index: FleetShipIndex,
    ship: MeleeShip,
) {
    queue_packet(
        conn,
        Packet::Fleet {
            side: side as u8,
            ships: vec![(index as u8, ship as u8)],
        },
    );
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendChecksum(
    conn: *mut NetConnection,
    frame: BattleFrameCounter,
    checksum: Checksum,
) {
    queue_packet(conn, Packet::Checksum { frame, checksum });
}
