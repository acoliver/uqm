//! Kernel resource teardown.

use uqm_engine::battle::access::lifecycle;

/// Release the display, the resource system and the video player.
pub fn free_kernel(kernel: &mut uqm_engine::Kernel) {
    kernel.uninit_resources();
    kernel.close_screen();
}

fn uninit_contexts(kernel: &mut uqm_engine::Kernel) {
    lifecycle::uninit_elements(&mut *kernel);
    kernel.destroy_contexts();
}

fn uninit_kernel(kernel: &mut uqm_engine::Kernel) {
    kernel.uninit_space();
    kernel.release_menu_sounds();
    kernel.release_encounter_text();
    kernel.release_tiny_font();
    kernel.release_battle_font();
    kernel.release_status_frames();
    kernel.reset_race_capacities(0);
}

/// Release the navigation, lander and hyperspace data of the active game.
pub fn free_game_data(cx: &mut uqm_engine::battle::Cx<'_>) {
    cx.kernel.release_navigation_graphics();
    cx.kernel.lander_resources = None;
    cx.kernel.solar_presentation.assets = None;
    uqm_modes::navigation::hyper_data::free_hyper_data(cx.kernel);
}

/// Tear the kernel down if it was initialized, after the active game stopped.
pub fn uninit_game_kernel(cx: &mut uqm_engine::battle::Cx<'_>) {
    let initialized = !cx.kernel.activity_frame.is_null();
    if initialized {
        free_game_data(cx);
        uninit_kernel(cx.kernel);
        uninit_contexts(cx.kernel);
    }
}
