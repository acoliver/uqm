#[cfg(not(test))]
use crate::comm::live::spathi as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut desc = CLocData {
        alien_frame_res: c"comm.spathi.graphics".as_ptr(),
        alien_font_res: c"comm.spathi.font".as_ptr(),
        alien_colormap_res: c"comm.spathi.colortable".as_ptr(),
        alien_song_res: c"comm.spathi.music".as_ptr(),
        conversation_phrases_res: c"comm.spathi.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 164, y: 0 },
        alien_text_width: 288,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 8,
        ..CLocData::default()
    };
    desc.alien_ambient_array[0] = animation(1, 6, 2, (42, 0), (0, 840));
    desc.alien_ambient_array[0].block_mask = 2;
    desc.alien_ambient_array[1] = animation(7, 9, 2, (42, 0), (42, 0));
    desc.alien_ambient_array[1].block_mask = 1;
    desc.alien_ambient_array[2] = animation(16, 4, 2, (84, 56), (84, 56));
    desc.alien_ambient_array[2].block_mask = 16;
    desc.alien_ambient_array[3] = animation(20, 4, 4, (56, 56), (56, 56));
    desc.alien_ambient_array[3].block_mask = 32;
    desc.alien_ambient_array[4] = animation(24, 5, 4, (56, 0), (840, 2520));
    desc.alien_ambient_array[4].block_mask = 4;
    desc.alien_ambient_array[5] = animation(34, 4, 4, (56, 0), (840, 2520));
    desc.alien_ambient_array[5].block_mask = 8;
    desc.alien_ambient_array[6] = animation(38, 3, 4, (56, 0), (840, 2520));
    desc.alien_ambient_array[7] = animation(41, 3, 4, (56, 0), (840, 2520));
    desc.alien_talk_desc = animation(0, 0, 0, (0, 0), (0, 0));
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
        as_comm_data(&live::init_spathi_comm(_comm, _cx))
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
