use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.umgah.graphics"),
        alien_font_res: Some(c"comm.umgah.font"),
        alien_colormap_res: Some(c"comm.umgah.colortable"),
        alien_song_res: Some(c"comm.umgah.music"),
        conversation_phrases_res: Some(c"comm.umgah.dialogue"),
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
        num_animations: 16,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(5, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[0].block_mask = 32;
    d.alien_ambient_array[1] = animation(8, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[1].block_mask = 64;
    d.alien_ambient_array[2] = animation(11, 2, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[2].block_mask = 128;
    d.alien_ambient_array[3] = animation(13, 2, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[3].block_mask = 256;
    d.alien_ambient_array[4] = animation(15, 2, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[4].block_mask = 512;
    d.alien_ambient_array[5] = animation(17, 3, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[5].block_mask = 1;
    d.alien_ambient_array[6] = animation(20, 3, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[6].block_mask = 2;
    d.alien_ambient_array[7] = animation(23, 2, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[7].block_mask = 4;
    d.alien_ambient_array[8] = animation(25, 2, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[8].block_mask = 8;
    d.alien_ambient_array[9] = animation(27, 2, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[9].block_mask = 16;
    d.alien_ambient_array[10] = animation(29, 3, 4, (84, 0), (840, 2520));
    d.alien_ambient_array[11] = animation(32, 3, 4, (84, 0), (840, 2520));
    d.alien_ambient_array[12] = animation(35, 5, 4, (84, 0), (840, 2520));
    d.alien_ambient_array[13] = animation(40, 6, 4, (84, 0), (84, 0));
    d.alien_ambient_array[14] = animation(46, 2, 2, (168, 0), (840, 2520));
    d.alien_ambient_array[14].block_mask = 32768;
    d.alien_ambient_array[15] = animation(48, 2, 2, (168, 0), (840, 2520));
    d.alien_ambient_array[15].block_mask = 16384;
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 4, 0, (56, 0), (70, 0));
    d
}
