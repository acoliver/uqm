use super::{
    RACE_ALT_MUSIC, RACE_COLOR_MAP, RACE_CONVERSATION_PHRASES, RACE_FONT, RACE_MUSIC,
    RACE_PMAP_ANIM,
};
use crate::comm::races::descriptor::{Color, CommDescriptor, Point};
use crate::comm::races::script::animation;

pub(crate) fn descriptor(screen_width: i32) -> CommDescriptor {
    let mut d = CommDescriptor {
        alien_frame_res: Some(RACE_PMAP_ANIM),
        alien_font_res: Some(RACE_FONT),
        alien_colormap_res: Some(RACE_COLOR_MAP),
        alien_song_res: Some(RACE_MUSIC),
        alien_alt_song_res: Some(RACE_ALT_MUSIC),
        alien_song_flags: 1,
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
        alien_text_width: (screen_width - 96) as u16,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 14,
        ..CommDescriptor::default()
    };
    for (i, (start, frames, flags, frame_rate, restart_rate, mask)) in [
        (1, 3, 2, (42, 0), (840, 2520), 0),
        (4, 5, 2, (42, 0), (840, 2520), 0),
        (9, 4, 2, (42, 0), (840, 2520), (1 << 10) | (1 << 11)),
        (13, 6, 2, (42, 0), (42, 0), (1 << 4) | (1 << 5)),
        (19, 3, 4, (42, 0), (840, 2520), (1 << 3) | (1 << 5)),
        (
            22,
            4,
            4,
            (84, 28),
            (84, 28),
            (1 << 3) | (1 << 4) | (1 << 10),
        ),
        (26, 3, 4, (84, 28), (8400, 2520), 1 << 10),
        (29, 3, 4, (84, 28), (8400, 2520), 0),
        (32, 7, 2, (42, 0), (42, 0), (1 << 9) | (1 << 10)),
        (39, 3, 4, (42, 0), (840, 2520), (1 << 8) | (1 << 10)),
        (
            42,
            4,
            4,
            (84, 28),
            (28, 0),
            (1 << 8) | (1 << 9) | (1 << 6) | (1 << 2) | (1 << 11) | (1 << 5),
        ),
        (46, 4, 4, (84, 28), (84, 28), (1 << 2) | (1 << 10)),
        (50, 6, 2, (42, 0), (42, 0), 1 << 13),
        (56, 3, 4, (42, 0), (840, 2520), 1 << 12),
    ]
    .into_iter()
    .enumerate()
    {
        d.alien_ambient_array[i] = animation(start, frames, flags, frame_rate, restart_rate);
        d.alien_ambient_array[i].block_mask = mask;
    }
    d
}
