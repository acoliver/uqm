use super::text_layout;
use crate::comm::races::descriptor::{Color, CommDescriptor};
use crate::comm::races::script::animation;

pub(crate) fn descriptor(screen_width: i32) -> CommDescriptor {
    let (baseline, width) = text_layout(screen_width);
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.urquan.graphics"),
        alien_font_res: Some(c"comm.urquan.font"),
        alien_colormap_res: Some(c"comm.urquan.colortable"),
        alien_song_res: Some(c"comm.urquan.music"),
        conversation_phrases_res: Some(c"comm.urquan.dialogue"),
        alien_text_fcolor: Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        alien_text_bcolor: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_baseline: baseline,
        alien_text_width: width,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 7,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(7, 6, 1 << 1, (84, 0), (84, 0));
    for (index, start) in [13, 16, 19].into_iter().enumerate() {
        d.alien_ambient_array[index + 1] = animation(start, 3, 1 << 2, (42, 0), (840, 2520));
    }
    for (index, (start, frames)) in [(22, 7), (29, 7), (36, 8)].into_iter().enumerate() {
        d.alien_ambient_array[index + 4] = animation(start, frames, 1 << 1, (56, 0), (840, 2520));
    }
    d.alien_transition_desc = animation(1, 2, 0, (140, 0), (0, 0));
    d.alien_talk_desc = animation(2, 5, 0, (56, 0), (70, 0));
    d
}
