//! Presentation scripts and legacy-video input, ported from uqm/intro.c.

pub(super) mod args;
mod commands;
#[cfg(test)]
mod owned_tests;
mod records;
mod render;
#[cfg(test)]
mod render_tests;
pub(super) mod text;

use args::rgb as parse_rgb;
use records::PresentationState;
use std::ffi::{CStr, CString};
use uqm_engine::battle::Cx;
use uqm_engine::graphics::{
    context::ContextLease, drawable::ContextFrame, drawable::LoadedDrawable, geometry::Color,
    geometry::Rect,
};
use uqm_engine::input::InputSession;
use uqm_engine::observe::CheckpointView;
use uqm_engine::observe::Io;
use uqm_engine::sound::music;
use uqm_engine::sound::resource_types::LoadedMusic;
use uqm_engine::strings::owned_table::LoadedTextTable;
use uqm_engine::video::legacy::LoadedVideo;
use uqm_engine::video::legacy::VideoPlayback;
use uqm_engine::world::activity::activity_flags::CHECK_ABORT;
use uqm_engine::{
    input::frames::{
        KEY_MENU_CANCEL, KEY_MENU_LEFT, KEY_MENU_RIGHT, KEY_MENU_SELECT, KEY_MENU_SPECIAL,
    },
    time::counter_abi::{get_time_counter, ONE_SECOND},
};

pub(super) const BLACK_COLOR: Color = crate::ui::rgb15(0, 0, 0);

fn bounded(bytes: &[u8], limit: usize) -> CString {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len())
        .min(limit);
    CString::new(&bytes[..end])
        .unwrap_or_else(|error| panic!("bounded presentation string contains NUL: {error}"))
}

fn menu(input: &InputSession, key: u32) -> bool {
    input.pulsed().menu[key as usize] != 0
}

fn admit(cx: &mut Cx<'_>, io: &mut Io<'_>, state: &mut PresentationState) -> Option<bool> {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    if menu(input, KEY_MENU_CANCEL) || cx.world.activity() & CHECK_ABORT != 0 {
        io.observer
            .checkpoint(&CheckpointView::PresentationCancelled);
        return Some(false);
    }
    if state.time_out == 0 {
        return None;
    }
    let mut delay = ONE_SECOND / 84;
    if get_time_counter() >= state.time_out {
        if state.movie_frame >= 0 {
            state.movie_frame();
            state.movie_frame += 1;
            if state.movie_frame > state.movie_end_frame {
                state.movie_frame = -1;
            }
            delay = state.interframe_delay as u32;
        } else {
            state.time_out = 0;
            return Some(true);
        }
    }
    if state.time_out_on_skip
        && (menu(input, KEY_MENU_SELECT)
            || menu(input, KEY_MENU_SPECIAL)
            || menu(input, KEY_MENU_RIGHT))
    {
        state.time_out = 0;
        state.movie_frame = -1;
        return Some(true);
    }
    runtime.sleep(cx, delay);
    Some(true)
}

fn presentation_frame(cx: &mut Cx<'_>, io: &mut Io<'_>, state: &mut PresentationState) -> bool {
    if let Some(keep) = admit(cx, io, state) {
        return keep;
    }
    loop {
        match commands::advance(cx, io, state) {
            commands::Step::Input(keep) => return keep,
            commands::Step::Call(path) => {
                show_slides(
                    cx,
                    io,
                    LoadedTextTable::load_file(cx.kernel.content().files(), &path).ok(),
                    false,
                );
            }
        }
    }
}

struct DisplayRestore {
    context: ContextLease,
    font: Option<uqm_engine::graphics::font::FontLease>,
    clip: Rect,
}
impl Drop for DisplayRestore {
    fn drop(&mut self) {
        self.context.replace_font(self.font.take());
        self.context.set_clip(self.clip);
    }
}

fn show_slides(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    table: Option<LoadedTextTable>,
    ending: bool,
) -> bool {
    let Some(table) = table else {
        io.observer
            .checkpoint(&CheckpointView::PresentationCancelled);
        return false;
    };
    if ending {
        io.observer.checkpoint(&CheckpointView::PresentationStarted);
    }
    let context = ContextLease::require(cx.kernel.screen_context);
    context.with_selected(|| {
        let restore = DisplayRestore {
            font: context.font_lease(),
            clip: context.clip_rect(),
            context: context.clone(),
        };
        context.replace_font(None);
        context.set_background(BLACK_COLOR);
        cx.kernel.input_controls().set_menu_sounds(0, 0);
        let mut state = PresentationState::new(table, context.clone(), ending);
        cx.run_input(io, true, |cx, io| presentation_frame(cx, io, &mut state));
        cx.kernel.input_runtime().sleep_until(
            cx,
            uqm_engine::sound::heart_ffi::fade_music(0, ONE_SECOND as i16),
        );
        music::stop_requested_music();
        uqm_engine::sound::heart_ffi::fade_music(160, 0);
        drop(state.music.take());
        drop(state.rotated.take());
        drop(state.frame.take());
        state.fonts = std::array::from_fn(|_| None);
        drop(restore);
        true
    })
}

pub fn show_presentation(cx: &mut Cx<'_>, io: &mut Io<'_>, presentation: &CStr) -> bool {
    let content = cx.kernel.content();
    let Some(kind) = content.kind(presentation.to_str().unwrap_or("")) else {
        return false;
    };
    match kind {
        "STRTAB" => show_slides(
            cx,
            io,
            LoadedTextTable::load(&cx.kernel.content(), presentation).ok(),
            presentation == c"slides.ending",
        ),
        "3DOVID" => show_video(
            cx,
            io,
            LoadedVideo::load(&cx.kernel.content(), presentation).ok(),
        ),
        _ => {
            log::warn!(
                "Tried to present '{}', of non-presentable type '{kind}'",
                presentation.to_string_lossy()
            );
            false
        }
    }
}

fn video_input(cx: &mut Cx<'_>, video: &VideoPlayback) -> bool {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    if !video.is_playing()
        || menu(input, KEY_MENU_SELECT)
        || menu(input, KEY_MENU_CANCEL)
        || menu(input, KEY_MENU_SPECIAL)
        || cx.world.activity() & CHECK_ABORT != 0
    {
        return false;
    }
    if menu(input, KEY_MENU_LEFT) || menu(input, KEY_MENU_RIGHT) {
        let position = video.position() as i32;
        let position = if menu(input, KEY_MENU_LEFT) {
            position.wrapping_sub(2000)
        } else {
            position.wrapping_add(1000)
        };
        video.seek(position.max(0) as u32);
    } else {
        if !video.process_frame() {
            return false;
        }
        runtime.sleep(cx, ONE_SECOND / 40);
    }
    true
}

fn fade_clear_screen(cx: &mut Cx<'_>) {
    let context = ContextLease::require(cx.kernel.screen_context);
    let deadline = context.fade_screen(252, (ONE_SECOND / 2) as i16);
    cx.kernel.input_runtime().sleep_until(cx, deadline);
    context.select();
    context.set_background(BLACK_COLOR);
    context.clear();
    context.fade_screen(253, 0);
}

fn show_video(cx: &mut Cx<'_>, io: &mut Io<'_>, video: Option<LoadedVideo>) -> bool {
    fade_clear_screen(cx);
    let Some(playback) = video.as_ref().and_then(|video| match video.play() {
        Ok(playback) => Some(playback),
        Err(error) => {
            log::warn!("Video unavailable: {error}");
            None
        }
    }) else {
        return false;
    };
    cx.kernel.input_controls().set_menu_sounds(0, 0);
    cx.run_input(io, true, |cx, _| video_input(cx, &playback));
    drop(playback);
    fade_clear_screen(cx);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_colors_use_rgb_bytes_and_zero_alpha() {
        let color = parse_rgb(b" 0x1234ab trailing").unwrap();
        assert_eq!((color.r, color.g, color.b, color.a), (0x12, 0x34, 0xab, 0));
        assert!(parse_rgb(b"invalid").is_none());
        assert_eq!((color.r, color.g, color.b, color.a), (0x12, 0x34, 0xab, 0));
        let color = parse_rgb(b"ff00ff!").unwrap();
        assert_eq!((color.r, color.g, color.b, color.a), (255, 0, 255, 0));
    }
    #[test]
    fn rgb15_expands_all_five_bits_and_is_opaque() {
        let color = crate::ui::rgb15(1, 10, 31);
        assert_eq!((color.r, color.g, color.b, color.a), (8, 82, 255, 255));
    }
    #[test]
    fn line_parser_preserves_empty_rows_and_ignores_trailing_newline() {
        let lines = text::owned_lines(b"first\n\nlast\n\0", 5);
        let result: Vec<_> = lines.iter().map(|line| line.to_bytes()).collect();
        assert_eq!(result, vec![b"first".as_slice(), b"", b"last"]);
    }
    #[test]
    fn line_parser_honors_capacity_and_empty_input() {
        let lines = text::owned_lines(b"a\nb\nc\0", 2);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].as_c_str(), c"b");
        assert!(text::owned_lines(b"\0", 2).is_empty());
    }
}
