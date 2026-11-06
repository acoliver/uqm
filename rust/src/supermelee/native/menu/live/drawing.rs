//! Super Melee menu rendering and selection feedback.
use super::*;
use crate::engine::Kernel as KernelOwner;

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
pub(crate) fn draw_melee_icon(state: &MeleeSession, index: u16) {
    state
        .menu_context
        .as_ref()
        .unwrap_or_else(|| panic!("menu context not admitted"))
        .draw_stamp(
            &state.menu_frames().context_frame(usize::from(index)),
            crate::graphics::native_abi::POINT { x: 0, y: 0 },
        );
}
pub(super) fn slot_rect(side: u16, row: u16, col: u16) -> RECT {
    let (x, y, width, height) = super::super::rules::slot_bounds(side, row, col);
    RECT {
        corner: POINT { x, y },
        extent: EXTENT { width, height },
    }
}
pub(super) fn ship_box(state: &MeleeSession, side: u16, index: usize, ship: u32, highlight: bool) {
    let r = slot_rect(side, (index / 7) as u16, (index % 7) as u16);
    let context = state
        .menu_context
        .as_ref()
        .unwrap_or_else(|| panic!("menu context not admitted"));
    let (light, dark, inside) = if highlight {
        (rgb(7, 0, 12), rgb(12, 0, 20), rgb(10, 0, 17))
    } else {
        (rgb(0, 0, 9), rgb(0, 0, 14), rgb(0, 0, 12))
    };
    context.draw_batch(|| {
        crate::ui::util::box_rectangles(
            r,
            1,
            light,
            dark,
            (ship != 255).then_some(inside),
            |color, rect| context.fill_rectangle(color, rect),
        );
        if ship != 255 {
            let frame = state
                .catalog
                .get(ship as usize)
                .map(|ship| ship.melee_icon.raw());
            let icon = crate::graphics::context_abi::ContextFrame::retain(
                frame.and_then(|frame| std::ptr::NonNull::new(frame.cast())),
            );
            context.draw_stamp(
                &icon,
                POINT {
                    x: r.corner.x + (r.extent.width >> 1),
                    y: r.corner.y + (r.extent.height >> 1),
                },
            );
        }
    });
}
pub(super) unsafe fn controls(state: &MeleeSession, side: usize, highlight: bool) {
    let control = KernelOwner::current().read(|owner| owner.player_control[side]);
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
    draw_melee_icon(state, icon);
}
pub(super) unsafe fn teams(state: &MeleeSession) {
    for side in [0usize, 1] {
        controls(state, side, false);
        for index in 0..14 {
            ship_box(
                state,
                side as u16,
                index,
                state.melee_setup.teams[side].ships[index] as u32,
                false,
            );
        }
        name(state, side as u16, 0, None);
        worth(state, side as u16, 0);
    }
}
/// # Safety
/// Call on the graphics thread with loaded melee resources. Pointer arguments
/// must be valid for the call, and player, row, column and ship indices must be in range.
pub unsafe fn repair_frame(state: &MeleeSession, rect: &RECT) {
    let mut r = *rect;
    r.corner.x += SAFE_X as i16;
    r.corner.y += SAFE_Y as i16;
    if r.corner.y & 1 != 0 {
        r.corner.y -= 1;
        r.extent.height += 1;
    }
    let context = state
        .menu_context
        .as_ref()
        .unwrap_or_else(|| panic!("menu context not admitted"));
    context.with_selected(|| {
        let mut clip = mem::zeroed();
        get_context_clip_rect(&mut clip);
        set_context_clip_rect(&mut r);
        let origin = set_context_origin(POINT {
            x: -r.corner.x + SAFE_X as i16,
            y: -r.corner.y + SAFE_Y as i16,
        });
        BatchGraphics();
        draw_melee_icon(state, 0);
        draw_melee_icon(state, 35);
        draw_melee_icon(state, 37);
        draw_melee_icon(state, 26);
        teams(state);
        if state.melee_option == 11 {
            crate::supermelee::native::build_picker::live::draw_pick_frame(state);
        }
        UnbatchGraphics();
        set_context_origin(origin);
        set_context_clip_rect(&mut clip);
    });
}
pub(super) unsafe fn redraw(state: &MeleeSession) {
    let extent = state
        .menu_target
        .as_ref()
        .unwrap_or_else(|| panic!("menu target not admitted"))
        .rect()
        .extent;
    let r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: extent.width,
            height: extent.height,
        },
    };
    repair_frame(state, &r);
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
pub(super) unsafe fn worth(state: &MeleeSession, side: u16, highlight: u16) {
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
        repair_frame(state, &r);
        return;
    }
    let context = state
        .menu_context
        .as_ref()
        .unwrap_or_else(|| panic!("menu context not admitted"));
    let font = state
        .menu_font
        .as_ref()
        .unwrap_or_else(|| panic!("menu font not admitted"));
    let value = CString::new(state.melee_setup.fleet_value[side as usize].to_string()).unwrap();
    context.draw_text(
        font,
        &value,
        POINT {
            x: r.corner.x + r.extent.width,
            y: r.corner.y + r.extent.height - 3,
        },
        TEXT_ALIGN_ALIGN_RIGHT,
        if highlight & 2 == 0 {
            rgb(15, 16, 27)
        } else {
            rgb(23, 24, 29)
        },
    );
}
pub(super) unsafe fn name(
    state: &MeleeSession,
    side: u16,
    highlight: u16,
    value: Option<&std::ffi::CStr>,
) -> u32 {
    let r = name_rect(side);
    if highlight == 4 {
        repair_frame(state, &r);
        return 1;
    }
    let context = state
        .menu_context
        .as_ref()
        .unwrap_or_else(|| panic!("menu context not admitted"));
    let font = state
        .menu_font
        .as_ref()
        .unwrap_or_else(|| panic!("menu font not admitted"));
    let value = value.unwrap_or_else(|| state.melee_setup.teams[side as usize].c_name());
    let baseline = POINT {
        x: r.corner.x + 1,
        y: r.corner.y + r.extent.height - 3,
    };
    context.set_font(font);
    context.draw_batch(|| {
        if highlight & 1 == 0 {
            context.draw_text(
                font,
                value,
                baseline,
                TEXT_ALIGN_ALIGN_LEFT,
                if highlight & 2 == 0 {
                    rgb(15, 16, 27)
                } else {
                    rgb(23, 24, 29)
                },
            );
        } else {
            let measured = font.measure(
                value,
                baseline,
                TEXT_ALIGN_ALIGN_LEFT,
                value.to_bytes().len() as u16,
            );
            if measured.rect.extent.width + 2 >= r.extent.width {
                return 0;
            }
            let mut bounds = r;
            context.fill_rectangle(rgb(20, 0, 20), bounds);
            let cursor = state.cur_index as usize;
            for delta in measured.deltas.iter().take(cursor) {
                bounds.corner.x += i16::from(*delta);
            }
            let count = usize::from(measured.characters);
            if cursor < count {
                bounds.corner.x -= 1;
            }
            bounds.extent.width = if highlight & 8 == 0 || cursor == count {
                1
            } else {
                i16::from(measured.deltas[cursor]) + if cursor + 1 == count { 2 } else { 1 }
            };
            bounds.corner.x += 1;
            bounds.corner.y += 1;
            bounds.extent.height -= 2;
            context.fill_rectangle(rgb(31, 31, 31), bounds);
            context.draw_text(font, value, baseline, TEXT_ALIGN_ALIGN_LEFT, rgb(0, 0, 0));
        }
        1
    })
}
pub(super) unsafe fn status(state: &MeleeSession, message: *const c_char) {
    let context = state
        .menu_context
        .as_ref()
        .unwrap_or_else(|| panic!("menu context not admitted"));
    context.with_selected(|| {
        let mut r = RECT {
            corner: POINT { x: 1, y: 201 },
            extent: EXTENT {
                width: 245,
                height: 38,
            },
        };
        repair_frame(state, &r);
        if *message != 0 {
            context.set_font(
                state
                    .menu_font
                    .as_ref()
                    .unwrap_or_else(|| panic!("menu font not admitted")),
            );
            SetContextForeGroundColor(rgb(0, 20, 0));
            BatchGraphics();
            let mut clip = mem::zeroed();
            get_context_clip_rect(&mut clip);
            set_context_clip_rect(&mut r);
            let mut leading = 0;
            get_context_font_leading(&mut leading);
            let mut t = text(1, 0, TEXT_ALIGN_ALIGN_LEFT, message);
            if r.extent.width > 1 {
                while *t.pStr != 0 {
                    let mut next = ptr::null();
                    t.baseline.y += leading;
                    t.CharCount = u16::MAX;
                    get_line_within_width(&mut t, &mut next, r.extent.width - 1, t.CharCount);
                    font_DrawText(&mut t);
                    t.pStr = next;
                }
            }
            set_context_clip_rect(&mut clip);
            UnbatchGraphics();
        }
    });
}
pub(super) unsafe fn status_player(state: &MeleeSession, player: isize) {
    assert!((-1..2).contains(&player));
    if player == -1 {
        status(state, c"".as_ptr());
        return;
    }
    let index = match connection(state, player as usize) {
        None => 0,
        Some(conn) => match conn.state {
            0 => 0,
            1 => {
                if conn.server {
                    1
                } else {
                    2
                }
            }
            2 | 3 => 3,
            _ => -1,
        },
    };
    status(
        state,
        if index < 0 {
            c"".as_ptr()
        } else {
            game_string(NETMELEE_STRING_BASE as i32 + index)
        },
    );
}
pub(super) unsafe fn highlight(state: &MeleeSession, option: u8, on: bool) {
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
        draw_melee_icon(state, icon);
        return;
    }
    match option {
        1 | 7 => controls(state, usize::from(option == 1), on),
        10 => {
            let s = state;
            if s.phase == crate::supermelee::session::MenuPhase::Editing {
                if s.row < 2 {
                    ship_box(
                        s,
                        s.side,
                        (s.row * 7 + s.col) as usize,
                        s.melee_setup.teams[s.side as usize].ships[(s.row * 7 + s.col) as usize]
                            as u32,
                        on,
                    );
                } else if s.cur_index == u16::MAX {
                    name(state, s.side, if on { 2 } else { 0 }, None);
                    worth(state, s.side, if on { 2 } else { 0 });
                }
            }
        }
        11 => state
            .build_picker
            .as_ref()
            .unwrap_or_else(|| panic!("popup resources not admitted"))
            .draw_icon(&state.catalog, state.current_ship, !on),
        _ => {}
    }
}
/// # Safety
/// Call on the graphics thread with loaded melee resources. Pointer arguments
/// must be valid for the call, and player, row, column and ship indices must be in range.
pub unsafe fn flash_selection(state: &mut MeleeSession) {
    let now = GetTimeCounter();
    if let Some(selected) = state.selection_flash(now) {
        let context = state
            .menu_context
            .as_ref()
            .unwrap_or_else(|| panic!("menu context not admitted"));
        context.with_selected(|| highlight(state, state.melee_option, selected));
    }
}
pub(super) unsafe fn init_screen(state: &MeleeSession) {
    let context = state
        .menu_context
        .as_ref()
        .unwrap_or_else(|| panic!("menu context not admitted"));
    let target = state
        .menu_target
        .as_ref()
        .unwrap_or_else(|| panic!("menu target not admitted"));
    let extent = target.rect().extent;
    context.set_frame(target.clone());
    context.clear_clip();
    context.set_background(crate::graphics::native_abi::Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    });
    context.clear();
    context.set_clip(crate::graphics::native_abi::RECT {
        corner: crate::graphics::native_abi::POINT {
            x: SAFE_X as i16,
            y: SAFE_Y as i16,
        },
        extent: crate::graphics::native_abi::EXTENT {
            width: extent.width - 2 * SAFE_X as i16,
            height: extent.height - 2 * SAFE_Y as i16,
        },
    });
    redraw(state);
}
/// # Safety
/// Call on the graphics thread with loaded melee resources. Pointer arguments
/// must be valid for the call, and player, row, column and ship indices must be in range.
pub unsafe fn draw_ship_strings(state: &MeleeSession, ship: u32) {
    let (context, master, font) = KernelOwner::current().read(|owner| {
        (
            owner.status_context,
            crate::ships::master_native::master_in(owner, ship as u16),
            owner.battle_font(),
        )
    });
    let old = SetContext(context);
    let mut clip = mem::zeroed();
    get_context_clip_rect(&mut clip);
    let mut r = clip;
    r.corner.x += (SAFE_X as i16 * 2 - 32) + 29;
    r.corner.y += 76;
    r.extent.height = SHIP_INFO_HEIGHT as i16;
    set_context_clip_rect(&mut r);
    BatchGraphics();
    if ship == 255 {
        ClearShipStatus(0);
        SetContextFont(KernelOwner::current().read(|owner| owner.star_con_font));
        let mut rect = RECT {
            corner: POINT { x: 3, y: 4 },
            extent: EXTENT {
                width: 57,
                height: 60,
            },
        };
        SetContextForeGroundColor(rgb(0, 0, 0));
        DrawRectangle(&mut rect);
        let base = MELEE_STRING_BASE as i32 + if state.row < 2 { 0 } else { 2 };
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
        let mut master = master.unwrap_or_else(|| panic!("melee master ship {ship}"));
        crate::ui::shipstat::init_ship_status(
            font.as_ref(),
            &mut master.info,
            None,
            ptr::null_mut(),
        );
    }
    UnbatchGraphics();
    set_context_clip_rect(&mut clip);
    SetContext(old);
}
pub(super) unsafe fn current(state: &mut MeleeSession) {
    state.current_ship = if state.row == 2 {
        255
    } else {
        state.melee_setup.teams[state.side as usize].ships[(state.row * 7 + state.col) as usize]
            as u32
    };
    draw_ship_strings(state, state.current_ship);
}
