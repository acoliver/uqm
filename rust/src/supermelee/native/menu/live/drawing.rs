//! Super Melee menu rendering and selection feedback.
use super::*;

#[no_mangle]
pub static mut MeleeFrame: FRAME = ptr::null_mut();
#[no_mangle]
pub static mut pMeleeState: *mut MELEE_STATE = ptr::null_mut();

pub(super) fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
pub(super) unsafe fn text(x: i16, y: i16, align: u32, s: *const c_char) -> TEXT {
    TEXT {
        baseline: POINT { x, y },
        align,
        pStr: s,
        CharCount: u16::MAX,
    }
}
#[no_mangle]
pub unsafe extern "C" fn DrawMeleeIcon(index: u16) {
    let mut s = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: SetAbsFrameIndex(MeleeFrame, index),
    };
    DrawStamp(&mut s);
}
#[no_mangle]
pub unsafe extern "C" fn GetShipBox(r: *mut RECT, side: u16, row: u16, col: u16) {
    *r = RECT {
        corner: POINT {
            x: 2 + col as i16 * 35,
            y: 21 + side as i16 * 92 + row as i16 * 35,
        },
        extent: EXTENT {
            width: 34,
            height: 34,
        },
    };
}
pub(super) unsafe fn ship_box(side: u16, index: usize, ship: u32, highlight: bool) {
    let mut r = mem::zeroed();
    GetShipBox(&mut r, side, (index / 7) as u16, (index % 7) as u16);
    BatchGraphics();
    let (light, dark, inside) = if highlight {
        (rgb(7, 0, 12), rgb(12, 0, 20), rgb(10, 0, 17))
    } else {
        (rgb(0, 0, 9), rgb(0, 0, 14), rgb(0, 0, 12))
    };
    DrawStarConBox(&mut r, 1, light, dark, u32::from(ship != 255), inside);
    if ship != 255 {
        let mut s = STAMP {
            origin: POINT {
                x: r.corner.x + (r.extent.width >> 1),
                y: r.corner.y + (r.extent.height >> 1),
            },
            frame: crate::ships::master_native::GetShipMeleeIconsFromIndex(ship),
        };
        DrawStamp(&mut s);
    }
    UnbatchGraphics();
}
pub(super) unsafe fn controls(side: usize, highlight: bool) {
    let control = PlayerControl[side];
    let icon = if control & NETWORK_CONTROL != 0 {
        31 + u16::from(highlight) + 2 * (1 - side as u16)
    } else {
        let rating = if control & HUMAN_CONTROL != 0 {
            0
        } else {
            match control & (16 | 32 | 64) {
                16 => 1,
                32 => 2,
                64 => 3,
                _ => 0,
            }
        };
        1 + 8 * (1 - side as u16) + 4 * u16::from(highlight) + rating
    };
    DrawMeleeIcon(icon);
}
pub(super) unsafe fn teams() {
    for side in [0usize, 1] {
        controls(side, false);
        for index in 0..14 {
            ship_box(
                side as u16,
                index,
                MeleeSetup_getShip((*pMeleeState).meleeSetup, side, index as u16),
                false,
            );
        }
        name(pMeleeState, side as u16, 0, ptr::null());
        worth(pMeleeState, side as u16, 0);
    }
}
#[no_mangle]
pub unsafe extern "C" fn RepairMeleeFrame(rect: *const RECT) {
    let mut r = *rect;
    r.corner.x += SAFE_X as i16;
    r.corner.y += SAFE_Y as i16;
    if r.corner.y & 1 != 0 {
        r.corner.y -= 1;
        r.extent.height += 1;
    }
    let old = SetContext(SpaceContext);
    let mut clip = mem::zeroed();
    GetContextClipRect(&mut clip);
    SetContextClipRect(&mut r);
    let origin = SetContextOrigin(POINT {
        x: -r.corner.x + SAFE_X as i16,
        y: -r.corner.y + SAFE_Y as i16,
    });
    BatchGraphics();
    DrawMeleeIcon(0);
    DrawMeleeIcon(35);
    DrawMeleeIcon(37);
    DrawMeleeIcon(26);
    teams();
    if (*pMeleeState).MeleeOption == 11 {
        crate::supermelee::native::build_picker::live::DrawPickFrame(pMeleeState);
    }
    UnbatchGraphics();
    SetContextOrigin(origin);
    SetContextClipRect(&mut clip);
    SetContext(old);
}
pub(super) unsafe fn redraw() {
    let r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: ScreenWidth as i16,
            height: ScreenHeight as i16,
        },
    };
    RepairMeleeFrame(&r);
}
pub(super) fn name_rect(side: u16) -> RECT {
    RECT {
        corner: POINT {
            x: 1,
            y: (side as i16 + 1) * 93,
        },
        extent: EXTENT {
            width: 216,
            height: 13,
        },
    }
}
pub(super) unsafe fn worth(state: *mut MELEE_STATE, side: u16, highlight: u16) {
    let r = RECT {
        corner: POINT {
            x: 217,
            y: (side as i16 + 1) * 93,
        },
        extent: EXTENT {
            width: 29,
            height: 13,
        },
    };
    if highlight == 4 {
        RepairMeleeFrame(&r);
        return;
    }
    SetContextFont(MicroFont);
    let value =
        CString::new(MeleeSetup_getFleetValue((*state).meleeSetup, side as usize).to_string())
            .unwrap();
    let mut t = text(
        r.corner.x + r.extent.width,
        r.corner.y + r.extent.height - 3,
        TEXT_ALIGN_ALIGN_RIGHT,
        value.as_ptr(),
    );
    SetContextForeGroundColor(if highlight & 2 == 0 {
        rgb(15, 16, 27)
    } else {
        rgb(23, 24, 29)
    });
    font_DrawText(&mut t);
}
pub(super) unsafe fn name(
    state: *mut MELEE_STATE,
    side: u16,
    highlight: u16,
    value: *const c_char,
) -> u32 {
    let r = name_rect(side);
    if highlight == 4 {
        RepairMeleeFrame(&r);
        return 1;
    }
    SetContextFont(MicroFont);
    let value = if value.is_null() {
        MeleeSetup_getTeamName((*state).meleeSetup, side as usize)
    } else {
        value
    };
    let mut t = text(
        r.corner.x + 1,
        r.corner.y + r.extent.height - 3,
        TEXT_ALIGN_ALIGN_LEFT,
        value,
    );
    t.CharCount = libc::strlen(value) as u16;
    BatchGraphics();
    if highlight & 1 == 0 {
        SetContextForeGroundColor(if highlight & 2 == 0 {
            rgb(15, 16, 27)
        } else {
            rgb(23, 24, 29)
        });
        font_DrawText(&mut t);
    } else {
        let mut bounds = mem::zeroed();
        let mut delta = [0u8; 30];
        TextRect(&mut t, &mut bounds, delta.as_mut_ptr());
        if bounds.extent.width + 2 >= r.extent.width {
            UnbatchGraphics();
            return 0;
        }
        bounds = r;
        SetContextForeGroundColor(rgb(20, 0, 20));
        DrawFilledRectangle(&mut bounds);
        let cursor = (*state).CurIndex as usize;
        for d in delta.iter().take(cursor) {
            bounds.corner.x += i16::from(*d);
        }
        if cursor < t.CharCount as usize {
            bounds.corner.x -= 1;
        }
        bounds.extent.width = if highlight & 8 == 0 || cursor == t.CharCount as usize {
            1
        } else {
            i16::from(delta[cursor])
                + if cursor + 1 == t.CharCount as usize {
                    2
                } else {
                    1
                }
        };
        bounds.corner.x += 1;
        bounds.corner.y += 1;
        bounds.extent.height -= 2;
        SetContextForeGroundColor(rgb(31, 31, 31));
        DrawFilledRectangle(&mut bounds);
        SetContextForeGroundColor(rgb(0, 0, 0));
        font_DrawText(&mut t);
    }
    UnbatchGraphics();
    1
}
pub(super) unsafe fn status(message: *const c_char) {
    let old = SetContext(SpaceContext);
    let mut r = RECT {
        corner: POINT { x: 1, y: 201 },
        extent: EXTENT {
            width: 245,
            height: 38,
        },
    };
    RepairMeleeFrame(&r);
    if *message != 0 {
        SetContextFont(MicroFont);
        SetContextForeGroundColor(rgb(0, 20, 0));
        BatchGraphics();
        let mut clip = mem::zeroed();
        GetContextClipRect(&mut clip);
        SetContextClipRect(&mut r);
        let mut leading = 0;
        GetContextFontLeading(&mut leading);
        let mut t = text(1, 0, TEXT_ALIGN_ALIGN_LEFT, message);
        if r.extent.width > 1 {
            while *t.pStr != 0 {
                let mut next = ptr::null();
                t.baseline.y += leading;
                t.CharCount = u16::MAX;
                getLineWithinWidth(&mut t, &mut next, r.extent.width - 1, t.CharCount);
                font_DrawText(&mut t);
                t.pStr = next;
            }
        }
        SetContextClipRect(&mut clip);
        UnbatchGraphics();
    }
    SetContext(old);
}
pub(super) unsafe fn status_player(player: isize) {
    assert!((-1..2).contains(&player));
    if player == -1 {
        status(c"".as_ptr());
        return;
    }
    let conn = connection(player as usize);
    let index = if conn.is_null() {
        0
    } else {
        match NetConnection_getState(conn) {
            0 => 0,
            1 => {
                if is_server(conn) {
                    1
                } else {
                    2
                }
            }
            2 | 3 => 3,
            _ => -1,
        }
    };
    status(if index < 0 {
        c"".as_ptr()
    } else {
        game_string(NETMELEE_STRING_BASE as i32 + index)
    });
}
pub(super) unsafe fn highlight(option: u8, on: bool) {
    let icon = match option {
        4 => Some(if on { 26 } else { 25 }),
        3 => Some(if on { 19 } else { 17 }),
        5 => Some(if on { 24 } else { 22 }),
        2 => Some(if on { 20 } else { 18 }),
        6 => Some(if on { 23 } else { 21 }),
        0 => Some(if on { 36 } else { 35 }),
        8 => Some(if on { 38 } else { 37 }),
        9 => Some(if on { 30 } else { 29 }),
        _ => None,
    };
    if let Some(icon) = icon {
        DrawMeleeIcon(icon);
        return;
    }
    match option {
        1 | 7 => controls(usize::from(option == 1), on),
        10 => {
            let s = &*pMeleeState;
            if s.InputFunc.map(|f| f as usize) == Some(edit as *const () as usize) {
                if s.row < 2 {
                    ship_box(
                        s.side,
                        (s.row * 7 + s.col) as usize,
                        MeleeSetup_getShip(s.meleeSetup, s.side as usize, s.row * 7 + s.col),
                        on,
                    );
                } else if s.CurIndex == u16::MAX {
                    name(pMeleeState, s.side, if on { 2 } else { 0 }, ptr::null());
                    worth(pMeleeState, s.side, if on { 2 } else { 0 });
                }
            }
        }
        11 => crate::supermelee::native::build_picker::live::DrawPickIcon(
            (*pMeleeState).currentShip,
            !on,
        ),
        _ => {}
    }
}
#[no_mangle]
pub unsafe extern "C" fn Melee_flashSelection(state: *mut MELEE_STATE) {
    static mut NEXT: u32 = 0;
    static mut SELECT: bool = false;
    let now = GetTimeCounter();
    if now >= NEXT {
        NEXT = now.wrapping_add(840 / 9);
        SELECT = !SELECT;
        let old = SetContext(SpaceContext);
        highlight((*state).MeleeOption, SELECT);
        SetContext(old);
    }
}
pub(super) unsafe fn init_screen() {
    SetContext(SpaceContext);
    drawable_abi::SetContextFGFrame(Screen.cast());
    SetContextClipRect(ptr::null_mut());
    SetContextBackGroundColor(rgb(0, 0, 0));
    ClearDrawable();
    let mut r = RECT {
        corner: POINT {
            x: SAFE_X as i16,
            y: SAFE_Y as i16,
        },
        extent: EXTENT {
            width: ScreenWidth as i16 - 2 * SAFE_X as i16,
            height: ScreenHeight as i16 - 2 * SAFE_Y as i16,
        },
    };
    SetContextClipRect(&mut r);
    redraw();
}
#[no_mangle]
pub unsafe extern "C" fn DrawMeleeShipStrings(state: *mut MELEE_STATE, ship: u32) {
    let old = SetContext(StatusContext);
    let mut clip = mem::zeroed();
    GetContextClipRect(&mut clip);
    let mut r = clip;
    r.corner.x += (SAFE_X as i16 * 2 - 32) + 29;
    r.corner.y += 76;
    r.extent.height = SHIP_INFO_HEIGHT as i16;
    SetContextClipRect(&mut r);
    BatchGraphics();
    if ship == 255 {
        ClearShipStatus(0);
        SetContextFont(StarConFont);
        let mut rect = RECT {
            corner: POINT { x: 3, y: 4 },
            extent: EXTENT {
                width: 57,
                height: 60,
            },
        };
        SetContextForeGroundColor(rgb(0, 0, 0));
        DrawRectangle(&mut rect);
        let base = MELEE_STRING_BASE as i32 + if (*state).row < 2 { 0 } else { 2 };
        let mut t = text(
            STATUS_WIDTH as i16 >> 1,
            32,
            TEXT_ALIGN_ALIGN_CENTER,
            game_string(base),
        );
        font_DrawText(&mut t);
        t.pStr = game_string(base + 1);
        t.baseline.y += TINY_TEXT_HEIGHT as i16;
        font_DrawText(&mut t);
    } else {
        let master =
            GetStarShipFromIndex(&raw mut master_q, ship as u16).cast::<MASTER_SHIP_INFO>();
        InitShipStatus(&mut (*master).ShipInfo, ptr::null_mut(), ptr::null_mut());
    }
    UnbatchGraphics();
    SetContextClipRect(&mut clip);
    SetContext(old);
}
pub(super) unsafe fn current(state: *mut MELEE_STATE) {
    (*state).currentShip = if (*state).row == 2 {
        255
    } else {
        MeleeSetup_getShip(
            (*state).meleeSetup,
            (*state).side as usize,
            (*state).row * 7 + (*state).col,
        )
    };
    DrawMeleeShipStrings(state, (*state).currentShip);
}
