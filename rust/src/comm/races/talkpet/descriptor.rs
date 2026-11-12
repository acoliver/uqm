use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.talkingpet.graphics"),
        alien_font_res: Some(c"comm.talkingpet.font"),
        alien_colormap_res: Some(c"comm.talkingpet.colortable"),
        alien_song_res: Some(c"comm.talkingpet.music"),
        conversation_phrases_res: Some(c"comm.talkingpet.dialogue"),
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
        num_animations: 17,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(7, 3, 12, (56, 56), (840, 2520));
    d.alien_ambient_array[1] = animation(10, 3, 12, (56, 56), (840, 2520));
    d.alien_ambient_array[2] = animation(13, 3, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(16, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[4] = animation(18, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[4].block_mask = 8192;
    d.alien_ambient_array[5] = animation(21, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[5].block_mask = 20480;
    d.alien_ambient_array[6] = animation(24, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[7] = animation(26, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[8] = animation(28, 4, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[9] = animation(32, 3, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[10] = animation(35, 5, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[11] = animation(40, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[12] = animation(42, 6, 10, (84, 0), (840, 2520));
    d.alien_ambient_array[12].block_mask = 32;
    d.alien_ambient_array[13] = animation(48, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[13].block_mask = 16;
    d.alien_ambient_array[14] = animation(50, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[14].block_mask = 32;
    d.alien_ambient_array[15] = animation(52, 2, 9, (84, 0), (840, 2520));
    d.alien_ambient_array[16] = animation(1, 30, 178, (42, 0), (0, 0));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 6, 0, (56, 0), (70, 0));
    d
}
