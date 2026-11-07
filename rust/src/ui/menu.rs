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

pub(crate) mod hyperspace;
