use super::{CSisState, EncounterApi, Handler};

pub(super) trait MelnormApi: EncounterApi<Handler> {
    fn ship(&self) -> CSisState;
    fn set_ship(&mut self, ship: CSisState);
    fn module_cost(&self, id: usize) -> u8;
    fn set_module_cost(&mut self, id: usize, cost: u8);
    fn number(&mut self, number: i32);
    fn status_mode(&mut self, mode: i32) -> i32;
    fn draw_status(&mut self);
    fn draw_cargo(&mut self, old: u8, new: u8);
    fn sleep(&mut self, ticks: u32);
    fn now(&mut self) -> u32;
    fn skip_sale(&mut self) -> bool;
    fn clear_display(&mut self);
}
#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::locdata::CGlobData;
    use crate::comm::races::script::live::LiveScript;
    use std::ffi::c_char;
    extern "C" {
        static mut GlobData: CGlobData;
        fn NPCNumber(number: i32, fmt: *const c_char);
        fn SetStatusMessageMode(mode: i32) -> i32;
        fn DrawStatusMessage(message: *const c_char);
        fn DrawCargoStrings(old: u8, new: u8);
        fn SleepThread(ticks: i32);
        fn AnyButtonPress(special: u8) -> u8;
        fn ClearSISRect(flags: u16);
    }
    impl MelnormApi for LiveScript<Handler> {
        fn ship(&self) -> CSisState {
            unsafe { GlobData.sis_state }
        }
        fn set_ship(&mut self, ship: CSisState) {
            unsafe {
                GlobData.sis_state = ship;
            }
        }
        fn module_cost(&self, id: usize) -> u8 {
            unsafe { GlobData.game_state.module_cost[id] }
        }
        fn set_module_cost(&mut self, id: usize, cost: u8) {
            unsafe {
                GlobData.game_state.module_cost[id] = cost;
            }
        }
        fn number(&mut self, number: i32) {
            unsafe {
                NPCNumber(number, std::ptr::null());
            }
        }
        fn status_mode(&mut self, mode: i32) -> i32 {
            unsafe { SetStatusMessageMode(mode) }
        }
        fn draw_status(&mut self) {
            unsafe {
                DrawStatusMessage(std::ptr::null());
            }
        }
        fn draw_cargo(&mut self, old: u8, new: u8) {
            unsafe {
                DrawCargoStrings(old, new);
            }
        }
        fn sleep(&mut self, ticks: u32) {
            unsafe {
                SleepThread(ticks as i32);
            }
        }
        fn now(&mut self) -> u32 {
            crate::time::counter_abi::GetTimeCounter()
        }
        fn skip_sale(&mut self) -> bool {
            unsafe { AnyButtonPress(1) != 0 }
        }
        fn clear_display(&mut self) {
            unsafe {
                ClearSISRect(8);
            }
        }
    }
}
