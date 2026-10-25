//! Explicit development setup for the Super Melee mutual-loss regression.

#[cfg(not(test))]
pub(crate) fn trigger() -> Result<(), String> {
    use crate::battle::battle_native::battle_counter;
    use crate::ships::native::{abi::*, element, starship};
    use std::{mem, ptr};

    // The coordinator invokes this synchronously on the game thread, only for
    // the named fixture action. Death and replacement remain engine work.
    unsafe {
        if GlobData.Game_state.CurrentActivity != (SUPER_MELEE | IN_BATTLE) as u16
            || battle_counter != [2, 2]
        {
            return Err("mutual-loss fixture requires an active 2-vs-2 Super Melee".into());
        }
        let mut ships = [ptr::null_mut(); 2];
        let mut handle = disp_q.head;
        while !handle.is_null() {
            let e = element(handle);
            handle = (*e).succ;
            if (*e).state_flags & PLAYER_SHIP as u16 != 0 {
                ships[(*e).playerNr as usize] = e;
            }
        }
        if ships.iter().any(|ship| ship.is_null()) {
            return Err("mutual-loss fixture requires two live ship elements".into());
        }
        let scout = starship(ships[0]);
        let enemy = starship(ships[1]);
        if (*scout).SpeciesID != SPECIES_ID_SHOFIXTI_ID
            || (*(*enemy).RaceDescPtr).ship_info.crew_level > 6
        {
            return Err("mutual-loss fixture requires a Shofixti against at most six crew".into());
        }
        let desc = (*scout).RaceDescPtr;
        let captain = (*desc).ship_data.captain_control.special;
        let postprocess = (*desc).postprocess_func.unwrap();
        for (e, x) in [(ships[0], 4000), (ships[1], 4160)] {
            (*e).current.location = POINT { x, y: 4000 };
            (*e).next.location = (*e).current.location;
            (*e).velocity = mem::zeroed();
        }
        // Arrange the last safety switch, then use the real special callback.
        (*desc).ship_data.captain_control.special =
            SetAbsFrameIndex(captain, GetFrameCount(captain) - 4);
        (*scout).old_status_flags &= !(SPECIAL as u16);
        (*scout).cur_status_flags |= SPECIAL as u16;
        postprocess(ships[0]);
        assert_eq!((*ships[0]).__bindgen_anon_1.life_span, 0);
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn trigger() -> Result<(), String> {
    Err("mutual-loss fixture requires the linked game".into())
}
