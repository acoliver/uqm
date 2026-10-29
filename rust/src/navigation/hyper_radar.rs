//! Hyperspace radar, nearby star and portal elements.
#![cfg(not(test))]
#![allow(non_snake_case)]
use super::abi::*;
use super::globals::{GlobData, RadarContext};
use super::hyper_data::HYPERSTARS;
use super::hyper_encounters::{allocate, ambient, process_all, XOFFS, YOFFS};
use super::hyper_transition::{portal_collision, portal_death, star_collision, star_death};
use super::hyperspace::live::{universe_x, universe_y};
use super::{color, get, set};
use crate::planets::starmap::CurStarDescPtr;
use std::ptr;

unsafe fn grid(ux: i16, uy: i16, ox: i16, oy: i16) {
    ClearDrawable();
    SetContextForeGroundColor(color(0, 16, 0));
    let sx = (ux as i32 - 378).max(0) / 200 * 200;
    let ex = (ux as i32 + 378).min(10000);
    let sy = (uy as i32 - 300).max(0) / 200 * 200;
    let ey = (uy as i32 + 300).min(10000);
    let mut rect = RECT {
        corner: POINT {
            x: 0,
            y: (((9999 - ey) * 53 / 600) as i16).wrapping_sub(oy),
        },
        extent: EXTENT {
            width: 1,
            height: 0,
        },
    };
    rect.extent.height = (((9999 - sy) * 53 / 600) as i16)
        .wrapping_sub(oy)
        .wrapping_sub(rect.corner.y)
        .wrapping_add(1);
    for x in (sx..=ex).step_by(200) {
        rect.corner.x = ((x * 56 / 756) as i16).wrapping_sub(ox);
        DrawFilledRectangle(&mut rect);
    }
    rect.corner.x = ((sx * 56 / 756) as i16).wrapping_sub(ox);
    rect.extent.width = ((ex * 56 / 756) as i16)
        .wrapping_sub(ox)
        .wrapping_sub(rect.corner.x)
        .wrapping_add(1);
    rect.extent.height = 1;
    for y in (sy..=ey).step_by(200) {
        rect.corner.y = (((9999 - y) * 53 / 600) as i16).wrapping_sub(oy);
        DrawFilledRectangle(&mut rect);
    }
}
fn radar(point: POINT, ox: i16, oy: i16) -> POINT {
    POINT {
        x: ((point.x as i32 * 56 / 756) as i16).wrapping_sub(ox),
        y: (((9999 - point.y as i32) * 53 / 600) as i16).wrapping_sub(oy),
    }
}

/// # Safety
/// Hyperspace resources, contexts and queues must be initialized on the game thread.
pub unsafe fn SeedUniverse() {
    static mut PORTAL_POINT: POINT = POINT { x: 0, y: 0 };
    let mut universe = POINT {
        x: universe_x(GlobData.SIS_state.log_x),
        y: universe_y(GlobData.SIS_state.log_y),
    };
    let blip = SetAbsFrameIndex(stars_in_space, 90);
    SetContext(RadarContext);
    BatchGraphics();
    let ox = (universe.x as i32 * 56 / 756 - 28) as i16;
    let oy = ((9999 - universe.y as i32) * 53 / 600 - 26) as i16;
    let counter = get("ARILOU_SPACE_COUNTER");
    let side = get("ARILOU_SPACE_SIDE");
    grid(universe.x, universe.y, ox, oy);
    let mut stamp = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: ptr::null_mut(),
    };
    let mut star = ptr::null_mut();
    loop {
        star = FindStar(star, &mut universe, XOFFS, YOFFS);
        if star.is_null() {
            break;
        }
        let point = (*star).star_pt;
        let mut kind = ((*star).star_type >> 3) & 3;
        if side >= 2 && point.x == ARILOU_HOME_X as i16 && point.y == ARILOU_HOME_Y as i16 {
            kind = SUPER_GIANT_STAR as u8;
        }
        stamp.origin = radar(point, ox, oy);
        stamp.frame = SetRelFrameIndex(blip, kind as i16 + 2);
        DrawStamp(&mut stamp);
    }
    let mut portal = get("PORTAL_COUNTER");
    if portal != 0 || counter != 0 {
        let mut portals = [(POINT { x: 0, y: 0 }, 0u16); 2];
        let mut count = 0usize;
        if portal != 0 {
            if portal == 1 {
                PORTAL_POINT = universe;
            }
            portals[count] = (PORTAL_POINT, ((portal as u16 - 1) >> 1) + 18);
            count += 1;
            portal += 1;
            if portal == 11 {
                portal = 10;
            }
            set("PORTAL_COUNTER", portal);
        }
        if counter != 0 {
            let (point, index) = if side <= 1 {
                (
                    POINT {
                        x: ARILOU_SPACE_X as i16,
                        y: ARILOU_SPACE_Y as i16,
                    },
                    (counter as u16 >> 1) + 18,
                )
            } else {
                (
                    POINT {
                        x: QUASI_SPACE_X as i16,
                        y: QUASI_SPACE_Y as i16,
                    },
                    counter as u16 >> 1,
                )
            };
            portals[count] = (point, index);
            count += 1;
        }
        for i in (0..count).rev() {
            let (point, index) = portals[i];
            let sx = point.x.wrapping_sub(universe.x).wrapping_add(XOFFS);
            let sy = point.y.wrapping_sub(universe.y).wrapping_add(YOFFS);
            if sx < 0 || sy < 0 || sx >= XOFFS * 2 || sy >= YOFFS * 2 {
                continue;
            }
            stamp.origin = radar(point, ox, oy);
            stamp.frame = SetAbsFrameIndex(stars_in_space, 95);
            DrawStamp(&mut stamp);
            if (point.x as i32 - universe.x as i32).abs() > XOFFS as i32 / 12
                || (point.y as i32 - universe.y as i32).abs() > YOFFS as i32 / 12
            {
                continue;
            }
            let handle = allocate(point);
            if handle.is_null() {
                continue;
            }
            let element = handle.cast::<ELEMENT>();
            crate::collections::observation::image!(frame; (*element).current.image.frame, SetAbsFrameIndex(
                HYPERSTARS[1 + (get("ARILOU_SPACE_SIDE") >> 1) as usize],
                index,
            ));
            (*element).preprocess_func = None;
            (*element).postprocess_func = None;
            (*element).collision_func = Some(CollisionHook::Legacy(portal_collision));
            SetUpElement(element);
            if side == 1 || side == 2 {
                (*element).death_func = Some(ElementHook::Legacy(portal_death));
            } else {
                (*element).death_func = None;
                crate::collections::observation::image!(frame; (*element).IntersectControl.IntersectStamp.frame, DecFrameIndex(stars_in_space));
            }
            InsertQueue(ptr::addr_of_mut!(disp_q), handle, disp_q.head);
        }
    }
    star = ptr::null_mut();
    loop {
        star = FindStar(star, &mut universe, XOFFS, YOFFS);
        if star.is_null() {
            break;
        }
        let point = (*star).star_pt;
        if (point.x as i32 - universe.x as i32).abs() > XOFFS as i32 / 12
            || (point.y as i32 - universe.y as i32).abs() > YOFFS as i32 / 12
        {
            continue;
        }
        let handle = allocate(point);
        if handle.is_null() {
            continue;
        }
        let element = handle.cast::<ELEMENT>();
        let kind = (*star).star_type;
        crate::collections::observation::image!(frame; (*element).current.image.frame, SetAbsFrameIndex(
            HYPERSTARS[1 + (get("ARILOU_SPACE_SIDE") >> 1) as usize],
            (((kind >> 3) & 3) as u16 * NUM_STAR_COLORS as u16) + (kind >> 5) as u16,
        ));
        (*element).preprocess_func = None;
        (*element).postprocess_func = None;
        (*element).collision_func = Some(CollisionHook::Native(star_collision));
        SetUpElement(element);
        if star.cast() == CurStarDescPtr && get("PORTAL_COUNTER") == 0 {
            (*element).death_func = Some(ElementHook::Native(star_death));
        } else {
            (*element).death_func = None;
            crate::collections::observation::image!(frame; (*element).IntersectControl.IntersectStamp.frame, DecFrameIndex(stars_in_space));
        }
        InsertQueue(ptr::addr_of_mut!(disp_q), handle, disp_q.head);
    }
    process_all(universe, ox, oy);
    stamp.origin = POINT { x: 28, y: 26 };
    stamp.frame = blip;
    DrawStamp(&mut stamp);
    SetContextForeGroundColor(color(14, 14, 14));
    let mut rect = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 55,
            height: 1,
        },
    };
    DrawFilledRectangle(&mut rect);
    rect.extent = EXTENT {
        width: 1,
        height: 52,
    };
    DrawFilledRectangle(&mut rect);
    SetContextForeGroundColor(color(6, 6, 6));
    rect.corner = POINT { x: 55, y: 1 };
    DrawFilledRectangle(&mut rect);
    rect.corner = POINT { x: 1, y: 52 };
    rect.extent = EXTENT {
        width: 54,
        height: 1,
    };
    DrawFilledRectangle(&mut rect);
    SetContextForeGroundColor(color(8, 8, 8));
    rect.corner.x = 0;
    DrawPoint(&mut rect.corner);
    rect.corner = POINT { x: 55, y: 0 };
    DrawPoint(&mut rect.corner);
    UnbatchGraphics();
    SetContext(StatusContext);
    if TFB_Random() as u16 & 7 == 0 {
        ambient();
    }
    if universe.x != GlobData.Game_state.ShipStamp.origin.x
        || universe.y != GlobData.Game_state.ShipStamp.origin.y
    {
        GlobData.Game_state.ShipStamp.origin = universe;
        DrawHyperCoords(universe);
    }
}
