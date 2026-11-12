use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.thraddash.graphics"),
        alien_font_res: Some(c"comm.thraddash.font"),
        alien_colormap_res: Some(c"comm.thraddash.colortable"),
        alien_song_res: Some(c"comm.thraddash.music"),
        conversation_phrases_res: Some(c"comm.thraddash.dialogue"),
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
        num_animations: 8,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = {
        let mut a = animation(8, 4, 1, (56, 56), (56, 56));
        a.block_mask = 16;
        a
    };
    d.alien_ambient_array[1] = animation(12, 9, 2, (42, 0), (840, 2520));
    d.alien_ambient_array[2] = animation(21, 6, 1, (42, 0), (840, 2520));
    d.alien_ambient_array[3] = {
        let mut a = animation(27, 3, 4, (42, 0), (840, 2520));
        a.block_mask = 16;
        a
    };
    d.alien_ambient_array[4] = {
        let mut a = animation(30, 12, 10, (70, 0), (840, 840));
        a.block_mask = 41;
        a
    };
    d.alien_ambient_array[5] = {
        let mut a = animation(42, 5, 4, (84, 28), (840, 2520));
        a.block_mask = 80;
        a
    };
    d.alien_ambient_array[6] = {
        let mut a = animation(47, 5, 4, (84, 28), (840, 2520));
        a.block_mask = 32;
        a
    };
    d.alien_ambient_array[7] = animation(52, 4, 1, (42, 0), (42, 0));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 7, 0, (56, 0), (70, 0));
    d
}
