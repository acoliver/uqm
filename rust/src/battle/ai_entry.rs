//! Live computer-input routing, preserving intel.c's battle and selection rules.

use crate::ships::native::abi::{IN_LAST_BATTLE, RPG_PLAYER_NUM, SUPER_MELEE};

#[derive(Debug, PartialEq)]
enum Route {
    None,
    Human,
    Tactical(bool),
    Select,
    Unexpected,
}

fn route(activity: u8, ship: Option<(u8, i16)>, player_control: u8) -> Route {
    if activity == IN_LAST_BATTLE as u8 {
        return Route::None;
    }
    if let Some((control, player)) = ship {
        if control & (1 << 1) != 0 {
            Route::Tactical(player == RPG_PLAYER_NUM as i16)
        } else {
            Route::Human
        }
    } else if player_control & (1 << 2) == 0 {
        Route::None
    } else if activity == SUPER_MELEE as u8 {
        Route::Select
    } else {
        Route::Unexpected
    }
}

/// # Safety
/// Context and any non-null ship must be live engine records, on the game thread.
#[cfg(any(not(test), feature = "linked_c_archive"))]
#[no_mangle]
pub unsafe extern "C" fn computer_intelligence(
    context: *mut crate::ships::native::abi::ComputerInputContext,
    ship: *mut crate::ships::native::abi::STARSHIP,
) -> u8 {
    use crate::ships::native::abi::*;
    let activity = GlobData.Game_state.CurrentActivity as u8;
    if activity == IN_LAST_BATTLE as u8 {
        return 0;
    }
    let player = (*context).playerNr;
    let ship_control = if ship.is_null() {
        None
    } else {
        Some(((*ship).control, (*ship).playerNr))
    };
    let player_control = if ship.is_null() {
        *(&raw const PlayerControl).cast::<u8>().add(player as usize)
    } else {
        0
    };
    match route(activity, ship_control, player_control) {
        Route::None => 0,
        Route::Human => CurrentInputToBattleInput(player),
        Route::Tactical(escape) => {
            let mut input = tactical_intelligence(context, ship);
            if escape {
                input |= CurrentInputToBattleInput(player) & (1 << 5);
            }
            input
        }
        Route::Select => {
            SleepThread(ONE_SECOND >> 1);
            1 << 3
        }
        Route::Unexpected => {
            crate::log_abi::log_message(3, "Warning: Unexpected state in computer_intelligence().");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_battle_disables_even_human_input() {
        assert_eq!(route(1, Some((0, 0)), 4), Route::None);
    }

    #[test]
    fn only_psytron_selects_a_random_melee_ship() {
        assert_eq!(route(0, None, 4), Route::Select);
        assert_eq!(route(0, None, 2), Route::None);
        assert_eq!(route(2, None, 4), Route::Unexpected);
    }

    #[test]
    fn cyborg_only_allows_the_rpg_players_escape_input() {
        assert_eq!(route(2, Some((2, 0)), 0), Route::Tactical(true));
        assert_eq!(route(2, Some((2, 1)), 0), Route::Tactical(false));
        assert_eq!(route(2, Some((1, 0)), 0), Route::Human);
    }
}
