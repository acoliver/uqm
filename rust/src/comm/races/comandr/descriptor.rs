use super::ONE_SECOND;
use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor() -> CommDescriptor {
    let mut desc = CommDescriptor {
        alien_frame_res: Some(c"comm.commander.graphics"),
        alien_font_res: Some(c"comm.commander.font"),
        alien_colormap_res: Some(c"comm.commander.colortable"),
        alien_song_res: Some(c"comm.commander.music"),
        conversation_phrases_res: Some(c"comm.commander.dialogue"),
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
        alien_text_baseline: Point { x: 164, y: 20 },
        alien_text_width: 143,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 3,
        alien_talk_desc: animation(
            4,
            6,
            0,
            (ONE_SECOND / 10, ONE_SECOND / 15),
            (ONE_SECOND * 7 / 60, ONE_SECOND / 12),
        ),
        ..CommDescriptor::default()
    };
    desc.alien_ambient_array[0] =
        animation(1, 3, 1 << 2, (ONE_SECOND / 15, 0), (0, ONE_SECOND * 8));
    desc.alien_ambient_array[1] =
        animation(10, 30, 1 << 1, (ONE_SECOND / 40, 0), (ONE_SECOND * 2, 0));
    desc.alien_ambient_array[2] = animation(
        1,
        3,
        1 | (1 << 4),
        (0, ONE_SECOND / 30),
        (0, ONE_SECOND / 15),
    );
    desc
}
