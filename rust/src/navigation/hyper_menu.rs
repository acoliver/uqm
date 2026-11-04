//! Hyperspace menu dispatch.
#![cfg(not(test))]
use super::abi::*;
use crate::engine::{battle::Cx, observe::Io, StoryKey};
use std::{mem, ptr};

unsafe fn input(cx: &mut Cx<'_>, io: &mut Io<'_>, state: *mut MENU_STATE) -> BOOLEAN {
    let mut select = PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0;
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) as u16 != 0
        || cx.world.data.flagship.crew_enlisted == u16::MAX
    {
        return 0;
    }
    if cx.with_legacy(|| DoMenuChooser(state, PM_STARMAP as u8)) != 0 || !select {
        return 1;
    }
    cx.with_legacy(|| SetFlashRect(ptr::null()));
    match (*state).CurState {
        2 => {
            select = crate::planets::devices::DevicesMenu(cx, io) != 0;
            if cx.world.story_state(StoryKey::PORTAL_COUNTER) != 0
                || cx.world.activity() & START_ENCOUNTER as u16 != 0
            {
                return 0;
            }
        }
        3 => crate::planets::cargo::live::CargoMenu(cx, io),
        4 => select = crate::planets::roster::live::RosterMenu(cx, io) != 0,
        5 => {
            if crate::front_end::gameopt::runtime::game_options(cx, io) == 0 {
                return 0;
            }
        }
        1 => {
            crate::planets::pstarmap::StarMap(cx, io);
            return 0;
        }
        6 => return 0,
        _ => {}
    }
    if cx.world.activity() & CHECK_ABORT as u16 == 0 {
        cx.with_legacy(|| {
            if select {
                if optWhichMenu != 2 {
                    (*state).CurState = 6;
                }
                DrawMenuStateStrings(PM_STARMAP as u8, (*state).CurState as i16);
            }
            SetFlashRect(usize::MAX as *const RECT);
        });
    }
    1
}

/// # Safety
/// Input, graphics and world state must be initialized on the game thread.
pub unsafe fn hyperspace_menu(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let (old_context, old_color) = cx.with_legacy(|| {
        UnbatchGraphics();
        let old_context =
            SetContext(crate::engine::Kernel::current().read(|owner| owner.space_context));
        let old_color = SetContextBackGroundColor(super::color(0, 0, 0));
        DrawMenuStateStrings(PM_STARMAP as u8, 1);
        SetFlashRect(usize::MAX as *const RECT);
        SetMenuSounds(15, 16);
        (old_context, old_color)
    });
    let mut state: MENU_STATE = mem::zeroed();
    state.Initialized = 1;
    state.CurState = 1;
    cx.run_input(io, true, |cx, io| input(cx, io, &mut state) != 0);
    cx.with_legacy(|| {
        SetFlashRect(ptr::null());
        SetContext(crate::engine::Kernel::current().read(|owner| owner.space_context));
    });
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) as u16 == 0 {
        cx.with_legacy(|| ClearSISRect(CLEAR_SIS_RADAR as u8));
        crate::ui::util::live::wait_for_no_input(cx, io, ONE_SECOND / 2, 0);
    }
    cx.with_legacy(|| {
        SetContextBackGroundColor(old_color);
        SetContext(old_context);
    });
    if cx.world.activity() & IN_BATTLE as u16 == 0 {
        super::hyper_transition::cleanup(cx);
    }
    cx.with_legacy(|| BatchGraphics());
}
