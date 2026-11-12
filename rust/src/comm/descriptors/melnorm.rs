#[cfg(not(test))]
use crate::comm::live::melnorm as live;
use crate::comm::locdata::*;
use crate::comm::races::melnorm::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.melnorme.graphics".as_ptr(),
        alien_font_res: c"comm.melnorme.font".as_ptr(),
        alien_colormap_res: c"comm.melnorme.colortable".as_ptr(),
        alien_song_res: c"comm.melnorme.music".as_ptr(),
        conversation_phrases_res: c"comm.melnorme.dialogue".as_ptr(),
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
        num_animations: 4,
        alien_number_speech: Some(numbers::descriptor()),
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(6, 5, 4, (70, 0), (3360, 3360));
    d.alien_ambient_array[0].block_mask = 2;
    d.alien_ambient_array[1] = animation(11, 9, 4, (42, 0), (3360, 3360));
    d.alien_ambient_array[1].block_mask = 1;
    d.alien_ambient_array[2] = animation(20, 2, 4, (84, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(22, 2, 4, (84, 56), (840, 2520));
    d.alien_talk_desc = animation(1, 5, 0, (56, 0), (70, 0));
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
        as_comm_data(&live::init_melnorme_comm(_comm, _cx))
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
