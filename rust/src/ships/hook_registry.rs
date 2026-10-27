//! Stable callback names. Addresses are lookup keys only and never serialized.

mod assigned;
mod ships;
#[cfg(not(test))]
pub fn hooks() -> Vec<(usize, &'static str)> {
    ships::hooks()
        .into_iter()
        .chain(assigned::hooks())
        .collect()
}

#[cfg(test)]
pub fn hooks() -> Vec<(usize, &'static str)> {
    Vec::new()
}

pub fn name(hooks: &[(usize, &'static str)], address: usize) -> &'static str {
    name_for_species(hooks, address, 0)
}

pub fn name_for_species(
    hooks: &[(usize, &'static str)],
    address: usize,
    species: u32,
) -> &'static str {
    if address == 0 {
        return "none";
    }
    const RACES: [&str; 29] = [
        "",
        "arilou",
        "chmmr",
        "human",
        "orz",
        "pkunk",
        "shofixti",
        "spathi",
        "supox",
        "thraddash",
        "utwig",
        "vux",
        "yehat",
        "melnorme",
        "druuge",
        "ilwrath",
        "mycon",
        "slylandro",
        "umgah",
        "urquan",
        "zoqfot",
        "syreen",
        "blackurq",
        "androsynth",
        "chenjesu",
        "mmrnmhrm",
        "sis",
        "samatra",
        "slylandro",
    ];
    let race = RACES.get(species as usize).copied().unwrap_or("");
    let prefix = format!("ships::races::{race}_native::");
    // Linkers may fold equal machine bodies. The live owner disambiguates the
    // declared race callback without treating its machine address as identity.
    hooks
        .iter()
        .filter(|(key, _)| *key == address)
        .min_by_key(|(_, name)| (!name.starts_with(&prefix), *name))
        .map(|(_, name)| *name)
        .unwrap_or_else(|| panic!("unregistered callback at {address:#x}, species {species}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn folded_race_callbacks_use_the_live_declared_owner() {
        let merged = [
            (42, "ships::races::pkunk_native::uninit_pkunk"),
            (42, "ships::races::androsynth_native::uninit_androsynth"),
        ];
        let distinct = [(41, merged[0].1), (43, merged[1].1)];
        assert_eq!(
            super::name_for_species(&merged, 42, 23),
            super::name_for_species(&distinct, 43, 23)
        );
        assert_eq!(
            super::name_for_species(&merged, 42, 5),
            super::name_for_species(&distinct, 41, 5)
        );
    }

    #[test]
    fn callback_names_are_resolved_and_unknown_callbacks_fail() {
        let entries = [(42, "ships::vux::limpet")];
        assert_eq!(super::name(&entries, 0), "none");
        assert_eq!(super::name(&entries, 42), "ships::vux::limpet");
        assert!(std::panic::catch_unwind(|| super::name(&entries, 43)).is_err());
    }
}
