//! Main-thread lifecycle callbacks, ported from starcon.c (GPL-2.0-or-later).

use uqm_engine::species::species_id::{SPECIES_ID_ARILOU_ID, SPECIES_ID_LAST_MELEE_ID};
use uqm_engine::world::activity::activity_flags::{CHECK_ABORT, CHECK_LOAD};
use uqm_engine::StoryKey;

fn on_battle_frame(cx: &mut uqm_engine::battle::Cx<'_>, _io: &mut uqm_engine::observe::Io<'_>) {
    uqm_modes::navigation::events::game_clock_tick(cx);
    let opening = uqm_modes::navigation::get(cx.world, StoryKey::ARILOU_SPACE) == 1;
    let counter = uqm_modes::navigation::get(cx.world, StoryKey::ARILOU_SPACE_COUNTER);
    uqm_modes::navigation::set(
        cx.world,
        StoryKey::ARILOU_SPACE_COUNTER,
        if opening {
            counter + u8::from(counter < 9)
        } else {
            counter - u8::from(counter > 0)
        },
    );
    let coordinates = if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0 {
        let catalog = cx.world.retain_catalog();
        uqm_modes::navigation::hyper_radar::seed_universe(cx, &catalog)
    } else {
        None
    };
    if let Some(universe) = coordinates {
        uqm_modes::ui::sis::draw_hyper_coords(
            cx.kernel,
            cx.screen_dimensions().0,
            uqm_engine::graphics::geometry::Point {
                x: universe.x,
                y: universe.y,
            },
        );
    }
    uqm_modes::ui::sis::draw_autopilot_message(
        cx.kernel,
        cx.world,
        cx.screen_dimensions().0,
        uqm_engine::time::counter_abi::get_time_counter,
    );
}

fn background_init_kernel(cx: &mut uqm_engine::battle::Cx<'_>, timeout: u32) {
    for species in SPECIES_ID_ARILOU_ID..=SPECIES_ID_LAST_MELEE_ID {
        uqm_engine::threading::yield_task();
        uqm_content::ships::master_native::engine::load_entry(cx.world, cx.kernel, species);
    }
    uqm_engine::threading::yield_task();
    crate::game_init::kernel_abi::init_game_kernel(cx.kernel, cx.world.activity());
    let input = cx.kernel.input_runtime();
    while crate::time::counter_abi::get_time_counter() <= timeout
        && cx.world.activity() & CHECK_ABORT == 0
    {
        input.update(cx);
        if uqm_engine::host::is_replay() {
            uqm_engine::threading::pacing::linked::sleep_thread_until(timeout.saturating_add(1));
        } else {
            uqm_engine::threading::yield_task();
        };
    }
}

pub fn uqm_splash_with_bg_init_kernel(
    cx: &mut uqm_engine::battle::Cx<'_>,
    io: &mut uqm_engine::observe::Io<'_>,
) {
    uqm_modes::front_end::fmv::splash_screen(cx, io, |cx, timeout| {
        background_init_kernel(cx, timeout);
    });
}

pub fn battle_with_frame_callback(
    cx: &mut uqm_engine::battle::Cx<'_>,
    io: &mut uqm_engine::observe::Io<'_>,
) {
    uqm_modes::battle::battle_native::battle(cx, io, Some(on_battle_frame));
}
