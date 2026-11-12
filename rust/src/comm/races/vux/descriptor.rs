use super::text_layout;
use crate::comm::races::descriptor::{Color, CommDescriptor};
use crate::comm::races::script::animation;

pub(crate) fn descriptor(screen_width: i32) -> CommDescriptor {
    let (baseline, width) = text_layout(screen_width);
    let mut desc = CommDescriptor {
        alien_frame_res: Some(c"comm.vux.graphics"),
        alien_font_res: Some(c"comm.vux.font"),
        alien_colormap_res: Some(c"comm.vux.colortable"),
        alien_song_res: Some(c"comm.vux.music"),
        conversation_phrases_res: Some(c"comm.vux.dialogue"),
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
        num_animations: 17,
        ..CommDescriptor::default()
    };
    let specs = [
        (12, 3, 1 << 0, (28, 28), (28, 28), 0),
        (15, 5, 1 << 0, (28, 28), (28, 28), 0),
        (20, 14, 1 << 1, (28, 0), (28, 0), 0),
        (34, 7, 1 << 1, (28, 28), (28, 28), 0),
        (41, 6, 1 << 0, (28, 28), (28, 28), 0),
        (47, 11, 1 << 1, (28, 28), (28, 28), 0),
        (58, 3, 1 << 0, (28, 28), (28, 28), 0),
        (61, 4, 1 << 0, (28, 28), (28, 28), 0),
        (65, 4, 1 << 0, (28, 28), (28, 28), 0),
        (69, 2, 1 << 0, (28, 28), (28, 28), 0),
        (71, 3, 1 << 2, (42, 0), (840, 2520), 0),
        (74, 6, 1 << 2, (56, 56), (56, 56), 0),
        (80, 5, 1 << 2, (56, 56), (56, 56), 1 << 14),
        (85, 5, 1 << 2, (56, 56), (56, 56), 0),
        (90, 5, 1 << 2, (56, 56), (56, 56), 1 << 12),
        (95, 4, 1 << 2, (56, 56), (4200, 4200), 0),
        (99, 4, 1 << 2, (56, 56), (4200, 4200), 0),
    ];
    for (index, (start, frames, flags, frame_rate, restart_rate, block_mask)) in
        specs.into_iter().enumerate()
    {
        desc.alien_ambient_array[index] = animation(start, frames, flags, frame_rate, restart_rate);
        desc.alien_ambient_array[index].block_mask = block_mask;
    }
    desc.alien_talk_desc = animation(1, 11, 0, (56, 0), (70, 0));
    desc
}
