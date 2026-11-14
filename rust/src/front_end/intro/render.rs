use super::{args::Arguments, records::PresentationState, *};
use crate::graphics::native_canvas::{GSCALE_IDENTITY, TFB_SCALE_BILINEAR, TFB_SCALE_NEAREST};
use crate::starbase::ids::{DRIVE_TOP, JET_TOP, MODULE_TOP, NUM_MODULES, SHIP_PIECE_OFFSET};

pub(super) fn text(
    state: &PresentationState,
    string: &CStr,
    baseline: POINT,
    fore: Color,
    back: Color,
) {
    let Some(font) = state.context.font_lease() else {
        return;
    };
    if state.text_effect == b'T' {
        state
            .context
            .draw_traced_text(&font, string, baseline, 1, fore, back);
    } else {
        state.context.draw_text(&font, string, baseline, 1, fore);
    }
}

pub(super) fn text_lines(state: &PresentationState, fore: Color, back: Color) {
    for line in &state.lines {
        text(state, &line.text, line.baseline, fore, back);
    }
}

pub(super) fn text_transition(state: &PresentationState, fore: Color, back: Color) {
    state.context.transition_source(Some(state.fade));
    let batch = state.context.batch();
    text_lines(state, fore, back);
    state.context.transition(Some(state.fade));
    drop(batch);
}

pub(super) fn generate_sis(cx: &Cx<'_>) -> Option<LoadedDrawable> {
    let context = ContextLease::require(cx.kernel.off_screen_context);
    let skeleton = LoadedDrawable::load(c"graphics.flagshipskeleton")?;
    let modules = LoadedDrawable::load(c"graphics.outfitmodules")?;
    let skeleton_frame = skeleton.lease().context_frame(0);
    let bounds = skeleton_frame.rect();
    let sis = LoadedDrawable::create(2, bounds.extent.width, bounds.extent.height, 1)?;
    let frame = sis.lease().context_frame(0);
    context.with_selected(|| {
        context.set_frame(frame.clone());
        context.clear_clip();
        let background = crate::ui::rgb15(1, 1, 1);
        context.set_background(background);
        context.clear();
        frame.transparent_color(background);
        context.draw_stamp(&skeleton_frame, POINT { x: 0, y: 0 });
        let flagship = &cx.world.data.flagship;
        for (pieces, top) in [
            (flagship.drive_slots.as_slice(), DRIVE_TOP),
            (flagship.jet_slots.as_slice(), JET_TOP),
            (flagship.module_slots.as_slice(), MODULE_TOP),
        ] {
            for (slot, &piece) in pieces.iter().enumerate() {
                if u16::from(piece) < NUM_MODULES {
                    context.draw_stamp(
                        &modules.lease().context_frame(usize::from(piece)),
                        POINT {
                            x: top.x + slot as i16 * SHIP_PIECE_OFFSET,
                            y: top.y - 79,
                        },
                    );
                }
            }
        }
        drop(skeleton);
        drop(modules);
        frame.replace_hotspot(POINT {
            x: bounds.extent.width / 2,
            y: bounds.extent.height / 2,
        });
    });
    context.flush();
    Some(sis)
}

pub(super) fn draw(cx: &Cx<'_>, state: &mut PresentationState, source: &[u8]) {
    let mut words = Arguments::new(source);
    let sis = words
        .word(15)
        .is_some_and(|word| word.eq_ignore_ascii_case(b"SIS"));
    let mut args = Arguments::new(source);
    let mut values = [0; 5];
    let mut count = 0;
    if sis {
        args.word(usize::MAX);
        count = 1;
    }
    while count < values.len() {
        let Some(value) = args.integer() else {
            break;
        };
        values[count] = value;
        count += 1;
    }
    if count < 1 {
        log::warn!("Bad DRAW command '{}'", String::from_utf8_lossy(source));
        return;
    }
    if count < 5 {
        values[4] = 0;
    }
    if count < 4 {
        values[3] = GSCALE_IDENTITY;
    }
    if count < 3 {
        values[1] = 0;
        values[2] = 0;
    }
    let [index, x, y, scale, angle] = values;
    let kind = i32::from(sis);
    let source = if sis {
        if state.sis.is_none() {
            state.sis = generate_sis(cx);
        }
        &state.sis
    } else {
        &state.frame
    };
    let mut frame = source.as_ref().map_or_else(
        || ContextFrame::retain(None),
        |image| {
            image
                .lease()
                .context_frame(0)
                .wrapped_frame(if sis { 0 } else { index as u16 })
        },
    );
    if angle != 0 {
        if angle != state.last_angle || kind != state.last_draw_kind {
            drop(state.rotated.take());
            state.rotated = LoadedDrawable::rotated(&frame, -angle);
            state.last_angle = angle;
            state.last_draw_kind = kind;
        }
        frame = state.rotated.as_ref().map_or_else(
            || ContextFrame::retain(None),
            |image| image.lease().context_frame(0),
        );
    }
    state.context.draw_scaled_stamp(
        &frame,
        POINT {
            x: x as i16,
            y: y as i16,
        },
        scale,
        if sis {
            TFB_SCALE_NEAREST
        } else {
            TFB_SCALE_BILINEAR
        },
    );
}
