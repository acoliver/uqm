//! Main-thread lifecycle callbacks, ported from starcon.c (GPL-2.0-or-later).

use crate::engine::StoryKey;
use crate::ships::native::abi::*;

fn on_battle_frame(
    cx: &mut crate::engine::battle::Cx<'_>,
    _io: &mut crate::engine::observe::Io<'_>,
) {
    cx.with_legacy(|| {
        // SAFETY: the published campaign owner retains the live game clock.
        unsafe { crate::time::clock_abi::live::GameClockTick() }
    });
    let opening = crate::navigation::get(cx.world, StoryKey::ARILOU_SPACE) == 1;
    let counter = crate::navigation::get(cx.world, StoryKey::ARILOU_SPACE_COUNTER);
    crate::navigation::set(
        cx.world,
        StoryKey::ARILOU_SPACE_COUNTER,
        if opening {
            counter + u8::from(counter < 9)
        } else {
            counter - u8::from(counter > 0)
        },
    );
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) as u16 == 0 {
        cx.with_legacy(|| {
            // SAFETY: the published campaign owner retains initialized radar state.
            unsafe { crate::navigation::hyper_radar::SeedUniverse() }
        });
    }
    cx.with_legacy(|| {
        // SAFETY: the published campaign owner retains the active SIS display.
        unsafe { crate::ui::sis::DrawAutoPilotMessage(0) }
    });
}

unsafe fn background_init_kernel(cx: &mut crate::engine::battle::Cx<'_>, timeout: u32) {
    for species in SPECIES_ID_ARILOU_ID..=SPECIES_ID_LAST_MELEE_ID {
        cx.with_legacy(|| crate::threading::legacy_abi::TaskSwitch());
        crate::ships::master_native::engine::load_entry(cx.world, cx.kernel, species);
    }
    cx.with_legacy(|| crate::threading::legacy_abi::TaskSwitch());
    crate::game_init::kernel_abi::InitGameKernel(cx.kernel, cx.world.activity());
    let input = cx.kernel.input_runtime();
    while crate::time::counter_abi::GetTimeCounter() <= timeout
        && cx.world.activity() & CHECK_ABORT as u16 == 0
    {
        cx.with_legacy(|| input.update());
        cx.with_legacy(|| {
            if crate::time::source::is_replay() {
                crate::threading::pacing::linked::sleep_thread_until(timeout.saturating_add(1));
            } else {
                crate::threading::legacy_abi::TaskSwitch();
            }
        });
    }
}

/// # Safety
/// Called by the game loop on the SDL owner thread during kernel initialization.
pub unsafe fn uqm_splash_with_bg_init_kernel(
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) {
    crate::front_end::fmv::runtime::splash_screen(cx, io, Some(background_init_kernel));
}

/// # Safety
/// The game kernel and battle state must be initialized on the SDL owner thread.
pub unsafe fn battle_with_frame_callback(
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) {
    crate::battle::battle_native::battle(cx, io, Some(on_battle_frame));
}

/// # Safety
/// Called by the game thread after immediate input has been sampled.
pub unsafe fn process_utility_keys(input: &crate::engine::input::InputSession) {
    let state = input.immediate();
    if state.menu[KEY_ABORT as usize] != 0 {
        crate::log_abi::log_showBox(false, false);
        std::process::exit(0);
    }
    if state.menu[KEY_FULLSCREEN as usize] != 0 {
        let flags = GfxFlags ^ TFB_GFXFLAGS_FULLSCREEN as i32;
        FlushInput(input);
        crate::graphics::tfb_draw_abi::TFB_DrawScreen_ReinitVideo(
            GraphicsDriver,
            flags,
            ScreenWidthActual,
            ScreenHeightActual,
        );
    }
    #[cfg(feature = "debug-process")]
    {
        let pressed = state.menu[KEY_DEBUG as usize];
        if pressed != 0 && input.debug_key() == 0 {
            crate::front_end::uqmdebug::debug_key_pressed();
        }
        input.replace_debug_key(pressed);
    }
}
