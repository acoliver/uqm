use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.syreen.graphics"),
        alien_font_res: Some(c"comm.syreen.font"),
        alien_colormap_res: Some(c"comm.syreen.colortable"),
        alien_song_res: Some(c"comm.syreen.music"),
        conversation_phrases_res: Some(c"comm.syreen.dialogue"),
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
        num_animations: 15,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(5, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[1] = animation(7, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[2] = animation(9, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(11, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[4] = animation(13, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[5] = animation(15, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[5].block_mask = 4096;
    d.alien_ambient_array[6] = animation(17, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[7] = animation(19, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[7].block_mask = 8192;
    d.alien_ambient_array[8] = animation(21, 6, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[9] = animation(27, 4, 4, (56, 56), (8400, 2520));
    d.alien_ambient_array[9].block_mask = 16384;
    d.alien_ambient_array[10] = animation(31, 6, 2, (56, 56), (840, 2520));
    d.alien_ambient_array[11] = animation(37, 4, 1, (56, 56), (56, 56));
    d.alien_ambient_array[12] = animation(41, 3, 4, (84, 56), (8400, 2520));
    d.alien_ambient_array[12].block_mask = 32;
    d.alien_ambient_array[13] = animation(44, 4, 12, (140, 0), (2520, 840));
    d.alien_ambient_array[13].block_mask = 16512;
    d.alien_ambient_array[14] = animation(48, 3, 12, (112, 56), (8400, 840));
    d.alien_ambient_array[14].block_mask = 8704;
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 4, 0, (56, 0), (70, 0));
    d
}
