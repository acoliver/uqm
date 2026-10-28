use super::abi::*;
use super::abi::{
    UqmNetplayConstants_UQM_NETPLAY_BATTLE_FRAME_RATE as BATTLE_FRAME_RATE,
    UqmNetplayConstants_UQM_NETPLAY_NUM_MELEE_SHIPS as NUM_MELEE_SHIPS,
};
use super::packetsenders::{
    sendAbort, sendAck, sendHandshake0, sendHandshake1, sendHandshakeCancelAck,
};

pub unsafe fn dispatch(conn: *mut NetConnection, packet: &Packet) -> i32 {
    match packet {
        Packet::Init { protocol, version } => handle_Init(conn, *protocol, *version),
        Packet::Ping(value) => handle_Ping(conn, *value),
        Packet::Ack(_) => handle_Ack(conn),
        Packet::Ready => handle_Ready(conn),
        Packet::Fleet { side, ships } => handle_Fleet(conn, *side, ships),
        Packet::TeamName { side, name } => handle_TeamName(conn, *side, name),
        Packet::Handshake0 => handle_Handshake0(conn),
        Packet::Handshake1 => handle_Handshake1(conn),
        Packet::HandshakeCancel => handle_HandshakeCancel(conn),
        Packet::HandshakeCancelAck => handle_HandshakeCancelAck(conn),
        Packet::SeedRandom(value) => handle_SeedRandom(conn, *value),
        Packet::InputDelay(value) => handle_InputDelay(conn, *value),
        Packet::SelectShip(value) => handle_SelectShip(conn, *value),
        Packet::BattleInput(value) => handle_BattleInput(conn, *value),
        Packet::FrameCount(value) => handle_FrameCount(conn, *value),
        Packet::Checksum { frame, checksum } => handle_Checksum(conn, *frame, *checksum),
        Packet::Abort(value) => handle_Abort(conn, *value),
        Packet::Reset(value) => handle_Reset(conn, *value),
    }
}

unsafe fn bad_message() -> i32 {
    super::set_errno(libc::EBADMSG);
    -1
}

unsafe fn test_state(condition: bool, name: &str) -> bool {
    if !condition {
        log::error!("Packet of type '{name}' received from wrong state.");
        super::set_errno(libc::EBADMSG);
    }
    condition
}

unsafe fn reset_status(conn: *const NetConnection) -> Option<i32> {
    if (*conn).stateFlags.reset.localReset() {
        Some(0)
    } else if (*conn).stateFlags.reset.remoteReset() {
        Some(bad_message())
    } else {
        None
    }
}

macro_rules! guard {
    ($conn:expr, $condition:expr, $name:literal) => {
        if let Some(result) = reset_status($conn) {
            return result;
        }
        if !test_state($condition, $name) {
            return -1;
        }
    };
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Init(conn: *mut NetConnection, protocol: [u8; 2], version: [u8; 3]) -> i32 {
    guard!(
        conn,
        (*conn).state == NetState_NetState_init && !(*conn).stateFlags.ready.remoteReady(),
        "Init"
    );
    let protocol_ok = protocol[0] == NETPLAY_PROTOCOL_VERSION_MAJOR as u8
        && protocol[1] == NETPLAY_PROTOCOL_VERSION_MINOR as u8;
    let version = (version[0], version[1], version[2]);
    let minimum = (
        NETPLAY_MIN_UQM_VERSION_MAJOR as u8,
        NETPLAY_MIN_UQM_VERSION_MINOR as u8,
        NETPLAY_MIN_UQM_VERSION_PATCH as u8,
    );
    if !protocol_ok || version < minimum {
        sendAbort(conn, NetplayAbortReason_AbortReason_versionMismatch);
        abortFeedback(conn, NetplayAbortReason_AbortReason_versionMismatch);
        if !protocol_ok {
            log::error!(
                "Protocol version {}.{} not supported.",
                protocol[0],
                protocol[1]
            );
        } else {
            log::error!("Remote side is running a version of UQM that is too old ({}.{}.{}; {}.{}.{} is required).", version.0, version.1, version.2, minimum.0, minimum.1, minimum.2);
        }
        super::set_errno(libc::ENOSYS);
        return -1;
    }
    Netplay_remoteReady(conn);
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Ping(conn: *mut NetConnection, id: u32) -> i32 {
    if !test_state((*conn).state > NetState_NetState_init, "Ping") {
        return -1;
    }
    // Preserve the C handler's pass-through of the stored ID to sendAck.
    sendAck(conn, u32::from_ne_bytes(id.to_be_bytes()));
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Ack(conn: *mut NetConnection) -> i32 {
    if !test_state((*conn).state > NetState_NetState_init, "Ack") {
        return -1;
    }
    0
}

fn ready_meaningful(state: NetState) -> bool {
    [
        NetState_NetState_init,
        NetState_NetState_preBattle,
        NetState_NetState_selectShip,
        NetState_NetState_interBattle,
        NetState_NetState_inBattle,
        NetState_NetState_endingBattle,
        NetState_NetState_endingBattle2,
    ]
    .contains(&state)
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Ready(conn: *mut NetConnection) -> i32 {
    guard!(
        conn,
        ready_meaningful((*conn).state) && !(*conn).stateFlags.ready.remoteReady(),
        "Ready"
    );
    Netplay_remoteReady(conn);
    0
}

unsafe fn local_side(conn: *const NetConnection, side: u8) -> COUNT {
    if side == NetplaySide_NetplaySide_local as u8 {
        (*conn).player as COUNT
    } else {
        (1 - (*conn).player) as COUNT
    }
}

unsafe fn cancel_confirmation(conn: *mut NetConnection, data: *mut BattleStateData) {
    if (*conn).stateFlags.handshake.localOk() {
        Netplay_cancelConfirmation(conn);
        confirmationCancelled((*data).meleeState, (*conn).player as COUNT);
    }
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Fleet(conn: *mut NetConnection, side: u8, ships: &[(u8, u8)]) -> i32 {
    guard!(conn, (*conn).state == NetState_NetState_inSetup, "Fleet");
    let player = local_side(conn, side);
    let data = (*conn).stateData.cast::<BattleStateData>();
    cancel_confirmation(conn, data);
    for &(index, ship) in ships {
        let ship = ship as MeleeShip;
        let position = index as FleetShipIndex;
        if !(ship < NUM_MELEE_SHIPS || ship == MeleeShip_MELEE_NONE) {
            log::warn!(
                "Invalid ship type number {ship} (max = {}).",
                NUM_MELEE_SHIPS - 1
            );
            return bad_message();
        }
        if u32::from(position) >= MELEE_FLEET_SIZE {
            log::warn!(
                "Invalid ship position number {position} (max = {}).",
                MELEE_FLEET_SIZE - 1
            );
            return bad_message();
        }
        Melee_RemoteChange_ship((*data).meleeState, conn, player, position, ship);
    }
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_TeamName(conn: *mut NetConnection, side: u8, name: &[u8]) -> i32 {
    guard!(conn, (*conn).state == NetState_NetState_inSetup, "Fleet");
    let data = (*conn).stateData.cast::<BattleStateData>();
    cancel_confirmation(conn, data);
    let side = local_side(conn, side);
    let len = name.len().min(MAX_TEAM_CHARS as usize);
    let mut buffer = [0u8; MAX_TEAM_CHARS as usize + 1];
    buffer[..len].copy_from_slice(&name[..len]);
    Melee_RemoteChange_teamName((*data).meleeState, conn, side, buffer.as_ptr().cast());
    0
}

unsafe fn handshake_complete(conn: *mut NetConnection) {
    assert!(!(*conn).stateFlags.handshake.localOk());
    assert!(!(*conn).stateFlags.handshake.remoteOk());
    assert!(!(*conn).stateFlags.handshake.canceling());
    assert_eq!((*conn).state, NetState_NetState_inSetup);
    (*conn).state = NetState_NetState_preBattle;
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Handshake0(conn: *mut NetConnection) -> i32 {
    guard!(
        conn,
        (*conn).state == NetState_NetState_inSetup && !(*conn).stateFlags.handshake.remoteOk(),
        "Handshake0"
    );
    (*conn).stateFlags.handshake.set_remoteOk(true);
    if (*conn).stateFlags.handshake.localOk() && !(*conn).stateFlags.handshake.canceling() {
        sendHandshake1(conn);
    }
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Handshake1(conn: *mut NetConnection) -> i32 {
    guard!(
        conn,
        (*conn).state == NetState_NetState_inSetup
            && ((*conn).stateFlags.handshake.localOk() || (*conn).stateFlags.handshake.canceling()),
        "Handshake1"
    );
    if (*conn).stateFlags.handshake.canceling() {
        (*conn).stateFlags.handshake.set_remoteOk(true);
    } else {
        let remote_was_ok = (*conn).stateFlags.handshake.remoteOk();
        (*conn).stateFlags.handshake.set_localOk(false);
        (*conn).stateFlags.handshake.set_remoteOk(false);
        if !remote_was_ok {
            sendHandshake1(conn);
        }
        handshake_complete(conn);
    }
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_HandshakeCancel(conn: *mut NetConnection) -> i32 {
    guard!(
        conn,
        (*conn).state == NetState_NetState_inSetup && (*conn).stateFlags.handshake.remoteOk(),
        "HandshakeCancel"
    );
    (*conn).stateFlags.handshake.set_remoteOk(false);
    sendHandshakeCancelAck(conn);
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_HandshakeCancelAck(conn: *mut NetConnection) -> i32 {
    guard!(
        conn,
        (*conn).state == NetState_NetState_inSetup && (*conn).stateFlags.handshake.canceling(),
        "HandshakeCancelAck"
    );
    (*conn).stateFlags.handshake.set_canceling(false);
    if (*conn).stateFlags.handshake.localOk() {
        if (*conn).stateFlags.handshake.remoteOk() {
            sendHandshake1(conn);
        } else {
            sendHandshake0(conn);
        }
    }
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_SeedRandom(conn: *mut NetConnection, seed: u32) -> i32 {
    guard!(
        conn,
        (*conn).state == NetState_NetState_preBattle && !(*conn).stateFlags.discriminant,
        "SeedRandom"
    );
    let data = (*conn).stateData.cast::<BattleStateData>();
    updateRandomSeed((*data).meleeState, (*conn).player as COUNT, seed);
    (*conn).stateFlags.agreement.set_randomSeed(true);
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_InputDelay(conn: *mut NetConnection, delay: u32) -> i32 {
    guard!(
        conn,
        (*conn).state == NetState_NetState_preBattle,
        "InputDelay"
    );
    if delay > BATTLE_FRAME_RATE {
        log::error!(
            "NETPLAY: [{}] Received absurdly large input delay value ({delay}).",
            (*conn).player as COUNT
        );
        return -1;
    }
    (*conn).stateFlags.inputDelay = delay as usize;
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_SelectShip(conn: *mut NetConnection, ship: u16) -> i32 {
    guard!(
        conn,
        (*conn).state == NetState_NetState_selectShip,
        "SelectShip"
    );
    let data = (*conn).stateData.cast::<BattleStateData>();
    if !updateMeleeSelection((*data).getMeleeState, (*conn).player as COUNT, ship) {
        return bad_message();
    }
    0
}

fn battle_active(state: NetState) -> bool {
    [
        NetState_NetState_inBattle,
        NetState_NetState_endingBattle,
        NetState_NetState_endingBattle2,
    ]
    .contains(&state)
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_BattleInput(conn: *mut NetConnection, input: u8) -> i32 {
    guard!(conn, battle_active((*conn).state), "BattleInput");
    if !(*(*conn).session).push_input((*conn).player as usize, input) {
        return -1;
    }
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_FrameCount(conn: *mut NetConnection, frame: u32) -> i32 {
    guard!(
        conn,
        (*conn).state == NetState_NetState_endingBattle,
        "FrameCount"
    );
    let data = (*conn).stateData.cast::<BattleStateData>();
    (*data).endFrameCount = (*data).endFrameCount.max(frame);
    Netplay_remoteReady(conn);
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Checksum(conn: *mut NetConnection, frame: u32, checksum: u32) -> i32 {
    guard!(conn, battle_active((*conn).state), "Checksum");
    let interval = (*conn).stateFlags.checksumInterval;
    let delay = (*(*conn).session).get_battle_input_delay();
    if !(frame as usize).is_multiple_of(interval) {
        log::warn!("NETPLAY: [{}] Received checksum for frame {frame}, expected a multiple of {interval}; discarding.", (*conn).player);
        return 0;
    }
    if frame as usize > (*(*conn).session).frame_count.get() as usize + delay + 1 {
        log::warn!(
            "NETPLAY: [{}] Received checksum too far in the future ({frame}); discarding.",
            (*conn).player as COUNT
        );
        return 0;
    }
    if (frame as usize) + delay < (*(*conn).session).frame_count.get() as usize {
        log::warn!(
            "NETPLAY: [{}] Received checksum too far in the past ({frame}); discarding.",
            (*conn).player as COUNT
        );
        return 0;
    }
    (*(*conn).session).add_remote_checksum(conn, frame, checksum);
    0
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Abort(conn: *mut NetConnection, reason: u16) -> i32 {
    // Match the existing abort feedback's interpretation of the stored reason.
    abortFeedback(
        conn,
        u16::from_ne_bytes(reason.to_be_bytes()) as NetplayAbortReason,
    );
    -1
}

/// # Safety
/// Called on the game thread with a live connection and a validated,
/// complete packet of this handler's type.
unsafe fn handle_Reset(conn: *mut NetConnection, reason: u16) -> i32 {
    if !test_state(!(*conn).stateFlags.reset.remoteReset(), "Reset") {
        return -1;
    }
    Netplay_remoteReset(conn, reason as NetplayResetReason);
    0
}

#[cfg(test)]
mod tests {
    use super::super::packet::*;
    use super::super::packetq::*;
    use super::*;

    unsafe fn connection(state: NetState) -> Box<NetConnection> {
        let mut conn: Box<NetConnection> = Box::new(std::mem::zeroed());
        conn.state = state;
        conn.stateFlags.connected = true;
        PacketQueue_init(&raw mut conn.queue);
        conn
    }

    unsafe fn queued_type(conn: &NetConnection) -> u16 {
        (*(*conn.queue.first).packet).kind() as u16
    }

    #[test]
    fn handshake_order_cancel_and_reconfirmation() {
        unsafe {
            let mut conn = connection(NetState_NetState_inSetup);
            conn.stateFlags.handshake.set_localOk(true);
            let p = Packet::Handshake0;
            assert_eq!(dispatch(&mut *conn, &p), 0);
            assert!(conn.stateFlags.handshake.remoteOk());
            assert_eq!(queued_type(&conn), 7);
            let p = Packet::Handshake1;
            assert_eq!(dispatch(&mut *conn, &p), 0);
            assert_eq!(conn.state, NetState_NetState_preBattle);
            assert!(!conn.stateFlags.handshake.localOk());
            assert!(!conn.stateFlags.handshake.remoteOk());
            PacketQueue_uninit(&raw mut conn.queue);

            let mut conn = connection(NetState_NetState_inSetup);
            conn.stateFlags.handshake.set_localOk(true);
            let p = Packet::Handshake1;
            assert_eq!(dispatch(&mut *conn, &p), 0);
            assert_eq!(queued_type(&conn), 7);
            assert_eq!(conn.state, NetState_NetState_preBattle);
            PacketQueue_uninit(&raw mut conn.queue);

            for remote_ok in [false, true] {
                let mut conn = connection(NetState_NetState_inSetup);
                conn.stateFlags.handshake.set_canceling(true);
                conn.stateFlags.handshake.set_localOk(true);
                conn.stateFlags.handshake.set_remoteOk(remote_ok);
                let p = Packet::HandshakeCancelAck;
                assert_eq!(dispatch(&mut *conn, &p), 0);
                assert!(!conn.stateFlags.handshake.canceling());
                assert_eq!(queued_type(&conn), if remote_ok { 7 } else { 6 });
                PacketQueue_uninit(&raw mut conn.queue);
            }

            let mut conn = connection(NetState_NetState_inSetup);
            conn.stateFlags.handshake.set_remoteOk(true);
            let p = Packet::HandshakeCancel;
            assert_eq!(dispatch(&mut *conn, &p), 0);
            assert!(!conn.stateFlags.handshake.remoteOk());
            assert_eq!(queued_type(&conn), 9);
            PacketQueue_uninit(&raw mut conn.queue);
        }
    }

    #[test]
    fn reset_gate_and_wrong_state_do_not_change_handshake() {
        unsafe {
            let mut conn = connection(NetState_NetState_inBattle);
            let p = Packet::Handshake0;
            assert_eq!(dispatch(&mut *conn, &p), -1);
            assert!(!conn.stateFlags.handshake.remoteOk());
            conn.stateFlags.reset.set_localReset(true);
            assert_eq!(dispatch(&mut *conn, &p), 0);
            conn.stateFlags.reset.set_localReset(false);
            conn.stateFlags.reset.set_remoteReset(true);
            assert_eq!(dispatch(&mut *conn, &p), -1);
            assert!(!conn.stateFlags.handshake.remoteOk());
            assert_eq!(conn.queue.size, 0);
            PacketQueue_uninit(&raw mut conn.queue);
        }
    }

    #[test]
    fn input_delay_decodes_and_rejects_over_limit() {
        unsafe {
            let mut conn = connection(NetState_NetState_preBattle);
            for delay in [0, BATTLE_FRAME_RATE] {
                let p = Packet::InputDelay(delay);
                assert_eq!(dispatch(&mut *conn, &p), 0);
                assert_eq!(conn.stateFlags.inputDelay, delay as usize);
            }
            let p = Packet::InputDelay(BATTLE_FRAME_RATE + 1);
            assert_eq!(dispatch(&mut *conn, &p), -1);
            assert_eq!(conn.stateFlags.inputDelay, BATTLE_FRAME_RATE as usize);
            PacketQueue_uninit(&raw mut conn.queue);
        }
    }

    #[test]
    fn stream_packets_decode_without_native_alignment() {
        unsafe {
            for offset in 0..4 {
                let mut storage = [0u8; 16];
                let bytes = &mut storage[offset..offset + 8];
                bytes.copy_from_slice(&[0, 8, 0, 11, 0, 0, 0, 17]);
                let mut conn = connection(NetState_NetState_preBattle);
                assert_eq!(dispatch(&mut *conn, &Packet::decode(bytes).unwrap().0), 0);
                assert_eq!(conn.stateFlags.inputDelay, 17);
                bytes.copy_from_slice(&[0, 8, 0, 1, 0x12, 0x34, 0x56, 0x78]);
                assert_eq!(dispatch(&mut *conn, &Packet::decode(bytes).unwrap().0), 0);
                let ack = &*(*conn.queue.first).packet;
                // The C handler passes the stored, network-order ID to sendAck.
                assert_eq!(
                    match ack {
                        Packet::Ack(id) => *id,
                        other => panic!("expected Ack, got {other:?}"),
                    },
                    u32::from_ne_bytes([0x12, 0x34, 0x56, 0x78])
                );
                PacketQueue_uninit(&raw mut conn.queue);
            }
        }
    }
}
