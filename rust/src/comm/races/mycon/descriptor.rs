use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.mycon.graphics"),
        alien_font_res: Some(c"comm.mycon.font"),
        alien_colormap_res: Some(c"comm.mycon.colortable"),
        alien_song_res: Some(c"comm.mycon.music"),
        conversation_phrases_res: Some(c"comm.mycon.dialogue"),
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
        num_animations: 5,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(12, 6, 2, (63, 0), (63, 0));
    d.alien_ambient_array[0].block_mask = 2;
    d.alien_ambient_array[1] = animation(18, 4, 2, (63, 0), (63, 0));
    d.alien_ambient_array[1].block_mask = 1;
    d.alien_ambient_array[2] = animation(22, 6, 2, (63, 0), (63, 0));
    d.alien_ambient_array[3] = animation(28, 5, 2, (63, 0), (63, 0));
    d.alien_ambient_array[4] = animation(33, 4, 4, (63, 0), (63, 0));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 11, 0, (56, 0), (70, 0));
    d
}
