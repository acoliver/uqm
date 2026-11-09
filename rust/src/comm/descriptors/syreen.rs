#[cfg(not(test))]
use crate::comm::live::syreen as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.syreen.graphics".as_ptr(),
        alien_font_res: c"comm.syreen.font".as_ptr(),
        alien_colormap_res: c"comm.syreen.colortable".as_ptr(),
        alien_song_res: c"comm.syreen.music".as_ptr(),
        conversation_phrases_res: c"comm.syreen.dialogue".as_ptr(),
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
        num_animations: 15,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(5, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[1] = animation(7, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[2] = animation(9, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(11, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[4] = animation(13, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[5] = animation(15, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[5].block_mask = 4096;
    d.alien_ambient_array[6] = animation(17, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[7] = animation(19, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[7].block_mask = 8192;
    d.alien_ambient_array[8] = animation(21, 6, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[9] = animation(27, 4, 4, (56, 56), (8400, 2520));
    d.alien_ambient_array[9].block_mask = 16384;
    d.alien_ambient_array[10] = animation(31, 6, 2, (56, 56), (840, 2520));
    d.alien_ambient_array[11] = animation(37, 4, 1, (56, 56), (56, 56));
    d.alien_ambient_array[12] = animation(41, 3, 4, (84, 56), (8400, 2520));
    d.alien_ambient_array[12].block_mask = 32;
    d.alien_ambient_array[13] = animation(44, 4, 12, (140, 0), (2520, 840));
    d.alien_ambient_array[13].block_mask = 16512;
    d.alien_ambient_array[14] = animation(48, 3, 12, (112, 56), (8400, 840));
    d.alien_ambient_array[14].block_mask = 8704;
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 4, 0, (56, 0), (70, 0));
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
        as_comm_data(&live::init_syreen_comm(_comm, _cx))
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
