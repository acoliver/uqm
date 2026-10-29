//! Live element, collision traversal, camera and display-list processing.
//! Integer narrowing follows `uqm/process.c` and its header macros.

#[cfg(not(test))]
use crate::engine::battle::legacy::DisplayLinks;
use crate::math::wrap_delta;
use crate::ships::native::abi::*;

mod collision_motion;
pub(crate) mod dispatch;
pub mod invoke_collision;
pub mod invoke_element;
pub mod invoke_race;
pub(crate) mod primitives;
mod queue_view;
use primitives::insert_prim_links;

const SKIP_COLLISION: u16 = NONSOLID as u16 | DISAPPEARING as u16;
const COLLISION_BIT: u16 = COLLISION as u16;
const PRE_PROCESS_BIT: u16 = PRE_PROCESS as u16;
const POST_PROCESS_BIT: u16 = POST_PROCESS as u16;
const CHANGE_BITS: u16 = APPEARING as u16 | CHANGING as u16;
const SCALE_STEP: i32 = 0;
const SCALE_TRILINEAR: i32 = 3;
const END: u16 = u16::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
enum ViewState {
    Stable = VIEW_STATE_VIEW_STABLE,
    Scroll = VIEW_STATE_VIEW_SCROLL,
    Change = VIEW_STATE_VIEW_CHANGE,
}

fn display_align(value: i32) -> i16 {
    (value as COORD) & !((1 << ONE_SHIFT) - 1)
}

fn wrap_value(value: i32, extent: i32) -> i16 {
    (if value < 0 {
        value + extent
    } else if value >= extent {
        value - extent
    } else {
        value
    } as COUNT) as COORD
}

fn calc_zoom_stuff(zoom: SIZE) -> (COUNT, COUNT) {
    let index = if i32::from(zoom) < 2 << ZOOM_SHIFT {
        0
    } else if i32::from(zoom) < 4 << ZOOM_SHIFT {
        1
    } else {
        2
    };
    (
        index,
        ((1 << (u32::from(index) + ZOOM_SHIFT + 8)) / i32::from(zoom)) as COUNT,
    )
}

fn calc_reduction(
    mut dx: SIZE,
    mut dy: SIZE,
    zoom: SIZE,
    step: bool,
    activity: u8,
    width: i32,
    height: i32,
) -> COUNT {
    if activity > IN_ENCOUNTER as u8 {
        return if step { 0 } else { 1 << ZOOM_SHIFT };
    }
    if step {
        let (sdx, sdy) = (dx, dy);
        let mut reduction = MAX_VIS_REDUCTION as COUNT;
        // Each compound assignment narrows to SIZE before the comparison.
        while {
            dx = (i32::from(dx) << REDUCTION_SHIFT) as SIZE;
            i32::from(dx) <= width >> (MAX_REDUCTION - MAX_VIS_REDUCTION)
        } && {
            dy = (i32::from(dy) << REDUCTION_SHIFT) as SIZE;
            i32::from(dy) <= height >> (MAX_REDUCTION - MAX_VIS_REDUCTION)
        } && reduction > 0
        {
            reduction = reduction.wrapping_sub(REDUCTION_SHIFT as COUNT);
        }
        if i32::from(reduction) < i32::from(zoom) && i32::from(zoom) <= MAX_VIS_REDUCTION as i32 {
            let shift = MAX_VIS_REDUCTION - u32::from(reduction);
            if ((i32::from(sdx) + (24 << ONE_SHIFT)) << shift)
                > width >> (MAX_REDUCTION - MAX_VIS_REDUCTION)
                || ((i32::from(sdy) + (20 << ONE_SHIFT)) << shift)
                    > height >> (MAX_REDUCTION - MAX_VIS_REDUCTION)
            {
                reduction = reduction.wrapping_add(REDUCTION_SHIFT as COUNT);
            }
        }
        if reduction == 0 && activity == IN_LAST_BATTLE as u8 {
            reduction = reduction.wrapping_add(REDUCTION_SHIFT as COUNT);
        }
        reduction
    } else {
        dx = (i32::from(dx) * MAX_ZOOM_OUT as i32 / (width >> 2)) as SIZE;
        dx = dx.clamp(1 << ZOOM_SHIFT, MAX_ZOOM_OUT as SIZE);
        dy = (i32::from(dy) * MAX_ZOOM_OUT as i32 / (height >> 2)) as SIZE;
        dy = dy.clamp(1 << ZOOM_SHIFT, MAX_ZOOM_OUT as SIZE);
        let mut reduction = dx.max(dy) as COUNT;
        if reduction < 2 << ZOOM_SHIFT && activity == IN_LAST_BATTLE as u8 {
            reduction = 2 << ZOOM_SHIFT;
        }
        reduction
    }
}

fn view_scroll(origin: POINT, ships_alive: COUNT, width: i32, height: i32) -> (SIZE, SIZE) {
    let mut dx = wrap_delta(
        i32::from((width >> 1) as COORD).wrapping_sub(i32::from(origin.x)) as SIZE,
        width,
    );
    let mut dy = wrap_delta(
        i32::from((height >> 1) as COORD).wrapping_sub(i32::from(origin.y)) as SIZE,
        height,
    );
    if ships_alive == 1 {
        let jump_x = display_align(width / 75);
        let jump_y = display_align(height / 75);
        dx = dx.clamp(-jump_x, jump_x);
        dy = dy.clamp(-jump_y, jump_y);
    }
    (dx, dy)
}

fn view_origin(
    zoom: SIZE,
    mut reduction: SIZE,
    ships_alive: COUNT,
    step: bool,
    width: i32,
    height: i32,
) -> (POINT, SIZE) {
    let (x, y) = if step {
        (
            i32::from((width >> 1) as COORD)
                - (width >> (MAX_REDUCTION as i32 + 1 - i32::from(reduction))),
            i32::from((height >> 1) as COORD)
                - (height >> (MAX_REDUCTION as i32 + 1 - i32::from(reduction))),
        )
    } else {
        let jump = (1 << ZOOM_SHIFT) >> 3;
        if ships_alive == 1
            && zoom > reduction
            && i32::from(zoom) <= MAX_ZOOM_OUT as i32
            && i32::from(zoom) - i32::from(reduction) > jump
        {
            reduction = (i32::from(zoom) - jump) as SIZE;
        }
        (
            i32::from(display_align(
                (width >> 1) - width * i32::from(reduction) / ((MAX_ZOOM_OUT as i32) << 2),
            )),
            i32::from(display_align(
                (height >> 1) - height * i32::from(reduction) / ((MAX_ZOOM_OUT as i32) << 2),
            )),
        )
    };
    (
        POINT {
            x: x as COORD,
            y: y as COORD,
        },
        reduction,
    )
}

fn collision_eligible(
    af: ELEMENT_FLAGS,
    bf: ELEMENT_FLAGS,
    same_parent: bool,
    a_mass: u8,
    b_mass: u8,
) -> bool {
    af & SKIP_COLLISION == 0
        && (bf & af) & COLLISION_BIT == 0
        && ((bf & af) & IGNORE_SIMILAR as u16 == 0 || !same_parent)
        && (b_mass != 0 || a_mass != 0)
}

const fn make_links(pred: COUNT, succ: COUNT) -> PRIM_LINKS {
    pred as PRIM_LINKS | ((succ as PRIM_LINKS) << 16)
}

fn calc_display_coord(c: COORD, origin: COORD, reduction: SIZE, step: bool) -> COORD {
    let delta = i32::from(c) - i32::from(origin);
    (if step {
        delta >> reduction
    } else {
        (delta << ZOOM_SHIFT) / i32::from(reduction)
    }) as COORD
}

#[cfg(not(test))]
use helpers::*;
mod helpers {
    use super::*;
    use crate::collections::observation::image;
    use crate::graphics::pixmap_abi::SetEquFrameIndex;
    pub(super) fn width(pixels: i32) -> i32 {
        (pixels - STATUS_WIDTH as i32) << (ONE_SHIFT + MAX_REDUCTION)
    }
    pub(super) fn height(pixels: i32) -> i32 {
        pixels << (ONE_SHIFT + MAX_REDUCTION)
    }
    pub(super) fn primitive(index: COUNT) -> *mut PRIMITIVE {
        (&raw mut DisplayArray)
            .cast::<PRIMITIVE>()
            .wrapping_add(usize::from(index))
    }

    #[inline]
    pub(super) unsafe fn locked(handle: HELEMENT) -> *mut ELEMENT {
        // LockLink/UnlockLink are queue-table address casts, with assertions in C.
        if !handle.is_null() {
            assert!(!disp_q.pq_tab.is_null());
            assert!((handle as usize) >= disp_q.pq_tab as usize);
            assert!(
                (handle as usize)
                    < disp_q.pq_tab as usize
                        + usize::from(disp_q.object_size) * usize::from(disp_q.num_objects)
            );
        }
        handle.cast()
    }
    #[inline]
    pub(super) fn colliding(e: &ELEMENT) -> bool {
        e.state_flags & SKIP_COLLISION == 0
    }
    #[inline]
    pub(super) fn init_intersect_start(e: &mut ELEMENT) {
        e.IntersectControl.IntersectStamp.origin.x = e.current.location.x >> ONE_SHIFT;
        e.IntersectControl.IntersectStamp.origin.y = e.current.location.y >> ONE_SHIFT;
    }
    #[inline]
    pub(super) fn init_intersect_end(e: &mut ELEMENT) {
        e.IntersectControl.EndPoint.x = e.next.location.x >> ONE_SHIFT;
        e.IntersectControl.EndPoint.y = e.next.location.y >> ONE_SHIFT;
    }
    #[inline]
    pub(super) unsafe fn init_intersect_frame(e: *mut ELEMENT) {
        image!(frame; (*e).IntersectControl.IntersectStamp.frame, SetEquFrameIndex(*(*e).next.image.farray, (*e).next.image.frame));
    }
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod engine {
    use super::invoke_element::invoke_element;
    use super::*;
    use crate::collections::observation::image;
    use crate::engine::battle::{Cx, ElementId};
    use crate::ships::native::frame_index;
    use std::ptr;

    extern "C" {}

    unsafe fn damage(cx: &mut Cx<'_>, id: ElementId) {
        let pointer = cx.elements().raw(id).cast();
        let amount = cx.elements()[id].__bindgen_anon_2.hit_points as SIZE;
        crate::app::bridge::publish(cx, || do_damage(pointer, amount));
    }

    fn untarget(cx: &mut Cx<'_>, dead: ElementId) {
        let mut next = cx.elements().head();
        while let Some(id) = next {
            next = cx.elements().succ(id);
            if cx.target(id) == Some(dead) {
                cx.set_target(id, None);
            }
        }
    }

    unsafe fn preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.elements()[e].__bindgen_anon_1.life_span == 0 {
            if !cx.elements()[e].pParent.is_null() {
                untarget(cx, e);
            }
            cx.records()[e].state_flags |= DISAPPEARING as ELEMENT_FLAGS;
            if let Some(death) = cx.elements()[e].death_func {
                invoke_element(cx, death, e);
            }
        }
        let mut flags = cx.elements()[e].state_flags;
        if flags & DISAPPEARING as ELEMENT_FLAGS == 0 {
            if flags & APPEARING as ELEMENT_FLAGS != 0 {
                image!(copy; cx.records()[e].next, cx.elements()[e].current);
                if colliding(&cx.elements()[e]) {
                    init_intersect_start(&mut cx.records()[e]);
                    init_intersect_end(&mut cx.records()[e]);
                    init_intersect_frame(cx.elements().raw(e).cast());
                }
                if flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 {
                    flags &= !(APPEARING as ELEMENT_FLAGS);
                }
            }
            if let Some(pre) = cx.elements()[e].preprocess_func {
                if flags & APPEARING as ELEMENT_FLAGS == 0 {
                    invoke_element(cx, pre, e);
                    flags = cx.elements()[e].state_flags;
                    if flags & CHANGING as ELEMENT_FLAGS != 0 && colliding(&cx.elements()[e]) {
                        init_intersect_frame(cx.elements().raw(e).cast());
                    }
                }
            }
            if flags & IGNORE_VELOCITY as ELEMENT_FLAGS == 0 {
                let (mut dx, mut dy) = (0, 0);
                GetNextVelocityComponents(&mut cx.records()[e].velocity, &mut dx, &mut dy, 1);
                if dx != 0 || dy != 0 {
                    flags |= CHANGING as ELEMENT_FLAGS;
                    cx.records()[e].next.location.x =
                        cx.elements()[e].next.location.x.wrapping_add(dx);
                    cx.records()[e].next.location.y =
                        cx.elements()[e].next.location.y.wrapping_add(dy);
                }
            }
            if colliding(&cx.elements()[e]) {
                init_intersect_end(&mut cx.records()[e]);
            }
            if flags & FINITE_LIFE as ELEMENT_FLAGS != 0 {
                cx.records()[e].__bindgen_anon_1.life_span =
                    cx.elements()[e].__bindgen_anon_1.life_span.wrapping_sub(1);
            }
        }
        cx.records()[e].state_flags =
            (flags & !((POST_PROCESS | COLLISION) as ELEMENT_FLAGS)) | PRE_PROCESS as ELEMENT_FLAGS;
    }
    unsafe fn collide_native(a: *mut ELEMENT, b: *mut ELEMENT) {
        let masses = [
            (*a).__bindgen_anon_3.mass_points,
            (*b).__bindgen_anon_3.mass_points,
        ];
        let locations = [
            ((*a).current.location, (*a).next.location),
            ((*b).current.location, (*b).next.location),
        ];
        let (scalar, angles) = collision_motion::impulse(
            &mut collision_motion::Motion {
                flags: &mut (*a).state_flags,
                velocity: &mut (*a).velocity,
            },
            &mut collision_motion::Motion {
                flags: &mut (*b).state_flags,
                velocity: &mut (*b).velocity,
            },
            locations,
            masses,
        );
        let total = SIZE::from(masses[0]) + SIZE::from(masses[1]);
        for (e, mass, angle) in [(a, masses[0], angles[0]), (b, masses[1], angles[1])] {
            if !collision_motion::movable(mass) {
                continue;
            }
            if (*e).state_flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 {
                let ship = (*e).pParent.cast::<STARSHIP>();
                (*ship).cur_status_flags &=
                    !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as STATUS_FLAGS);
            }
            let mut waits = (
                (*e).__bindgen_anon_4.turn_wait,
                (*e).__bindgen_anon_5.thrust_wait,
            );
            collision_motion::respond(
                &mut collision_motion::Motion {
                    flags: &mut (*e).state_flags,
                    velocity: &mut (*e).velocity,
                },
                mass,
                total,
                angle,
                scalar,
                &mut waits,
            );
            (*e).__bindgen_anon_4.turn_wait = waits.0;
            (*e).__bindgen_anon_5.thrust_wait = waits.1;
        }
    }

    /// # Safety
    /// Elements must belong to the initialized display queue on the game thread,
    /// with live collision images and callbacks on every solid element.
    pub unsafe fn process_collisions(
        cx: &mut Cx<'_>,
        mut next_test: Option<ElementId>,
        el: ElementId,
        min_time: TIME_VALUE,
        process_flags: ELEMENT_FLAGS,
    ) -> ELEMENT_FLAGS {
        while let Some(test) = next_test {
            if cx.elements()[test].state_flags & process_flags == 0 {
                preprocess(cx, test);
            }
            // A death callback can retire every element owned by its ship,
            // including the active projectile, and clear their callbacks.
            if !colliding(&cx.elements()[el]) {
                cx.records()[el].state_flags |= COLLISION as ELEMENT_FLAGS;
                return COLLISION as ELEMENT_FLAGS;
            }
            next_test = cx.elements().succ(test);
            if test == el {
                continue;
            }
            if collision_eligible(
                cx.elements()[test].state_flags,
                cx.elements()[el].state_flags,
                cx.elements()[test].pParent == cx.elements()[el].pParent,
                cx.elements()[test].__bindgen_anon_3.mass_points,
                cx.elements()[el].__bindgen_anon_3.mass_points,
            ) {
                let mut state_flags = cx.elements()[el].state_flags;
                let mut test_state_flags = cx.elements()[test].state_flags;
                let finite_life = FINITE_LIFE as ELEMENT_FLAGS;
                let mut time_value;
                if (state_flags | test_state_flags) & finite_life != 0
                    && (((state_flags & APPEARING as ELEMENT_FLAGS != 0)
                        && cx.elements()[el].__bindgen_anon_1.life_span > 1)
                        || ((test_state_flags & APPEARING as ELEMENT_FLAGS != 0)
                            && cx.elements()[test].__bindgen_anon_1.life_span > 1))
                {
                    time_value = 0;
                } else {
                    loop {
                        time_value = DrawablesIntersect(
                            &raw mut (*cx.elements().raw(el).cast::<ELEMENT>()).IntersectControl,
                            &raw mut (*cx.elements().raw(test).cast::<ELEMENT>()).IntersectControl,
                            min_time,
                        );
                        if time_value != 1 || (state_flags | test_state_flags) & finite_life != 0 {
                            break;
                        }
                        if state_flags & COLLISION_BIT as ELEMENT_FLAGS != 0 {
                            init_intersect_end(&mut cx.records()[test]);
                            cx.records()[test].IntersectControl.IntersectStamp.origin =
                                cx.elements()[test].IntersectControl.EndPoint;
                            time_value = DrawablesIntersect(
                                &raw mut (*cx.elements().raw(el).cast::<ELEMENT>())
                                    .IntersectControl,
                                &raw mut (*cx.elements().raw(test).cast::<ELEMENT>())
                                    .IntersectControl,
                                1,
                            );
                            init_intersect_start(&mut cx.records()[test]);
                        }
                        if time_value == 1 {
                            let cur = cx.elements()[el].current.image.frame;
                            let next = cx.elements()[el].next.image.frame;
                            let tcur = cx.elements()[test].current.image.frame;
                            let tnext = cx.elements()[test].next.image.frame;
                            if next == cur && tnext == tcur {
                                if test_state_flags & APPEARING as ELEMENT_FLAGS != 0 {
                                    damage(cx, test);
                                    if !cx.elements()[test].pParent.is_null() {
                                        untarget(cx, test);
                                    }
                                    cx.records()[test].state_flags |=
                                        (COLLISION | DISAPPEARING) as ELEMENT_FLAGS;
                                    if let Some(death) = cx.elements()[test].death_func {
                                        invoke_element(cx, death, test);
                                    }
                                }
                                if state_flags & APPEARING as ELEMENT_FLAGS != 0 {
                                    damage(cx, el);
                                    if !cx.elements()[el].pParent.is_null() {
                                        untarget(cx, el);
                                    }
                                    cx.records()[el].state_flags |=
                                        (COLLISION | DISAPPEARING) as ELEMENT_FLAGS;
                                    if let Some(death) = cx.elements()[el].death_func {
                                        invoke_element(cx, death, el);
                                    }
                                    return COLLISION_BIT as ELEMENT_FLAGS;
                                }
                                time_value = 0;
                            } else {
                                if frame_index(cur) != frame_index(next) {
                                    image!(frame; cx.records()[el].next.image.frame, SetEquFrameIndex(next, cur));
                                } else if next != cur {
                                    image!(copy; cx.records()[el].next.image, cx.elements()[el].current.image);
                                    if cx.elements()[el].__bindgen_anon_1.life_span
                                        > NORMAL_LIFE as COUNT
                                    {
                                        cx.records()[el].__bindgen_anon_1.life_span =
                                            NORMAL_LIFE as COUNT;
                                    }
                                }
                                if frame_index(tcur) != frame_index(tnext) {
                                    image!(frame; cx.records()[test].next.image.frame, SetEquFrameIndex(tnext, tcur));
                                } else if tnext != tcur {
                                    image!(copy; cx.records()[test].next.image, cx.elements()[test].current.image);
                                    if cx.elements()[test].__bindgen_anon_1.life_span
                                        > NORMAL_LIFE as COUNT
                                    {
                                        cx.records()[test].__bindgen_anon_1.life_span =
                                            NORMAL_LIFE as COUNT;
                                    }
                                }
                                init_intersect_start(&mut cx.records()[el]);
                                init_intersect_end(&mut cx.records()[el]);
                                init_intersect_frame(cx.elements().raw(el).cast());
                                if state_flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 {
                                    (*(cx.elements()[el].pParent as *mut STARSHIP)).ShipFacing =
                                        frame_index(cx.elements()[el].next.image.frame);
                                }
                                init_intersect_start(&mut cx.records()[test]);
                                init_intersect_end(&mut cx.records()[test]);
                                init_intersect_frame(cx.elements().raw(test).cast());
                                if test_state_flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 {
                                    (*(cx.elements()[test].pParent as *mut STARSHIP)).ShipFacing =
                                        frame_index(cx.elements()[test].next.image.frame);
                                }
                            }
                        }
                        if time_value == 0 {
                            init_intersect_end(&mut cx.records()[el]);
                            init_intersect_end(&mut cx.records()[test]);
                            break;
                        }
                    }
                }
                if time_value > 0 {
                    let mut save = cx.elements()[el].IntersectControl.EndPoint;
                    let mut test_save = cx.elements()[test].IntersectControl.EndPoint;
                    init_intersect_end(&mut cx.records()[el]);
                    init_intersect_end(&mut cx.records()[test]);
                    let collided = time_value == 1
                        || ((state_flags & COLLISION_BIT as ELEMENT_FLAGS != 0
                            || process_collisions(
                                cx,
                                next_test,
                                el,
                                time_value - 1,
                                process_flags,
                            ) == 0)
                            && (test_state_flags & COLLISION_BIT as ELEMENT_FLAGS != 0
                                || process_collisions(
                                    cx,
                                    if cx.elements()[test].state_flags & APPEARING as ELEMENT_FLAGS
                                        == 0
                                    {
                                        cx.elements().succ(el)
                                    } else {
                                        cx.elements().head()
                                    },
                                    test,
                                    time_value - 1,
                                    process_flags,
                                ) == 0));
                    if collided {
                        state_flags = cx.elements()[el].state_flags;
                        test_state_flags = cx.elements()[test].state_flags;
                        super::invoke_collision::invoke_pair(
                            cx,
                            el,
                            &mut save,
                            test,
                            &mut test_save,
                            test_state_flags & PLAYER_SHIP as ELEMENT_FLAGS != 0,
                        );
                        if cx.elements()[test].state_flags & COLLISION_BIT as ELEMENT_FLAGS != 0
                            && test_state_flags & COLLISION_BIT as ELEMENT_FLAGS == 0
                        {
                            cx.records()[test].IntersectControl.IntersectStamp.origin = test_save;
                            cx.records()[test].next.location.x = test_save.x << ONE_SHIFT;
                            cx.records()[test].next.location.y = test_save.y << ONE_SHIFT;
                            init_intersect_end(&mut cx.records()[test]);
                        }
                        if cx.elements()[el].state_flags & COLLISION_BIT as ELEMENT_FLAGS != 0 {
                            if state_flags & COLLISION_BIT as ELEMENT_FLAGS == 0 {
                                cx.records()[el].IntersectControl.IntersectStamp.origin = save;
                                cx.records()[el].next.location.x = save.x << ONE_SHIFT;
                                cx.records()[el].next.location.y = save.y << ONE_SHIFT;
                                init_intersect_end(&mut cx.records()[el]);
                                if state_flags & finite_life == 0
                                    && test_state_flags & finite_life == 0
                                {
                                    collide_native(
                                        cx.elements().raw(el).cast(),
                                        cx.elements().raw(test).cast(),
                                    );
                                    process_collisions(
                                        cx,
                                        cx.elements().head(),
                                        el,
                                        MAX_TIME_VALUE as TIME_VALUE,
                                        process_flags,
                                    );
                                    process_collisions(
                                        cx,
                                        cx.elements().head(),
                                        test,
                                        MAX_TIME_VALUE as TIME_VALUE,
                                        process_flags,
                                    );
                                }
                            }
                            return COLLISION_BIT as ELEMENT_FLAGS;
                        }
                        if !colliding(&cx.elements()[el]) {
                            cx.records()[el].state_flags |= COLLISION_BIT as ELEMENT_FLAGS;
                            return COLLISION_BIT as ELEMENT_FLAGS;
                        }
                    }
                }
            }
        }
        cx.elements()[el].state_flags & COLLISION_BIT as ELEMENT_FLAGS
    }

    unsafe fn preprocess_queue(
        cx: &mut Cx<'_>,
        scroll_x: *mut SIZE,
        scroll_y: *mut SIZE,
    ) -> ViewState {
        let active_sides = COUNT::from(cx.b.counters[0] != 0) + COUNT::from(cx.b.counters[1] != 0);
        let mut bounds = queue_view::CameraBounds::new(
            optMeleeScale == SCALE_STEP,
            (width(ScreenWidth), height(ScreenHeight)),
        );
        let mut handle = cx.elements().head();
        while let Some(e) = handle {
            if cx.elements()[e].state_flags & PRE_PROCESS_BIT == 0 {
                preprocess(cx, e);
            }
            let next = cx.elements().succ(e);
            if colliding(&cx.elements()[e]) && cx.elements()[e].state_flags & COLLISION_BIT == 0 {
                process_collisions(cx, next, e, MAX_TIME_VALUE as TIME_VALUE, PRE_PROCESS_BIT);
            }
            if cx.elements()[e].state_flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 {
                bounds.include(
                    cx.elements()[e].next.location,
                    cx.elements()[e].playerNr,
                    active_sides,
                    &queue_view::ViewConfig {
                        limit: cx.b.max_zoom_out,
                        zoom: cx.b.zoom_out,
                        step: optMeleeScale == SCALE_STEP,
                        activity: cx.world.activity() as BYTE,
                        dimensions: (width(ScreenWidth), height(ScreenHeight)),
                    },
                );
            }
            handle = next;
        }
        let minimum = bounds.finish(
            cx.b.max_zoom_out,
            cx.b.zoom_out,
            optMeleeScale == SCALE_STEP,
        );
        calc_view_engine(
            cx,
            &mut bounds.origin,
            minimum,
            scroll_x,
            scroll_y,
            bounds.ships_alive,
        )
    }

    unsafe fn postprocess(cx: &mut Cx<'_>, e: ElementId) {
        if let Some(post) = cx.elements()[e].postprocess_func {
            invoke_element(cx, post, e);
        }
        image!(copy; cx.records()[e].current, cx.elements()[e].next);
        if colliding(&cx.elements()[e]) {
            init_intersect_start(&mut cx.records()[e]);
            init_intersect_end(&mut cx.records()[e]);
        }
        cx.records()[e].state_flags = (cx.elements()[e].state_flags
            & !((PRE_PROCESS | CHANGING | APPEARING) as ELEMENT_FLAGS))
            | POST_PROCESS_BIT;
    }

    /// # Safety
    /// The initialized element queue and display pool are exclusively game-thread owned.
    #[no_mangle]
    pub unsafe extern "C" fn AllocElement() -> HELEMENT {
        crate::app::bridge::enter(|cx| {
            cx.alloc_element()
                .map_or(ptr::null_mut(), |id| cx.elements().raw(id))
        })
    }

    /// # Safety
    /// `handle` is null or an unlinked allocated element from `disp_q`.
    #[no_mangle]
    pub unsafe extern "C" fn FreeElement(handle: HELEMENT) {
        crate::app::bridge::enter(|cx| {
            if let Some(id) = cx.elements().id(handle) {
                cx.dispose(id);
            }
        });
    }

    unsafe fn calc_view_engine(
        cx: &mut Cx<'_>,
        origin: *mut POINT,
        next_reduction: SIZE,
        scroll_x: *mut SIZE,
        scroll_y: *mut SIZE,
        ships_alive: COUNT,
    ) -> ViewState {
        let (mut dx, mut dy) = view_scroll(
            *origin,
            ships_alive,
            width(ScreenWidth),
            height(ScreenHeight),
        );
        if (dx != 0 || dy != 0) && inHQSpace() != 0 {
            crate::app::bridge::publish(cx, || MoveSIS(&mut dx, &mut dy));
        }
        let state = if cx.b.zoom_out == next_reduction {
            if dx == 0 && dy == 0 && inHQSpace() == 0 {
                ViewState::Stable
            } else {
                ViewState::Scroll
            }
        } else {
            let (new_origin, reduction) = view_origin(
                cx.b.zoom_out,
                next_reduction,
                ships_alive,
                optMeleeScale == SCALE_STEP,
                width(ScreenWidth),
                height(ScreenHeight),
            );
            (*cx.kernel()).space_origin = new_origin;
            cx.b.zoom_out = reduction;
            ViewState::Change
        };
        if (cx.world.activity() as u8) <= IN_HYPERSPACE as u8 {
            crate::app::bridge::publish(cx, || MoveGalaxy(state as VIEW_STATE, dx, dy));
        }
        *scroll_x = dx;
        *scroll_y = dy;
        state
    }

    /// # Safety
    /// `e` is live, with a valid image array if it is collidable.
    #[no_mangle]
    pub unsafe extern "C" fn SetUpElement(e: *mut ELEMENT) {
        image!(copy; (*e).next, (*e).current);
        if colliding(&*e) {
            init_intersect_start(&mut *e);
            init_intersect_end(&mut *e);
            init_intersect_frame(e);
        }
    }

    /// # Safety
    /// The primitive and insertion indices belong to DisplayArray, or `before` is END_OF_LIST.
    #[no_mangle]
    pub unsafe extern "C" fn InsertPrim(list: *mut PRIM_LINKS, index: COUNT, before: COUNT) {
        insert_prim_links((&raw mut DisplayArray).cast(), list, index, before);
    }

    /// # Safety
    /// The element queue has been initialized; no elements may remain in use.
    #[no_mangle]
    pub unsafe extern "C" fn InitDisplayList() {
        crate::app::bridge::enter(|cx| {
            if optMeleeScale == SCALE_STEP {
                cx.b.zoom_out = (MAX_VIS_REDUCTION + 1) as SIZE;
                cx.b.max_zoom_out = MAX_VIS_REDUCTION as SIZE;
            } else {
                cx.b.zoom_out = (MAX_ZOOM_OUT + (1 << ZOOM_SHIFT)) as SIZE;
                cx.b.max_zoom_out = MAX_ZOOM_OUT as SIZE;
            }
            crate::collections::observation::record(
                crate::collections::observation::Event::ResetPool,
            );
            cx.records().reinit();
            for i in 0..MAX_DISPLAY_PRIMS as COUNT {
                (*primitive(i)).Links = make_links(END, i + 1);
            }
            (*primitive(MAX_DISPLAY_PRIMS as COUNT - 1)).Links = make_links(END, END);
            DisplayFreeList = 0;
            DisplayLinks = make_links(END, END);
        });
    }

    /// # Safety
    /// `handle` is currently linked in `disp_q`.
    #[no_mangle]
    pub unsafe extern "C" fn RemoveElement(handle: HELEMENT) {
        if optStereoSFX != 0 {
            let e = locked(handle);
            if !e.is_null() {
                RemoveSoundsForObject(e);
            }
            let _ = locked(handle);
        }
        RemoveQueue(&raw mut disp_q, handle);
    }

    /// # Safety
    /// `dead` and all target handles in the queue refer to live pool elements.
    #[no_mangle]
    pub unsafe extern "C" fn Untarget(dead: *mut ELEMENT) {
        crate::app::bridge::enter(|cx| {
            let dead = cx
                .records()
                .id(dead.cast())
                .unwrap_or_else(|| panic!("null dead element"));
            untarget(cx, dead);
        });
    }

    unsafe fn retire(cx: &mut Cx<'_>, id: ElementId) {
        let raw = cx.elements().raw(id);
        crate::app::bridge::publish(cx, || {
            RemoveElement(raw);
            FreeElement(raw);
        });
    }

    unsafe fn postprocess_queue(
        cx: &mut Cx<'_>,
        view_state: ViewState,
        mut scroll_x: SIZE,
        mut scroll_y: SIZE,
    ) {
        let reduction = if optMeleeScale == SCALE_STEP {
            (i32::from(cx.b.zoom_out) + ONE_SHIFT as i32) as SIZE
        } else {
            (i32::from(cx.b.zoom_out) << ONE_SHIFT) as SIZE
        };
        let mut handle = cx.elements().head();
        while let Some(e) = handle {
            let mut flags = cx.elements()[e].state_flags;
            let (delta_x, delta_y);
            let next_handle;
            if flags & PRE_PROCESS_BIT != 0 {
                if flags & COLLISION_BIT == 0 {
                    cx.records()[e].state_flags &= !(DEFY_PHYSICS as ELEMENT_FLAGS);
                } else {
                    cx.records()[e].state_flags &= !COLLISION_BIT;
                }
                if flags & POST_PROCESS_BIT != 0 {
                    delta_x = 0;
                    delta_y = 0;
                } else {
                    delta_x = scroll_x;
                    delta_y = scroll_y;
                }
            } else {
                let mut pending = handle;
                while let Some(post) = pending {
                    if cx.elements()[post].state_flags & PRE_PROCESS_BIT == 0 {
                        preprocess(cx, post);
                    }
                    let next = cx.elements().succ(post);
                    if colliding(&cx.elements()[post])
                        && cx.elements()[post].state_flags & COLLISION_BIT == 0
                    {
                        process_collisions(
                            cx,
                            cx.elements().head(),
                            post,
                            MAX_TIME_VALUE as TIME_VALUE,
                            PRE_PROCESS_BIT | POST_PROCESS_BIT,
                        );
                    }
                    pending = next;
                }
                scroll_x = 0;
                scroll_y = 0;
                delta_x = 0;
                delta_y = 0;
                flags = cx.elements()[e].state_flags;
            }
            if flags & DISAPPEARING as ELEMENT_FLAGS != 0 {
                next_handle = cx.elements().succ(e);
                retire(cx, e);
            } else {
                let primitive = primitive(cx.elements()[e].PrimIndex);
                let kind = (*primitive).Type;
                if view_state != ViewState::Stable || flags & CHANGE_BITS != 0 {
                    let projection = queue_view::Projection {
                        dimensions: (width(ScreenWidth), height(ScreenHeight)),
                        origin: (*cx.kernel()).space_origin,
                        reduction,
                        step: optMeleeScale == SCALE_STEP,
                    };
                    let next;
                    if kind == gfx_object_LINE_PRIM as GRAPHICS_PRIM {
                        let (location, first, second) = projection.line(
                            cx.elements()[e].current.location,
                            cx.elements()[e].next.location,
                            (delta_x, delta_y),
                        );
                        next = location;
                        (*primitive).Object.Line.first = first;
                        (*primitive).Object.Line.second = second;
                    } else {
                        next =
                            projection.scroll(cx.elements()[e].next.location, (delta_x, delta_y));
                        (*primitive).Object.Point = projection.display(next);
                        if kind == gfx_object_STAMP_PRIM as GRAPHICS_PRIM
                            || kind == gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM
                        {
                            if view_state == ViewState::Change || flags & CHANGE_BITS != 0 {
                                let (index, scale) = if optMeleeScale == SCALE_STEP {
                                    (cx.b.zoom_out as COUNT, GSCALE_IDENTITY as COUNT)
                                } else {
                                    calc_zoom_stuff(cx.b.zoom_out)
                                };
                                image!(frame; cx.records()[e].next.image.frame, SetEquFrameIndex(
                                    *cx.elements()[e].next.image.farray.wrapping_add(usize::from(index)),
                                    cx.elements()[e].next.image.frame,
                                ));
                                if optMeleeScale == SCALE_TRILINEAR
                                    && index < 2
                                    && scale != GSCALE_IDENTITY as COUNT
                                {
                                    let frame = cx.elements()[e].next.image.frame;
                                    let mmframe = SetEquFrameIndex(
                                        *cx.elements()[e]
                                            .next
                                            .image
                                            .farray
                                            .add(usize::from(index) + 1),
                                        frame,
                                    );
                                    if !frame.is_null() && !mmframe.is_null() {
                                        let hot = GetFrameHot(mmframe);
                                        TFB_DrawScreen_SetMipmap(
                                            (*frame).image,
                                            (*mmframe).image,
                                            i32::from(hot.x),
                                            i32::from(hot.y),
                                        );
                                    }
                                }
                            }
                            (*primitive).Object.Stamp.frame = cx.elements()[e].next.image.frame;
                        }
                    }
                    cx.records()[e].next.location = next;
                }
                postprocess(cx, e);
                if kind < gfx_object_NUM_PRIMS as GRAPHICS_PRIM {
                    InsertPrim(&raw mut DisplayLinks, cx.elements()[e].PrimIndex, END);
                }
                next_handle = cx.elements().succ(e);
            }
            handle = next_handle;
        }
    }

    /// # Safety
    /// Engine contexts, element/display pools and sound system are initialized on the game thread.
    unsafe fn redraw_queue(cx: &mut Cx<'_>, clear: BOOLEAN) {
        SetContext(StatusContext);
        let (mut sx, mut sy) = (0, 0);
        let state = preprocess_queue(cx, &mut sx, &mut sy);
        postprocess_queue(cx, state, sx, sy);
        if optStereoSFX != 0 {
            crate::battle::sounds::engine::update_positions(cx);
        }
        SetContext(SpaceContext);
        let activity = cx.world.activity();
        if activity as u8 == SUPER_MELEE as u8
            || activity & (CHECK_ABORT | CHECK_LOAD) as UWORD == 0
        {
            let skip = (cx.b.nth_frame >> 8) as BYTE;
            if skip != BYTE::MAX
                && (skip == 0 || {
                    cx.b.nth_frame = cx.b.nth_frame.wrapping_sub(1);
                    cx.b.nth_frame & 0x00ff == 0
                })
            {
                cx.b.nth_frame = cx.b.nth_frame.wrapping_add(UWORD::from(skip));
                if clear != 0 {
                    ClearDrawable();
                }
                if optMeleeScale != SCALE_STEP {
                    let (_, scale) = calc_zoom_stuff(cx.b.zoom_out);
                    SetGraphicScale(i32::from(scale));
                }
                DrawBatch((&raw mut DisplayArray).cast(), DisplayLinks, 0);
                SetGraphicScale(0);
            }
            crate::battle::sounds::engine::flush_sounds(cx);
        } else {
            crate::app::bridge::publish(cx, || ProcessSound((!0usize) as SOUND, ptr::null_mut()));
            crate::battle::sounds::engine::flush_sounds(cx);
        }
        DisplayLinks = make_links(END, END);
    }
    /// # Safety
    /// Game thread, inside the published lifecycle, with initialized rendering state.
    #[no_mangle]
    pub unsafe extern "C" fn RedrawQueue(clear: BOOLEAN) {
        crate::app::bridge::enter(|cx| redraw_queue(cx, clear));
    }
}

#[cfg(not(test))]
pub use engine::{
    process_collisions, AllocElement, FreeElement, InitDisplayList, InsertPrim, RedrawQueue,
    RemoveElement, SetUpElement, Untarget,
};

#[cfg(test)]
mod tests {
    use super::primitives::{alloc_display_prim, free_display_prim};
    use super::*;

    #[test]
    #[serial_test::serial]
    fn live_helpers_preserve_record_coordinates_and_pool_addresses() {
        use helpers::*;
        assert_eq!(width(320), 8192);
        assert_eq!(height(240), 7680);
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        let mut record: ELEMENT = unsafe { std::mem::zeroed() };
        record.current.location = POINT { x: 44, y: 52 };
        record.next.location = POINT { x: 60, y: 68 };
        assert!(colliding(&record));
        init_intersect_start(&mut record);
        init_intersect_end(&mut record);
        assert_eq!(record.IntersectControl.IntersectStamp.origin.x, 11);
        assert_eq!(record.IntersectControl.EndPoint.y, 17);
        record.state_flags = NONSOLID as u16;
        assert!(!colliding(&record));
        let mut frame = std::ptr::null_mut();
        record.next.image.farray = &raw mut frame;
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        unsafe {
            init_intersect_frame(&raw mut record);
            assert!(record.IntersectControl.IntersectStamp.frame.is_null());
            let original = disp_q;
            disp_q.pq_tab = (&raw mut record).cast();
            disp_q.object_size = size_of::<ELEMENT>() as u16;
            disp_q.num_objects = 1;
            assert_eq!(locked((&raw mut record).cast()), &raw mut record);
            assert!(locked(std::ptr::null_mut()).is_null());
            assert_eq!(primitive(0), (&raw mut DisplayArray).cast());
            disp_q = original;
        }
    }

    #[test]
    fn zoom_mip_thresholds_and_integer_scale() {
        for (zoom, expected) in [
            (256, (0, 256)),
            (257, (0, 255)),
            (511, (0, 128)),
            (512, (1, 256)),
            (513, (1, 255)),
            (1023, (1, 128)),
            (1024, (2, 256)),
        ] {
            assert_eq!(calc_zoom_stuff(zoom), expected);
        }
    }

    #[test]
    fn stepped_reduction_thresholds_hysteresis_and_activity() {
        let reduction =
            |dx, dy, zoom, activity| calc_reduction(dx, dy, zoom, true, activity, 8192, 7680);
        assert_eq!(reduction(0, 0, 3, IN_ENCOUNTER as u8), 0);
        assert_eq!(reduction(1024, 960, 3, IN_ENCOUNTER as u8), 0);
        assert_eq!(reduction(1025, 0, 3, IN_ENCOUNTER as u8), 1);
        assert_eq!(reduction(2048, 1920, 3, IN_ENCOUNTER as u8), 1);
        assert_eq!(reduction(2049, 0, 3, IN_ENCOUNTER as u8), 2);
        assert_eq!(reduction(1024, 0, 1, IN_ENCOUNTER as u8), 1);
        assert_eq!(reduction(928, 880, 1, IN_ENCOUNTER as u8), 0);
        assert_eq!(reduction(929, 0, 1, IN_ENCOUNTER as u8), 1);
        assert_eq!(reduction(0, 881, 1, IN_ENCOUNTER as u8), 1);
        assert_eq!(reduction(0, 0, 3, IN_LAST_BATTLE as u8), 1);
        assert_eq!(reduction(2049, 2000, 2, IN_ENCOUNTER as u8 + 1), 0);
    }

    #[test]
    fn reduction_narrows_size_assignments_before_clamp_or_compare() {
        // The continuous quotient 65536 is narrowed to SIZE before clamping.
        assert_eq!(
            calc_reduction(32704, 0, 256, false, IN_ENCOUNTER as u8, 2044, 7680),
            256
        );
        // dx <<= 1 produces -25536, not 40000, for the first stepped comparison.
        assert_eq!(
            calc_reduction(20000, 0, 3, true, IN_ENCOUNTER as u8, 8192, 7680),
            1
        );
    }

    #[test]
    fn continuous_reduction_uses_both_axes_and_last_battle_floor() {
        let reduction = |dx, dy, activity| calc_reduction(dx, dy, 256, false, activity, 8192, 7680);
        assert_eq!(reduction(0, 0, IN_ENCOUNTER as u8), 256);
        assert_eq!(reduction(1024, 0, IN_ENCOUNTER as u8), 512);
        assert_eq!(reduction(0, 960, IN_ENCOUNTER as u8), 512);
        assert_eq!(reduction(4096, 0, IN_ENCOUNTER as u8), 1024);
        assert_eq!(reduction(0, 0, IN_LAST_BATTLE as u8), 512);
        assert_eq!(reduction(4096, 4096, IN_ENCOUNTER as u8 + 1), 256);
    }

    #[test]
    fn coordinate_wrapping_is_single_step_and_keeps_half_extent_ties() {
        assert_eq!(wrap_value(-1, 100), 99);
        assert_eq!(wrap_value(100, 100), 0);
        assert_eq!(wrap_value(201, 100), 101);
        assert_eq!(wrap_value(-201, 100), -101);
        assert_eq!(wrap_value(65535, 100), -101);
        assert_eq!(wrap_delta(-60, 100), 40);
        assert_eq!(wrap_delta(-50, 100), -50);
        assert_eq!(wrap_delta(50, 100), 50);
        assert_eq!(wrap_delta(60, 100), -40);
        assert_eq!(wrap_delta(i16::MIN, 8192), -24576);
    }

    #[test]
    fn alignment_narrows_to_coord_before_masking() {
        assert_eq!(display_align(7), 4);
        assert_eq!(display_align(-1), -4);
        assert_eq!(display_align(32769), -32768);
        assert_eq!(display_align(65539), 0);
    }

    #[test]
    fn display_coordinate_promotes_subtraction_and_narrows_only_result() {
        assert_eq!(calc_display_coord(120, 100, 2, true), 5);
        assert_eq!(calc_display_coord(120, 100, 1024, false), 5);
        assert_eq!(calc_display_coord(99, 100, 2, true), -1);
        assert_eq!(calc_display_coord(99, 100, 1024, false), 0);
        assert_eq!(calc_display_coord(32767, -32768, 2, true), 16383);
        let narrowed_endpoint = 32760i16.wrapping_add(20);
        assert_eq!(calc_display_coord(narrowed_endpoint, 0, 2, true), -8189);
    }

    #[test]
    fn camera_scroll_clamps_only_one_surviving_ship() {
        let origin = POINT { x: 0, y: 0 };
        assert_eq!(view_scroll(origin, 1, 8192, 7680), (108, 100));
        assert_eq!(view_scroll(origin, 2, 8192, 7680), (4096, 3840));
        let origin = POINT { x: 8191, y: 7679 };
        assert_eq!(view_scroll(origin, 1, 8192, 7680), (-108, -100));
        assert_eq!(
            view_scroll(POINT { x: 4096, y: 3840 }, 0, 8192, 7680),
            (0, 0)
        );
    }

    #[test]
    fn camera_zoom_updates_stepped_zoom_and_smooths_only_one_ship_zoom_in() {
        for (reduction, x, y) in [(0, 3584, 3360), (1, 3072, 2880), (2, 2048, 1920)] {
            let (origin, zoom) = view_origin(3, reduction, 2, true, 8192, 7680);
            assert_eq!((origin.x, origin.y, zoom), (x, y, reduction));
        }
        let (origin, zoom) = view_origin(1024, 256, 1, false, 8192, 7680);
        assert_eq!((origin.x, origin.y, zoom), (2112, 1980, 992));
        let (origin, zoom) = view_origin(1024, 256, 2, false, 8192, 7680);
        assert_eq!((origin.x, origin.y, zoom), (3584, 3360, 256));
        assert_eq!(view_origin(1280, 256, 1, false, 8192, 7680).1, 256);
        assert_eq!(view_origin(512, 480, 1, false, 8192, 7680).1, 480);
        assert_eq!(view_origin(256, 1024, 1, false, 8192, 7680).1, 1024);
    }

    #[test]
    fn collision_eligibility_matches_the_asymmetric_macro() {
        let collision = COLLISION as ELEMENT_FLAGS;
        let similar = IGNORE_SIMILAR as ELEMENT_FLAGS;
        assert!(collision_eligible(0, NONSOLID as u16, false, 1, 0));
        assert!(!collision_eligible(NONSOLID as u16, 0, false, 1, 0));
        assert!(!collision_eligible(DISAPPEARING as u16, 0, false, 1, 0));
        assert!(collision_eligible(collision, 0, false, 1, 0));
        assert!(!collision_eligible(collision, collision, false, 1, 0));
        assert!(collision_eligible(similar, 0, true, 0, 1));
        assert!(!collision_eligible(similar, similar, true, 0, 1));
        assert!(collision_eligible(similar, similar, false, 0, 1));
        assert!(!collision_eligible(0, 0, false, 0, 0));
        assert!(collision_eligible(0, 0, true, 0, 1));
        assert!(collision_eligible(
            PRE_PROCESS_BIT | POST_PROCESS_BIT | CHANGE_BITS,
            0,
            true,
            1,
            0
        ));
    }

    #[test]
    fn primitive_insertion_preserves_head_tail_and_neighbor_order() {
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        let mut array: [PRIMITIVE; 5] = unsafe { std::mem::zeroed() };
        let mut list = make_links(END, END);
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        unsafe {
            insert_prim_links(array.as_mut_ptr(), &mut list, 0, END);
            insert_prim_links(array.as_mut_ptr(), &mut list, 1, END);
            insert_prim_links(array.as_mut_ptr(), &mut list, 2, 0);
            insert_prim_links(array.as_mut_ptr(), &mut list, 3, 1);
        }
        assert_eq!(list, make_links(2, 1));
        assert_eq!(array[2].Links, make_links(END, 0));
        assert_eq!(array[0].Links, make_links(2, 3));
        assert_eq!(array[3].Links, make_links(0, 1));
        assert_eq!(array[1].Links, make_links(3, END));
    }

    #[test]
    fn primitive_pool_allocates_in_order_and_reuses_frees_lifo() {
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        let mut array: [PRIMITIVE; 4] = unsafe { std::mem::zeroed() };
        for (i, primitive) in array.iter_mut().enumerate() {
            primitive.Links = make_links(END, if i == 3 { END } else { i as COUNT + 1 });
            primitive.Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
        }
        let mut free = 0;
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        unsafe {
            assert_eq!(alloc_display_prim(array.as_mut_ptr(), &mut free), 0);
            assert_eq!(alloc_display_prim(array.as_mut_ptr(), &mut free), 1);
            free_display_prim(array.as_mut_ptr(), &mut free, 0);
            free_display_prim(array.as_mut_ptr(), &mut free, 1);
            assert_eq!(free, 1);
            assert_eq!(array[1].Links, make_links(END, 0));
            assert_eq!(array[0].Links, make_links(END, 2));
            for expected in [1, 0, 2, 3] {
                assert_eq!(alloc_display_prim(array.as_mut_ptr(), &mut free), expected);
            }
        }
        assert_eq!(free, END);
        assert!(array
            .iter()
            .all(|p| p.Type == gfx_object_STAMP_PRIM as GRAPHICS_PRIM));
    }

    #[test]
    fn scaler_and_view_discriminants_match_header_values() {
        assert_eq!(SCALE_STEP, 0);
        assert_eq!(SCALE_TRILINEAR, 3);
        assert_eq!(ViewState::Stable as u32, 0);
        assert_eq!(ViewState::Scroll as u32, 1);
        assert_eq!(ViewState::Change as u32, 2);
    }
}
