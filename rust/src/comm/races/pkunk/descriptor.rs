use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.pkunk.graphics"),
        alien_font_res: Some(c"comm.pkunk.font"),
        alien_colormap_res: Some(c"comm.pkunk.colortable"),
        alien_song_res: Some(c"comm.pkunk.music"),
        conversation_phrases_res: Some(c"comm.pkunk.dialogue"),
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
        num_animations: 3,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(3, 4, 10, (28, 0), (840, 2520));
    d.alien_ambient_array[1] = {
        let mut a = animation(7, 4, 12, (84, 0), (840, 2520));
        a.block_mask = 4;
        a
    };
    d.alien_ambient_array[2] = {
        let mut a = animation(11, 3, 12, (28, 0), (840, 2520));
        a.block_mask = 2;
        a
    };
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 2, 0, (56, 140), (70, 420));
    d
}
