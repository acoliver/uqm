//! Protocol unit tests cannot enter the window/input loop or mutate melee UI.
#![allow(non_snake_case)]
use super::abi::{NetConnection, MELEE_STATE};
use std::ffi::{c_char, c_void};

pub fn abortFeedback(_conn: *mut NetConnection, _reason: u32) {
    panic!("protocol test entered live abort feedback");
}
pub fn resetFeedback(_conn: *mut NetConnection, _reason: u32, _remote: bool) {
    panic!("protocol test entered live reset feedback");
}
pub fn closeFeedback(_conn: *mut NetConnection) {
    panic!("protocol test entered live close feedback");
}
pub fn errorFeedback(_conn: *mut NetConnection) {
    panic!("protocol test entered live error feedback");
}
pub fn connectedFeedback(_conn: *mut NetConnection) {
    panic!("protocol test entered live connected feedback");
}
pub fn confirmationCancelled(_state: *mut MELEE_STATE, _side: u16) {
    panic!("protocol test entered live confirmation feedback");
}
pub fn Melee_RemoteChange_ship(
    _state: *mut MELEE_STATE,
    _conn: *mut NetConnection,
    _side: u16,
    _index: u16,
    _ship: u32,
) {
    panic!("protocol test entered live fleet editing");
}
pub fn Melee_RemoteChange_teamName(
    _state: *mut MELEE_STATE,
    _conn: *mut NetConnection,
    _side: u16,
    _value: *const c_char,
) {
    panic!("protocol test entered live team editing");
}
pub fn Melee_bootstrapSyncTeam(_state: *mut MELEE_STATE, _side: usize) {
    panic!("protocol test entered live team synchronization");
}
pub fn updateRandomSeed(_state: *mut MELEE_STATE, _side: u16, _seed: u32) {
    panic!("protocol test entered live gameplay RNG");
}
pub fn updateMeleeSelection(
    _state: *mut crate::ships::native::abi::GETMELEE_STATE,
    _side: u16,
    _ship: u16,
) -> bool {
    panic!("protocol test entered live ship selection");
}
pub fn DoInput(_state: *mut c_void, _reset: u32) {
    panic!("protocol test entered live input loop");
}
