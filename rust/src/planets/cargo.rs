//! Cargo display and mineral-discard menu, ported from `planets/cargo.c`.

mod display;
pub use display::{draw_cargo, show_capacity};

fn next_element(current: u8, back: bool, forward: bool) -> u8 {
    if back {
        if current == 0 {
            7
        } else {
            current - 1
        }
    } else if forward {
        if current == 7 {
            0
        } else {
            current + 1
        }
    } else {
        current
    }
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::next_element;
    use crate::c_bindings::controller_input::*;
    use crate::engine::{battle::Cx, observe::Io};
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::Gfx;
    use std::ffi::CString;

    use crate::mainloop::input_abi::live::SetMenuSounds;

    const CARGO_STRINGS: i32 = 162;

    fn description(cx: &mut Cx<'_>, element: u8) {
        let message = cx.with_legacy(|| {
            CString::new(CffiPlatform.game_string(CARGO_STRINGS + 2 + i32::from(element))).unwrap()
        });
        crate::ui::sis::draw_status_message(cx.kernel, cx.world, Some(&message));
    }

    struct Menu {
        current: u8,
    }

    unsafe fn discard(
        cx: &mut Cx<'_>,
        input: &crate::engine::input::InputSession,
        menu: &mut Menu,
        display: &super::display::CargoDisplay,
    ) -> i32 {
        let pulse = input.pulsed();
        if cx.world.activity() & (1 << 14) != 0 || pulse.menu[KEY_MENU_CANCEL as usize] != 0 {
            return 0;
        }
        if pulse.menu[KEY_MENU_SELECT as usize] != 0 {
            let amount = &mut cx.world.data.flagship.element_amounts[usize::from(menu.current)];
            if *amount != 0 {
                *amount -= 1;
                // SAFETY: the cargo menu retains the initialized campaign display.
                display.draw(cx.world, menu.current, menu.current);
                cx.world.data.flagship.total_element_mass -= 1;
                // SAFETY: the cargo menu retains the initialized capacity display.
                display.show_capacity(cx.world);
            } else {
                crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
            }
        } else {
            let next = next_element(
                menu.current,
                pulse.menu[KEY_MENU_UP as usize] != 0 || pulse.menu[KEY_MENU_LEFT as usize] != 0,
                pulse.menu[KEY_MENU_DOWN as usize] != 0 || pulse.menu[KEY_MENU_RIGHT as usize] != 0,
            );
            if next != menu.current {
                // SAFETY: the cargo menu retains its selected element and display resources.
                display.draw(cx.world, menu.current, next);
                description(cx, next);
                menu.current = next;
            }
        }
        // SAFETY: the admitted menu keeps initialized game-thread pacing callbacks live.
        cx.with_legacy(|| crate::battle::c_bridge::SleepThread(840 / 30));
        1
    }

    /// # Safety
    /// Call on the game thread with campaign globals and screen resources initialized.
    pub unsafe fn cargo_menu(cx: &mut Cx<'_>, io: &mut Io<'_>) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let mut menu = Menu { current: 0 };
        let display = super::display::CargoDisplay::new(cx.kernel);
        display.draw(cx.world, u8::MAX, 0);
        description(cx, 0);
        cx.with_legacy(|| SetMenuSounds(input, 15, 16));
        cx.run_input(io, true, |cx, _| {
            discard(cx, input, &mut menu, &display) != 0
        });
        let dimensions = cx.screen_dimensions();
        crate::ui::sis::clear_sis_rect(cx.kernel, cx.world, dimensions, 8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn cargo_uses_supplied_campaign_and_retains_its_display_resources() {
        use crate::graphics::{
            context_abi::ContextLease, drawable_abi::LoadedDrawable, native_font::LoadedFont,
        };
        let mut game = crate::engine::Game::new();
        let mut heading = None;
        let display = crate::campaign_display_test::CampaignDisplay::install_with_resources(
            &mut game,
            || {
                heading = LoadedFont::load(c"fixture.tiny");
            },
        )
        .unwrap();
        let heading = heading.unwrap();
        game.kernel.star_con_font = heading.as_raw();
        let icons = LoadedDrawable::create(2, 1, 1, 74).unwrap();
        game.kernel.misc_data_frame = icons.lease().frame(0).unwrap();
        let context = ContextLease::require(game.kernel.status_context);
        let canvas = LoadedDrawable::create(2, 64, 160, 1).unwrap();
        context.set_frame(canvas.lease().context_frame(0));
        let cargo = super::display::CargoDisplay::new(&game.kernel);
        drop((icons, heading));
        let mut other = crate::engine::Game::new();
        game.world.data.flagship.module_slots[0] =
            crate::starbase::sis_modules::ModuleType::StorageBay as u8;
        game.world.data.flagship.total_element_mass = 1;
        game.world.data.flagship.element_amounts[0] = 1;
        other.world.data.flagship.total_element_mass = 2;
        other.world.data.flagship.element_amounts[0] = 2;
        assert_eq!(super::display::remaining_capacity(&game.world), 499);
        assert_eq!(
            super::display::remaining_capacity(&other.world),
            u32::MAX - 1
        );
        let pixel = || {
            context.flush();
            let mut pixels = vec![
                crate::graphics::native_abi::Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0
                };
                64 * 160
            ];
            assert_ne!(
                // SAFETY: the serialized fixture retains this RAM frame and its complete pixel buffer.
                unsafe {
                    crate::graphics::drawable_abi::ReadFramePixelColors(
                        canvas.lease().frame(0).unwrap(),
                        pixels.as_mut_ptr(),
                        64,
                        160,
                    )
                },
                0
            );
            let value = pixels[35 * 64 + 20];
            (value.r, value.g, value.b, value.a)
        };
        cargo.draw(&game.world, u8::MAX, 0);
        assert_eq!(pixel(), (82, 82, 255, 255));
        cargo.draw(&other.world, 0, 8);
        assert_eq!(pixel(), (0, 0, 165, 255));
        cargo.draw(&game.world, 8, u8::MAX);
        cargo.show_capacity(&other.world);
        context.flush();
        assert_eq!(game.world.data.flagship.element_amounts[0], 1);
        assert_eq!(other.world.data.flagship.element_amounts[0], 2);
        drop((cargo, context, canvas, other));
        game.kernel.star_con_font = std::ptr::null_mut();
        game.kernel.misc_data_frame = std::ptr::null_mut();
        display.finish(game);
    }

    #[test]
    fn mineral_navigation_wraps_and_back_takes_priority() {
        assert_eq!(next_element(0, true, false), 7);
        assert_eq!(next_element(7, false, true), 0);
        assert_eq!(next_element(3, true, true), 2);
        assert_eq!(next_element(3, false, false), 3);
        for i in 0..8 {
            assert!(next_element(i, false, true) < 8);
        }
    }
}
