//! Legacy display and submenu composition for the retained hyperspace menu.

use super::*;

enum HyperspaceInput<'a, 'b> {
    Choose(&'a mut u8),
    Submenu(&'a mut crate::engine::observe::Io<'b>, u8),
}

unsafe fn input_frame(
    cx: &mut crate::engine::battle::Cx<'_>,
    event: HyperspaceInput<'_, '_>,
) -> bool {
    match event {
        HyperspaceInput::Choose(state) => {
            let runtime = cx.kernel.input_runtime();
            choose_menu_state(cx, runtime.input(), state, PM_STARMAP) != 0
        }
        // SAFETY: the later-owned submenu receives the retained context and observer.
        HyperspaceInput::Submenu(io, selection) => match selection {
            1 => {
                crate::planets::pstarmap::star_map(cx, io);
                true
            }
            2 => crate::planets::devices::devices_menu(cx, io) != 0,
            3 => {
                crate::planets::cargo::live::cargo_menu(cx, io);
                true
            }
            4 => crate::planets::roster::live::roster_menu(cx, io) != 0,
            5 => crate::front_end::gameopt::runtime::game_options(cx, io) != 0,
            _ => unreachable!(),
        },
    }
}

/// # Safety
/// The game-thread caller retains the initialized SIS display and submenu resources.
pub unsafe fn hyperspace_menu(
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut previous = None;
    crate::navigation::hyper_menu::run(
        cx,
        io,
        |cx, state| input_frame(cx, HyperspaceInput::Choose(state)),
        |cx, io, selection| input_frame(cx, HyperspaceInput::Submenu(io, selection)),
        |cx, io, operation| {
            use crate::navigation::hyper_menu::Display;
            let space = cx.kernel.space_context;
            if operation == Display::Wait {
                crate::ui::util::live::wait_for_no_input(cx, io, ONE_SECOND / 2, 0);
                return;
            }
            // SAFETY: all raw drawing resources remain owned by the suspended context.
            cx.with_legacy(|| match operation {
                Display::Open => {
                    UnbatchGraphics();
                    let context = SetContext(space);
                    let color = SetContextBackGroundColor(crate::navigation::color(0, 0, 0));
                    previous = Some((context, color));
                    DrawMenuStateStrings(PM_STARMAP, 1);
                    SetFlashRect(usize::MAX as *const RECT);
                    input.set_menu_sounds(15, 16);
                }
                Display::HideFlash => {
                    SetFlashRect(std::ptr::null());
                }
                Display::Selection(state) => DrawMenuStateStrings(PM_STARMAP, state as i16),
                Display::ShowFlash => {
                    SetFlashRect(usize::MAX as *const RECT);
                }
                Display::Close => {
                    SetFlashRect(std::ptr::null());
                    SetContext(space);
                }
                Display::ClearRadar => crate::ui::sis::ClearSISRect(CLEAR_SIS_RADAR as u8),
                Display::Restore => {
                    let (context, color) = previous
                        .take()
                        .unwrap_or_else(|| panic!("hyperspace menu was not opened"));
                    SetContextBackGroundColor(color);
                    SetContext(context);
                }
                Display::Batch => {
                    BatchGraphics();
                }
                Display::Wait => unreachable!(),
            });
        },
    );
}
