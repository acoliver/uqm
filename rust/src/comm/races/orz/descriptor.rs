use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.orz.graphics"),
        alien_font_res: Some(c"comm.orz.font"),
        alien_colormap_res: Some(c"comm.orz.colortable"),
        alien_song_res: Some(c"comm.orz.music"),
        conversation_phrases_res: Some(c"comm.orz.dialogue"),
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
        num_animations: 12,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(4, 6, 2, (84, 0), (840, 2520));
    d.alien_ambient_array[1] = animation(10, 5, 2, (84, 0), (840, 2520));
    d.alien_ambient_array[2] = animation(15, 2, 1, (84, 84), (840, 2520));
    d.alien_ambient_array[3] = animation(17, 3, 4, (84, 84), (840, 2520));
    d.alien_ambient_array[4] = {
        let mut a = animation(20, 2, 4, (84, 0), (84, 2520));
        a.block_mask = 128;
        a
    };
    d.alien_ambient_array[5] = {
        let mut a = animation(22, 8, 2, (84, 0), (84, 2520));
        a.block_mask = 64;
        a
    };
    d.alien_ambient_array[6] = {
        let mut a = animation(30, 3, 2, (84, 0), (84, 2520));
        a.block_mask = 32;
        a
    };
    d.alien_ambient_array[7] = {
        let mut a = animation(33, 3, 4, (84, 0), (84, 2520));
        a.block_mask = 16;
        a
    };
    d.alien_ambient_array[8] = animation(36, 25, 2, (14, 56), (840, 2520));
    d.alien_ambient_array[9] = animation(61, 15, 2, (14, 56), (840, 2520));
    d.alien_ambient_array[10] = {
        let mut a = animation(76, 17, 2, (14, 56), (840, 2520));
        a.block_mask = 4096;
        a
    };
    d.alien_ambient_array[11] = animation(93, 25, 2, (14, 56), (840, 2520));
    d.alien_ambient_array[12] = {
        let mut a = animation(118, 11, 2, (14, 56), (840, 2520));
        a.block_mask = 1024;
        a
    };
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 3, 0, (56, 56), (70, 315));
    d
}
