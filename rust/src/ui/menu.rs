use super::palette::*;
use crate::engine::input::frames::*;
use crate::engine::world::campaign::*;
use crate::engine::Kernel;
use crate::graphics::native_abi::*;

use super::menu_state::{
    alternate, end_state, PM_CHANGE_CAPTAIN, PM_CHANGE_SHIP, PM_CREW, PM_CYBORG_OFF,
    PM_CYBORG_SUPER, PM_EXIT_SETTINGS, PM_FUEL, PM_MUSIC_OFF, PM_MUSIC_ON, PM_SOUND_OFF,
    PM_SOUND_ON, PM_STARMAP,
};
const OPT_PC: i32 = 2;

mod render;
pub use render::draw_menu_state;

/// Draw a mode's menu using the owners and viewport supplied by its campaign.
pub(crate) fn draw_campaign_menu(cx: &crate::engine::battle::Cx<'_>, base: u8, state: i16) {
    draw_menu_state(cx.kernel, cx.world, cx.screen_dimensions(), base, state);
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
    draw_campaign_menu(cx, step.display.0, step.display.1);
    *state = step.state;
    if step.paced {
        cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 20);
    }
    1
}

pub mod hyperspace;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_keys_preserve_selection_without_display_or_clock_admission() {
        let mut world = crate::engine::World::new();
        let mut kernel = Kernel::new();
        let mut battle = crate::engine::battle::Battle::new();
        let mut cx = crate::engine::battle::Cx::new(&mut battle, &mut world, &mut kernel, || {
            panic!("menu sampled random stream")
        });
        let input = crate::engine::input::InputSession::default();
        let mut selection = 1;
        assert_eq!(
            choose_menu_state(&mut cx, &input, &mut selection, PM_CREW),
            0
        );
        assert_eq!(selection, 1);
        assert!(cx.kernel.sis_highlight.context.borrow().is_none());
    }
}
