//! Retained graphics-context inspection with supplied input and diagnostic output.
use super::*;
use crate::graphics::{
    context_abi::{ContextFrame, LoadedContext},
    drawable_abi::LoadedDrawable,
    native_abi::{POINT, RECT, TEXT_ALIGN_ALIGN_CENTER},
};
use std::io::{self, Write};

struct ScreenRestore {
    context: ContextLease,
    frame: ContextFrame,
}
impl Drop for ScreenRestore {
    fn drop(&mut self) {
        self.context.draw_stamp(&self.frame, POINT { x: 0, y: 0 });
    }
}

pub fn debug_contexts(cx: &mut Cx<'_>, io: &mut Io<'_>, out: &mut impl Write) -> io::Result<()> {
    let Some(_active) = cx.kernel.diagnostics.enter_contexts() else {
        return Ok(());
    };
    let screen = ContextLease::require(cx.kernel.screen_context);
    screen.with_selected(|| {
        let contexts = screen.diagnostic_contexts();
        let target = ContextFrame::retain(std::ptr::NonNull::new(cx.kernel.screen));
        let visible = contexts
            .iter()
            .filter(|context| context.with_selected(|| context.target().same_frame(&target)))
            .count();
        if visible == 0 {
            return Ok(());
        }
        let dimensions = screen.screen_dimensions();
        let saved = LoadedDrawable::capture(
            &screen,
            Some(RECT {
                corner: POINT { x: 0, y: 0 },
                extent: crate::graphics::native_abi::EXTENT {
                    width: dimensions.0 as i16,
                    height: dimensions.1 as i16,
                },
            }),
        )
        .unwrap_or_else(|| panic!("debug screen capture failed"));
        screen.flush();
        let owner = LoadedContext::create(c"debugDrawContext");
        let context = owner.lease();
        let saved_frame = saved.lease().context_frame(0);
        context.set_frame(saved_frame.clone());
        let clone = LoadedDrawable::capture(&context, None)
            .unwrap_or_else(|| panic!("debug overlay capture failed"));
        context.set_frame(clone.lease().context_frame(0));
        let font = cx
            .kernel
            .tiny_font_lease()
            .unwrap_or_else(|| panic!("debug contexts require the loaded tiny font"));
        let increment = 360.0 / visible as f64;
        let mut count = 0;
        for source in contexts {
            if source.with_selected(|| source.target().same_frame(&target)) {
                draw_context(&context, &source, &font, count as f64 * increment);
                count += 1;
            }
            describe_context(out, &source)?;
        }
        let restore = ScreenRestore {
            context: screen.clone(),
            frame: saved_frame,
        };
        screen.draw_stamp(&clone.lease().context_frame(0), POINT { x: 0, y: 0 });
        let runtime = cx.kernel.input_runtime();
        context.select();
        runtime.run(cx, io, true, &mut |cx: &mut Cx<'_>, _: &mut Io<'_>| {
            let input = runtime.input().pulsed();
            if input.menu[KEY_MENU_SELECT as usize] != 0
                || input.menu[KEY_MENU_CANCEL as usize] != 0
            {
                return false;
            }
            runtime.sleep(cx, ONE_SECOND / 20);
            true
        });
        drop(owner);
        drop(clone);
        drop(restore);
        Ok(())
    })
}

fn draw_context(
    context: &ContextLease,
    source: &ContextLease,
    font: &crate::graphics::native_font::FontLease,
    hue: f64,
) {
    let rectangle = colors::hsva_to_rgba(hue, 1.0, 0.5, 100);
    let line = colors::hsva_to_rgba(hue, 1.0, 1.0, 90);
    let rect = source.with_selected(|| source.clip_rect());
    let p1 = rect.corner;
    let p2 = POINT {
        x: (i32::from(p1.x) + i32::from(rect.extent.width) - 1) as i16,
        y: p1.y,
    };
    let p3 = POINT {
        x: p1.x,
        y: (i32::from(p1.y) + i32::from(rect.extent.height) - 1) as i16,
    };
    let p4 = POINT { x: p2.x, y: p3.y };
    context.with_text_state(|| {
        context.fill_rectangle(rectangle, rect);
        for (first, second) in [(p1, p2), (p2, p4), (p1, p3), (p3, p4), (p1, p4), (p2, p3)] {
            context.draw_line(line, first, second);
        }
        let name = source.diagnostic_name();
        context.draw_text_alpha(
            font,
            &name,
            POINT {
                x: ((i32::from(p1.x) + i32::from(p2.x) + 1) / 2) as i16,
                y: (i32::from(p1.y) + 8) as i16,
            },
            TEXT_ALIGN_ALIGN_CENTER,
            line,
        );
    });
}

fn describe_context(out: &mut impl Write, source: &ContextLease) -> io::Result<()> {
    let rect = source.with_selected(|| source.clip_rect());
    let name = source.diagnostic_name();
    out.write_all(b"Context '")?;
    out.write_all(name.as_bytes())?;
    writeln!(
        out,
        "':\n\tClipRect = ({}, {})-({}, {})  ({} x {})",
        rect.corner.x,
        rect.corner.y,
        i32::from(rect.corner.x) + i32::from(rect.extent.width),
        i32::from(rect.corner.y) + i32::from(rect.extent.height),
        rect.extent.width,
        rect.extent.height
    )
}
