#[cfg(not(test))]
use crate::comm::live::utwig as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.utwig.graphics".as_ptr(),
        alien_font_res: c"comm.utwig.font".as_ptr(),
        alien_colormap_res: c"comm.utwig.colortable".as_ptr(),
        alien_song_res: c"comm.utwig.music".as_ptr(),
        conversation_phrases_res: c"comm.utwig.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 121, y: 70 },
        alien_text_width: 224,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 16,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(4, 3, 4, (56, 28), (840, 2520));
    d.alien_ambient_array[1] = animation(7, 4, 4, (56, 28), (8400, 2520));
    d.alien_ambient_array[1].block_mask = 4;
    d.alien_ambient_array[2] = animation(11, 2, 12, (112, 28), (8400, 2520));
    d.alien_ambient_array[2].block_mask = 2;
    d.alien_ambient_array[3] = animation(13, 5, 2, (70, 0), (70, 0));
    d.alien_ambient_array[4] = animation(18, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[5] = animation(20, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[6] = animation(22, 3, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[7] = animation(25, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[8] = animation(27, 3, 4, (112, 28), (8400, 2520));
    d.alien_ambient_array[9] = animation(30, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[10] = animation(32, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[11] = animation(34, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[12] = animation(36, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[13] = animation(38, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[14] = animation(40, 2, 1, (112, 28), (8400, 2520));
    d.alien_ambient_array[15] = animation(42, 3, 4, (112, 28), (8400, 2520));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 3, 0, (42, 42), (98, 420));
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
        as_comm_data(&live::init_utwig_comm(_comm, _cx))
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
