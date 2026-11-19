//! Phrase-selection helpers shared by Vux visit counters.

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

pub(super) fn hostile_info(visits: u8) -> (Option<u32>, u8) {
    let (case, next) = visit_phrase(visits, 3);
    let phrase = match case {
        Some(0) => Some(super::GENERAL_INFO_HOSTILE_1),
        Some(1) => Some(super::GENERAL_INFO_HOSTILE_2),
        Some(2) => Some(super::GENERAL_INFO_HOSTILE_3),
        Some(3) => Some(super::GENERAL_INFO_HOSTILE_4),
        _ => None,
    };
    (phrase, next)
}

#[cfg(test)]
mod tests {
    use super::{hostile_info, visit_phrase};

    #[test]
    fn capped_and_wrapping_visit_counters_match_c_byte_switches() {
        assert_eq!(visit_phrase(0, 3), (Some(0), 1));
        assert_eq!(visit_phrase(3, 3), (Some(3), 3));
        assert_eq!(visit_phrase(4, 3), (None, 5));
        assert_eq!(visit_phrase(u8::MAX, 3), (None, 0));
    }

    #[test]
    fn hostile_information_branch_order_and_final_repeat_are_preserved() {
        assert_eq!(
            hostile_info(0),
            (Some(super::super::GENERAL_INFO_HOSTILE_1), 1)
        );
        assert_eq!(
            hostile_info(1),
            (Some(super::super::GENERAL_INFO_HOSTILE_2), 2)
        );
        assert_eq!(
            hostile_info(2),
            (Some(super::super::GENERAL_INFO_HOSTILE_3), 3)
        );
        assert_eq!(
            hostile_info(3),
            (Some(super::super::GENERAL_INFO_HOSTILE_4), 3)
        );
    }
}
