use super::text_layout;
use crate::comm::races::descriptor::{Color, CommDescriptor};
use crate::comm::races::script::animation;

pub(crate) fn descriptor(screen_width: i32) -> CommDescriptor {
    let (baseline, width) = text_layout(screen_width);
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.shofixti.graphics"),
        alien_font_res: Some(c"comm.shofixti.font"),
        alien_colormap_res: Some(c"comm.shofixti.colortable"),
        alien_song_res: Some(c"comm.shofixti.music"),
        conversation_phrases_res: Some(c"comm.shofixti.dialogue"),
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
        num_animations: 11,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(5, 15, 2, (28, 0), (28, 0));
    d.alien_ambient_array[1] = animation(20, 3, 1, (84, 0), (420, 1260));
    d.alien_ambient_array[2] = animation(23, 3, 1, (84, 0), (420, 1260));
    d.alien_ambient_array[3] = animation(26, 3, 4, (28, 0), (840, 2520));
    d.alien_ambient_array[4] = animation(29, 4, 4, (56, 0), (840, 2520));
    d.alien_ambient_array[5] = animation(33, 6, 2, (42, 28), (42, 28));
    d.alien_ambient_array[6] = animation(39, 7, 2, (42, 28), (42, 28));
    d.alien_ambient_array[6].block_mask = 1 << 7;
    d.alien_ambient_array[7] = animation(46, 6, 2, (42, 28), (42, 28));
    d.alien_ambient_array[7].block_mask = 1 << 6;
    d.alien_ambient_array[8] = animation(52, 4, 2, (42, 28), (42, 28));
    d.alien_ambient_array[9] = animation(56, 7, 2, (42, 28), (42, 28));
    d.alien_ambient_array[9].block_mask = 1 << 10;
    d.alien_ambient_array[10] = animation(63, 6, 2, (42, 28), (42, 28));
    d.alien_ambient_array[10].block_mask = 1 << 9;
    d.alien_talk_desc = animation(1, 4, 0, (42, 0), (56, 0));
    d
}
