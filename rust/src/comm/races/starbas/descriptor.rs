use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut desc = CommDescriptor {
        alien_frame_res: Some(c"comm.commander.graphics"),
        alien_font_res: Some(c"comm.commander.font"),
        alien_colormap_res: Some(c"comm.commander.colortable"),
        alien_song_res: Some(c"comm.commander.music"),
        alien_alt_song_res: Some(c"comm.starbase.music"),
        alien_song_flags: 1,
        conversation_phrases_res: Some(c"comm.starbase.dialogue"),
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
        alien_text_baseline: Point { x: 164, y: 20 },
        alien_text_width: 143,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 10,
        ..CommDescriptor::default()
    };
    desc.alien_ambient_array[0] = animation(1, 3, 4, (56, 0), (0, 6720));
    desc.alien_ambient_array[1] = animation(10, 30, 2, (21, 0), (1680, 0));
    desc.alien_ambient_array[2] = animation(40, 7, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[3] = animation(47, 8, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[4] = animation(55, 6, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[5] = animation(61, 6, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[6] = animation(67, 7, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[7] = animation(74, 11, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[8] = animation(85, 10, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[9] = animation(95, 1, 0, (0, 0), (0, 0));
    desc.alien_talk_desc = animation(4, 6, 0, (84, 56), (98, 70));
    desc
}
