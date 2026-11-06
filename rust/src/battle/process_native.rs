//! Live element, collision traversal, camera and display-list processing.
//! Integer narrowing follows `uqm/process.c` and its header macros.

use crate::math::wrap_delta;
use crate::ships::native::abi::*;

mod collision_motion;
pub(crate) mod dispatch;
pub mod invoke_collision;
pub mod invoke_element;
mod queue_view;

const SKIP_COLLISION: u16 = NONSOLID as u16 | DISAPPEARING as u16;
const COLLISION_BIT: u16 = COLLISION as u16;
const PRE_PROCESS_BIT: u16 = PRE_PROCESS as u16;
const POST_PROCESS_BIT: u16 = POST_PROCESS as u16;
const CHANGE_BITS: u16 = APPEARING as u16 | CHANGING as u16;
const SCALE_STEP: i32 = 0;
const SCALE_TRILINEAR: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
enum ViewState {
    Stable = VIEW_STATE_VIEW_STABLE,
    Scroll = VIEW_STATE_VIEW_SCROLL,
    Change = VIEW_STATE_VIEW_CHANGE,
}

use queue_view::draw_frame;

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
    pub(super) fn width(pixels: i32) -> i32 {
        (pixels - STATUS_WIDTH as i32) << (ONE_SHIFT + MAX_REDUCTION)
    }
    pub(super) fn height(pixels: i32) -> i32 {
        pixels << (ONE_SHIFT + MAX_REDUCTION)
    }

    pub(super) fn colliding(flags: u16) -> bool {
        flags & SKIP_COLLISION == 0
    }
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::invoke_element::invoke_element;
    use super::*;
    use crate::engine::battle::{Cx, ElementId};

    fn damage(cx: &mut Cx<'_>, id: ElementId) {
        let amount = cx.element(id).hits as SIZE;
        cx.damage(id, amount);
    }

    pub(crate) fn untarget(cx: &mut Cx<'_>, dead: ElementId) {
        let mut next = cx.elements().head();
        while let Some(id) = next {
            next = cx.elements().succ(id);
            if cx.target(id) == Some(dead) {
                cx.set_target(id, None);
            }
        }
    }

    fn preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.elements()[e].life == 0 {
            if cx.elements()[e].parent != crate::engine::battle::Parent::None {
                untarget(cx, e);
            }
            cx.element_mut(e).flags |= DISAPPEARING as ELEMENT_FLAGS;
            if let Some(death) = cx.elements()[e].death_func {
                invoke_element(cx, death, e);
            }
        }
        let mut flags = cx.elements()[e].state_flags;
        if flags & DISAPPEARING as ELEMENT_FLAGS == 0 {
            if flags & APPEARING as ELEMENT_FLAGS != 0 {
                cx.copy_current_to_next(e);
                if colliding(cx.element(e).flags) {
                    cx.init_intersect_start(e);
                    cx.init_intersect_end(e);
                    cx.refresh_intersect_frame(e);
                }
                if flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 {
                    flags &= !(APPEARING as ELEMENT_FLAGS);
                }
            }
            if let Some(pre) = cx.elements()[e].preprocess_func {
                if flags & APPEARING as ELEMENT_FLAGS == 0 {
                    invoke_element(cx, pre, e);
                    flags = cx.elements()[e].state_flags;
                    if flags & CHANGING as ELEMENT_FLAGS != 0 && colliding(cx.element(e).flags) {
                        cx.refresh_intersect_frame(e);
                    }
                }
            }
            if flags & IGNORE_VELOCITY as ELEMENT_FLAGS == 0 {
                let (velocity_x, velocity_y) = (cx.element_mut(e).velocity).get_next_components(1);
                let (dx, dy) = (velocity_x as i16, velocity_y as i16);
                if dx != 0 || dy != 0 {
                    flags |= CHANGING as ELEMENT_FLAGS;
                    cx.element_mut(e).next.location.x =
                        cx.elements()[e].next.location.x.wrapping_add(dx);
                    cx.element_mut(e).next.location.y =
                        cx.elements()[e].next.location.y.wrapping_add(dy);
                }
            }
            if colliding(cx.element(e).flags) {
                cx.init_intersect_end(e);
            }
            if flags & FINITE_LIFE as ELEMENT_FLAGS != 0 {
                cx.element_mut(e).life = cx.elements()[e].life.wrapping_sub(1);
            }
        }
        cx.element_mut(e).flags =
            (flags & !((POST_PROCESS | COLLISION) as ELEMENT_FLAGS)) | PRE_PROCESS as ELEMENT_FLAGS;
    }
    fn collide_native(cx: &mut Cx<'_>, a: ElementId, b: ElementId) {
        let mut left = cx.element(a);
        let mut right = cx.element(b);
        let masses = [left.mass, right.mass];
        let locations = [
            (left.current.location, left.next.location),
            (right.current.location, right.next.location),
        ];
        let (scalar, angles) = collision_motion::impulse(
            &mut collision_motion::Motion {
                flags: &mut left.flags,
                velocity: &mut left.velocity,
            },
            &mut collision_motion::Motion {
                flags: &mut right.flags,
                velocity: &mut right.velocity,
            },
            locations,
            masses,
        );
        cx.element_mut(a).flags = left.flags;
        cx.element_mut(a).velocity = left.velocity;
        cx.element_mut(b).flags = right.flags;
        cx.element_mut(b).velocity = right.velocity;
        let total = SIZE::from(masses[0]) + SIZE::from(masses[1]);
        for (e, mass, angle) in [(a, masses[0], angles[0]), (b, masses[1], angles[1])] {
            if !collision_motion::movable(mass) {
                continue;
            }
            if cx.element(e).flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 {
                let ship = cx.owner(e);
                cx.ship_state_mut(ship).status &=
                    !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as STATUS_FLAGS);
            }
            let mut state = cx.element(e);
            let mut waits = (state.turn, state.thrust);
            collision_motion::respond(
                &mut collision_motion::Motion {
                    flags: &mut state.flags,
                    velocity: &mut state.velocity,
                },
                mass,
                total,
                angle,
                scalar,
                &mut waits,
            );
            cx.element_mut(e).flags = state.flags;
            cx.element_mut(e).velocity = state.velocity;
            cx.element_mut(e).turn = waits.0;
            cx.element_mut(e).thrust = waits.1;
        }
    }

    /// # Safety
    /// Elements must belong to the initialized display queue on the game thread,
    /// with live collision images and callbacks on every solid element.
    fn align_contact_frame(
        cx: &mut Cx<'_>,
        id: ElementId,
        current: &crate::engine::battle::frames::FrameSelection,
        next: &crate::engine::battle::frames::FrameSelection,
    ) {
        if cx.frame_number(current.view()) != cx.frame_number(next.view()) {
            cx.set_frame(id, true, cx.equivalent_frame(next.view(), current.view()));
        } else if next != current {
            cx.copy_current_image_to_next(id);
            if cx.element(id).life > NORMAL_LIFE as COUNT {
                cx.element_mut(id).life = NORMAL_LIFE as COUNT;
            }
        }
    }

    pub fn process_collisions(
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
            if !colliding(cx.element(el).flags) {
                cx.element_mut(el).flags |= COLLISION as ELEMENT_FLAGS;
                return COLLISION as ELEMENT_FLAGS;
            }
            next_test = cx.elements().succ(test);
            if test == el {
                continue;
            }
            if collision_eligible(
                cx.elements()[test].state_flags,
                cx.elements()[el].state_flags,
                cx.elements()[test].parent == cx.elements()[el].parent,
                cx.elements()[test].mass,
                cx.elements()[el].mass,
            ) {
                let mut state_flags = cx.elements()[el].state_flags;
                let mut test_state_flags = cx.elements()[test].state_flags;
                let finite_life = FINITE_LIFE as ELEMENT_FLAGS;
                let mut time_value;
                if (state_flags | test_state_flags) & finite_life != 0
                    && (((state_flags & APPEARING as ELEMENT_FLAGS != 0)
                        && cx.elements()[el].life > 1)
                        || ((test_state_flags & APPEARING as ELEMENT_FLAGS != 0)
                            && cx.elements()[test].life > 1))
                {
                    time_value = 0;
                } else {
                    loop {
                        time_value = cx.intersect_elements(el, test, min_time);
                        if time_value != 1 || (state_flags | test_state_flags) & finite_life != 0 {
                            break;
                        }
                        if state_flags & COLLISION_BIT as ELEMENT_FLAGS != 0 {
                            cx.init_intersect_end(test);
                            cx.set_intersect_origin(
                                test,
                                cx.elements()[test].IntersectControl.EndPoint,
                            );
                            time_value = cx.intersect_elements(el, test, 1);
                            cx.init_intersect_start(test);
                        }
                        if time_value == 1 {
                            let cur = cx.elements()[el].current.image.frame.clone();
                            let next = cx.elements()[el].next.image.frame.clone();
                            let tcur = cx.elements()[test].current.image.frame.clone();
                            let tnext = cx.elements()[test].next.image.frame.clone();
                            if next == cur && tnext == tcur {
                                if test_state_flags & APPEARING as ELEMENT_FLAGS != 0 {
                                    damage(cx, test);
                                    if cx.elements()[test].parent
                                        != crate::engine::battle::Parent::None
                                    {
                                        untarget(cx, test);
                                    }
                                    cx.element_mut(test).flags |=
                                        (COLLISION | DISAPPEARING) as ELEMENT_FLAGS;
                                    if let Some(death) = cx.elements()[test].death_func {
                                        invoke_element(cx, death, test);
                                    }
                                }
                                if state_flags & APPEARING as ELEMENT_FLAGS != 0 {
                                    damage(cx, el);
                                    if cx.elements()[el].parent
                                        != crate::engine::battle::Parent::None
                                    {
                                        untarget(cx, el);
                                    }
                                    cx.element_mut(el).flags |=
                                        (COLLISION | DISAPPEARING) as ELEMENT_FLAGS;
                                    if let Some(death) = cx.elements()[el].death_func {
                                        invoke_element(cx, death, el);
                                    }
                                    return COLLISION_BIT as ELEMENT_FLAGS;
                                }
                                time_value = 0;
                            } else {
                                align_contact_frame(cx, el, &cur, &next);
                                align_contact_frame(cx, test, &tcur, &tnext);
                                cx.init_intersect_start(el);
                                cx.init_intersect_end(el);
                                cx.refresh_intersect_frame(el);
                                if state_flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 {
                                    let owner = cx.owner(el);
                                    cx.ship_state_mut(owner).facing =
                                        cx.frame_number(cx.elements()[el].next.image.frame.view());
                                }
                                cx.init_intersect_start(test);
                                cx.init_intersect_end(test);
                                cx.refresh_intersect_frame(test);
                                if test_state_flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 {
                                    let owner = cx.owner(test);
                                    cx.ship_state_mut(owner).facing = cx
                                        .frame_number(cx.elements()[test].next.image.frame.view());
                                }
                            }
                        }
                        if time_value == 0 {
                            cx.init_intersect_end(el);
                            cx.init_intersect_end(test);
                            break;
                        }
                    }
                }
                if time_value > 0 {
                    let mut save = cx.elements()[el].IntersectControl.EndPoint;
                    let mut test_save = cx.elements()[test].IntersectControl.EndPoint;
                    cx.init_intersect_end(el);
                    cx.init_intersect_end(test);
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
                            cx.set_intersect_origin(test, test_save);
                            cx.element_mut(test).next.location.x = test_save.x << ONE_SHIFT;
                            cx.element_mut(test).next.location.y = test_save.y << ONE_SHIFT;
                            cx.init_intersect_end(test);
                        }
                        if cx.elements()[el].state_flags & COLLISION_BIT as ELEMENT_FLAGS != 0 {
                            if state_flags & COLLISION_BIT as ELEMENT_FLAGS == 0 {
                                cx.set_intersect_origin(el, save);
                                cx.element_mut(el).next.location.x = save.x << ONE_SHIFT;
                                cx.element_mut(el).next.location.y = save.y << ONE_SHIFT;
                                cx.init_intersect_end(el);
                                if state_flags & finite_life == 0
                                    && test_state_flags & finite_life == 0
                                {
                                    collide_native(cx, el, test);
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
                        if !colliding(cx.element(el).flags) {
                            cx.element_mut(el).flags |= COLLISION_BIT as ELEMENT_FLAGS;
                            return COLLISION_BIT as ELEMENT_FLAGS;
                        }
                    }
                }
            }
        }
        cx.elements()[el].state_flags & COLLISION_BIT as ELEMENT_FLAGS
    }

    fn view_dimensions(cx: &Cx<'_>) -> (i32, i32) {
        let (w, h) = cx.screen_dimensions();
        (width(w), height(h))
    }

    fn preprocess_queue(cx: &mut Cx<'_>, scale: i32) -> (queue_view::CameraBounds, SIZE) {
        let active_sides = COUNT::from(cx.b.counters[0] != 0) + COUNT::from(cx.b.counters[1] != 0);
        let mut bounds = queue_view::CameraBounds::new(scale == SCALE_STEP, view_dimensions(cx));
        let mut handle = cx.elements().head();
        while let Some(e) = handle {
            if cx.elements()[e].state_flags & PRE_PROCESS_BIT == 0 {
                preprocess(cx, e);
            }
            let next = cx.elements().succ(e);
            if colliding(cx.element(e).flags) && cx.elements()[e].state_flags & COLLISION_BIT == 0 {
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
                        step: scale == SCALE_STEP,
                        activity: cx.world.activity() as BYTE,
                        dimensions: view_dimensions(cx),
                    },
                );
            }
            handle = next;
        }
        let minimum = bounds.finish(cx.b.max_zoom_out, cx.b.zoom_out, scale == SCALE_STEP);
        (bounds, minimum)
    }

    fn postprocess(cx: &mut Cx<'_>, e: ElementId) {
        if let Some(post) = cx.elements()[e].postprocess_func {
            invoke_element(cx, post, e);
        }
        cx.copy_next_to_current(e);
        if colliding(cx.element(e).flags) {
            cx.init_intersect_start(e);
            cx.init_intersect_end(e);
        }
        cx.element_mut(e).flags = (cx.elements()[e].state_flags
            & !((PRE_PROCESS | CHANGING | APPEARING) as ELEMENT_FLAGS))
            | POST_PROCESS_BIT;
    }

    fn calc_view_engine(
        cx: &mut Cx<'_>,
        next_reduction: SIZE,
        scroll: (SIZE, SIZE),
        ships_alive: COUNT,
        scale: i32,
    ) -> ViewState {
        let (dx, dy) = scroll;
        let state = if cx.b.zoom_out == next_reduction {
            if dx == 0 && dy == 0 && in_hqspace(cx.world) == 0 {
                ViewState::Stable
            } else {
                ViewState::Scroll
            }
        } else {
            let (w, h) = view_dimensions(cx);
            let (new_origin, reduction) = view_origin(
                cx.b.zoom_out,
                next_reduction,
                ships_alive,
                scale == SCALE_STEP,
                w,
                h,
            );
            cx.kernel.space_origin = new_origin;
            cx.b.zoom_out = reduction;
            ViewState::Change
        };
        if (cx.world.activity() as u8) <= IN_HYPERSPACE as u8 {
            move_galaxy(cx, state as VIEW_STATE, dx, dy, scale);
        }
        state
    }

    pub fn init_display_list(cx: &mut Cx<'_>, step_zoom: bool) {
        if step_zoom {
            cx.b.zoom_out = (MAX_VIS_REDUCTION + 1) as SIZE;
            cx.b.max_zoom_out = MAX_VIS_REDUCTION as SIZE;
        } else {
            cx.b.zoom_out = (MAX_ZOOM_OUT + (1 << ZOOM_SHIFT)) as SIZE;
            cx.b.max_zoom_out = MAX_ZOOM_OUT as SIZE;
        }
        cx.reset_elements();
        cx.reset_display();
    }

    fn retire(cx: &mut Cx<'_>, id: ElementId) {
        cx.unlink(id);
        cx.dispose(id);
    }

    fn postprocess_queue(
        cx: &mut Cx<'_>,
        view_state: ViewState,
        mut scroll_x: SIZE,
        mut scroll_y: SIZE,
        scale_mode: i32,
    ) {
        let reduction = if scale_mode == SCALE_STEP {
            (i32::from(cx.b.zoom_out) + ONE_SHIFT as i32) as SIZE
        } else {
            (i32::from(cx.b.zoom_out) << ONE_SHIFT) as SIZE
        };
        let mut handle = cx.elements().head();
        while let Some(e) = handle {
            let mut flags = cx.elements()[e].state_flags;
            let (delta_x, delta_y, next_handle);
            if flags & PRE_PROCESS_BIT != 0 {
                if flags & COLLISION_BIT == 0 {
                    cx.element_mut(e).flags &= !(DEFY_PHYSICS as ELEMENT_FLAGS);
                } else {
                    cx.element_mut(e).flags &= !COLLISION_BIT;
                }
                (delta_x, delta_y) = if flags & POST_PROCESS_BIT != 0 {
                    (0, 0)
                } else {
                    (scroll_x, scroll_y)
                };
            } else {
                let mut pending = handle;
                while let Some(post) = pending {
                    if cx.elements()[post].state_flags & PRE_PROCESS_BIT == 0 {
                        preprocess(cx, post);
                    }
                    let next = cx.elements().succ(post);
                    if colliding(cx.element(post).flags)
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
                let index = cx.elements()[e].PrimIndex;
                let kind = cx.display_primitive(index).kind;
                if view_state != ViewState::Stable || flags & CHANGE_BITS != 0 {
                    let projection = queue_view::Projection {
                        dimensions: {
                            let (w, h) = cx.screen_dimensions();
                            (width(w), height(h))
                        },
                        origin: cx.kernel.space_origin,
                        reduction,
                        step: scale_mode == SCALE_STEP,
                    };
                    let next;
                    if kind == gfx_object_LINE_PRIM as GRAPHICS_PRIM {
                        let (location, first, second) = projection.line(
                            cx.elements()[e].current.location,
                            cx.elements()[e].next.location,
                            (delta_x, delta_y),
                        );
                        next = location;
                        let line = LINE { first, second };
                        cx.display_primitive(index).set_line(line);
                    } else {
                        next =
                            projection.scroll(cx.elements()[e].next.location, (delta_x, delta_y));
                        cx.display_primitive(index)
                            .set_point(projection.display(next));
                        if kind == gfx_object_STAMP_PRIM as GRAPHICS_PRIM
                            || kind == gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM
                        {
                            if view_state == ViewState::Change || flags & CHANGE_BITS != 0 {
                                let (index, scale) = if scale_mode == SCALE_STEP {
                                    (cx.b.zoom_out as COUNT, GSCALE_IDENTITY as COUNT)
                                } else {
                                    calc_zoom_stuff(cx.b.zoom_out)
                                };
                                let frame = cx.equivalent_frame(
                                    cx.elements()[e].next.image.frames.view(usize::from(index)),
                                    cx.elements()[e].next.image.frame.view(),
                                );
                                cx.set_frame(e, true, frame);
                                if scale_mode == SCALE_TRILINEAR
                                    && index < 2
                                    && scale != GSCALE_IDENTITY as COUNT
                                {
                                    let frame = cx.elements()[e].next.image.frame.clone();
                                    let mmframe = cx.equivalent_frame(
                                        cx.elements()[e]
                                            .next
                                            .image
                                            .frames
                                            .view(usize::from(index) + 1),
                                        frame.view(),
                                    );
                                    if !frame.is_absent() && !mmframe.is_absent() {
                                        cx.set_mipmap(frame.view(), mmframe.view());
                                    }
                                }
                            }
                            cx.project_display_stamp(e, projection.display(next));
                        }
                    }
                    cx.element_mut(e).next.location = next;
                }
                postprocess(cx, e);
                if kind < gfx_object_NUM_PRIMS as GRAPHICS_PRIM {
                    cx.append_display(cx.elements()[e].PrimIndex);
                }
                next_handle = cx.elements().succ(e);
            }
            handle = next_handle;
        }
    }

    /// # Safety
    /// Engine contexts, element/display pools and sound system are initialized on the game thread.
    pub(crate) unsafe fn redraw_queue(cx: &mut Cx<'_>, clear: bool) {
        SetContext(cx.kernel.status_context);
        let scale = cx.kernel.configuration.preferences().melee_scale;
        let (bounds, minimum) = preprocess_queue(cx, scale);
        let (w, h) = view_dimensions(cx);
        let (mut sx, mut sy) = view_scroll(bounds.origin, bounds.ships_alive, w, h);
        if (sx != 0 || sy != 0) && in_hqspace(cx.world) != 0 {
            crate::app::bridge::publish(cx, || MoveSIS(&mut sx, &mut sy));
        }
        let state = calc_view_engine(cx, minimum, (sx, sy), bounds.ships_alive, scale);
        postprocess_queue(cx, state, sx, sy, scale);
        if cx.kernel.configuration.preferences().stereo_sfx != 0 {
            crate::battle::sounds::engine::update_positions(cx);
        }
        SetContext(cx.kernel.space_context);
        let activity = cx.world.activity();
        if activity as u8 == SUPER_MELEE as u8
            || activity & (CHECK_ABORT | CHECK_LOAD) as UWORD == 0
        {
            if draw_frame(&mut cx.b.nth_frame) {
                if clear {
                    ClearDrawable();
                }
                let scale = if cx.kernel.configuration.preferences().melee_scale != SCALE_STEP {
                    i32::from(calc_zoom_stuff(cx.b.zoom_out).1)
                } else {
                    0
                };
                if let Some(context) = std::ptr::NonNull::new(cx.kernel.space_context) {
                    let context = crate::graphics::context_abi::ContextLease::retain(context);
                    context.draw_primitives(&mut cx.display_batch(), scale);
                }
            }
            crate::battle::sounds::engine::flush_sounds(cx);
        } else {
            cx.kernel.sounds.silence();
            crate::battle::sounds::engine::flush_sounds(cx);
        }
        cx.clear_display_order();
    }
}

#[cfg(not(test))]
pub use engine::{init_display_list, process_collisions};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::battle::display::DisplayPool;

    #[test]
    fn frame_skip_preserves_byte_counter_cycles_and_suppression() {
        for value in 0..=u8::MAX {
            let mut unthrottled = u16::from(value);
            assert!(draw_frame(&mut unthrottled));
            assert_eq!(unthrottled, u16::from(value));
            let mut suppressed = 0xff00 | u16::from(value);
            assert!(!draw_frame(&mut suppressed));
            assert_eq!(suppressed, 0xff00 | u16::from(value));
        }
        let mut counter = 0x0303;
        for _ in 0..3 {
            assert!(!draw_frame(&mut counter));
            assert_eq!(counter, 0x0302);
            assert!(!draw_frame(&mut counter));
            assert_eq!(counter, 0x0301);
            assert!(draw_frame(&mut counter));
            assert_eq!(counter, 0x0303);
        }
        let mut borrow = 0x0100;
        assert!(!draw_frame(&mut borrow));
        assert_eq!(borrow, 0x00ff);
    }

    #[test]
    fn live_helper_dimensions_and_collision_flags() {
        assert_eq!(helpers::width(320), 8192);
        assert_eq!(helpers::height(240), 7680);
        assert!(helpers::colliding(0));
        assert!(!helpers::colliding(NONSOLID as u16));
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
        let mut display = DisplayPool::new();
        for _ in 0..5 {
            display.alloc().unwrap();
        }
        display.insert_before(0, None);
        display.insert_before(1, None);
        display.insert_before(2, Some(0));
        display.insert_before(3, Some(1));
        assert_eq!(display.order(), vec![2, 0, 3, 1]);
    }

    #[test]
    fn primitive_pool_allocates_in_order_and_reuses_frees_lifo() {
        let mut display = DisplayPool::new();
        for index in 0..330 {
            assert_eq!(display.alloc(), Some(index));
            display.get_mut(index).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
        }
        display.free(0);
        display.free(1);
        assert_eq!(display.alloc(), Some(1));
        assert_eq!(display.alloc(), Some(0));
        assert!(display.alloc().is_none());
        assert!(
            (0..330).all(|index| display.get(index).kind == gfx_object_STAMP_PRIM as GRAPHICS_PRIM)
        );
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
