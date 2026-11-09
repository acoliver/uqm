#[cfg(not(test))]
use crate::comm::live::druuge as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.druuge.graphics".as_ptr(),
        alien_font_res: c"comm.druuge.font".as_ptr(),
        alien_colormap_res: c"comm.druuge.colortable".as_ptr(),
        alien_song_res: c"comm.druuge.music".as_ptr(),
        conversation_phrases_res: c"comm.druuge.dialogue".as_ptr(),
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
        num_animations: 11,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(5, 4, 2, (63, 0), (63, 0));
    d.alien_ambient_array[1] = animation(9, 4, 2, (63, 0), (63, 0));
    d.alien_ambient_array[2] = animation(13, 6, 2, (63, 0), (63, 0));
    d.alien_ambient_array[3] = animation(19, 3, 2, (42, 0), (840, 2520));
    d.alien_ambient_array[4] = animation(22, 3, 1, (70, 0), (840, 2520));
    d.alien_ambient_array[5] = animation(25, 3, 1, (70, 0), (840, 2520));
    d.alien_ambient_array[6] = animation(28, 3, 1, (70, 0), (840, 2520));
    d.alien_ambient_array[7] = animation(31, 2, 2, (70, 0), (840, 2520));
    d.alien_ambient_array[8] = animation(33, 7, 10, (70, 0), (5880, 2520));
    d.alien_ambient_array[9] = animation(40, 4, 2, (252, 0), (252, 0));
    d.alien_ambient_array[10] = animation(44, 4, 2, (168, 0), (168, 0));
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 4, 0, (56, 56), (70, 70));
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
        as_comm_data(&live::init_druuge_comm(_comm, _cx))
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
