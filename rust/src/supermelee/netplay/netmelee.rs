//! Netplay orchestration, serialized on the game thread.
use super::abi::*;
use super::{
    netconnection::{
        NetConnection_close, NetConnection_open, NetConnection_setReadyCallback,
        NetConnection_setState,
    },
    netmisc::{NetMelee_closeCallback, NetMelee_connectCallback, NetMelee_errorCallback},
    npconfirm::{Netplay_cancelConfirmation, Netplay_confirm},
    packetq::flushPacketQueue,
    packetsenders::sendAbort,
    ready::Netplay_localReady,
    reset::{Netplay_isLocalReset, Netplay_localReset, Netplay_setResetCallback},
};
use std::{ffi::c_void, ptr};

/// # Safety
/// Called on the game thread with live connection, state and callback arguments.
#[no_mangle]
pub unsafe extern "C" fn NetMelee_getMeleeState(conn: *mut NetConnection) -> *mut melee_state {
    if (*conn).state > NetState_NetState_connecting {
        (*(*conn).stateData.cast::<BattleStateData>()).melee_state
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
        (*(*conn).stateData.cast::<BattleStateData>()).battle_state
    } else {
        ptr::null_mut()
    }
}

unsafe extern "C" fn deleted(conn: *mut NetConnection) {
    (*(*conn).session).remove_net_connection((*conn).player);
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
    (*(*(*state).conn).session).net_input_blocking(NETWORK_POLL_DELAY);
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
    (*(*(*state).conn).session).net_input_blocking(NETWORK_POLL_DELAY);
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

impl super::session::SessionCore {
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn add_net_connection(&self, conn: *mut NetConnection, player: i32) {
        let mut connections = self.connections.get();
        connections[player as usize] = conn;
        self.connections.set(connections);
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn remove_net_connection(&self, player: i32) {
        let mut connections = self.connections.get();
        connections[player as usize] = ptr::null_mut();
        self.connections.set(connections);
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn get_num_net_connections(&self) -> usize {
        self.connections
            .get()
            .iter()
            .filter(|conn| !conn.is_null())
            .count()
    }
    pub(crate) unsafe fn connection(&self, player: usize) -> *mut NetConnection {
        self.connections.get()[player]
    }
    pub(crate) unsafe fn each(&self, mut action: impl FnMut(*mut NetConnection) -> bool) -> bool {
        for player in 0..NETPLAY_NUM_PLAYERS as usize {
            let conn = self.connection(player);
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
    pub unsafe fn for_each_connected_player(
        &self,
        callback: ForEachConnectionCallback,
        arg: *mut c_void,
    ) -> bool {
        self.each(|conn| callback.expect("connection callback")(conn, arg))
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn close_all_connections(&self) {
        for player in 0..NETPLAY_NUM_PLAYERS as usize {
            if !self.connection(player).is_null() {
                self.close_player_network_connection(player as COUNT);
            }
        }
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn close_disconnected_connections(&self) {
        for player in 0..NETPLAY_NUM_PLAYERS as usize {
            let conn = self.connection(player);
            if !conn.is_null() && !(*conn).stateFlags.connected {
                self.close_player_network_connection(player as COUNT);
            }
        }
    }
    unsafe fn input_aux(&self, timeout: u32) {
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_millis(u64::from(timeout));
        let before = self.connection_progress();
        loop {
            Async_process();
            for conn in self.connections.get() {
                if !conn.is_null() && !(*conn).stateFlags.disconnected {
                    super::netconnection::poll(conn);
                }
            }
            self.flush_packet_queues();
            Callback_process();
            if self.connection_progress() != before || std::time::Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(
                deadline
                    .saturating_duration_since(std::time::Instant::now())
                    .min(std::time::Duration::from_millis(1)),
            );
        }
    }
    unsafe fn connection_progress(&self) -> [(u32, usize); 2] {
        self.connections.get().map(|conn| {
            if conn.is_null() {
                (NetState_NetState_unconnected, 0)
            } else {
                ((*conn).state, (*conn).statistics.packetsReceived)
            }
        })
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn net_input(&self) {
        self.input_aux(0);
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn net_input_blocking(&self, timeout: u32) {
        self.input_aux(timeout.min(Async_timeBeforeNextMs()));
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn flush_packet_queues(&self) {
        for player in 0..NETPLAY_NUM_PLAYERS as usize {
            let conn = self.connection(player);
            if conn.is_null() || !(*conn).stateFlags.connected {
                continue;
            }
            if flushPacketQueue(conn) == -1 {
                let err = crate::platform::errno();
                if err != libc::EAGAIN && err != libc::EWOULDBLOCK {
                    self.close_player_network_connection(player as COUNT);
                }
            }
        }
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn confirm_connections(&self) {
        self.each(|conn| {
            Netplay_confirm(conn);
            true
        });
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn cancel_confirmations(&self) {
        self.each(|conn| {
            Netplay_cancelConfirmation(conn);
            true
        });
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn connections_local_ready(
        &self,
        callback: NetConnection_ReadyCallback,
        arg: *mut c_void,
    ) {
        self.each(|conn| {
            Netplay_localReady(conn, callback, arg, true);
            true
        });
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn all_connected(&self) -> bool {
        for player in 0..NETPLAY_NUM_PLAYERS as usize {
            let conn = self.connection(player);
            if !conn.is_null() && !(*conn).stateFlags.connected {
                return false;
            }
        }
        true
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn init_battle_state_data_connections(&self) {
        for player in 0..NETPLAY_NUM_PLAYERS as usize {
            let conn = self.connection(player);
            if !conn.is_null() {
                (*(*conn).stateData.cast::<BattleStateData>()).end_frame_count = 0;
            }
        }
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn set_battle_state_connections(&self, state: *mut battlestate_struct) {
        for player in 0..NETPLAY_NUM_PLAYERS as usize {
            let conn = self.connection(player);
            if !conn.is_null() {
                (*(*conn).stateData.cast::<BattleStateData>()).battle_state = state;
            }
        }
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn open_player_network_connection(
        &self,
        player: COUNT,
        extra: *mut c_void,
    ) -> *mut NetConnection {
        assert!(self.connections.get()[player as usize].is_null());
        let conn = NetConnection_open(
            player as i32,
            &self.options.peer[player as usize],
            Some(NetMelee_connectCallback),
            Some(NetMelee_closeCallback),
            Some(NetMelee_errorCallback),
            Some(deleted),
            extra,
            self,
        );
        self.add_net_connection(conn, player as i32);
        conn
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn close_player_network_connection(&self, player: COUNT) {
        let conn = self.connections.get()[player as usize];
        assert!(!conn.is_null());
        NetConnection_close(conn);
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn setup_input_delay(&self, local_delay: usize) -> bool {
        let mut have_network = false;
        let mut delay = 0;
        self.each(|conn| {
            have_network = true;
            delay = delay.max((*conn).stateFlags.inputDelay);
            true
        });
        if have_network && delay < local_delay {
            delay = local_delay;
        }
        self.set_battle_input_delay(delay);
        true
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn set_state_connections(&self, state: NetState) -> bool {
        self.each(|conn| {
            NetConnection_setState(conn, state);
            true
        })
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn send_abort_connections(&self, reason: NetplayAbortReason) -> bool {
        self.each(|conn| {
            sendAbort(conn, reason);
            true
        })
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn reset_connections(&self, reason: NetplayResetReason) -> bool {
        self.each(|conn| {
            Netplay_localReset(conn, reason);
            true
        })
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn local_ready_connections(
        &self,
        callback: NetConnection_ReadyCallback,
        arg: *mut c_void,
        notify: bool,
    ) -> bool {
        self.each(|conn| {
            Netplay_localReady(conn, callback, arg, notify);
            true
        })
    }
    unsafe fn wait_connections(&self, mut wait: impl FnMut(*mut NetConnection)) -> bool {
        let mut disconnected = 0;
        for player in 0..NETPLAY_NUM_PLAYERS as usize {
            let conn = self.connection(player);
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
    pub unsafe fn negotiate_ready_connections(&self, notify: bool, next: NetState) -> bool {
        self.wait_connections(|conn| {
            negotiateReady(conn, notify, next);
        })
    }
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn wait_reset_connections(&self, next: NetState) -> bool {
        self.wait_connections(|conn| {
            waitReset(conn, next);
        })
    }
}

impl super::session::SessionCore {
    /// # Safety
    /// Called on the game thread with live connection, state and callback arguments.
    pub unsafe fn network_battle_input(&self, player: usize) -> BATTLE_INPUT_STATE {
        use crate::mainloop::{
            activity_flags::CHECK_ABORT, get_current_activity, set_current_activity, ActivityValue,
        };
        let session = self;
        loop {
            if let Some(input) = session.pop_input(player) {
                return input;
            }
            let conn = session.connection(player);
            session.net_input();
            if let Some(input) = session.pop_input(player) {
                return input;
            }
            if !(*conn).stateFlags.connected {
                set_current_activity(ActivityValue(get_current_activity().0 | CHECK_ABORT));
                return 0;
            }
            if get_current_activity().0 & CHECK_ABORT != 0 {
                return 0;
            }
            session.net_input_blocking(500);
            if !(*conn).stateFlags.connected {
                set_current_activity(ActivityValue(get_current_activity().0 | CHECK_ABORT));
                return 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn connection_iteration_order_short_circuit_and_input_delay_negotiation() {
        unsafe {
            let session = super::super::NetplaySession::default();
            assert!(session.all_connected());
            assert!(session.setup_input_delay(7));
            assert_eq!(session.get_battle_input_delay(), 0);
            let mut a: NetConnection = std::mem::zeroed();
            let mut b: NetConnection = std::mem::zeroed();
            a.player = 0;
            b.player = 1;
            a.stateFlags.connected = true;
            a.stateFlags.inputDelay = 3;
            b.stateFlags.inputDelay = 5;
            session.add_net_connection(&raw mut a, 0);
            session.add_net_connection(&raw mut b, 1);
            assert_eq!(session.get_num_net_connections(), 2);
            assert!(!session.all_connected());
            let mut visited = Vec::new();
            assert!(session.each(|conn| {
                visited.push((*conn).player);
                true
            }));
            assert_eq!(visited, [0]);
            (*session.connection(1)).stateFlags.connected = true;
            visited.clear();
            assert!(!session.each(|conn| {
                visited.push((*conn).player);
                false
            }));
            assert_eq!(visited, [0]);
            visited.clear();
            assert!(session.each(|conn| {
                visited.push((*conn).player);
                true
            }));
            assert_eq!(visited, [0, 1]);
            assert!(session.setup_input_delay(2));
            assert_eq!(session.get_battle_input_delay(), 5);
            assert!(session.setup_input_delay(7));
            assert_eq!(session.get_battle_input_delay(), 7);
            session.remove_net_connection(0);
            session.remove_net_connection(1);
            assert_eq!(session.get_num_net_connections(), 0);
            assert!(session.all_connected());
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
