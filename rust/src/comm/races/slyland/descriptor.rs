use super::*;
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::races::script::animation;

pub(super) fn descriptor(screen_width: i32) -> CLocData {
    let mut d = CLocData {
        alien_frame_res: RACE_PMAP_ANIM.as_ptr().cast(),
        alien_font_res: RACE_FONT.as_ptr().cast(),
        alien_colormap_res: RACE_COLOR_MAP.as_ptr().cast(),
        alien_song_res: RACE_MUSIC.as_ptr().cast(),
        conversation_phrases_res: RACE_CONVERSATION_PHRASES.as_ptr().cast(),
        alien_number_speech: super::numbers::descriptor().cast(),
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
        alien_text_width: (screen_width - 80 - 16) as u16,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 6,
        ..CLocData::default()
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
    #[cfg(not(test))]
    {
        d.init_encounter_func = Some(live::intro);
        d.post_encounter_func = Some(live::post);
        d.uninit_encounter_func = Some(live::uninit);
    }
    d
}
