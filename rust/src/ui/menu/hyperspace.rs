//! Retained hyperspace menu presentation with supplied submenu routing.
use super::*;
use crate::graphics::menu as draw;

pub fn hyperspace_menu(
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    mut submenu: impl FnMut(
        &mut crate::engine::battle::Cx<'_>,
        &mut crate::engine::observe::Io<'_>,
        u8,
    ) -> bool,
) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut previous = None;
    crate::navigation::hyper_menu::run(
        cx,
        io,
        |cx, state| choose_menu_state(cx, input, state, PM_STARMAP) != 0,
        |cx, io, selection| submenu(cx, io, selection),
        |cx, io, operation| {
            use crate::navigation::hyper_menu::Display;
            let space = cx.kernel.space_context;
            if operation == Display::Wait {
                crate::ui::util::wait_for_no_input(cx, io, ONE_SECOND / 2, 0);
                return;
            }
            match operation {
                Display::Open => {
                    draw::unbatch();
                    let context = draw::select(space);
                    let color = draw::background(crate::navigation::color(0, 0, 0));
                    previous = Some((context, color));
                    crate::ui::menu::draw_menu_state(
                        cx.kernel,
                        cx.world,
                        (cx.screen_dimensions().0, cx.screen_dimensions().1),
                        PM_STARMAP,
                        1,
                    );
                    crate::ui::sis::set_menu_flash(
                        cx.kernel,
                        (cx.screen_dimensions().0, cx.screen_dimensions().1),
                        false,
                    );
                    input.set_menu_sounds(15, 16);
                }
                Display::HideFlash => {
                    crate::ui::sis::clear_flash(cx.kernel);
                }
                Display::Selection(state) => crate::ui::menu::draw_menu_state(
                    cx.kernel,
                    cx.world,
                    (cx.screen_dimensions().0, cx.screen_dimensions().1),
                    PM_STARMAP,
                    state as i16,
                ),
                Display::ShowFlash => {
                    crate::ui::sis::set_menu_flash(
                        cx.kernel,
                        (cx.screen_dimensions().0, cx.screen_dimensions().1),
                        false,
                    );
                }
                Display::Close => {
                    crate::ui::sis::clear_flash(cx.kernel);
                    draw::select(space);
                }
                Display::ClearRadar => crate::ui::sis::clear_sis_rect(
                    cx.kernel,
                    cx.world,
                    (cx.screen_dimensions().0, cx.screen_dimensions().1),
                    CLEAR_SIS_RADAR as u8,
                ),
                Display::Restore => {
                    let (context, color) = previous
                        .take()
                        .unwrap_or_else(|| panic!("hyperspace menu was not opened"));
                    draw::background(color);
                    draw::select(context);
                }
                Display::Batch => {
                    draw::batch();
                }
                Display::Wait => unreachable!(),
            };
        },
    );
}
