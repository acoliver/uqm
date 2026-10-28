use super::abi::*;
use super::packetsenders::sendInit;
use super::ready::Netplay_localReady;
use std::ptr;

unsafe extern "C" fn release_battle(data: *mut NetConnectionStateData) {
    libc::free(data.cast());
}

/// # Safety
/// `conn` is newly connected and its extra pointer holds the live melee state.
#[no_mangle]
pub unsafe extern "C" fn NetMelee_connectCallback(conn: *mut NetConnection) {
    let melee = (*conn).extra.cast::<melee_state>();
    let data = libc::malloc(size_of::<BattleStateData>()).cast::<BattleStateData>();
    assert!(!data.is_null());
    // endFrameCount is initialized separately before battle, as in C.
    (*data).releaseFunction = Some(release_battle);
    (*data).meleeState = melee;
    (*data).battleState = ptr::null_mut();
    (*data).getMeleeState = ptr::null_mut();
    (*conn).stateData = data.cast();
    (*conn).extra = ptr::null_mut();
    MeleeSetup_resetSentTeams((*melee).meleeSetup);
    sendInit(conn);
    Netplay_localReady(conn, Some(enter_setup), ptr::null_mut(), false);
}
/// # Safety
/// `conn` is live and owns the melee state used by feedback.
#[no_mangle]
pub unsafe extern "C" fn NetMelee_closeCallback(conn: *mut NetConnection) {
    closeFeedback(conn);
}
/// # Safety
/// `conn` and `error` are live during this callback.
#[no_mangle]
pub unsafe extern "C" fn NetMelee_errorCallback(
    conn: *mut NetConnection,
    _error: *const NetConnectionError,
) {
    errorFeedback(conn);
}

unsafe extern "C" fn enter_setup(conn: *mut NetConnection, _arg: *mut libc::c_void) {
    NetConnection_setState(conn, NetState_NetState_inSetup);
    let data = (*conn).stateData.cast::<BattleStateData>();
    let melee = (*data).meleeState;
    let player = (*conn).player;
    connectedFeedback(conn);
    assert_eq!(NETPLAY_NUM_PLAYERS, 2);
    Melee_bootstrapSyncTeam(melee, player as usize);
    (*(*conn).session).flush_packet_queues();
}
