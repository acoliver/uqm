use super::*;

#[derive(Default)]
struct Flags {
    button: bool,
    interrupted: bool,
    drawn: bool,
}

fn interrupted(cx: &mut Cx<'_>, input: &InputSession, flags: &mut Flags) -> bool {
    if flags.button {
        if !button_pressed(cx, input) {
            flags.button = false;
        }
    } else {
        flags.interrupted = flags.interrupted || button_pressed(cx, input);
        if flags.interrupted {
            flags.drawn = true;
            return true;
        }
    }
    false
}

fn button_pressed(cx: &mut Cx<'_>, input: &InputSession) -> bool {
    cx.kernel.input_runtime().update(cx);
    input.current().any_button_pressed()
}

impl StarMap {
    fn repair_map(&mut self, world: &World, race: u16, last: &mut RECT, next: RECT) {
        let r = repair_rectangle(
            *last,
            next,
            self.state.projection.width,
            self.state.projection.height,
        );
        *last = next;
        self.draw_map(world, race, Some(r), true);
    }

    fn update_sphere_position(
        &mut self,
        cx: &mut Cx<'_>,
        input: &InputSession,
        id: LinkId<Fleet>,
        index: u16,
        flags: &mut Flags,
    ) {
        let fleet = *cx.world.avail_races().get(id);
        let dx = fleet.loc.x.wrapping_sub(fleet.known_loc.x);
        let dy = fleet.loc.y.wrapping_sub(fleet.known_loc.y);
        if dx != 0 || dy != 0 {
            let mut walk = LineWalk::new(dx, dy);
            if !flags.drawn {
                self.draw_map(cx.world, u16::MAX, None, true);
                flags.drawn = true;
            }
            let (initial, mut last) = self.display.sphere_rect(
                cx.world,
                cx.world.avail_races_ref().get(id),
                self.state.projection,
            );
            enlarge(&mut last);
            let mut visible = false;
            let mut final_r = zero_rect();
            let mut canceled = false;
            loop {
                let (sphere, mut r) = loop {
                    walk.step(&mut cx.world.avail_races().get_mut(id).known_loc);
                    let pair = self.display.sphere_rect(
                        cx.world,
                        cx.world.avail_races_ref().get(id),
                        self.state.projection,
                    );
                    let before = walk.delta;
                    walk.delta = walk.delta.wrapping_sub(1);
                    if before == 0
                        || (walk.delta & 0x1f == 0
                            && (initial.corner.x != pair.0.corner.x
                                || initial.corner.y != pair.0.corner.y))
                    {
                        break pair;
                    }
                };
                if interrupted(cx, input, flags) {
                    canceled = true;
                    break;
                }
                enlarge(&mut r);
                if initial.corner.x != sphere.corner.x || initial.corner.y != sphere.corner.y {
                    visible = true;
                    self.repair_map(cx.world, index, &mut last, r);
                }
                final_r = r;
                if walk.delta < 0 {
                    break;
                }
            }
            if !canceled && visible {
                self.repair_map(cx.world, u16::MAX, &mut last, final_r);
            }
            {
                let mut queue = cx.world.avail_races();
                let fleet = queue.get_mut(id);
                fleet.known_loc = fleet.loc;
            };
        }
    }

    pub(super) fn update_map(&mut self, cx: &mut Cx<'_>, input: &InputSession) {
        cx.kernel.input_runtime().flush();
        let mut flags = Flags {
            button: true,
            ..Flags::default()
        };
        let mut index = 1u16;
        let mut handle = cx.world.avail_races().head();
        while let Some(id) = handle {
            let next = cx.world.avail_races().succ(id);
            interrupted(cx, input, &mut flags);
            let fleet = *cx.world.avail_races().get(id);
            if fleet.known_strength != 0 {
                self.update_sphere_position(cx, input, id, index, &mut flags);
                self.update_sphere_strength(cx, input, id, index, &mut flags);
            }
            handle = next;
            index += 1;
        }
    }
    fn update_sphere_strength(
        &mut self,
        cx: &mut Cx<'_>,
        input: &InputSession,
        id: LinkId<Fleet>,
        index: u16,
        flags: &mut Flags,
    ) {
        let fleet = *cx.world.avail_races().get(id);
        let mut delta = (i32::from(fleet.actual_strength) - i32::from(fleet.known_strength)) as i16;
        if delta != 0 {
            if !flags.drawn {
                self.draw_map(cx.world, u16::MAX, None, true);
                flags.drawn = true;
            }
            let dx = if delta > 0 {
                1i16
            } else {
                delta = delta.wrapping_neg();
                -1i16
            };
            delta = delta.wrapping_sub(1);
            let (initial, mut last) = self.display.sphere_rect(
                cx.world,
                cx.world.avail_races_ref().get(id),
                self.state.projection,
            );
            enlarge(&mut last);
            let mut visible = false;
            let mut final_r = zero_rect();
            let mut final_sphere = initial;
            let mut canceled = false;
            loop {
                let (sphere, mut r) = loop {
                    {
                        let mut queue = cx.world.avail_races();
                        let fleet = queue.get_mut(id);
                        fleet.known_strength = fleet.known_strength.wrapping_add(dx as u16);
                    };
                    let pair = self.display.sphere_rect(
                        cx.world,
                        cx.world.avail_races_ref().get(id),
                        self.state.projection,
                    );
                    let before = delta;
                    delta = delta.wrapping_sub(1);
                    if before == 0
                        || (delta & 0xf == 0 && initial.extent.height != pair.0.extent.height)
                    {
                        break pair;
                    }
                };
                if interrupted(cx, input, flags) {
                    canceled = true;
                    break;
                }
                enlarge(&mut r);
                if initial.extent.height != sphere.extent.height {
                    visible = true;
                    self.repair_map(cx.world, index, &mut last, r);
                }
                final_r = r;
                final_sphere = sphere;
                if delta < 0 {
                    break;
                }
            }
            if !canceled && (visible || initial.extent.width != final_sphere.extent.width) {
                self.repair_map(cx.world, u16::MAX, &mut last, final_r);
            }
            {
                let mut queue = cx.world.avail_races();
                let fleet = queue.get_mut(id);
                fleet.known_strength = fleet.actual_strength;
            };
        }
    }
}
