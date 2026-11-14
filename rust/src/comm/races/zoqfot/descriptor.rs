use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.zoqfotpik.graphics"),
        alien_font_res: Some(c"comm.zoqfotpik.font"),
        alien_colormap_res: Some(c"comm.zoqfotpik.colortable"),
        alien_song_res: Some(c"comm.zoqfotpik.music"),
        conversation_phrases_res: Some(c"comm.zoqfotpik.dialogue"),
        alien_text_fcolor: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_bcolor: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_baseline: Point { x: 0, y: 0 },
        alien_text_width: 119,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 3,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(1, 4, 12, (35, 0), (0, 8400));
    d.alien_ambient_array[1] = animation(5, 5, 2, (49, 0), (1680, 0));
    d.alien_ambient_array[2] = animation(10, 8, 2, (56, 0), (0, 8400));
    d.alien_transition_desc = animation(23, 3, 0, (28, 0), (0, 0));
    d.alien_talk_desc = animation(18, 5, 0, (56, 0), (70, 0));
    d
}
