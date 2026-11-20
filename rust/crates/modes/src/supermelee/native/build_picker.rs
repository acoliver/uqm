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

pub(crate) mod live {
    use super::navigate;

    use crate::engine::input_port::frames::{
        KEY_MENU_CANCEL, KEY_MENU_DOWN, KEY_MENU_LEFT, KEY_MENU_RIGHT, KEY_MENU_SELECT,
        KEY_MENU_SPECIAL, KEY_MENU_UP,
    };
    use crate::engine::time::counter_abi::get_time_counter;
    use crate::engine::world::activity::activity_flags::CHECK_ABORT;
    use crate::graphics::geometry::Extent;
    use crate::graphics::geometry::Point;
    use crate::graphics::geometry::Rect;
    use crate::graphics::{
        context::ContextLease, drawable::ContextFrame, drawable::DrawableLease,
        drawable::LoadedDrawable,
    };
    use crate::ships::master_native::ShipChoice;
    use crate::supermelee::native::menu::live::drawing::{draw_ship_strings, flash_selection};
    use crate::supermelee::session::MeleeSession;

    pub(crate) struct BuildPicker {
        drawable: LoadedDrawable,
        screen: ContextLease,
    }

    impl BuildPicker {
        pub fn rect(&self) -> Rect {
            let bounds = self.drawable.lease().frame_rect(0);
            Rect {
                corner: Point {
                    x: bounds.corner.x,
                    y: bounds.corner.y,
                },
                extent: Extent {
                    width: bounds.extent.width,
                    height: bounds.extent.height,
                },
            }
        }

        fn draw_icon_in(
            context: &ContextLease,
            frame: &DrawableLease,
            catalog: &[ShipChoice],
            ship: u32,
            draw: bool,
        ) {
            let rect = frame.frame_rect(0);
            let origin = crate::graphics::geometry::Point {
                x: rect.corner.x + 20 + (ship % 5) as i16 * 18,
                y: rect.corner.y + 5 + (ship / 5) as i16 * 18,
            };
            let ship = catalog
                .get(ship as usize)
                .unwrap_or_else(|| panic!("popup ship {ship}"));
            let icon = ContextFrame::retain(ship.icon.raw());
            if draw {
                context.draw_stamp(&icon, origin);
            } else {
                context.draw_filled_stamp_preserving_color(
                    &icon,
                    origin,
                    crate::graphics::geometry::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 255,
                    },
                );
            }
        }

        pub fn draw_icon(&self, catalog: &[ShipChoice], ship: u32, draw: bool) {
            Self::draw_icon_in(&self.screen, &self.drawable.lease(), catalog, ship, draw);
        }

        fn draw_frame(&self, side: u16) {
            let frame = self.drawable.lease().context_frame(0);
            let rect = frame.rect();
            let (x, y) = crate::supermelee::native::menu::rules::popup_hotspot(
                side,
                rect.extent.width,
                rect.extent.height,
            );
            frame.replace_hotspot(crate::graphics::geometry::Point { x, y });
            self.screen
                .draw_stamp(&frame, crate::graphics::geometry::Point { x: 0, y: 0 });
        }
    }

    pub fn build_build_pick_frame(
        context: ContextLease,
        screen: ContextLease,
        source: ContextFrame,
        catalog: &[ShipChoice],
    ) -> BuildPicker {
        let rect = source.rect();
        let drawable = LoadedDrawable::create(2, rect.extent.width, rect.extent.height, 1)
            .unwrap_or_else(|| panic!("popup drawable allocation"));
        let frame = drawable.lease();
        context.draw_batch(|| {
            context.set_frame(frame.context_frame(0));
            source.replace_hotspot(crate::graphics::geometry::Point { x: 0, y: 0 });
            context.draw_stamp(&source, crate::graphics::geometry::Point { x: 0, y: 0 });
            for ship in 0..25 {
                BuildPicker::draw_icon_in(&context, &frame, catalog, ship, true);
            }
        });
        BuildPicker { drawable, screen }
    }

    /// State must be the active fleet editor on the game thread.
    pub fn draw_pick_frame(cx: &mut crate::engine::battle::Cx<'_>, state: &MeleeSession) {
        state.build_picker().draw_frame(state.side);
        draw_ship_strings(cx, state, state.current_ship);
    }

    fn pick(
        state: &mut MeleeSession,
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        input: &crate::engine::input::InputSession,
    ) -> u32 {
        let time = get_time_counter();
        input.set_paused(false);
        if cx.world.activity() & CHECK_ABORT != 0 {
            state.build_pick_confirmed = 0;
            return 0;
        }
        input.set_menu_sounds(15, 16);
        let keys = input.pulsed().menu;
        if keys[KEY_MENU_SELECT as usize] != 0 || keys[KEY_MENU_CANCEL as usize] != 0 {
            state.build_pick_confirmed = u32::from(keys[KEY_MENU_CANCEL as usize] == 0);
            return 0;
        }
        let current_ship = state.current_ship;
        if keys[KEY_MENU_SPECIAL as usize] != 0 && current_ship != 255 {
            crate::front_end::fmv::do_ship_spin(cx, io, current_ship as u16, None);
            return 1;
        }
        let selected = navigate(
            current_ship,
            keys[KEY_MENU_LEFT as usize] != 0,
            keys[KEY_MENU_RIGHT as usize] != 0,
            keys[KEY_MENU_UP as usize] != 0,
            keys[KEY_MENU_DOWN as usize] != 0,
        );
        if selected != current_ship {
            let catalog = &state.catalog;
            state.build_picker().draw_icon(catalog, current_ship, true);
            state.current_ship = selected;
            draw_ship_strings(cx, state, selected);
        }
        {
            flash_selection(cx, state);
            cx.kernel
                .input_runtime()
                .sleep_until(cx, time.wrapping_add(840 / 30));
        };
        1
    }
    /// State must be the active fleet editor on the game thread.
    pub fn build_pick_ship(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        input: &crate::engine::input::InputSession,
        state: &mut MeleeSession,
    ) -> bool {
        cx.kernel.input_runtime().flush();
        if state.current_ship == 255 {
            state.current_ship = 0;
        }
        draw_pick_frame(cx, state);
        state.phase = crate::supermelee::session::MenuPhase::Building;
        {
            let mut frame = crate::supermelee::native::menu::live::admission::SessionFrame::new(
                state,
                |state: &mut MeleeSession,
                 cx: &mut crate::engine::battle::Cx<'_>,
                 io: &mut crate::engine::observe::Io<'_>| {
                    pick(state, cx, io, input) != 0
                },
            );
            cx.run_input_frame(io, false, &mut frame);
        }
        state.build_pick_confirmed != 0
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
