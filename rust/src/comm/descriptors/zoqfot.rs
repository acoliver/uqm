#[cfg(not(test))]
use crate::comm::live::zoqfot as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.zoqfotpik.graphics".as_ptr(),
        alien_font_res: c"comm.zoqfotpik.font".as_ptr(),
        alien_colormap_res: c"comm.zoqfotpik.colortable".as_ptr(),
        alien_song_res: c"comm.zoqfotpik.music".as_ptr(),
        conversation_phrases_res: c"comm.zoqfotpik.dialogue".as_ptr(),
        alien_text_fcolor: CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_bcolor: CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_baseline: CPoint { x: 0, y: 0 },
        alien_text_width: 119,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 3,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(1, 4, 12, (35, 0), (0, 8400));
    d.alien_ambient_array[1] = animation(5, 5, 2, (49, 0), (1680, 0));
    d.alien_ambient_array[2] = animation(10, 8, 2, (56, 0), (0, 8400));
    d.alien_transition_desc = animation(23, 3, 0, (28, 0), (0, 0));
    d.alien_talk_desc = animation(18, 5, 0, (56, 0), (70, 0));
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
        as_comm_data(&live::init_zoqfot_comm(_comm, _cx))
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
