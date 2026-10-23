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
        alien_text_width: (screen_width - 80) as u16,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 13,
        ..CLocData::default()
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
    #[cfg(not(test))]
    {
        d.init_encounter_func = Some(live::intro);
        d.post_encounter_func = Some(live::post);
        d.uninit_encounter_func = Some(live::uninit);
    }
    d
}
