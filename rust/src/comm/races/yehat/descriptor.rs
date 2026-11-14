use super::text_layout;
use crate::comm::races::descriptor::{Color, CommDescriptor};
use crate::comm::races::script::animation;

pub(crate) fn descriptor(screen_width: i32) -> CommDescriptor {
    let (baseline, width) = text_layout(screen_width);
    let mut desc = CommDescriptor {
        alien_frame_res: Some(c"comm.yehat.graphics"),
        alien_font_res: Some(c"comm.yehat.font"),
        alien_colormap_res: Some(c"comm.yehat.colortable"),
        alien_song_res: Some(c"comm.yehat.music"),
        conversation_phrases_res: Some(c"comm.yehat.dialogue"),
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
        alien_text_valign: 1,
        num_animations: 15,
        ..CommDescriptor::default()
    };
    let specs = [
        (
            4,
            3,
            (1 << 2) | (1 << 3),
            (84, 84),
            (210, 420),
            (1 << 6) | (1 << 7),
        ),
        (
            7,
            3,
            (1 << 2) | (1 << 3),
            (84, 84),
            (210, 420),
            (1 << 6) | (1 << 7),
        ),
        (10, 3, 1 << 2, (42, 0), (840, 2520), (1 << 4) | (1 << 14)),
        (13, 3, 1 << 2, (42, 0), (840, 2520), 1 << 5),
        (16, 5, 1 << 2, (56, 56), (5040, 2520), (1 << 2) | (1 << 14)),
        (21, 5, 1 << 2, (56, 56), (5040, 2520), 1 << 3),
        (
            26,
            2,
            (1 << 2) | (1 << 3),
            (56, 56),
            (5040, 2520),
            (1 << 0) | (1 << 1),
        ),
        (
            28,
            2,
            (1 << 2) | (1 << 3),
            (56, 56),
            (5040, 2520),
            (1 << 0) | (1 << 1),
        ),
        (30, 3, 1 << 2, (28, 28), (28, 28), 0),
        (33, 3, 1 << 2, (28, 28), (28, 28), 0),
        (36, 3, 1 << 2, (28, 28), (28, 28), 0),
        (39, 3, 1 << 2, (28, 28), (28, 28), 0),
        (42, 3, 1 << 2, (28, 28), (28, 28), 0),
        (45, 3, 1 << 2, (28, 28), (28, 28), 0),
        (
            48,
            4,
            (1 << 2) | (1 << 3),
            (28, 0),
            (840, 2520),
            (1 << 2) | (1 << 4),
        ),
    ];
    for (index, (start, frames, flags, frame_rate, restart_rate, block_mask)) in
        specs.into_iter().enumerate()
    {
        desc.alien_ambient_array[index] = animation(start, frames, flags, frame_rate, restart_rate);
        desc.alien_ambient_array[index].block_mask = block_mask;
    }
    desc.alien_talk_desc = animation(1, 3, 0, (56, 0), (70, 0));
    desc
}
