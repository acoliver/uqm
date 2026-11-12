use super::numbers;
use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.melnorme.graphics"),
        alien_font_res: Some(c"comm.melnorme.font"),
        alien_colormap_res: Some(c"comm.melnorme.colortable"),
        alien_song_res: Some(c"comm.melnorme.music"),
        conversation_phrases_res: Some(c"comm.melnorme.dialogue"),
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
        num_animations: 4,
        alien_number_speech: Some(numbers::descriptor()),
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(6, 5, 4, (70, 0), (3360, 3360));
    d.alien_ambient_array[0].block_mask = 2;
    d.alien_ambient_array[1] = animation(11, 9, 4, (42, 0), (3360, 3360));
    d.alien_ambient_array[1].block_mask = 1;
    d.alien_ambient_array[2] = animation(20, 2, 4, (84, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(22, 2, 4, (84, 56), (840, 2520));
    d.alien_talk_desc = animation(1, 5, 0, (56, 0), (70, 0));
    d
}
