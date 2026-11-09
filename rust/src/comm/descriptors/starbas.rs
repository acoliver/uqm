#[cfg(not(test))]
use crate::comm::live::starbas as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut desc = CLocData {
        alien_frame_res: c"comm.commander.graphics".as_ptr(),
        alien_font_res: c"comm.commander.font".as_ptr(),
        alien_colormap_res: c"comm.commander.colortable".as_ptr(),
        alien_song_res: c"comm.commander.music".as_ptr(),
        conversation_phrases_res: c"comm.starbase.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 164, y: 20 },
        alien_text_width: 143,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 10,
        ..CLocData::default()
    };
    desc.alien_ambient_array[0] = animation(1, 3, 4, (56, 0), (0, 6720));
    desc.alien_ambient_array[1] = animation(10, 30, 2, (21, 0), (1680, 0));
    desc.alien_ambient_array[2] = animation(40, 7, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[3] = animation(47, 8, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[4] = animation(55, 6, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[5] = animation(61, 6, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[6] = animation(67, 7, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[7] = animation(74, 11, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[8] = animation(85, 10, 2, (21, 0), (0, 6720));
    desc.alien_ambient_array[9] = animation(95, 1, 0, (0, 0), (0, 0));
    desc.alien_talk_desc = animation(4, 6, 0, (84, 56), (98, 70));
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
        as_comm_data(&live::init_starbase_comm(_comm, _cx))
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
