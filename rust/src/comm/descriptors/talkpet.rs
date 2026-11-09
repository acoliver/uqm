#[cfg(not(test))]
use crate::comm::live::talkpet as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.talkingpet.graphics".as_ptr(),
        alien_font_res: c"comm.talkingpet.font".as_ptr(),
        alien_colormap_res: c"comm.talkingpet.colortable".as_ptr(),
        alien_song_res: c"comm.talkingpet.music".as_ptr(),
        conversation_phrases_res: c"comm.talkingpet.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 121, y: 0 },
        alien_text_width: 224,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 17,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(7, 3, 12, (56, 56), (840, 2520));
    d.alien_ambient_array[1] = animation(10, 3, 12, (56, 56), (840, 2520));
    d.alien_ambient_array[2] = animation(13, 3, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(16, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[4] = animation(18, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[4].block_mask = 8192;
    d.alien_ambient_array[5] = animation(21, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[5].block_mask = 20480;
    d.alien_ambient_array[6] = animation(24, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[7] = animation(26, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[8] = animation(28, 4, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[9] = animation(32, 3, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[10] = animation(35, 5, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[11] = animation(40, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[12] = animation(42, 6, 10, (84, 0), (840, 2520));
    d.alien_ambient_array[12].block_mask = 32;
    d.alien_ambient_array[13] = animation(48, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[13].block_mask = 16;
    d.alien_ambient_array[14] = animation(50, 2, 1, (84, 0), (840, 2520));
    d.alien_ambient_array[14].block_mask = 32;
    d.alien_ambient_array[15] = animation(52, 2, 9, (84, 0), (840, 2520));
    d.alien_ambient_array[16] = animation(1, 30, 178, (42, 0), (0, 0));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 6, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        d.init_encounter_func = Some(live::intro);
        d.post_encounter_func = Some(live::post);
        d.uninit_encounter_func = Some(live::uninit);
    }
    d
}

pub(crate) fn init(_comm: &mut Comm, _cx: &mut Cx<'_>) -> CommData {
    #[cfg(not(test))]
    {
        as_comm_data(&live::init_talkpet_comm(_comm, _cx))
    }
    #[cfg(test)]
    {
        as_comm_data(&descriptor())
    }
}

pub(crate) fn intro(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::intro(_comm, _cx);
}

pub(crate) fn post_encounter(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::post(_comm, _cx);
}
