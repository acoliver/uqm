use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.druuge.graphics"),
        alien_font_res: Some(c"comm.druuge.font"),
        alien_colormap_res: Some(c"comm.druuge.colortable"),
        alien_song_res: Some(c"comm.druuge.music"),
        conversation_phrases_res: Some(c"comm.druuge.dialogue"),
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
        alien_text_baseline: Point { x: 121, y: 70 },
        alien_text_width: 224,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 11,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(5, 4, 2, (63, 0), (63, 0));
    d.alien_ambient_array[1] = animation(9, 4, 2, (63, 0), (63, 0));
    d.alien_ambient_array[2] = animation(13, 6, 2, (63, 0), (63, 0));
    d.alien_ambient_array[3] = animation(19, 3, 2, (42, 0), (840, 2520));
    d.alien_ambient_array[4] = animation(22, 3, 1, (70, 0), (840, 2520));
    d.alien_ambient_array[5] = animation(25, 3, 1, (70, 0), (840, 2520));
    d.alien_ambient_array[6] = animation(28, 3, 1, (70, 0), (840, 2520));
    d.alien_ambient_array[7] = animation(31, 2, 2, (70, 0), (840, 2520));
    d.alien_ambient_array[8] = animation(33, 7, 10, (70, 0), (5880, 2520));
    d.alien_ambient_array[9] = animation(40, 4, 2, (252, 0), (252, 0));
    d.alien_ambient_array[10] = animation(44, 4, 2, (168, 0), (168, 0));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 4, 0, (56, 56), (70, 70));
    d
}
