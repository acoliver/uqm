use super::{
    numbers, RACE_COLOR_MAP, RACE_CONVERSATION_PHRASES, RACE_FONT, RACE_MUSIC, RACE_PMAP_ANIM,
};
use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor(screen_width: i32) -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(RACE_PMAP_ANIM),
        alien_font_res: Some(RACE_FONT),
        alien_colormap_res: Some(RACE_COLOR_MAP),
        alien_song_res: Some(RACE_MUSIC),
        conversation_phrases_res: Some(RACE_CONVERSATION_PHRASES),
        alien_number_speech: Some(numbers::descriptor()),
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
        alien_text_width: (screen_width - 80 - 16) as u16,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 6,
        ..CommDescriptor::default()
    };
    d.alien_ambient_array[0] = animation(1, 9, 2, (21, 0), (0, 2520));
    d.alien_ambient_array[0].block_mask = 8;
    d.alien_ambient_array[1] = animation(10, 8, 2, (21, 0), (0, 2520));
    d.alien_ambient_array[1].block_mask = 16;
    d.alien_ambient_array[2] = animation(18, 8, 2, (21, 0), (0, 2520));
    d.alien_ambient_array[2].block_mask = 32;
    d.alien_ambient_array[3] = animation(26, 8, 2, (21, 0), (0, 2520));
    d.alien_ambient_array[3].block_mask = 1;
    d.alien_ambient_array[4] = animation(34, 8, 2, (21, 0), (0, 2520));
    d.alien_ambient_array[4].block_mask = 2;
    d.alien_ambient_array[5] = animation(42, 9, 2, (21, 0), (0, 2520));
    d.alien_ambient_array[5].block_mask = 4;
    d
}
