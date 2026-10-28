use super::abi::*;
use super::packetsenders::{
    sendBattleInput, sendChecksum, sendFleet, sendFleetShip, sendFrameCount, sendInputDelay,
    sendSeedRandom, sendSelectShip, sendTeamName,
};

fn battle_active(state: NetState) -> bool {
    [
        NetState_NetState_inBattle,
        NetState_NetState_endingBattle,
        NetState_NetState_endingBattle2,
    ]
    .contains(&state)
}
unsafe fn side(conn: *const NetConnection, player: i32) -> NetplaySide {
    if player == (*conn).player {
        NetplaySide_NetplaySide_remote
    } else {
        NetplaySide_NetplaySide_local
    }
}
unsafe fn setup(conn: *const NetConnection) {
    assert_eq!((*conn).state, NetState_NetState_inSetup);
    assert!(!(*conn).stateFlags.handshake.localOk());
}
/// # Safety
/// The connection and any payload pointers are live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_Notify_shipSelected(
    conn: *mut NetConnection,
    index: FleetShipIndex,
) {
    assert_eq!((*conn).state, NetState_NetState_selectShip);
    sendSelectShip(conn, index);
}
/// # Safety
/// The connection and any payload pointers are live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_Notify_battleInput(
    conn: *mut NetConnection,
    input: BATTLE_INPUT_STATE,
) {
    assert!(battle_active((*conn).state));
    sendBattleInput(conn, input);
}
/// # Safety
/// The connection and any payload pointers are live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_Notify_setTeamName(
    conn: *mut NetConnection,
    player: i32,
    name: *const libc::c_char,
    len: usize,
) {
    setup(conn);
    sendTeamName(conn, side(conn, player), name, len);
}
/// # Safety
/// The connection and any payload pointers are live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_Notify_setFleet(
    conn: *mut NetConnection,
    player: i32,
    fleet: *const MeleeShip,
    size: usize,
) {
    setup(conn);
    sendFleet(conn, side(conn, player), fleet, size);
}
/// # Safety
/// The connection and any payload pointers are live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_Notify_setShip(
    conn: *mut NetConnection,
    player: i32,
    index: FleetShipIndex,
    ship: MeleeShip,
) {
    setup(conn);
    sendFleetShip(conn, side(conn, player), index, ship);
}
/// # Safety
/// The connection and any payload pointers are live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_Notify_seedRandom(conn: *mut NetConnection, seed: u32) {
    assert_eq!((*conn).state, NetState_NetState_preBattle);
    sendSeedRandom(conn, seed);
    (*conn).stateFlags.agreement.set_randomSeed(true);
}
/// # Safety
/// The connection and any payload pointers are live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_Notify_inputDelay(conn: *mut NetConnection, delay: u32) {
    assert_eq!((*conn).state, NetState_NetState_preBattle);
    sendInputDelay(conn, delay);
}
/// # Safety
/// The connection and any payload pointers are live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_Notify_frameCount(
    conn: *mut NetConnection,
    frame: BattleFrameCounter,
) {
    assert_eq!((*conn).state, NetState_NetState_endingBattle);
    sendFrameCount(conn, frame);
}
/// # Safety
/// The connection and any payload pointers are live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_Notify_checksum(
    conn: *mut NetConnection,
    frame: BattleFrameCounter,
    checksum: Checksum,
) {
    assert!(battle_active((*conn).state));
    sendChecksum(conn, frame, checksum);
}

#[cfg(test)]
mod tests {
    use super::super::packetq::{PacketQueue_init, PacketQueue_uninit};
    use super::*;
    #[test]
    fn fleet_side_is_relative_and_seed_marks_agreement_after_queueing() {
        unsafe {
            let mut conn: NetConnection = std::mem::zeroed();
            conn.player = 1;
            conn.stateFlags.connected = true;
            conn.state = NetState_NetState_inSetup;
            PacketQueue_init(&raw mut conn.queue);
            Netplay_Notify_setShip(&raw mut conn, 1, 4, MeleeShip_MELEE_ARILOU);
            assert_eq!(
                &*(*conn.queue.first).packet,
                &Packet::Fleet {
                    side: NetplaySide_NetplaySide_remote as u8,
                    ships: vec![(4, MeleeShip_MELEE_ARILOU as u8)]
                }
            );
            assert_eq!(side(&conn, 0), NetplaySide_NetplaySide_local);
            conn.state = NetState_NetState_preBattle;
            Netplay_Notify_seedRandom(&raw mut conn, 123);
            assert!(conn.stateFlags.agreement.randomSeed());
            assert_eq!(conn.queue.size, 2);
            PacketQueue_uninit(&raw mut conn.queue);
        }
    }
}
