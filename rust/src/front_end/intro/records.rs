use super::*;
use crate::graphics::{context_abi::BatchGuard, native_font::LoadedFont};

pub(super) struct PresentationState {
    pub table: LoadedTextTable,
    pub context: ContextLease,
    pub fonts: [Option<LoadedFont>; 5],
    pub frame: Option<LoadedDrawable>,
    pub music: Option<LoadedMusic>,
    pub batched: Option<BatchGuard>,
    pub sis: Option<LoadedDrawable>,
    pub rotated: Option<LoadedDrawable>,
    pub time_out: u32,
    pub time_out_on_skip: bool,
    pub start_time: u32,
    pub last_sync_time: u32,
    pub ending: bool,
    pub oper_index: u16,
    pub movie_frame: i32,
    pub movie_end_frame: i32,
    pub interframe_delay: i32,
    pub last_angle: i32,
    pub last_draw_kind: i32,
    pub text_color: Color,
    pub text_back_color: Color,
    pub text_fade_color: Color,
    pub text_effect: u8,
    pub text_vpos: u8,
    pub clip: RECT,
    pub fade: RECT,
    pub lines: Vec<text::TextLine>,
}

impl PresentationState {
    pub fn new(table: LoadedTextTable, context: ContextLease, ending: bool) -> Self {
        let now = GetTimeCounter();
        Self {
            table,
            context,
            fonts: std::array::from_fn(|_| None),
            frame: None,
            music: None,
            batched: None,
            sis: None,
            rotated: None,
            time_out: 0,
            time_out_on_skip: false,
            start_time: now,
            last_sync_time: now,
            ending,
            oper_index: 0,
            movie_frame: -1,
            movie_end_frame: 0,
            interframe_delay: 0,
            last_angle: 0,
            last_draw_kind: -1,
            text_color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            text_back_color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            text_fade_color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            text_effect: 0,
            text_vpos: b'B',
            clip: RECT::default(),
            fade: RECT::default(),
            lines: Vec::new(),
        }
    }

    pub fn unbatch(&mut self, cx: &mut Cx<'_>, yield_task: bool) {
        if let Some(batch) = self.batched.take() {
            drop(batch);
            if yield_task {
                cx.kernel.input_runtime().yield_task(cx);
            }
        }
    }

    pub fn movie_frame(&self) {
        let frame = self.frame.as_ref().map_or_else(
            || ContextFrame::retain(None),
            |image| image.lease().context_frame(0),
        );
        self.context.draw_stamp(
            &frame.wrapped_frame(self.movie_frame as u16),
            POINT { x: 0, y: 0 },
        );
    }
}
