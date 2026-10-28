use super::abi::*;
use super::netconnection::{NetConnection_getState, NetConnection_getStateData};
use super::netmelee::{negotiateReady, waitReady};
use super::notify::{Netplay_Notify_frameCount, Netplay_Notify_shipSelected};
use super::ready::{Netplay_isLocalReady, Netplay_localReady};
use super::session::SessionCore;
use crate::engine::battle::net::BattleNet;
use std::{ffi::c_void, ptr};

unsafe extern "C" fn readyToEnd2Callback(conn: *mut NetConnection, _arg: *mut c_void) {
    NetConnection_setState(conn, NetState_NetState_endingBattle2);
}

unsafe extern "C" fn readyToEndCallback(conn: *mut NetConnection, _arg: *mut c_void) {
    let data = NetConnection_getStateData(conn).cast::<BattleStateData>();
    NetConnection_setState(conn, NetState_NetState_endingBattle);
    let target = (*(*conn).session).frame_count.get().wrapping_add(1);
    if target > (*data).endFrameCount {
        (*data).endFrameCount = target;
    }
    Netplay_Notify_frameCount(conn, target);
    flushPacketQueue(conn.cast());
    Netplay_localReady(
        conn.cast(),
        Some(readyToEnd2Callback),
        ptr::null_mut(),
        false,
    );
}

unsafe fn readyForBattleEndPlayer(conn: *mut NetConnection) -> bool {
    let data = NetConnection_getStateData(conn).cast::<BattleStateData>();
    let state = NetConnection_getState(conn);
    if state == NetState_NetState_interBattle || state == NetState_NetState_inSetup {
        return true;
    }
    if state == NetState_NetState_inBattle {
        if Netplay_isLocalReady(conn.cast()) {
            return false;
        }
        Netplay_localReady(conn.cast(), Some(readyToEndCallback), ptr::null_mut(), true);
        if NetConnection_getState(conn) == NetState_NetState_inBattle {
            return false;
        }
    }
    let state = NetConnection_getState(conn);
    assert!(state == NetState_NetState_endingBattle || state == NetState_NetState_endingBattle2);
    if (*(*conn).session).frame_count.get() < (*data).endFrameCount {
        return false;
    }
    if NetConnection_getState(conn) == NetState_NetState_endingBattle {
        waitReady(conn);
        assert_eq!(
            NetConnection_getState(conn),
            NetState_NetState_endingBattle2
        );
        if (*(*conn).session).frame_count.get() < (*data).endFrameCount {
            return false;
        }
    }
    negotiateReady(conn, true, NetState_NetState_interBattle);
    true
}

impl BattleNet for SessionCore {
    fn poll(&self) {
        unsafe {
            self.net_input();
        }
    }
    fn flush(&self) {
        unsafe {
            self.flush_packet_queues();
        }
    }
    fn all_connected(&self) -> bool {
        unsafe { self.all_connected() }
    }
    fn peer_count(&self) -> usize {
        unsafe { self.get_num_net_connections() }
    }
    fn player_order(&self, index: u16) -> u16 {
        unsafe {
            let a = self.connection(0);
            let b = self.connection(1);
            if (!a.is_null() && !(*a).stateFlags.discriminant)
                || (!b.is_null() && (*b).stateFlags.discriminant)
            {
                index
            } else {
                1u16.wrapping_sub(index)
            }
        }
    }
    fn input_delay(&self) -> usize {
        self.get_battle_input_delay()
    }
    fn local_input(&self, side: usize, input: u8) -> u8 {
        unsafe {
            self.netplay_notify_all_battle_input(input);
            self.flush_packet_queues();
        }
        assert!(self.push_input(side, input), "local input buffer full");
        self.pop_input(side)
            .unwrap_or_else(|| panic!("local input buffer empty after push"))
    }
    fn remote_input(&self, side: usize) -> u8 {
        unsafe { self.network_battle_input(side) }
    }
    fn set_frame(&self, frame: u32) {
        self.frame_count.set(frame);
    }
    fn frame(&self) -> u32 {
        self.frame_count.get()
    }
    fn record_checksum(&self, frame: u32, checksum: u32) {
        unsafe {
            self.netplay_notify_all_checksum(frame, checksum);
            self.flush_packet_queues();
            self.add_local_checksum(frame, checksum);
        }
    }
    fn verify_checksum(&self, frame: u32) -> bool {
        unsafe { self.verify_checksums(frame) }
    }
    fn reset(&self, reason: u32) {
        unsafe {
            self.reset_connections(reason);
        }
    }
    fn negotiate(&self, notify: bool, state: u32) -> bool {
        unsafe { self.negotiate_ready_connections(notify, state) }
    }
    fn set_state(&self, state: u32) {
        unsafe {
            self.set_state_connections(state);
        }
    }
    fn wait_reset(&self, state: u32) {
        unsafe {
            self.wait_reset_connections(state);
        }
    }
    fn init_end_frames(&self) {
        unsafe {
            self.init_battle_state_data_connections();
        }
    }
    fn end_ready(&self, side: usize) -> bool {
        unsafe { readyForBattleEndPlayer(self.connection(side)) }
    }
    fn ship_selected(&self, choice: u16) {
        unsafe {
            self.each(|conn| {
                Netplay_Notify_shipSelected(conn, choice);
                true
            });
        }
    }
    unsafe fn begin(&self, state: *mut c_void) {
        self.init_battle_input_buffers();
        self.init_checksum_buffers();
        self.frame_count.set(0);
        self.set_battle_state_connections(state.cast());
    }
    fn finish(&self) {
        unsafe {
            self.uninit_battle_input_buffers();
            self.uninit_checksum_buffers();
            self.set_battle_state_connections(ptr::null_mut());
        }
    }
    unsafe fn bind_picker(&self, mask: u16, picker: *mut c_void) {
        for (side, conn) in self.connections.get().into_iter().enumerate() {
            if mask & (1 << side) != 0
                && !conn.is_null()
                && (picker.is_null() || (*conn).stateFlags.connected)
            {
                (*(*conn).stateData.cast::<BattleStateData>()).getMeleeState = picker.cast();
            }
        }
    }
}
