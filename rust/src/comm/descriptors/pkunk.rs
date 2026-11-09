#[cfg(not(test))]
use crate::comm::live::pkunk as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.pkunk.graphics".as_ptr(),
        alien_font_res: c"comm.pkunk.font".as_ptr(),
        alien_colormap_res: c"comm.pkunk.colortable".as_ptr(),
        alien_song_res: c"comm.pkunk.music".as_ptr(),
        conversation_phrases_res: c"comm.pkunk.dialogue".as_ptr(),
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
        num_animations: 3,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(3, 4, 10, (28, 0), (840, 2520));
    d.alien_ambient_array[1] = {
        let mut a = animation(7, 4, 12, (84, 0), (840, 2520));
        a.block_mask = 4;
        a
    };
    d.alien_ambient_array[2] = {
        let mut a = animation(11, 3, 12, (28, 0), (840, 2520));
        a.block_mask = 2;
        a
    };
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 2, 0, (56, 140), (70, 420));
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
        as_comm_data(&live::init_pkunk_comm(_comm, _cx))
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
