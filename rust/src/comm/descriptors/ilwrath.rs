#[cfg(not(test))]
use crate::comm::live::ilwrath as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut desc = CLocData {
        alien_frame_res: c"comm.ilwrath.graphics".as_ptr(),
        alien_font_res: c"comm.ilwrath.font".as_ptr(),
        alien_colormap_res: c"comm.ilwrath.colortable".as_ptr(),
        alien_song_res: c"comm.ilwrath.music".as_ptr(),
        conversation_phrases_res: c"comm.ilwrath.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 164, y: 70 },
        alien_text_width: 288,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 4,
        ..CLocData::default()
    };
    desc.alien_ambient_array[0] = animation(6, 5, 2, (42, 0), (840, 2520));
    desc.alien_ambient_array[1] = animation(11, 5, 2, (42, 0), (840, 2520));
    desc.alien_ambient_array[2] = animation(16, 5, 2, (42, 0), (840, 2520));
    desc.alien_ambient_array[3] = animation(21, 6, 2, (42, 0), (840, 2520));
    desc.alien_talk_desc = animation(1, 5, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        desc.init_encounter_func = Some(live::intro);
        desc.post_encounter_func = Some(live::post);
        desc.uninit_encounter_func = Some(live::uninit);
    }
    desc
}

pub(crate) fn init(_comm: &mut Comm, _cx: &mut Cx<'_>) -> CommData {
    #[cfg(not(test))]
    {
        as_comm_data(&live::init_ilwrath_comm(_comm, _cx))
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
