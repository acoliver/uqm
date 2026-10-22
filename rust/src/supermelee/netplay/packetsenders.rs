use super::abi::*;
use super::packet::*;
use super::packetq::queuePacket;
use std::ffi::c_char;

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendInit(conn: *mut NetConnection) {
    queuePacket(conn, Packet_Init_create().cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendPing(conn: *mut NetConnection, id: u32) {
    queuePacket(conn, Packet_Ping_create(id).cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendAck(conn: *mut NetConnection, id: u32) {
    queuePacket(conn, Packet_Ack_create(id).cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendReady(conn: *mut NetConnection) {
    queuePacket(conn, Packet_Ready_create().cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendHandshake0(conn: *mut NetConnection) {
    queuePacket(conn, Packet_Handshake0_create().cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendHandshake1(conn: *mut NetConnection) {
    queuePacket(conn, Packet_Handshake1_create().cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendHandshakeCancel(conn: *mut NetConnection) {
    queuePacket(conn, Packet_HandshakeCancel_create().cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendHandshakeCancelAck(conn: *mut NetConnection) {
    queuePacket(conn, Packet_HandshakeCancelAck_create().cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendSeedRandom(conn: *mut NetConnection, seed: u32) {
    queuePacket(conn, Packet_SeedRandom_create(seed).cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendInputDelay(conn: *mut NetConnection, delay: u32) {
    queuePacket(conn, Packet_InputDelay_create(delay).cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendSelectShip(conn: *mut NetConnection, index: FleetShipIndex) {
    queuePacket(conn, Packet_SelectShip_create(index).cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendBattleInput(conn: *mut NetConnection, input: BATTLE_INPUT_STATE) {
    queuePacket(conn, Packet_BattleInput_create(input).cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendFrameCount(conn: *mut NetConnection, frame: BattleFrameCounter) {
    queuePacket(conn, Packet_FrameCount_create(frame).cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendAbort(conn: *mut NetConnection, reason: NetplayAbortReason) {
    queuePacket(conn, Packet_Abort_create(reason as u16).cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendReset(conn: *mut NetConnection, reason: NetplayResetReason) {
    queuePacket(conn, Packet_Reset_create(reason as u16).cast());
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
    queuePacket(conn, Packet_TeamName_create(side, name, len).cast());
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
    let packet = Packet_Fleet_create(side, count);
    for index in 0..count {
        (*packet).ships.as_mut_ptr().add(index).write(FleetEntry {
            index: index as u8,
            ship: *ships.add(index) as u8,
        });
    }
    queuePacket(conn, packet.cast());
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
    let packet = Packet_Fleet_create(side, 1);
    (*packet).ships.as_mut_ptr().write(FleetEntry {
        index: index as u8,
        ship: ship as u8,
    });
    queuePacket(conn, packet.cast());
}

/// # Safety
/// `conn` must be connected with an initialized outgoing packet queue.
#[no_mangle]
pub unsafe extern "C" fn sendChecksum(
    conn: *mut NetConnection,
    frame: BattleFrameCounter,
    checksum: Checksum,
) {
    queuePacket(conn, Packet_Checksum_create(frame, checksum).cast());
}
