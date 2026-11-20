use super::{args::Arguments, records::PresentationState, *};
use crate::graphics::{font::LoadedFont, geometry::Extent, geometry::Point, geometry::Rect};

pub(super) enum Step {
    Input(bool),
    Call(CString),
}

pub(super) fn advance(cx: &mut Cx<'_>, io: &mut Io<'_>, state: &mut PresentationState) -> Step {
    while state.oper_index < state.table.len() as u16 {
        let table = state.table.clone();
        let source = table
            .text(usize::from(state.oper_index))
            .unwrap_or(c"")
            .to_bytes();
        state.oper_index = state.oper_index.wrapping_add(1);
        let mut args = Arguments::new(source);
        let Some(opcode) = args.word(15) else {
            continue;
        };
        let opcode = opcode.to_ascii_uppercase();
        // The original advances from the entry start by the bounded opcode length plus one.
        let source =
            &source[(opcode.len() + usize::from(source.len() > opcode.len())).min(source.len())..];
        if let Some(step) = command(cx, state, &opcode, source) {
            return step;
        }
    }
    if state.ending {
        io.observer
            .checkpoint(&CheckpointView::PresentationCompleted);
    }
    Step::Input(false)
}

fn command(
    cx: &mut Cx<'_>,
    state: &mut PresentationState,
    opcode: &[u8],
    source: &[u8],
) -> Option<Step> {
    match opcode {
        b"WAIT" | b"SYNC" | b"DSYNC" | b"FTC" | b"FTB" | b"FTW" | b"MOVIE" => {
            return timed(cx, state, opcode, source)
        }
        b"RESYNC" => {
            state.start_time = get_time_counter();
            state.last_sync_time = state.start_time;
        }
        b"FONT" | b"TEXT" | b"TFI" | b"TFO" => text_command(cx, state, opcode, source),
        b"TC" => {
            if let Some(color) = parse_rgb(source) {
                state.text_color = color;
            }
        }
        b"TBC" => {
            if let Some(color) = parse_rgb(source) {
                state.text_back_color = color;
            }
        }
        b"TFC" => {
            if let Some(color) = parse_rgb(source) {
                state.text_fade_color = color;
            }
        }
        b"TVA" => state.text_vpos = source.first().copied().unwrap_or(0).to_ascii_uppercase(),
        b"TE" => state.text_effect = source.first().copied().unwrap_or(0).to_ascii_uppercase(),
        b"DIMS" | b"ANI" | b"LINE" => graphics_command(state, opcode, source),
        b"MUSIC" => {
            if state.music.is_some() {
                music::stop_requested_music();
                drop(state.music.take());
            }
            state.music = LoadedMusic::load_file(&bounded(source, 511));
            music::remember_music(
                state
                    .music
                    .as_ref()
                    .map_or(music::MusicRequest::Absent, |music| {
                        music::MusicRequest::Track(music.playback())
                    }),
                false,
                1,
                cx.world.data.game.glob_flags & 8 == 0,
            );
        }
        b"DRAW" => render::draw(cx, state, source),
        b"BATCH" => {
            if state.batched.is_none() {
                state.batched = Some(state.context.batch());
            }
        }
        b"UNBATCH" => state.unbatch(cx, false),
        b"CLS" => {
            state.unbatch(cx, true);
            state.context.clear();
        }
        b"CALL" => {
            state.unbatch(cx, true);
            return Some(Step::Call(bounded(source, 511)));
        }
        b"SAVEBG" => state.context.save_screen_region(state.clip),
        b"RESTBG" => state.context.restore_screen_region(state.clip),
        _ => (),
    }
    None
}

fn timed(
    cx: &mut Cx<'_>,
    state: &mut PresentationState,
    opcode: &[u8],
    source: &[u8],
) -> Option<Step> {
    let mut args = Arguments::new(source);
    if opcode == b"MOVIE" {
        if let (Some(fps), Some(from), Some(to)) = (args.integer(), args.integer(), args.integer())
        {
            if fps > 0 && from >= 0 && to >= from {
                state.unbatch(cx, true);
                state.movie_frame = from;
                state.movie_end_frame = to;
                state.interframe_delay = ONE_SECOND as i32 / fps;
                state.time_out = get_time_counter();
                state.time_out_on_skip = true;
                return Some(Step::Input(true));
            }
        }
        log::warn!("Bad MOVIE command '{}'", String::from_utf8_lossy(source));
        return None;
    }
    state.unbatch(cx, true);
    let value = args.integer();
    if matches!(opcode, b"FTC" | b"FTB" | b"FTW") {
        if let Some(msecs) = value {
            let kind = match opcode {
                b"FTC" => 253,
                b"FTB" => 252,
                _ => 250,
            };
            state.time_out = state
                .context
                .fade_screen(kind, (msecs * ONE_SECOND as i32 / 1000) as i16)
                .wrapping_add(ONE_SECOND / 10);
            state.time_out_on_skip = false;
        }
        return Some(Step::Input(true));
    }
    let ticks = (value? * ONE_SECOND as i32 / 1000) as u32;
    state.time_out = match opcode {
        b"WAIT" => get_time_counter().wrapping_add(ticks),
        b"SYNC" => {
            state.last_sync_time = state.start_time.wrapping_add(ticks);
            state.last_sync_time
        }
        _ => state.last_sync_time.wrapping_add(ticks),
    };
    state.time_out_on_skip = opcode == b"WAIT";
    Some(Step::Input(true))
}

fn text_command(cx: &mut Cx<'_>, state: &mut PresentationState, opcode: &[u8], source: &[u8]) {
    let mut args = Arguments::new(source);
    match opcode {
        b"FONT" => {
            let Some(index) = args.integer().filter(|&index| (0..5).contains(&index)) else {
                log::warn!("Bad FONT command '{}'", String::from_utf8_lossy(source));
                return;
            };
            let font = &mut state.fonts[index as usize];
            if let Some(path) = args.line(255) {
                drop(font.take());
                *font = LoadedFont::load_file(&bounded(path, 255));
            }
            state
                .context
                .replace_font(font.as_ref().map(LoadedFont::lease));
        }
        b"TEXT" => {
            if let (Some(x), Some(y), Some(line)) = (args.integer(), args.integer(), args.line(255))
            {
                render::text(
                    state,
                    &bounded(line, 255),
                    Point {
                        x: x as i16,
                        y: y as i16,
                    },
                    state.text_color,
                    state.text_back_color,
                );
            }
        }
        b"TFI" => {
            state.lines = text::owned_lines(bounded(source, 511).to_bytes(), 15)
                .into_iter()
                .map(|text| text::TextLine {
                    text,
                    baseline: Point { x: 0, y: 0 },
                })
                .collect();
            state.unbatch(cx, true);
            let leading = state.context.font_lease().map_or(0, |font| font.leading());
            state.fade = text::position_lines(
                &mut state.lines,
                state.clip,
                leading,
                i32::from(state.text_vpos),
                state.context.screen_dimensions().0,
            );
            render::text_lines(state, state.text_fade_color, state.text_fade_color);
            render::text_transition(state, state.text_color, state.text_back_color);
        }
        b"TFO" => {
            state.unbatch(cx, true);
            render::text_transition(state, state.text_fade_color, state.text_fade_color);
        }
        _ => unreachable!(),
    }
}

fn graphics_command(state: &mut PresentationState, opcode: &[u8], source: &[u8]) {
    let mut args = Arguments::new(source);
    match opcode {
        b"DIMS" => {
            if let (Some(width), Some(height)) = (args.integer(), args.integer()) {
                let (screen_width, screen_height) = state.context.screen_dimensions();
                state.clip = Rect {
                    corner: Point {
                        x: ((screen_width - width) / 2) as i16,
                        y: ((screen_height - height) / 2) as i16,
                    },
                    extent: Extent {
                        width: width as i16,
                        height: height as i16,
                    },
                };
                state.context.set_clip(state.clip);
            }
        }
        b"ANI" => {
            drop(state.frame.take());
            state.frame = LoadedDrawable::load_file(&bounded(source, 511));
        }
        b"LINE" => {
            if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                args.integer(),
                args.integer(),
                args.integer(),
                args.integer(),
            ) {
                state.context.draw_line(
                    state.text_color,
                    Point {
                        x: x1 as i16,
                        y: y1 as i16,
                    },
                    Point {
                        x: x2 as i16,
                        y: y2 as i16,
                    },
                );
            } else {
                log::warn!("Bad LINE command '{}'", String::from_utf8_lossy(source));
            }
        }
        _ => unreachable!(),
    }
}
