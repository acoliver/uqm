//! Fleet-edit ship popup from buildpick.c.
fn navigate(ship: u32, left: bool, right: bool, up: bool, down: bool) -> u32 {
    let mut selected = ship;
    if left {
        if selected.is_multiple_of(5) {
            selected += 5;
        }
        selected -= 1;
    } else if right {
        selected += 1;
        if selected.is_multiple_of(5) {
            selected -= 5;
        }
    }
    if up {
        if selected >= 5 {
            selected -= 5;
        } else {
            selected += 20;
        }
    } else if down {
        if selected < 20 {
            selected += 5;
        } else {
            selected -= 20;
        }
    }
    selected
}

#[cfg(not(test))]
#[allow(non_snake_case)]
mod live {
    use super::navigate;
    use crate::graphics::{drawable_abi, geometry_ffi, native_abi};
    use crate::ships::native::abi::*;
    use std::{mem, ptr};

    static mut BUILD_PICK_FRAME: FRAME = ptr::null_mut();
    extern "C" {
        fn DoShipSpin(index: u16, music: MUSIC_REF);
    }

    unsafe fn hot(frame: FRAME, x: i16, y: i16) {
        drawable_abi::SetFrameHot(frame.cast(), native_abi::HOT_SPOT { x, y });
    }
    unsafe fn frame_rect(frame: FRAME) -> RECT {
        let mut rect = mem::zeroed();
        GetFrameRect(frame, &mut rect);
        rect
    }

    /// # Safety
    /// Game thread, with the melee graphics and master ship list loaded.
    #[no_mangle]
    pub unsafe extern "C" fn BuildBuildPickFrame() {
        let old = SetContext(OffScreenContext);
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: SetAbsFrameIndex(MeleeFrame, 27),
        };
        let rect = frame_rect(stamp.frame);
        BUILD_PICK_FRAME = CaptureDrawable(
            drawable_abi::CreateDrawable(2, rect.extent.width, rect.extent.height, 1).cast(),
        );
        drawable_abi::SetContextFGFrame(BUILD_PICK_FRAME.cast());
        hot(stamp.frame, 0, 0);
        DrawStamp(&mut stamp);
        for ship in 0..25 {
            DrawPickIcon(ship, true);
        }
        SetContext(old);
    }
    /// # Safety
    /// Game thread; the popup drawable must not be referenced elsewhere.
    #[no_mangle]
    pub unsafe extern "C" fn DestroyBuildPickFrame() {
        DestroyDrawable(ReleaseDrawable(BUILD_PICK_FRAME));
        BUILD_PICK_FRAME = ptr::null_mut();
    }
    /// # Safety
    /// Game thread, with the popup drawable and ship icons loaded.
    #[no_mangle]
    pub unsafe extern "C" fn DrawPickIcon(ship: MeleeShip, draw: bool) {
        let rect = frame_rect(BUILD_PICK_FRAME);
        let mut stamp = STAMP {
            origin: POINT {
                x: rect.corner.x + 20 + (ship % 5) as i16 * 18,
                y: rect.corner.y + 5 + (ship / 5) as i16 * 18,
            },
            frame: crate::ships::master_native::GetShipIconsFromIndex(ship),
        };
        if draw {
            DrawStamp(&mut stamp);
        } else {
            let old = SetContextForeGroundColor(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            });
            DrawFilledStamp(&mut stamp);
            SetContextForeGroundColor(old);
        }
    }
    /// # Safety
    /// State must be the active fleet editor on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn DrawPickFrame(state: *mut MELEE_STATE) {
        let mut first: RECT = mem::zeroed();
        let mut last: RECT = mem::zeroed();
        let mut ships: RECT = mem::zeroed();
        GetShipBox(&mut first, 0, 0, 0);
        GetShipBox(&mut last, 1, 1, 6);
        geometry_ffi::BoxUnion(
            (&mut first as *mut RECT).cast(),
            (&mut last as *mut RECT).cast(),
            (&mut ships as *mut RECT).cast(),
        );
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: SetAbsFrameIndex(BUILD_PICK_FRAME, 0),
        };
        let rect = frame_rect(stamp.frame);
        let x = -(ships.corner.x + ((ships.extent.width - rect.extent.width) >> 1));
        let y = if (*state).side != 0 {
            -ships.corner.y
        } else {
            -(ships.corner.y + ships.extent.height - rect.extent.height)
        };
        hot(stamp.frame, x, y);
        DrawStamp(&mut stamp);
        DrawMeleeShipStrings(state, (*state).currentShip);
    }
    /// # Safety
    /// Rect must be writable and the popup must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn GetBuildPickFrameRect(rect: *mut RECT) {
        GetFrameRect(BUILD_PICK_FRAME, rect);
    }

    unsafe extern "C" fn pick(state: *mut MELEE_STATE) -> u32 {
        let time = GetTimeCounter();
        GamePaused = 0;
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            (*state).buildPickConfirmed = 0;
            return 0;
        }
        SetMenuSounds(15, 16);
        let keys = PulsedInputState.menu;
        if keys[KEY_MENU_SELECT as usize] != 0 || keys[KEY_MENU_CANCEL as usize] != 0 {
            (*state).buildPickConfirmed = u32::from(keys[KEY_MENU_CANCEL as usize] == 0);
            return 0;
        }
        if keys[KEY_MENU_SPECIAL as usize] != 0 && (*state).currentShip != 255 {
            DoShipSpin((*state).currentShip as u16, ptr::null_mut());
            return 1;
        }
        let selected = navigate(
            (*state).currentShip,
            keys[KEY_MENU_LEFT as usize] != 0,
            keys[KEY_MENU_RIGHT as usize] != 0,
            keys[KEY_MENU_UP as usize] != 0,
            keys[KEY_MENU_DOWN as usize] != 0,
        );
        if selected != (*state).currentShip {
            DrawPickIcon((*state).currentShip, true);
            (*state).currentShip = selected;
            DrawMeleeShipStrings(state, selected);
        }
        Melee_flashSelection(state);
        SleepThreadUntil(time.wrapping_add(840 / 30));
        1
    }
    /// # Safety
    /// State must be the active fleet editor on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn BuildPickShip(state: *mut MELEE_STATE) -> bool {
        FlushInput();
        if (*state).currentShip == 255 {
            (*state).currentShip = 0;
        }
        DrawPickFrame(state);
        (*state).InputFunc = Some(pick);
        DoInput(state.cast(), 0);
        (*state).buildPickConfirmed != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picker_wraps_each_edge_and_applies_horizontal_before_vertical() {
        assert_eq!(navigate(0, true, false, false, false), 4);
        assert_eq!(navigate(4, false, true, false, false), 0);
        assert_eq!(navigate(0, false, false, true, false), 20);
        assert_eq!(navigate(24, false, false, false, true), 4);
        assert_eq!(navigate(0, true, true, true, true), 24);
        for ship in 0..25 {
            assert_eq!(
                navigate(
                    navigate(ship, true, false, false, false),
                    false,
                    true,
                    false,
                    false
                ),
                ship
            );
        }
    }
}
