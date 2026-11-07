#![cfg(not(test))]

use super::palette::*;
use crate::engine::{Kernel, WorldData};
use crate::navigation::abi::*;

use super::menu_state::{
    alternate, end_state, PM_CHANGE_CAPTAIN, PM_CHANGE_SHIP, PM_CREW, PM_CYBORG_OFF,
    PM_CYBORG_SUPER, PM_EXIT_SETTINGS, PM_FUEL, PM_MUSIC_OFF, PM_MUSIC_ON, PM_SOUND_OFF,
    PM_SOUND_ON, PM_STARMAP,
};
const OPT_PC: i32 = 2;

/// # Safety
/// Menu state and game settings must be valid on the game thread.
pub unsafe extern "C" fn previous_menu_state(base: BYTE, current: BYTE) -> BYTE {
    super::menu_state::previous_state(
        base,
        current,
        WorldData::current().read(|owner| owner.game.glob_flags),
    )
}

mod render;
pub use render::draw_menu_state;

/// # Safety
/// Menu state and input globals must be initialized on the game thread.
pub unsafe fn do_menu_chooser(
    input: &crate::engine::input::InputSession,
    menu: *mut MENU_STATE,
    base_state: BYTE,
) -> BOOLEAN {
    crate::app::bridge::enter(|cx| choose_menu_state(cx, input, &mut (*menu).CurState, base_state))
}

pub(crate) fn choose_menu_state(
    cx: &mut crate::engine::battle::Cx<'_>,
    input: &crate::engine::input::InputSession,
    state: &mut u8,
    base_state: BYTE,
) -> BOOLEAN {
    use super::menu_state::{navigate, MenuKeys};
    let pulsed = input.pulsed();
    let keys = MenuKeys {
        previous: pulsed.menu[KEY_MENU_LEFT as usize] != 0
            || pulsed.menu[KEY_MENU_UP as usize] != 0,
        next: pulsed.menu[KEY_MENU_RIGHT as usize] != 0 || pulsed.menu[KEY_MENU_DOWN as usize] != 0,
        select: pulsed.menu[KEY_MENU_SELECT as usize] != 0,
        cancel: pulsed.menu[KEY_MENU_CANCEL as usize] != 0,
    };
    let Some(step) = navigate(
        base_state,
        *state,
        input.preferences().menu == OPT_PC,
        cx.world.data.game.glob_flags,
        keys,
    ) else {
        return 0;
    };
    let dimensions = cx.screen_dimensions();
    draw_menu_state(
        cx.kernel,
        cx.world,
        dimensions,
        step.display.0,
        step.display.1,
    );
    *state = step.state;
    if step.paced {
        cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 20);
    }
    1
}

/// # Safety
/// The status context, graphics resources, and menu string tables must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawMenuStateStrings(begin: BYTE, state: SWORD) {
    let dimensions = (ScreenWidth, ScreenHeight);
    Kernel::current().read(|kernel| {
        crate::engine::World::current()
            .read(|world| draw_menu_state(kernel, world, dimensions, begin, state));
    });
}

enum HyperspaceInput<'a, 'b> {
    Choose(&'a mut u8),
    Submenu(&'a mut crate::engine::observe::Io<'b>, u8),
}

unsafe fn hyperspace_input(
    cx: &mut crate::engine::battle::Cx<'_>,
    event: HyperspaceInput<'_, '_>,
) -> bool {
    match event {
        HyperspaceInput::Choose(state) => {
            let runtime = cx.kernel.input_runtime();
            let flags = cx.world.data.game.glob_flags;
            // SAFETY: the existing renderer borrows the published display and retained input.
            cx.with_legacy(|| choose_menu_state(runtime.input(), state, PM_STARMAP, flags) != 0)
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
        |cx, state| hyperspace_input(cx, HyperspaceInput::Choose(state)),
        |cx, io, selection| hyperspace_input(cx, HyperspaceInput::Submenu(io, selection)),
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
