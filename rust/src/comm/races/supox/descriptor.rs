use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.supox.graphics"),
        alien_font_res: Some(c"comm.supox.font"),
        alien_colormap_res: Some(c"comm.supox.colortable"),
        alien_song_res: Some(c"comm.supox.music"),
        conversation_phrases_res: Some(c"comm.supox.dialogue"),
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
        alien_text_baseline: Point { x: 121, y: 0 },
        alien_text_width: 224,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 4,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(4, 5, 12, (84, 84), (840, 2520));
    d.alien_ambient_array[1] = animation(9, 10, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[2] = animation(19, 10, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[3] = animation(29, 13, 2, (56, 0), (840, 2520));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 3, 0, (56, 0), (70, 0));
    d
}
