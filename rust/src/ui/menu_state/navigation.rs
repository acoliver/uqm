//! Cursor decisions are separate from drawing and the post-navigation pause.
use super::*;

#[derive(Clone, Copy, Default)]
pub(crate) struct MenuKeys {
    pub previous: bool,
    pub next: bool,
    pub select: bool,
    pub cancel: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuStep {
    pub display: (u8, i16),
    pub state: u8,
    pub paced: bool,
}

pub(crate) fn navigate(
    original_base: u8,
    current: u8,
    pc: bool,
    flags: u8,
    keys: MenuKeys,
) -> Option<MenuStep> {
    let mut base = original_base;
    let mut state = current;
    let use_alt = pc && alternate(&mut base, &mut state);
    if keys.previous {
        state = previous_state(base, state, flags);
    } else if keys.next {
        state = next_state(base, state, flags);
    } else if use_alt && keys.select {
        return match convert_alternate(base, state) {
            ALT_MANIFEST => Some(MenuStep {
                display: (PM_ALT_CARGO, 0),
                state: PM_CARGO - PM_SCAN,
                paced: false,
            }),
            ALT_EXIT_MANIFEST => Some(exit_manifest(original_base)),
            _ => None,
        };
    } else if pc && keys.cancel && base == PM_ALT_CARGO {
        return Some(exit_manifest(original_base));
    } else {
        return None;
    }
    Some(MenuStep {
        display: (base, i16::from(state)),
        state: if use_alt {
            convert_alternate(base, state)
        } else {
            state
        },
        paced: true,
    })
}

fn exit_manifest(original_base: u8) -> MenuStep {
    let base = if original_base == PM_SCAN {
        PM_ALT_SCAN
    } else {
        PM_ALT_STARMAP
    };
    MenuStep {
        display: (base, i16::from(PM_ALT_MANIFEST - base)),
        state: ALT_MANIFEST,
        paced: false,
    }
}

pub(crate) fn alternate(base: &mut u8, current: &mut u8) -> bool {
    let adj = u8::from(*base == PM_STARMAP);
    let adjusted_base = base.wrapping_sub(adj);
    if *current & 0x80 != 0 {
        match *current {
            ALT_MANIFEST => {
                *base = PM_ALT_SCAN + adj;
                *current = PM_ALT_MANIFEST - PM_ALT_SCAN - adj;
                true
            }
            ALT_EXIT_MANIFEST => {
                *base = PM_ALT_CARGO;
                *current = PM_ALT_EXIT_MANIFEST - PM_ALT_CARGO;
                true
            }
            _ => {
                crate::log_abi::log_message(
                    crate::navigation::abi::log_Level_log_Error as i32,
                    &format!(
                        "Unknown state combination: {}, {}",
                        *base as i32, *current as i32
                    ),
                );
                false
            }
        }
    } else {
        let result = match adjusted_base.wrapping_add(*current) {
            PM_SCAN => Some((PM_ALT_SCAN, 0)),
            PM_STARMAP => Some((PM_ALT_SCAN + adj, PM_ALT_STARMAP - PM_ALT_SCAN - adj)),
            PM_DEVICES => Some((PM_ALT_CARGO, PM_ALT_DEVICES - PM_ALT_CARGO)),
            PM_CARGO => Some((PM_ALT_CARGO, 0)),
            PM_ROSTER => Some((PM_ALT_CARGO, PM_ALT_ROSTER - PM_ALT_CARGO)),
            PM_GAME_MENU => Some((PM_ALT_SCAN + adj, PM_ALT_GAME_MENU - PM_ALT_SCAN - adj)),
            PM_NAVIGATE => Some((PM_ALT_SCAN + adj, PM_ALT_NAVIGATE - PM_ALT_SCAN - adj)),
            PM_MIN_SCAN => Some((PM_ALT_MSCAN, 0)),
            PM_ENE_SCAN => Some((PM_ALT_MSCAN, PM_ALT_ESCAN - PM_ALT_MSCAN)),
            PM_BIO_SCAN => Some((PM_ALT_MSCAN, PM_ALT_BSCAN - PM_ALT_MSCAN)),
            PM_EXIT_SCAN => Some((PM_ALT_MSCAN, PM_ALT_EXIT_SCAN - PM_ALT_MSCAN)),
            PM_AUTO_SCAN => Some((PM_ALT_MSCAN, PM_ALT_ASCAN - PM_ALT_MSCAN)),
            PM_LAUNCH_LANDER => Some((PM_ALT_MSCAN, PM_ALT_DISPATCH - PM_ALT_MSCAN)),
            _ => None,
        };
        if let Some((new_base, new_offset)) = result {
            *base = new_base;
            *current = new_offset;
            true
        } else {
            false
        }
    }
}

fn convert_alternate(base: u8, state: u8) -> u8 {
    match base.wrapping_add(state) {
        PM_ALT_SCAN => 0,
        PM_ALT_STARMAP => PM_STARMAP - PM_SCAN,
        PM_ALT_MANIFEST => ALT_MANIFEST,
        PM_ALT_GAME_MENU => PM_GAME_MENU - PM_SCAN,
        PM_ALT_NAVIGATE => PM_NAVIGATE - PM_SCAN,
        PM_ALT_CARGO => PM_CARGO - PM_SCAN,
        PM_ALT_DEVICES => PM_DEVICES - PM_SCAN,
        PM_ALT_ROSTER => PM_ROSTER - PM_SCAN,
        PM_ALT_EXIT_MANIFEST => ALT_EXIT_MANIFEST,
        PM_ALT_MSCAN => 0,
        PM_ALT_ESCAN => PM_ENE_SCAN - PM_MIN_SCAN,
        PM_ALT_BSCAN => PM_BIO_SCAN - PM_MIN_SCAN,
        PM_ALT_ASCAN => PM_AUTO_SCAN - PM_MIN_SCAN,
        PM_ALT_DISPATCH => PM_LAUNCH_LANDER - PM_MIN_SCAN,
        PM_ALT_EXIT_SCAN => PM_EXIT_SCAN - PM_MIN_SCAN,
        _ => state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alternate_groups_keep_every_original_selection_offset() {
        for (base, state, group, offset, original) in [
            (PM_SCAN, 0, PM_ALT_SCAN, 0, 0),
            (PM_STARMAP, 1, PM_ALT_STARMAP, 0, 1),
            (
                PM_SCAN,
                PM_DEVICES,
                PM_ALT_CARGO,
                PM_ALT_DEVICES - PM_ALT_CARGO,
                PM_DEVICES,
            ),
            (PM_SCAN, PM_CARGO, PM_ALT_CARGO, 0, PM_CARGO),
            (
                PM_SCAN,
                PM_ROSTER,
                PM_ALT_CARGO,
                PM_ALT_ROSTER - PM_ALT_CARGO,
                PM_ROSTER,
            ),
            (
                PM_SCAN,
                PM_GAME_MENU,
                PM_ALT_SCAN,
                PM_ALT_GAME_MENU - PM_ALT_SCAN,
                PM_GAME_MENU,
            ),
            (
                PM_SCAN,
                PM_NAVIGATE,
                PM_ALT_SCAN,
                PM_ALT_NAVIGATE - PM_ALT_SCAN,
                PM_NAVIGATE,
            ),
            (PM_MIN_SCAN, 0, PM_ALT_MSCAN, 0, 0),
            (
                PM_MIN_SCAN,
                PM_ENE_SCAN - PM_MIN_SCAN,
                PM_ALT_MSCAN,
                PM_ALT_ESCAN - PM_ALT_MSCAN,
                PM_ENE_SCAN - PM_MIN_SCAN,
            ),
            (
                PM_MIN_SCAN,
                PM_BIO_SCAN - PM_MIN_SCAN,
                PM_ALT_MSCAN,
                PM_ALT_BSCAN - PM_ALT_MSCAN,
                PM_BIO_SCAN - PM_MIN_SCAN,
            ),
            (
                PM_MIN_SCAN,
                PM_EXIT_SCAN - PM_MIN_SCAN,
                PM_ALT_MSCAN,
                PM_ALT_EXIT_SCAN - PM_ALT_MSCAN,
                PM_EXIT_SCAN - PM_MIN_SCAN,
            ),
            (
                PM_MIN_SCAN,
                PM_AUTO_SCAN - PM_MIN_SCAN,
                PM_ALT_MSCAN,
                PM_ALT_ASCAN - PM_ALT_MSCAN,
                PM_AUTO_SCAN - PM_MIN_SCAN,
            ),
            (
                PM_MIN_SCAN,
                PM_LAUNCH_LANDER - PM_MIN_SCAN,
                PM_ALT_MSCAN,
                PM_ALT_DISPATCH - PM_ALT_MSCAN,
                PM_LAUNCH_LANDER - PM_MIN_SCAN,
            ),
        ] {
            let (mut actual_base, mut actual_state) = (base, state);
            assert!(alternate(&mut actual_base, &mut actual_state));
            assert_eq!((actual_base, actual_state), (group, offset));
            assert_eq!(convert_alternate(actual_base, actual_state), original);
        }
    }

    #[test]
    fn unrelated_and_unknown_states_do_not_enter_alternate_groups() {
        for state in [0, u8::MAX] {
            let (mut base, mut state) = (PM_CONVERSE, state);
            let before = (base, state);
            assert!(!alternate(&mut base, &mut state));
            assert_eq!((base, state), before);
        }
        assert_eq!(convert_alternate(u8::MAX, 0), 0);
    }
}
