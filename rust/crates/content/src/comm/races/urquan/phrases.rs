//! Phrase-selection helpers shared by Ur-Quan visit counters.

/// Apply the C scripts' `switch (visits++)` convention: ordinary cases advance,
/// the final phrase repeats without advancing, and byte arithmetic wraps.
pub(super) fn visit_phrase(visits: u8, final_case: u8) -> (Option<u8>, u8) {
    let next = visits.wrapping_add(1);
    if visits <= final_case {
        (
            Some(visits),
            if visits == final_case { visits } else { next },
        )
    } else {
        (None, next)
    }
}

pub(super) fn general_info(visits: u8) -> (Option<u32>, u8) {
    let (case, next) = visit_phrase(visits, 3);
    let phrase = match case {
        Some(0) => Some(super::GENERAL_INFO_1),
        Some(1) => Some(super::GENERAL_INFO_2),
        Some(2) => Some(super::GENERAL_INFO_3),
        Some(3) => Some(super::GENERAL_INFO_4),
        _ => None,
    };
    (phrase, next)
}

#[cfg(test)]
mod tests {
    use super::{general_info, visit_phrase};

    #[test]
    fn visit_counter_advances_until_final_phrase_then_repeats() {
        assert_eq!(visit_phrase(0, 3), (Some(0), 1));
        assert_eq!(visit_phrase(2, 3), (Some(2), 3));
        assert_eq!(visit_phrase(3, 3), (Some(3), 3));
        assert_eq!(visit_phrase(4, 3), (None, 5));
    }

    #[test]
    fn visit_counter_wraps_like_a_c_byte() {
        assert_eq!(visit_phrase(u8::MAX, 3), (None, 0));
    }

    #[test]
    fn general_information_selects_each_branch_and_holds_at_the_last() {
        assert_eq!(general_info(0), (Some(super::super::GENERAL_INFO_1), 1));
        assert_eq!(general_info(1), (Some(super::super::GENERAL_INFO_2), 2));
        assert_eq!(general_info(2), (Some(super::super::GENERAL_INFO_3), 3));
        assert_eq!(general_info(3), (Some(super::super::GENERAL_INFO_4), 3));
        assert_eq!(general_info(4), (None, 5));
    }
}
