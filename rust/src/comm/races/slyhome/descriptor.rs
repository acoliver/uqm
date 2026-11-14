use super::{RACE_COLOR_MAP, RACE_CONVERSATION_PHRASES, RACE_FONT, RACE_MUSIC, RACE_PMAP_ANIM};
use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor(screen_width: i32) -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(RACE_PMAP_ANIM),
        alien_font_res: Some(RACE_FONT),
        alien_colormap_res: Some(RACE_COLOR_MAP),
        alien_song_res: Some(RACE_MUSIC),
        conversation_phrases_res: Some(RACE_CONVERSATION_PHRASES),
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
        alien_text_baseline: Point {
            x: (1 + ((screen_width - 80) >> 1)) as i16,
            y: 0,
        },
        alien_text_width: (screen_width - 80) as u16,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 13,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(0, 5, 17, (105, 525), (105, 525));
    d.alien_ambient_array[1] = animation(1, 5, 4, (56, 56), (840, 2520));
    d.alien_ambient_array[2] = animation(6, 5, 4, (56, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(11, 5, 4, (56, 56), (840, 2520));
    d.alien_ambient_array[4] = animation(16, 6, 4, (56, 56), (840, 2520));
    d.alien_ambient_array[5] = animation(22, 8, 2, (56, 0), (56, 0));
    d.alien_ambient_array[6] = animation(30, 9, 2, (56, 56), (840, 2520));
    d.alien_ambient_array[6].block_mask = 768;
    d.alien_ambient_array[7] = animation(39, 4, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[8] = animation(43, 5, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[8].block_mask = 64;
    d.alien_ambient_array[9] = animation(48, 6, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[9].block_mask = 64;
    d.alien_ambient_array[10] = animation(54, 6, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[10].block_mask = 4096;
    d.alien_ambient_array[11] = animation(60, 7, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[11].block_mask = 4096;
    d.alien_ambient_array[12] = animation(67, 8, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[12].block_mask = 3072;
    d
}
