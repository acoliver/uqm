//! Splash, introduction, victory and ship-spin presentations, ported from uqm/fmv.c.

use super::intro::show_presentation;
use crate::engine::battle::Cx;
use crate::engine::observe::Io;
use crate::engine::world::activity::activity_flags::CHECK_ABORT;
use crate::graphics::{context::ContextLease, drawable::LoadedDrawable, geometry::Point};
use crate::sound::music;
use crate::sound::music::MusicRequest;
use crate::sound::resource_types::LoadedMusic;
use crate::ui::util::wait_for_any_button;
use std::ffi::CString;
use uqm_engine::time::counter_abi::ONE_SECOND;

fn fade(cx: &mut Cx<'_>, context: &ContextLease, kind: u32, interval: u32) {
    let deadline = context.fade_screen(kind, interval as i16);
    cx.kernel.input_runtime().sleep_until(cx, deadline);
}

pub fn do_ship_spin(cx: &mut Cx<'_>, io: &mut Io<'_>, index: u16, music: Option<&LoadedMusic>) {
    let context = ContextLease::require(cx.kernel.screen_context)
        .selected_context()
        .unwrap_or_else(|| panic!("ship spin requires the selected game context"));
    let (width, height) = context.screen_dimensions();
    let screen = crate::graphics::geometry::Rect {
        corner: Point { x: 0, y: 0 },
        extent: crate::graphics::geometry::Extent {
            width: width as i16,
            height: height as i16,
        },
    };
    context.save_screen_region(screen);
    if music.is_some() {
        music::stop_requested_music();
    }
    crate::engine::battle::hyperspace::free_hyper_data(cx.kernel);
    let name = CString::new(format!("slides.spins.{index:02}"))
        .unwrap_or_else(|error| panic!("ship spin resource contains NUL: {error}"));
    show_presentation(cx, io, &name);
    fade(cx, &context, 252, ONE_SECOND / 4);
    context.finish_color_transforms();
    let clip = context.clip_rect();
    context.clear_clip();
    context.restore_screen_region(screen);
    context.set_clip(clip);
    if let Some(music) = music {
        music::remember_music(
            MusicRequest::Track(music.playback()),
            true,
            1,
            cx.world.data.game.glob_flags & 8 == 0,
        );
    }
    fade(cx, &context, 253, ONE_SECOND / 4);
    context.finish_color_transforms();
}

pub fn splash_screen(cx: &mut Cx<'_>, io: &mut Io<'_>, process: impl FnOnce(&mut Cx<'_>, u32)) {
    let context = ContextLease::require(cx.kernel.screen_context);
    fade(cx, &context, 252, ONE_SECOND / 120);
    context.select();
    if let Some(title) = LoadedDrawable::load(c"graphics.title") {
        context.draw_stamp(&title.lease().context_frame(0), Point { x: 0, y: 0 });
    }
    let timeout = context.fade_screen(253, (ONE_SECOND / 2) as i16);
    process(cx, timeout);
    if cx.world.activity() & CHECK_ABORT != 0 {
        return;
    }
    wait_for_any_button(cx, io, 0, ONE_SECOND * 3, 1);
    if cx.world.activity() & CHECK_ABORT != 0 {
        return;
    }
    cx.world.set_activity(cx.world.activity() & !CHECK_ABORT);
    fade(cx, &context, 252, ONE_SECOND / 2);
}

pub fn introduction(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    show_presentation(cx, io, c"slides.intro");
    let context = ContextLease::require(cx.kernel.screen_context);
    fade(cx, &context, 252, ONE_SECOND / 2);
}

pub fn victory(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let context = ContextLease::require(cx.kernel.screen_context);
    fade(cx, &context, 252, ONE_SECOND / 2);
    show_presentation(cx, io, c"slides.ending");
    context.fade_screen(252, 0);
}
