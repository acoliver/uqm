use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut desc = CommDescriptor {
        alien_frame_res: Some(c"comm.ilwrath.graphics"),
        alien_font_res: Some(c"comm.ilwrath.font"),
        alien_colormap_res: Some(c"comm.ilwrath.colortable"),
        alien_song_res: Some(c"comm.ilwrath.music"),
        conversation_phrases_res: Some(c"comm.ilwrath.dialogue"),
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
        alien_text_baseline: Point { x: 164, y: 70 },
        alien_text_width: 288,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 4,
        ..CommDescriptor::default()
    };
    desc.alien_ambient_array[0] = animation(6, 5, 2, (42, 0), (840, 2520));
    desc.alien_ambient_array[1] = animation(11, 5, 2, (42, 0), (840, 2520));
    desc.alien_ambient_array[2] = animation(16, 5, 2, (42, 0), (840, 2520));
    desc.alien_ambient_array[3] = animation(21, 6, 2, (42, 0), (840, 2520));
    desc.alien_talk_desc = animation(1, 5, 0, (56, 0), (70, 0));
    desc
}
