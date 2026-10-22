//! Phrase-selection helpers shared by Yehat visit counters.

/// Preserve `switch (visits++)` byte semantics, including the repeating final
/// phrase used by the C dialogue.
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

pub(super) fn space_info(visits: u8) -> (Option<u32>, u8) {
    let (case, next) = visit_phrase(visits, 3);
    let phrase = match case {
        Some(0) => Some(super::GENERAL_INFO_SPACE_1),
        Some(1) => Some(super::GENERAL_INFO_SPACE_2),
        Some(2) => Some(super::GENERAL_INFO_SPACE_3),
        Some(3) => Some(super::GENERAL_INFO_SPACE_4),
        _ => None,
    };
    (phrase, next)
}

#[cfg(test)]
mod tests {
    use super::{space_info, visit_phrase};

    #[test]
    fn capped_and_wrapping_visit_counters_match_c_byte_switches() {
        assert_eq!(visit_phrase(0, 1), (Some(0), 1));
        assert_eq!(visit_phrase(1, 1), (Some(1), 1));
        assert_eq!(visit_phrase(2, 1), (None, 3));
        assert_eq!(visit_phrase(u8::MAX, 1), (None, 0));
    }

    #[test]
    fn space_information_branch_order_and_final_repeat_are_preserved() {
        assert_eq!(space_info(0), (Some(super::super::GENERAL_INFO_SPACE_1), 1));
        assert_eq!(space_info(1), (Some(super::super::GENERAL_INFO_SPACE_2), 2));
        assert_eq!(space_info(2), (Some(super::super::GENERAL_INFO_SPACE_3), 3));
        assert_eq!(space_info(3), (Some(super::super::GENERAL_INFO_SPACE_4), 3));
    }
}
