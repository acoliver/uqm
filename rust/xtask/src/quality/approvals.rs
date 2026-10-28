use std::collections::BTreeMap;

use super::layers::OS_CRATES;

/// Record only the reviewed direct engine OS edges; all other ratchets still apply.
pub fn engine_os(limits: &mut BTreeMap<String, u64>) {
    for dependency in OS_CRATES {
        limits.insert(format!("layers|crate_os|uqm-engine::{dependency}"), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::baseline::check_limits;

    #[test]
    fn engine_allowance_cannot_credit_other_counters_crates_or_unknown_dependencies() {
        let mut limits = BTreeMap::new();
        engine_os(&mut limits);
        assert_eq!(limits.len(), 7);
        assert!(check_limits(&limits, &limits, &BTreeMap::new(), true).is_ok());
        for key in [
            "layers|crate_os|uqm::sdl2",
            "layers|crate_os|uqm-engine::unknown",
            "ffi|static_mut|uqm-engine::uqm-engine",
            "allows|clippy::all|uqm-engine::graphics",
        ] {
            let mut current = limits.clone();
            current.insert(key.into(), 1);
            assert!(check_limits(&current, &limits, &BTreeMap::new(), true).is_err());
        }
        let mut current = limits.clone();
        current.insert("layers|crate_os|uqm-engine::sdl2".into(), 2);
        assert!(check_limits(&current, &limits, &BTreeMap::new(), true).is_err());
    }
}
