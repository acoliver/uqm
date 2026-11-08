//! Shipyard selection highlights and input mode feedback.
use crate::engine::{
    graphics::native_abi::{BYTE, EXTENT, POINT, RECT},
    input::InputSession,
};
use crate::ui::shipyard_layout::{
    DMS_GetEscortShipRect, HANGAR_DY, HANGAR_SHIPS_ROW, HANGAR_X_COORDS, HANGAR_Y, SHIP_WIN_HEIGHT,
    SHIP_WIN_WIDTH,
};
use crate::ui::sis::{highlight_rect, set_menu_flash};

#[derive(Clone, Copy)]
pub(super) enum ModifyMode {
    Navigate,
    AddEscort,
    EditCrew,
    Exit,
}

pub(super) const SOUND_UP: u16 = 1 << 0;
pub(super) const SOUND_DOWN: u16 = 1 << 1;
pub(super) const SOUND_ARROWS: u16 = 0x0f;
pub(super) const SOUND_SELECT: u16 = 1 << 4;
pub(super) const SOUND_CANCEL: u16 = 1 << 5;

pub(super) fn dms_flash_flag_ship(cx: &mut crate::engine::battle::Cx<'_>) {
    let r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: (cx.screen_dimensions().0
                - crate::engine::math::battle_types::STATUS_WIDTH
                - i32::from(crate::starbase::ids::SAFE_X) * 2
                - 14) as i16,
            height: 61,
        },
    };
    highlight_rect(cx.kernel, r);
}

pub(super) fn dms_flash_escort_ship(cx: &mut crate::engine::battle::Cx<'_>, slot: BYTE) {
    highlight_rect(cx.kernel, DMS_GetEscortShipRect(slot));
}

pub(super) fn dms_flash_flag_ship_crew_count(cx: &mut crate::engine::battle::Cx<'_>) {
    crate::graphics::context_abi::ContextLease::require(cx.kernel.status_context).select();
    let r = crate::ui::sis::gauges::gauge_rect(true);
    highlight_rect(cx.kernel, r);
    crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context).select();
}

pub(super) fn dms_flash_escort_ship_crew_count(cx: &mut crate::engine::battle::Cx<'_>, slot: BYTE) {
    let row = slot / HANGAR_SHIPS_ROW;
    let col = slot % HANGAR_SHIPS_ROW;
    let r = RECT {
        corner: POINT {
            x: HANGAR_X_COORDS[col as usize],
            y: HANGAR_Y + HANGAR_DY * row as i16 + SHIP_WIN_HEIGHT - 6,
        },
        extent: EXTENT {
            width: SHIP_WIN_WIDTH,
            height: 5,
        },
    };
    crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context).select();
    highlight_rect(cx.kernel, r);
}

pub(super) fn dms_flash_active_ship(cx: &mut crate::engine::battle::Cx<'_>, state: BYTE) {
    if state >> 4 != 0 {
        dms_flash_flag_ship(cx);
    } else {
        dms_flash_escort_ship(cx, state);
    }
}

pub(super) fn dms_set_mode(
    cx: &mut crate::engine::battle::Cx<'_>,
    input: &InputSession,
    state: BYTE,
    mode: ModifyMode,
) {
    match mode {
        ModifyMode::Navigate => {
            input.set_menu_sounds(SOUND_ARROWS, SOUND_SELECT);
            dms_flash_active_ship(cx, state);
        }
        ModifyMode::AddEscort => {
            input.set_menu_sounds(SOUND_ARROWS, SOUND_SELECT);
            set_menu_flash(cx.kernel, cx.screen_dimensions(), true);
        }
        ModifyMode::EditCrew => {
            input.set_menu_sounds(SOUND_UP | SOUND_DOWN, SOUND_SELECT | SOUND_CANCEL);
            if state >> 4 != 0 {
                dms_flash_flag_ship_crew_count(cx);
            } else {
                dms_flash_escort_ship_crew_count(cx, state);
            }
        }
        ModifyMode::Exit => {
            input.set_menu_sounds(SOUND_ARROWS, SOUND_SELECT);
            set_menu_flash(cx.kernel, cx.screen_dimensions(), false);
        }
    }
}
