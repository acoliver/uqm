use super::*;
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::races::script::animation;

pub(super) fn descriptor(screen_width: i32) -> CLocData {
    let mut d = CLocData {
        alien_frame_res: RACE_PMAP_ANIM.as_ptr().cast(),
        alien_font_res: RACE_FONT.as_ptr().cast(),
        alien_colormap_res: RACE_COLOR_MAP.as_ptr().cast(),
        alien_song_res: RACE_MUSIC.as_ptr().cast(),
        alien_alt_song_res: RACE_ALT_MUSIC.as_ptr().cast(),
        alien_song_flags: 1,
        conversation_phrases_res: RACE_CONVERSATION_PHRASES.as_ptr().cast(),
        alien_text_fcolor: CColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        alien_text_bcolor: CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_baseline: CPoint {
            x: (1 + ((screen_width - 80) >> 1)) as i16,
            y: 0,
        },
        alien_text_width: (screen_width - 96) as u16,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 14,
        ..CLocData::default()
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
    #[cfg(not(test))]
    {
        d.init_encounter_func = live::intro as *const () as *const _;
        d.post_encounter_func = live::post as *const () as *const _;
        d.uninit_encounter_func = live::uninit as *const () as *const _;
    }
    d
}
