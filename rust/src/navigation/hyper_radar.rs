//! Hyperspace radar, nearby star and portal elements.
#![cfg(not(test))]
#![allow(non_snake_case)]
use super::abi::*;
use crate::engine::StoryKey;

use super::hyper_encounters::{allocate, ambient, process_all, XOFFS, YOFFS};
use super::hyper_transition::{portal_collision, portal_death, star_collision, star_death};
use super::hyperspace::live::{universe_x, universe_y};
use super::{color, get};
use crate::engine::battle::views::FrameSelection as Frame;
use crate::planets::starmap::find_star_in;
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
    crate::app::bridge::enter(|cx| {
        let universe = POINT {
            x: universe_x(cx.world.data.flagship.log_x),
            y: universe_y(cx.world.data.flagship.log_y),
        };
        let blip = SetAbsFrameIndex(cx.kernel.stars_in_space, 90);
        SetContext(cx.kernel.radar_context);
        BatchGraphics();
        let ox = (universe.x as i32 * 56 / 756 - 28) as i16;
        let oy = ((9999 - universe.y as i32) * 53 / 600 - 26) as i16;
        let side = get(cx.world, StoryKey::ARILOU_SPACE_SIDE);
        grid(universe.x, universe.y, ox, oy);
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: ptr::null_mut(),
        };
        let mut star: *mut crate::engine::world::CStarDesc = ptr::null_mut();
        loop {
            star = find_star_in(cx.world, star.as_ref(), universe, XOFFS, YOFFS);
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
        let (portals, count) = super::portal::portals(cx.world, cx.kernel, universe);
        if count != 0 {
            for i in (0..count).rev() {
                let (point, index) = portals[i];
                let sx = point.x.wrapping_sub(universe.x).wrapping_add(XOFFS);
                let sy = point.y.wrapping_sub(universe.y).wrapping_add(YOFFS);
                if sx < 0 || sy < 0 || sx >= XOFFS * 2 || sy >= YOFFS * 2 {
                    continue;
                }
                stamp.origin = radar(point, ox, oy);
                stamp.frame = SetAbsFrameIndex(cx.kernel.stars_in_space, 95);
                DrawStamp(&mut stamp);
                if (point.x as i32 - universe.x as i32).abs() > XOFFS as i32 / 12
                    || (point.y as i32 - universe.y as i32).abs() > YOFFS as i32 / 12
                {
                    continue;
                }
                let Some(element) = allocate(cx, point) else {
                    continue;
                };
                cx.set_frame(
                    element,
                    false,
                    Frame::retain(SetAbsFrameIndex(
                        cx.kernel.hyperstars
                            [1 + (get(cx.world, StoryKey::ARILOU_SPACE_SIDE) >> 1) as usize],
                        index,
                    )),
                );
                cx.element_mut(element).pre = None;
                cx.element_mut(element).post = None;
                cx.element_mut(element).collide = Some(CollisionHook::Native(portal_collision));
                cx.setup_element(element);
                if side == 1 || side == 2 {
                    cx.element_mut(element).death = Some(ElementHook::Native(portal_death));
                } else {
                    cx.element_mut(element).death = None;
                    cx.set_intersect_frame(
                        element,
                        Frame::retain(DecFrameIndex(cx.kernel.stars_in_space)),
                    );
                }
                let head = cx.elements().head();
                cx.insert_before(element, head);
            }
        }
        star = ptr::null_mut();
        loop {
            star = find_star_in(cx.world, star.as_ref(), universe, XOFFS, YOFFS);
            if star.is_null() {
                break;
            }
            let point = (*star).star_pt;
            if (point.x as i32 - universe.x as i32).abs() > XOFFS as i32 / 12
                || (point.y as i32 - universe.y as i32).abs() > YOFFS as i32 / 12
            {
                continue;
            }
            let Some(element) = allocate(cx, point) else {
                continue;
            };
            let kind = (*star).star_type;
            cx.set_frame(
                element,
                false,
                Frame::retain(SetAbsFrameIndex(
                    cx.kernel.hyperstars
                        [1 + (get(cx.world, StoryKey::ARILOU_SPACE_SIDE) >> 1) as usize],
                    (((kind >> 3) & 3) as u16 * NUM_STAR_COLORS as u16) + (kind >> 5) as u16,
                )),
            );
            cx.element_mut(element).pre = None;
            cx.element_mut(element).post = None;
            cx.element_mut(element).collide = Some(CollisionHook::Native(star_collision));
            cx.setup_element(element);
            if star.cast()
                == cx
                    .world
                    .star_pointer(crate::planets::starmap::current_catalog())
                && get(cx.world, StoryKey::PORTAL_COUNTER) == 0
            {
                cx.element_mut(element).death = Some(ElementHook::Native(star_death));
            } else {
                cx.element_mut(element).death = None;
                cx.set_intersect_frame(
                    element,
                    Frame::retain(DecFrameIndex(cx.kernel.stars_in_space)),
                );
            }
            let head = cx.elements().head();
            cx.insert_before(element, head);
        }
        process_all(cx, universe, ox, oy);
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
        SetContext(cx.kernel.status_context);
        if TFB_Random() as u16 & 7 == 0 {
            ambient(cx);
        }
        if universe.x != cx.world.data.game.ship_stamp.origin.x
            || universe.y != cx.world.data.game.ship_stamp.origin.y
        {
            cx.world.data.game.ship_stamp.origin = universe;
            DrawHyperCoords(universe);
        }
    });
}
