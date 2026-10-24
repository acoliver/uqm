//! Netplay orchestration, serialized on the game thread.
use super::abi::*;
use super::{
    netconnection::{
        NetConnection_close, NetConnection_open, NetConnection_setReadyCallback,
        NetConnection_setState,
    },
    netinput::{getBattleInputBuffer, setBattleInputDelay, BattleInputBuffer_pop},
    netmisc::{NetMelee_closeCallback, NetMelee_connectCallback, NetMelee_errorCallback},
    netoptions::netplayOptions,
    npconfirm::{Netplay_cancelConfirmation, Netplay_confirm},
    packetq::flushPacketQueue,
    packetsenders::sendAbort,
    ready::Netplay_localReady,
    reset::{Netplay_isLocalReset, Netplay_localReset, Netplay_setResetCallback},
};
use std::{ffi::c_void, ptr};

#[no_mangle]
pub static mut netConnections: [*mut NetConnection; NETPLAY_NUM_PLAYERS as usize] =
    [ptr::null_mut(); NETPLAY_NUM_PLAYERS as usize];
pub static mut NUM_NET_CONNECTIONS: usize = 0;

/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn addNetConnection(conn: *mut NetConnection, player: i32) {
    netConnections[player as usize] = conn;
    NUM_NET_CONNECTIONS += 1;
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn removeNetConnection(player: i32) {
    netConnections[player as usize] = ptr::null_mut();
    NUM_NET_CONNECTIONS -= 1;
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn getNumNetConnections() -> usize {
    NUM_NET_CONNECTIONS
}

unsafe fn connection(player: usize) -> *mut NetConnection {
    *ptr::addr_of!(netConnections)
        .cast::<*mut NetConnection>()
        .add(player)
}

unsafe fn each(mut action: impl FnMut(*mut NetConnection) -> bool) -> bool {
    for player in 0..NETPLAY_NUM_PLAYERS as usize {
        let conn = connection(player);
        if conn.is_null() || !(*conn).stateFlags.connected {
            continue;
        }
        if !action(conn) {
            return false;
        }
    }
    true
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn forEachConnectedPlayer(
    callback: ForEachConnectionCallback,
    arg: *mut c_void,
) -> bool {
    each(|conn| callback.expect("connection callback")(conn, arg))
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn closeAllConnections() {
    for player in 0..NETPLAY_NUM_PLAYERS as usize {
        if !connection(player).is_null() {
            closePlayerNetworkConnection(player as COUNT);
        }
    }
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn closeDisconnectedConnections() {
    for player in 0..NETPLAY_NUM_PLAYERS as usize {
        let conn = connection(player);
        if !conn.is_null() && !(*conn).stateFlags.connected {
            closePlayerNetworkConnection(player as COUNT);
        }
    }
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn NetMelee_getMeleeState(conn: *mut NetConnection) -> *mut melee_state {
    if (*conn).state > NetState_NetState_connecting {
        (*(*conn).stateData.cast::<BattleStateData>()).meleeState
    } else {
        (*conn).extra.cast()
    }
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn NetMelee_getBattleState(
    conn: *mut NetConnection,
) -> *mut battlestate_struct {
    if (*conn).state > NetState_NetState_connecting {
        (*(*conn).stateData.cast::<BattleStateData>()).battleState
    } else {
        ptr::null_mut()
    }
}

unsafe fn input_aux(mut timeout: u32) {
    NetManager_process(&mut timeout);
    Async_process();
    flushPacketQueues();
    // A flush may detect a disconnect. Dispatch it before another flush.
    Callback_process();
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn netInput() {
    input_aux(0);
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn netInputBlocking(timeout: u32) {
    input_aux(timeout.min(Async_timeBeforeNextMs()));
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn flushPacketQueues() {
    for player in 0..NETPLAY_NUM_PLAYERS as usize {
        let conn = connection(player);
        if conn.is_null() || !(*conn).stateFlags.connected {
            continue;
        }
        if flushPacketQueue(conn) == -1 {
            let err = std::io::Error::last_os_error()
                .raw_os_error()
                .expect("errno");
            if err != libc::EAGAIN && err != libc::EWOULDBLOCK {
                closePlayerNetworkConnection(player as COUNT);
            }
        }
    }
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn confirmConnections() {
    each(|conn| {
        Netplay_confirm(conn);
        true
    });
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn cancelConfirmations() {
    each(|conn| {
        Netplay_cancelConfirmation(conn);
        true
    });
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn connectionsLocalReady(
    callback: NetConnection_ReadyCallback,
    arg: *mut c_void,
) {
    each(|conn| {
        Netplay_localReady(conn, callback, arg, true);
        true
    });
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn allConnected() -> bool {
    for player in 0..NETPLAY_NUM_PLAYERS as usize {
        let conn = connection(player);
        if !conn.is_null() && !(*conn).stateFlags.connected {
            return false;
        }
    }
    true
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn initBattleStateDataConnections() {
    for player in 0..NETPLAY_NUM_PLAYERS as usize {
        let conn = connection(player);
        if !conn.is_null() {
            (*(*conn).stateData.cast::<BattleStateData>()).endFrameCount = 0;
        }
    }
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn setBattleStateConnections(state: *mut battlestate_struct) {
    for player in 0..NETPLAY_NUM_PLAYERS as usize {
        let conn = connection(player);
        if !conn.is_null() {
            (*(*conn).stateData.cast::<BattleStateData>()).battleState = state;
        }
    }
}

/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn networkBattleInput(
    context: *mut NetworkInputContext,
    _ship: *mut STARSHIP,
) -> BATTLE_INPUT_STATE {
    use crate::mainloop::{
        activity_flags::CHECK_ABORT, get_current_activity, set_current_activity, ActivityValue,
    };
    let player = (*context).playerNr as usize;
    let buffer = getBattleInputBuffer(player);
    let mut result = 0;
    loop {
        if BattleInputBuffer_pop(buffer, &mut result) {
            return result;
        }
        let conn = connection(player);
        netInput();
        if !(*conn).stateFlags.connected {
            set_current_activity(ActivityValue(get_current_activity().0 | CHECK_ABORT));
            return 0;
        }
        if get_current_activity().0 & CHECK_ABORT != 0 {
            return 0;
        }
        netInputBlocking(500);
        if !(*conn).stateFlags.connected {
            set_current_activity(ActivityValue(get_current_activity().0 | CHECK_ABORT));
            return 0;
        }
    }
}
unsafe extern "C" fn deleted(conn: *mut NetConnection) {
    removeNetConnection((*conn).player);
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn openPlayerNetworkConnection(
    player: COUNT,
    extra: *mut c_void,
) -> *mut NetConnection {
    assert!(netConnections[player as usize].is_null());
    let conn = NetConnection_open(
        player as i32,
        ptr::addr_of!(netplayOptions.peer)
            .cast::<NetplayPeerOptions>()
            .add(player as usize),
        Some(NetMelee_connectCallback),
        Some(NetMelee_closeCallback),
        Some(NetMelee_errorCallback),
        Some(deleted),
        extra,
    );
    addNetConnection(conn, player as i32);
    conn
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn closePlayerNetworkConnection(player: COUNT) {
    let conn = netConnections[player as usize];
    assert!(!conn.is_null());
    NetConnection_close(conn);
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn setupInputDelay(local_delay: usize) -> bool {
    let mut have_network = false;
    let mut delay = 0;
    each(|conn| {
        have_network = true;
        delay = delay.max((*conn).stateFlags.inputDelay);
        true
    });
    if have_network && delay < local_delay {
        delay = local_delay;
    }
    setBattleInputDelay(delay);
    true
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn setStateConnections(state: NetState) -> bool {
    each(|conn| {
        NetConnection_setState(conn, state);
        true
    })
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn sendAbortConnections(reason: NetplayAbortReason) -> bool {
    each(|conn| {
        sendAbort(conn, reason);
        true
    })
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn resetConnections(reason: NetplayResetReason) -> bool {
    each(|conn| {
        Netplay_localReset(conn, reason);
        true
    })
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn localReadyConnections(
    callback: NetConnection_ReadyCallback,
    arg: *mut c_void,
    notify: bool,
) -> bool {
    each(|conn| {
        Netplay_localReady(conn, callback, arg, notify);
        true
    })
}

#[repr(C)]
struct TransitionWait {
    input: Option<unsafe extern "C" fn(*mut c_void) -> BOOLEAN>,
    conn: *mut NetConnection,
    next: NetState,
    done: bool,
}
const NETWORK_POLL_DELAY: u32 = crate::time::counter_abi::ONE_SECOND / 24;
unsafe extern "C" fn transition_input(arg: *mut c_void) -> BOOLEAN {
    let state = arg.cast::<TransitionWait>();
    netInputBlocking(NETWORK_POLL_DELAY);
    if !(*(*state).conn).stateFlags.connected {
        return 0;
    }
    u32::from(!(*state).done)
}
unsafe extern "C" fn ready_transition(conn: *mut NetConnection, arg: *mut c_void) {
    let state = arg.cast::<TransitionWait>();
    // Transition before the receiver can dispatch the next packet.
    NetConnection_setState(conn, (*state).next);
    (*state).done = true;
}
unsafe extern "C" fn reset_transition(conn: *mut NetConnection, arg: *mut c_void) {
    let state = arg.cast::<TransitionWait>();
    if (*state).next != NetState::MAX {
        NetConnection_setState(conn, (*state).next);
    }
    (*state).done = true;
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn negotiateReady(
    conn: *mut NetConnection,
    notify: bool,
    next: NetState,
) -> bool {
    let mut state = TransitionWait {
        input: Some(transition_input),
        conn,
        next,
        done: false,
    };
    Netplay_localReady(
        conn,
        Some(ready_transition),
        (&raw mut state).cast(),
        notify,
    );
    flushPacketQueue(conn);
    if !state.done {
        DoInput((&raw mut state).cast(), 0);
    }
    (*conn).stateFlags.connected
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn waitReset(conn: *mut NetConnection, next: NetState) -> bool {
    let mut state = TransitionWait {
        input: Some(transition_input),
        conn,
        next,
        done: false,
    };
    Netplay_setResetCallback(conn, Some(reset_transition), (&raw mut state).cast());
    if !state.done {
        if !Netplay_isLocalReset(conn) {
            Netplay_localReset(conn, NetplayResetReason_ResetReason_manualReset);
            flushPacketQueue(conn);
        }
        if !state.done {
            DoInput((&raw mut state).cast(), 0);
        }
    }
    (*conn).stateFlags.connected
}
unsafe fn wait_connections(mut wait: impl FnMut(*mut NetConnection)) -> bool {
    let mut disconnected = 0;
    for player in 0..NETPLAY_NUM_PLAYERS as usize {
        let conn = connection(player);
        if conn.is_null() {
            continue;
        }
        if !(*conn).stateFlags.connected {
            disconnected += 1;
            continue;
        }
        wait(conn);
    }
    disconnected == 0
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn negotiateReadyConnections(notify: bool, next: NetState) -> bool {
    wait_connections(|conn| {
        negotiateReady(conn, notify, next);
    })
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn waitResetConnections(next: NetState) -> bool {
    wait_connections(|conn| {
        waitReset(conn, next);
    })
}

#[repr(C)]
struct ReadyWait {
    input: Option<unsafe extern "C" fn(*mut c_void) -> BOOLEAN>,
    conn: *mut NetConnection,
    callback: NetConnection_ReadyCallback,
    arg: *mut c_void,
    done: bool,
}
unsafe extern "C" fn wait_ready_callback(conn: *mut NetConnection, arg: *mut c_void) {
    let state = arg.cast::<ReadyWait>();
    (*state).done = true;
    (*state).callback.expect("original ready callback")(conn, (*state).arg);
}
unsafe extern "C" fn wait_ready_input(arg: *mut c_void) -> BOOLEAN {
    let state = arg.cast::<ReadyWait>();
    netInputBlocking(NETWORK_POLL_DELAY);
    if !(*(*state).conn).stateFlags.connected {
        return 0;
    }
    u32::from(!(*state).done)
}
/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn waitReady(conn: *mut NetConnection) -> bool {
    let mut state = ReadyWait {
        input: Some(wait_ready_input),
        conn,
        callback: (*conn).readyCallback,
        arg: (*conn).readyCallbackArg,
        done: false,
    };
    NetConnection_setReadyCallback(conn, Some(wait_ready_callback), (&raw mut state).cast());
    DoInput((&raw mut state).cast(), 0);
    (*conn).stateFlags.connected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn connection_iteration_order_short_circuit_and_input_delay_negotiation() {
        unsafe {
            let saved_connections = netConnections;
            let saved_count = NUM_NET_CONNECTIONS;
            let saved_delay = getBattleInputDelay();
            netConnections = [ptr::null_mut(); 2];
            NUM_NET_CONNECTIONS = 0;
            assert!(allConnected());
            assert!(setupInputDelay(7));
            assert_eq!(getBattleInputDelay(), 0);
            let mut a: NetConnection = std::mem::zeroed();
            let mut b: NetConnection = std::mem::zeroed();
            a.player = 0;
            b.player = 1;
            a.stateFlags.connected = true;
            a.stateFlags.inputDelay = 3;
            b.stateFlags.inputDelay = 5;
            addNetConnection(&raw mut a, 0);
            addNetConnection(&raw mut b, 1);
            assert_eq!(getNumNetConnections(), 2);
            assert!(!allConnected());
            let mut visited = Vec::new();
            assert!(each(|conn| {
                visited.push((*conn).player);
                true
            }));
            assert_eq!(visited, [0]);
            (*connection(1)).stateFlags.connected = true;
            visited.clear();
            assert!(!each(|conn| {
                visited.push((*conn).player);
                false
            }));
            assert_eq!(visited, [0]);
            visited.clear();
            assert!(each(|conn| {
                visited.push((*conn).player);
                true
            }));
            assert_eq!(visited, [0, 1]);
            assert!(setupInputDelay(2));
            assert_eq!(getBattleInputDelay(), 5);
            assert!(setupInputDelay(7));
            assert_eq!(getBattleInputDelay(), 7);
            removeNetConnection(0);
            removeNetConnection(1);
            assert_eq!(getNumNetConnections(), 0);
            assert!(allConnected());
            netConnections = saved_connections;
            NUM_NET_CONNECTIONS = saved_count;
            setBattleInputDelay(saved_delay);
        }
    }

    #[test]
    fn ready_and_reset_callbacks_transition_before_returning_to_dispatch() {
        unsafe {
            let mut conn: NetConnection = std::mem::zeroed();
            conn.state = NetState_NetState_preBattle;
            let mut state = TransitionWait {
                input: Some(transition_input),
                conn: &raw mut conn,
                next: NetState_NetState_interBattle,
                done: false,
            };
            ready_transition(&raw mut conn, (&raw mut state).cast());
            assert_eq!(conn.state, NetState_NetState_interBattle);
            assert!(state.done);
            state.done = false;
            state.next = NetState::MAX;
            reset_transition(&raw mut conn, (&raw mut state).cast());
            assert_eq!(conn.state, NetState_NetState_interBattle);
            assert!(state.done);
            state.done = false;
            state.next = NetState_NetState_inSetup;
            reset_transition(&raw mut conn, (&raw mut state).cast());
            assert_eq!(conn.state, NetState_NetState_inSetup);
            assert!(state.done);
        }
    }
}
