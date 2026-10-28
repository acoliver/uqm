use super::abi::*;
use super::notify::{
    Netplay_Notify_battleInput, Netplay_Notify_checksum, Netplay_Notify_inputDelay,
    Netplay_Notify_setFleet, Netplay_Notify_setShip, Netplay_Notify_setTeamName,
};

impl super::session::SessionCore {
    unsafe fn each_notify(&self, mut action: impl FnMut(*mut NetConnection), setup_only: bool) {
        for player in 0..NETPLAY_NUM_PLAYERS as usize {
            let conn = self.connections.get()[player];
            if conn.is_null() || !(*conn).stateFlags.connected {
                continue;
            }
            if setup_only && (*conn).state != NetState_NetState_inSetup {
                continue;
            }
            action(conn);
        }
    }

    /// # Safety
    /// The melee setup and its connections are live on the game thread.
    pub unsafe fn netplay_notify_all_set_team_name(&self, melee: *mut MELEE_STATE, player: usize) {
        let name = MeleeSetup_getTeamName((*melee).meleeSetup, player);
        let len = libc::strlen(name);
        self.each_notify(
            |conn| Netplay_Notify_setTeamName(conn, player as i32, name, len),
            true,
        );
    }
    /// # Safety
    /// The melee setup and its connections are live on the game thread.
    pub unsafe fn netplay_notify_all_set_fleet(&self, melee: *mut MELEE_STATE, player: usize) {
        let ships = MeleeSetup_getFleet((*melee).meleeSetup, player);
        self.each_notify(
            |conn| Netplay_Notify_setFleet(conn, player as i32, ships, MELEE_FLEET_SIZE as usize),
            true,
        );
    }
    /// # Safety
    /// The melee setup and its connections are live on the game thread; index is a fleet slot.
    pub unsafe fn netplay_notify_all_set_ship(
        &self,
        melee: *mut MELEE_STATE,
        player: usize,
        index: usize,
    ) {
        let ship = MeleeSetup_getShip((*melee).meleeSetup, player, index as FleetShipIndex);
        self.each_notify(
            |conn| Netplay_Notify_setShip(conn, player as i32, index as FleetShipIndex, ship),
            true,
        );
    }
    /// # Safety
    /// Connections are accessed only on the game thread.
    pub unsafe fn netplay_notify_all_input_delay(&self, delay: usize) -> bool {
        self.each_notify(|conn| Netplay_Notify_inputDelay(conn, delay as u32), false);
        true
    }
    /// # Safety
    /// Connections are accessed only on the game thread.
    pub unsafe fn netplay_notify_all_checksum(
        &self,
        frame: BattleFrameCounter,
        checksum: Checksum,
    ) {
        self.each_notify(|conn| Netplay_Notify_checksum(conn, frame, checksum), false);
    }
    /// # Safety
    /// Connections are accessed only on the game thread.
    pub unsafe fn netplay_notify_all_battle_input(&self, input: BATTLE_INPUT_STATE) {
        self.each_notify(|conn| Netplay_Notify_battleInput(conn, input), false);
    }
}
