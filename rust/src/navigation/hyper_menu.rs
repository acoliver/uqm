//! Hyperspace menu dispatch.
#![cfg(not(test))]
#![allow(non_snake_case)]
use super::abi::*;
use super::get;
use super::globals::GlobData;
use super::hyper_transition::cleanup;
use std::{mem, ptr};

unsafe extern "C" fn input(state: *mut MENU_STATE) -> BOOLEAN {
    let mut select = PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0;
    if GlobData.Game_state.CurrentActivity & ((CHECK_ABORT | CHECK_LOAD) as u16) != 0
        || GlobData.SIS_state.CrewEnlisted == u16::MAX
    {
        return 0;
    }
    if DoMenuChooser(state, PM_STARMAP as u8) != 0 || !select {
        return 1;
    }
    SetFlashRect(ptr::null());
    match (*state).CurState {
        2 => {
            select = DevicesMenu() != 0;
            if get("PORTAL_COUNTER") != 0
                || GlobData.Game_state.CurrentActivity & START_ENCOUNTER as u16 != 0
            {
                return 0;
            }
        }
        3 => {
            CargoMenu();
        }
        4 => {
            select = RosterMenu() != 0;
        }
        5 => {
            if GameOptions() == 0 {
                return 0;
            }
        }
        1 => {
            StarMap();
            return 0;
        }
        6 => return 0,
        _ => {}
    }
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 == 0 {
        if select {
            if optWhichMenu != 2 {
                (*state).CurState = 6;
            }
            DrawMenuStateStrings(PM_STARMAP as u8, (*state).CurState as i16);
        }
        SetFlashRect(usize::MAX as *const RECT);
    }
    1
}
/// # Safety
/// Input, graphics and world state must be initialized on the game thread.
pub unsafe fn HyperspaceMenu() {
    UnbatchGraphics();
    let old_context = SetContext(SpaceContext);
    let old_color = SetContextBackGroundColor(super::color(0, 0, 0));
    let mut state: MENU_STATE = mem::zeroed();
    state.InputFunc = Some(input);
    state.Initialized = 1;
    state.CurState = 1;
    DrawMenuStateStrings(PM_STARMAP as u8, 1);
    SetFlashRect(usize::MAX as *const RECT);
    SetMenuSounds(15, 16);
    DoInput(ptr::addr_of_mut!(state).cast(), 1);
    SetFlashRect(ptr::null());
    SetContext(SpaceContext);
    if GlobData.Game_state.CurrentActivity & ((CHECK_ABORT | CHECK_LOAD) as u16) == 0 {
        ClearSISRect(CLEAR_SIS_RADAR as u8);
        WaitForNoInput(ONE_SECOND / 2, 0);
    }
    SetContextBackGroundColor(old_color);
    SetContext(old_context);
    if GlobData.Game_state.CurrentActivity & IN_BATTLE as u16 == 0 {
        crate::app::bridge::enter(cleanup);
    }
    BatchGraphics();
}
