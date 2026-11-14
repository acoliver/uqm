use super::text_layout;
use crate::comm::races::descriptor::{Color, CommDescriptor};
use crate::comm::races::script::animation;

pub(crate) fn descriptor(screen_width: i32) -> CommDescriptor {
    let (baseline, width) = text_layout(screen_width);
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.kohrah.graphics"),
        alien_font_res: Some(c"comm.kohrah.font"),
        alien_colormap_res: Some(c"comm.kohrah.colortable"),
        alien_song_res: Some(c"comm.kohrah.music"),
        conversation_phrases_res: Some(c"comm.kohrah.dialogue"),
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
        num_animations: 8,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(7, 6, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[1] = animation(13, 7, 2, (56, 0), (840, 2520));
    for (i, s) in [20, 23, 26].into_iter().enumerate() {
        d.alien_ambient_array[i + 2] = animation(s, 3, 4, (56, 0), (840, 2520));
    }
    d.alien_ambient_array[5] = animation(29, 4, 1, (84, 0), (84, 0));
    d.alien_ambient_array[6] = animation(33, 5, 0x0a, (84, 0), (840, 2520));
    d.alien_ambient_array[7] = animation(38, 4, 2, (84, 0), (840, 2520));
    d.alien_transition_desc = animation(1, 2, 0, (140, 0), (0, 0));
    d.alien_talk_desc = animation(2, 5, 0, (56, 0), (70, 0));
    d
}
