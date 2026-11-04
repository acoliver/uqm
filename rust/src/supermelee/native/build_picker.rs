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
pub(super) mod live {
    use super::navigate;
    use crate::engine::{Kernel as KernelOwner, WorldData as WorldFields};
    use crate::front_end::fmv::DoShipSpin;
    use crate::graphics::{drawable_abi, geometry_ffi, native_abi};
    use crate::ships::native::abi::*;
    use std::{mem, ptr};

    static mut BUILD_PICK_FRAME: FRAME = ptr::null_mut();

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
    pub unsafe extern "C" fn build_build_pick_frame() {
        let old = SetContext(KernelOwner::current().read(|owner| owner.off_screen_context));
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
            draw_pick_icon(
                &(*crate::supermelee::native::menu::live::drawing::P_MELEE_STATE).catalog,
                ship,
                true,
            );
        }
        SetContext(old);
    }
    /// # Safety
    /// Game thread; the popup drawable must not be referenced elsewhere.
    pub unsafe extern "C" fn destroy_build_pick_frame() {
        DestroyDrawable(ReleaseDrawable(BUILD_PICK_FRAME));
        BUILD_PICK_FRAME = ptr::null_mut();
    }
    /// # Safety
    /// Game thread, with the popup drawable and ship icons loaded.
    pub(crate) unsafe fn draw_pick_icon(
        catalog: &[crate::ships::master_native::ShipChoice],
        ship: MeleeShip,
        draw: bool,
    ) {
        let rect = frame_rect(BUILD_PICK_FRAME);
        let mut stamp = STAMP {
            origin: POINT {
                x: rect.corner.x + 20 + (ship % 5) as i16 * 18,
                y: rect.corner.y + 5 + (ship / 5) as i16 * 18,
            },
            frame: catalog
                .get(ship as usize)
                .map_or(ptr::null_mut(), |ship| ship.icon.raw().cast()),
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
    pub unsafe extern "C" fn draw_pick_frame(state: *mut MELEE_STATE) {
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
        DrawMeleeShipStrings(state, (*state).current_ship);
    }
    /// # Safety
    /// Rect must be writable and the popup must be initialized.
    pub unsafe extern "C" fn get_build_pick_frame_rect(rect: *mut RECT) {
        GetFrameRect(BUILD_PICK_FRAME, rect);
    }

    unsafe extern "C" fn pick(state: *mut MELEE_STATE) -> u32 {
        let time = GetTimeCounter();
        GamePaused = 0;
        if WorldFields::current().read(|owner| owner.game.current_activity) & CHECK_ABORT as u16
            != 0
        {
            (*state).build_pick_confirmed = 0;
            return 0;
        }
        SetMenuSounds(15, 16);
        let keys = PulsedInputState.menu;
        if keys[KEY_MENU_SELECT as usize] != 0 || keys[KEY_MENU_CANCEL as usize] != 0 {
            (*state).build_pick_confirmed = u32::from(keys[KEY_MENU_CANCEL as usize] == 0);
            return 0;
        }
        if keys[KEY_MENU_SPECIAL as usize] != 0 && (*state).current_ship != 255 {
            DoShipSpin((*state).current_ship as u16, ptr::null_mut());
            return 1;
        }
        let selected = navigate(
            (*state).current_ship,
            keys[KEY_MENU_LEFT as usize] != 0,
            keys[KEY_MENU_RIGHT as usize] != 0,
            keys[KEY_MENU_UP as usize] != 0,
            keys[KEY_MENU_DOWN as usize] != 0,
        );
        if selected != (*state).current_ship {
            draw_pick_icon(&(*state).catalog, (*state).current_ship, true);
            (*state).current_ship = selected;
            DrawMeleeShipStrings(state, selected);
        }
        Melee_flashSelection(state);
        SleepThreadUntil(time.wrapping_add(840 / 30));
        1
    }
    /// # Safety
    /// State must be the active fleet editor on the game thread.
    pub unsafe extern "C" fn build_pick_ship(state: *mut MELEE_STATE) -> bool {
        FlushInput();
        if (*state).current_ship == 255 {
            (*state).current_ship = 0;
        }
        draw_pick_frame(state);
        (*state).input_func = Some(pick);
        crate::app::bridge::enter(|cx| {
            let runtime = cx.kernel.input_runtime();
            let mut observer = runtime.observer();
            let mut io = crate::engine::observe::Io {
                observer: &mut *observer,
            };
            cx.run_input(&mut io, false, |cx, _| cx.with_legacy(|| pick(state) != 0));
        });
        (*state).build_pick_confirmed != 0
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
