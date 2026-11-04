//! Resource records retained for one synchronous presentation session.

use super::*;

pub(super) struct PresentationState {
    pub(super) start_time: TimeCount,
    pub(super) last_sync_time: TimeCount,
    pub(super) time_out: TimeCount,
    pub(super) time_out_on_skip: c_int,
    pub(super) slide_show: STRING,
    pub(super) fonts: [FONT; MAX_FONTS],
    pub(super) frame: FRAME,
    pub(super) music_ref: MUSIC_REF,
    pub(super) batched: BOOLEAN,
    pub(super) sis_frame: FRAME,
    pub(super) rotated_frame: FRAME,
    pub(super) last_draw_kind: c_int,
    pub(super) ending: bool,
    pub(super) last_angle: c_int,
    pub(super) oper_index: COUNT,
    pub(super) text_fade_color: Color,
    pub(super) text_color: Color,
    pub(super) text_back_color: Color,
    pub(super) text_vpos: c_int,
    pub(super) text_effect: c_int,
    pub(super) clip_r: RECT,
    pub(super) tfade_r: RECT,
    pub(super) text_lines: [TEXT; MAX_TEXT_LINES],
    pub(super) lines_count: COUNT,
    pub(super) buffer: [c_char; 512],
    pub(super) movie_frame: c_int,
    pub(super) movie_end_frame: c_int,
    pub(super) interframe_delay: c_int,
}
