use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.utwig.graphics"),
        alien_font_res: Some(c"comm.utwig.font"),
        alien_colormap_res: Some(c"comm.utwig.colortable"),
        alien_song_res: Some(c"comm.utwig.music"),
        conversation_phrases_res: Some(c"comm.utwig.dialogue"),
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
        num_animations: 16,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(4, 3, 4, (56, 28), (840, 2520));
    d.alien_ambient_array[1] = animation(7, 4, 4, (56, 28), (8400, 2520));
    d.alien_ambient_array[1].block_mask = 4;
    d.alien_ambient_array[2] = animation(11, 2, 12, (112, 28), (8400, 2520));
    d.alien_ambient_array[2].block_mask = 2;
    d.alien_ambient_array[3] = animation(13, 5, 2, (70, 0), (70, 0));
    d.alien_ambient_array[4] = animation(18, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[5] = animation(20, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[6] = animation(22, 3, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[7] = animation(25, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[8] = animation(27, 3, 4, (112, 28), (8400, 2520));
    d.alien_ambient_array[9] = animation(30, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[10] = animation(32, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[11] = animation(34, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[12] = animation(36, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[13] = animation(38, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[14] = animation(40, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[15] = animation(42, 3, 4, (112, 28), (8400, 2520));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 3, 0, (42, 42), (98, 420));
    d
}
