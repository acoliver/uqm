use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut desc = CommDescriptor {
        alien_frame_res: Some(c"comm.spathi.graphics"),
        alien_font_res: Some(c"comm.spathi.font"),
        alien_colormap_res: Some(c"comm.spathi.colortable"),
        alien_song_res: Some(c"comm.spathi.music"),
        conversation_phrases_res: Some(c"comm.spathi.dialogue"),
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
        alien_text_baseline: Point { x: 164, y: 0 },
        alien_text_width: 288,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 8,
        ..CommDescriptor::default()
    };
    desc.alien_ambient_array[0] = animation(1, 6, 2, (42, 0), (0, 840));
    desc.alien_ambient_array[0].block_mask = 2;
    desc.alien_ambient_array[1] = animation(7, 9, 2, (42, 0), (42, 0));
    desc.alien_ambient_array[1].block_mask = 1;
    desc.alien_ambient_array[2] = animation(16, 4, 2, (84, 56), (84, 56));
    desc.alien_ambient_array[2].block_mask = 16;
    desc.alien_ambient_array[3] = animation(20, 4, 4, (56, 56), (56, 56));
    desc.alien_ambient_array[3].block_mask = 32;
    desc.alien_ambient_array[4] = animation(24, 5, 4, (56, 0), (840, 2520));
    desc.alien_ambient_array[4].block_mask = 4;
    desc.alien_ambient_array[5] = animation(34, 4, 4, (56, 0), (840, 2520));
    desc.alien_ambient_array[5].block_mask = 8;
    desc.alien_ambient_array[6] = animation(38, 3, 4, (56, 0), (840, 2520));
    desc.alien_ambient_array[7] = animation(41, 3, 4, (56, 0), (840, 2520));
    desc.alien_talk_desc = animation(0, 0, 0, (0, 0), (0, 0));
    desc
}
