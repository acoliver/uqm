//! Save failure popup, preserving save.c's measurement and screen restoration.
use crate::engine::{battle::Cx, observe::Io};
use crate::graphics::{
    context_abi::ContextLease,
    drawable_abi::LoadedDrawable,
    native_abi::{Color, POINT, RECT, TEXT_ALIGN_ALIGN_CENTER},
    native_font::FontLease,
};
use crate::ui::util::{box_rectangles, live::wait_for_any_button};

fn color(v: u8) -> Color {
    super::rgb15(v, v, v)
}

pub fn save_problem(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let context = ContextLease::retain(
        std::ptr::NonNull::new(cx.kernel.space_context.cast())
            .unwrap_or_else(|| panic!("save error context not admitted")),
    );
    let font = cx
        .kernel
        .battle_font()
        .unwrap_or_else(|| panic!("save error font not admitted"));
    let text = cx.kernel.game_text(454 + 2).to_owned();
    render_save_problem(cx, io, &context, &font, &text);
}

pub fn render_save_problem(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    context: &ContextLease,
    font: &FontLease,
    text: &std::ffi::CStr,
) {
    context.with_selected(|| {
        context.set_font(font);
        let mut rect = if text.is_empty() {
            RECT {
                corner: POINT { x: 0, y: 0 },
                extent: crate::graphics::native_abi::EXTENT {
                    width: 0,
                    height: 0,
                },
            }
        } else {
            font.measure(
                text,
                POINT { x: 0, y: 0 },
                TEXT_ALIGN_ALIGN_CENTER,
                u16::MAX,
            )
            .rect
        };
        let size = context.target().rect().extent;
        let x =
            (((i32::from(size.width) - 78) >> 1) as i16 - (rect.extent.width >> 1)) - rect.corner.x;
        let y = (((i32::from(size.height) - 13) >> 1) as i16 - (rect.extent.height >> 1))
            - rect.corner.y;
        rect.corner.x += x - 4;
        rect.corner.y += y - 4;
        rect.extent.width += 8;
        rect.extent.height += 8;
        let saved = LoadedDrawable::capture(context, Some(rect))
            .unwrap_or_else(|| panic!("save error background capture failed"));
        context.draw_batch(|| {
            box_rectangles(
                rect,
                2,
                color(16),
                color(8),
                Some(color(10)),
                |color, rect| context.fill_rectangle(color, rect),
            );
            context.set_foreground(color(20));
            if !text.is_empty() {
                context.draw_text(
                    font,
                    text,
                    POINT { x, y },
                    TEXT_ALIGN_ALIGN_CENTER,
                    color(20),
                );
            }
        });
        context.flush();
        wait_for_any_button(cx, io, 1, u32::MAX, 0);
        context.draw_stamp(&saved.lease().context_frame(0), rect.corner);
    });
}

pub struct SaveBackground {
    context: ContextLease,
    drawable: LoadedDrawable,
    origin: POINT,
}

impl SaveBackground {
    pub fn restore(self) {
        self.context.with_selected(|| {
            self.context
                .draw_stamp(&self.drawable.lease().context_frame(0), self.origin);
        });
    }
}

pub fn save_confirmation(
    kernel: &crate::engine::Kernel,
    context: &ContextLease,
    saving: bool,
) -> Option<SaveBackground> {
    let font = FontLease::retain(
        std::ptr::NonNull::new(kernel.star_con_font.cast())
            .unwrap_or_else(|| panic!("save confirmation font")),
    );
    render_save_confirmation(
        context,
        &font,
        kernel.game_text(454 + usize::from(!saving)),
        saving,
    )
}

pub fn render_save_confirmation(
    context: &ContextLease,
    font: &FontLease,
    text: &std::ffi::CStr,
    saving: bool,
) -> Option<SaveBackground> {
    context.with_selected(|| {
        context.set_font(font);
        let clip = context.clip_rect();
        let baseline = POINT {
            x: clip.extent.width >> 1,
            y: (clip.extent.height >> 1) + 3,
        };
        let mut rect = font
            .measure(text, baseline, TEXT_ALIGN_ALIGN_CENTER, u16::MAX)
            .rect;
        rect.corner.x -= 4;
        rect.corner.y -= 4;
        rect.extent.width += 8;
        rect.extent.height += 8;
        let saved = saving.then(|| SaveBackground {
            context: context.clone(),
            drawable: LoadedDrawable::capture(context, Some(rect))
                .unwrap_or_else(|| panic!("save confirmation background capture failed")),
            origin: rect.corner,
        });
        box_rectangles(
            rect,
            2,
            color(16),
            color(8),
            Some(color(10)),
            |color, rect| context.fill_rectangle(color, rect),
        );
        context.set_foreground(color(20));
        context.draw_text(font, text, baseline, TEXT_ALIGN_ALIGN_CENTER, color(20));
        saved
    })
}
