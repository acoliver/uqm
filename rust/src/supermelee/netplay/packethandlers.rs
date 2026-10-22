use super::abi::*;
use super::abi::{
    UqmNetplayConstants_UQM_NETPLAY_BATTLE_FRAME_RATE as BATTLE_FRAME_RATE,
    UqmNetplayConstants_UQM_NETPLAY_NUM_MELEE_SHIPS as NUM_MELEE_SHIPS,
};
use super::checksum::addRemoteChecksum;
use super::packetsenders::{
    sendAbort, sendAck, sendHandshake0, sendHandshake1, sendHandshakeCancelAck,
};

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

// TeamName lengths can leave the next packet unaligned for native u32 loads.
macro_rules! field {
    ($packet:expr, $field:ident) => {
        std::ptr::addr_of!((*$packet).$field).read_unaligned()
    };
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

macro_rules! handler {
    ($name:ident, $ty:ty, |$conn:ident, $packet:ident| $body:block) => {
        /// # Safety
        /// Called on the game thread with a live connection and a validated,
        /// complete packet of this handler's type.
        #[no_mangle]
        pub unsafe extern "C" fn $name($conn: *mut NetConnection, $packet: *const $ty) -> i32 $body
    };
}

handler!(PacketHandler_Init, Packet_Init, |conn, packet| {
    guard!(
        conn,
        (*conn).state == NetState_NetState_init && !(*conn).stateFlags.ready.remoteReady(),
        "Init"
    );
    let packet = packet.read_unaligned();
    let protocol_ok = packet.protoVersion.major == NETPLAY_PROTOCOL_VERSION_MAJOR as u8
        && packet.protoVersion.minor == NETPLAY_PROTOCOL_VERSION_MINOR as u8;
    let version = (
        packet.uqmVersion.major,
        packet.uqmVersion.minor,
        packet.uqmVersion.patch,
    );
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
                packet.protoVersion.major,
                packet.protoVersion.minor
            );
        } else {
            log::error!("Remote side is running a version of UQM that is too old ({}.{}.{}; {}.{}.{} is required).", version.0, version.1, version.2, minimum.0, minimum.1, minimum.2);
        }
        super::set_errno(libc::ENOSYS);
        return -1;
    }
    Netplay_remoteReady(conn);
    0
});

handler!(PacketHandler_Ping, Packet_Ping, |conn, packet| {
    if !test_state((*conn).state > NetState_NetState_init, "Ping") {
        return -1;
    }
    // Preserve the C handler's pass-through of the stored ID to sendAck.
    sendAck(conn, field!(packet, id));
    0
});

handler!(PacketHandler_Ack, Packet_Ack, |conn, _packet| {
    if !test_state((*conn).state > NetState_NetState_init, "Ack") {
        return -1;
    }
    0
});

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

handler!(PacketHandler_Ready, Packet_Ready, |conn, _packet| {
    guard!(
        conn,
        ready_meaningful((*conn).state) && !(*conn).stateFlags.ready.remoteReady(),
        "Ready"
    );
    Netplay_remoteReady(conn);
    0
});

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

handler!(PacketHandler_Fleet, Packet_Fleet, |conn, packet| {
    guard!(conn, (*conn).state == NetState_NetState_inSetup, "Fleet");
    let count = u16::from_be(field!(packet, numShips)) as usize;
    let player = local_side(conn, field!(packet, side));
    let len = u16::from_be(field!(packet, header).len) as usize;
    if size_of::<Packet_Fleet>() + count * size_of::<FleetEntry>() > len {
        log::warn!(
            "Invalid fleet size. Specified size is {count}, actual size = {}",
            (len - size_of::<Packet_Fleet>()) / size_of::<FleetEntry>()
        );
        return bad_message();
    }
    let data = (*conn).stateData.cast::<BattleStateData>();
    cancel_confirmation(conn, data);
    for index in 0..count {
        let entry = packet
            .cast::<u8>()
            .add(size_of::<Packet_Fleet>() + index * size_of::<FleetEntry>())
            .cast::<FleetEntry>()
            .read_unaligned();
        let ship = entry.ship as MeleeShip;
        let position = entry.index as FleetShipIndex;
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
});

handler!(PacketHandler_TeamName, Packet_TeamName, |conn, packet| {
    guard!(conn, (*conn).state == NetState_NetState_inSetup, "Fleet");
    let packet_len = u16::from_be(field!(packet, header).len) as usize;
    if packet_len <= size_of::<Packet_TeamName>() {
        return bad_message();
    }
    let data = (*conn).stateData.cast::<BattleStateData>();
    cancel_confirmation(conn, data);
    let side = local_side(conn, field!(packet, side));
    let len = (packet_len - size_of::<Packet_TeamName>() - 1).min(MAX_TEAM_CHARS as usize);
    let mut buffer = [0u8; MAX_TEAM_CHARS as usize + 1];
    std::ptr::copy_nonoverlapping(
        packet.cast::<u8>().add(size_of::<Packet_TeamName>()),
        buffer.as_mut_ptr(),
        len,
    );
    Melee_RemoteChange_teamName((*data).meleeState, conn, side, buffer.as_ptr().cast());
    0
});

unsafe fn handshake_complete(conn: *mut NetConnection) {
    assert!(!(*conn).stateFlags.handshake.localOk());
    assert!(!(*conn).stateFlags.handshake.remoteOk());
    assert!(!(*conn).stateFlags.handshake.canceling());
    assert_eq!((*conn).state, NetState_NetState_inSetup);
    (*conn).state = NetState_NetState_preBattle;
}

handler!(
    PacketHandler_Handshake0,
    Packet_Handshake0,
    |conn, _packet| {
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
);

handler!(
    PacketHandler_Handshake1,
    Packet_Handshake1,
    |conn, _packet| {
        guard!(
            conn,
            (*conn).state == NetState_NetState_inSetup
                && ((*conn).stateFlags.handshake.localOk()
                    || (*conn).stateFlags.handshake.canceling()),
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
);

handler!(
    PacketHandler_HandshakeCancel,
    Packet_HandshakeCancel,
    |conn, _packet| {
        guard!(
            conn,
            (*conn).state == NetState_NetState_inSetup && (*conn).stateFlags.handshake.remoteOk(),
            "HandshakeCancel"
        );
        (*conn).stateFlags.handshake.set_remoteOk(false);
        sendHandshakeCancelAck(conn);
        0
    }
);

handler!(
    PacketHandler_HandshakeCancelAck,
    Packet_HandshakeCancelAck,
    |conn, _packet| {
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
);

handler!(
    PacketHandler_SeedRandom,
    Packet_SeedRandom,
    |conn, packet| {
        guard!(
            conn,
            (*conn).state == NetState_NetState_preBattle && !(*conn).stateFlags.discriminant,
            "SeedRandom"
        );
        let data = (*conn).stateData.cast::<BattleStateData>();
        updateRandomSeed(
            (*data).meleeState,
            (*conn).player as COUNT,
            u32::from_be(field!(packet, seed)),
        );
        (*conn).stateFlags.agreement.set_randomSeed(true);
        0
    }
);

handler!(
    PacketHandler_InputDelay,
    Packet_InputDelay,
    |conn, packet| {
        guard!(
            conn,
            (*conn).state == NetState_NetState_preBattle,
            "InputDelay"
        );
        let delay = u32::from_be(field!(packet, delay));
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
);

handler!(
    PacketHandler_SelectShip,
    Packet_SelectShip,
    |conn, packet| {
        guard!(
            conn,
            (*conn).state == NetState_NetState_selectShip,
            "SelectShip"
        );
        let data = (*conn).stateData.cast::<BattleStateData>();
        if !updateMeleeSelection(
            (*data).getMeleeState,
            (*conn).player as COUNT,
            u16::from_be(field!(packet, ship)),
        ) {
            return bad_message();
        }
        0
    }
);

fn battle_active(state: NetState) -> bool {
    [
        NetState_NetState_inBattle,
        NetState_NetState_endingBattle,
        NetState_NetState_endingBattle2,
    ]
    .contains(&state)
}

handler!(
    PacketHandler_BattleInput,
    Packet_BattleInput,
    |conn, packet| {
        guard!(conn, battle_active((*conn).state), "BattleInput");
        if !BattleInputBuffer_push(
            getBattleInputBuffer((*conn).player as usize),
            field!(packet, state),
        ) {
            return -1;
        }
        0
    }
);

handler!(
    PacketHandler_FrameCount,
    Packet_FrameCount,
    |conn, packet| {
        guard!(
            conn,
            (*conn).state == NetState_NetState_endingBattle,
            "FrameCount"
        );
        let frame = u32::from_be(field!(packet, frameCount));
        let data = (*conn).stateData.cast::<BattleStateData>();
        (*data).endFrameCount = (*data).endFrameCount.max(frame);
        Netplay_remoteReady(conn);
        0
    }
);

handler!(PacketHandler_Checksum, Packet_Checksum, |conn, packet| {
    guard!(conn, battle_active((*conn).state), "Checksum");
    let frame = u32::from_be(field!(packet, frameNr));
    let checksum = u32::from_be(field!(packet, checksum));
    let interval = (*conn).stateFlags.checksumInterval;
    let delay = getBattleInputDelay();
    if !(frame as usize).is_multiple_of(interval) {
        log::warn!("NETPLAY: [{}] Received checksum for frame {frame}, expected a multiple of {interval}; discarding.", (*conn).player);
        return 0;
    }
    if frame as usize > battleFrameCount as usize + delay + 1 {
        log::warn!(
            "NETPLAY: [{}] Received checksum too far in the future ({frame}); discarding.",
            (*conn).player as COUNT
        );
        return 0;
    }
    if (frame as usize) + delay < battleFrameCount as usize {
        log::warn!(
            "NETPLAY: [{}] Received checksum too far in the past ({frame}); discarding.",
            (*conn).player as COUNT
        );
        return 0;
    }
    addRemoteChecksum(conn, frame, checksum);
    0
});

handler!(PacketHandler_Abort, Packet_Abort, |conn, packet| {
    // Match the existing abort feedback's interpretation of the stored reason.
    abortFeedback(conn, field!(packet, reason) as NetplayAbortReason);
    -1
});

handler!(PacketHandler_Reset, Packet_Reset, |conn, packet| {
    if !test_state(!(*conn).stateFlags.reset.remoteReset(), "Reset") {
        return -1;
    }
    Netplay_remoteReset(
        conn,
        u16::from_be(field!(packet, reason)) as NetplayResetReason,
    );
    0
});

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
        u16::from_be((*(*conn.queue.first).packet).header.type_)
    }

    #[test]
    fn handshake_order_cancel_and_reconfirmation() {
        unsafe {
            let mut conn = connection(NetState_NetState_inSetup);
            conn.stateFlags.handshake.set_localOk(true);
            let p = Packet_Handshake0_create();
            assert_eq!(PacketHandler_Handshake0(&mut *conn, p), 0);
            Packet_delete(p.cast());
            assert!(conn.stateFlags.handshake.remoteOk());
            assert_eq!(queued_type(&conn), 7);
            let p = Packet_Handshake1_create();
            assert_eq!(PacketHandler_Handshake1(&mut *conn, p), 0);
            Packet_delete(p.cast());
            assert_eq!(conn.state, NetState_NetState_preBattle);
            assert!(!conn.stateFlags.handshake.localOk());
            assert!(!conn.stateFlags.handshake.remoteOk());
            PacketQueue_uninit(&raw mut conn.queue);

            let mut conn = connection(NetState_NetState_inSetup);
            conn.stateFlags.handshake.set_localOk(true);
            let p = Packet_Handshake1_create();
            assert_eq!(PacketHandler_Handshake1(&mut *conn, p), 0);
            Packet_delete(p.cast());
            assert_eq!(queued_type(&conn), 7);
            assert_eq!(conn.state, NetState_NetState_preBattle);
            PacketQueue_uninit(&raw mut conn.queue);

            for remote_ok in [false, true] {
                let mut conn = connection(NetState_NetState_inSetup);
                conn.stateFlags.handshake.set_canceling(true);
                conn.stateFlags.handshake.set_localOk(true);
                conn.stateFlags.handshake.set_remoteOk(remote_ok);
                let p = Packet_HandshakeCancelAck_create();
                assert_eq!(PacketHandler_HandshakeCancelAck(&mut *conn, p), 0);
                Packet_delete(p.cast());
                assert!(!conn.stateFlags.handshake.canceling());
                assert_eq!(queued_type(&conn), if remote_ok { 7 } else { 6 });
                PacketQueue_uninit(&raw mut conn.queue);
            }

            let mut conn = connection(NetState_NetState_inSetup);
            conn.stateFlags.handshake.set_remoteOk(true);
            let p = Packet_HandshakeCancel_create();
            assert_eq!(PacketHandler_HandshakeCancel(&mut *conn, p), 0);
            Packet_delete(p.cast());
            assert!(!conn.stateFlags.handshake.remoteOk());
            assert_eq!(queued_type(&conn), 9);
            PacketQueue_uninit(&raw mut conn.queue);
        }
    }

    #[test]
    fn reset_gate_and_wrong_state_do_not_change_handshake() {
        unsafe {
            let mut conn = connection(NetState_NetState_inBattle);
            let p = Packet_Handshake0_create();
            assert_eq!(PacketHandler_Handshake0(&mut *conn, p), -1);
            assert!(!conn.stateFlags.handshake.remoteOk());
            conn.stateFlags.reset.set_localReset(true);
            assert_eq!(PacketHandler_Handshake0(&mut *conn, p), 0);
            conn.stateFlags.reset.set_localReset(false);
            conn.stateFlags.reset.set_remoteReset(true);
            assert_eq!(PacketHandler_Handshake0(&mut *conn, p), -1);
            assert!(!conn.stateFlags.handshake.remoteOk());
            assert_eq!(conn.queue.size, 0);
            Packet_delete(p.cast());
            PacketQueue_uninit(&raw mut conn.queue);
        }
    }

    #[test]
    fn input_delay_decodes_and_rejects_over_limit() {
        unsafe {
            let mut conn = connection(NetState_NetState_preBattle);
            for delay in [0, BATTLE_FRAME_RATE] {
                let p = Packet_InputDelay_create(delay);
                assert_eq!(PacketHandler_InputDelay(&mut *conn, p), 0);
                assert_eq!(conn.stateFlags.inputDelay, delay as usize);
                Packet_delete(p.cast());
            }
            let p = Packet_InputDelay_create(BATTLE_FRAME_RATE + 1);
            assert_eq!(PacketHandler_InputDelay(&mut *conn, p), -1);
            assert_eq!(conn.stateFlags.inputDelay, BATTLE_FRAME_RATE as usize);
            Packet_delete(p.cast());
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
                assert_eq!(
                    PacketHandler_InputDelay(&mut *conn, bytes.as_ptr().cast()),
                    0
                );
                assert_eq!(conn.stateFlags.inputDelay, 17);
                bytes.copy_from_slice(&[0, 8, 0, 1, 0x12, 0x34, 0x56, 0x78]);
                assert_eq!(PacketHandler_Ping(&mut *conn, bytes.as_ptr().cast()), 0);
                let ack = (*conn.queue.first).packet.cast::<Packet_Ack>();
                // The C handler passes the stored, network-order ID to sendAck.
                assert_eq!(
                    u32::from_be((*ack).id),
                    u32::from_ne_bytes([0x12, 0x34, 0x56, 0x78])
                );
                PacketQueue_uninit(&raw mut conn.queue);
            }
        }
    }
}
