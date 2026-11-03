use super::*;
use coverage::{Absence, Coverage, Kind, SpeciesCallbacks, MELEE_SPECIES};

fn observed(trace: &str) -> Coverage {
    let mut coverage = Coverage::default();
    coverage.observe(trace).unwrap();
    coverage
}

fn fixture(absent_special: &[u32]) -> Vec<SpeciesCallbacks> {
    (1..=25)
        .map(|species| SpeciesCallbacks {
            species,
            weapon: Some("fixture_native::weapon"),
            special: (!absent_special.contains(&species)).then_some("fixture_native::special"),
        })
        .collect()
}

/// Every species fights both neighbours. `keep` selects which of each ship's declared
/// callbacks fire during those fights.
fn corpus(table: &[SpeciesCallbacks], keep: impl Fn(u32, Kind) -> bool) -> String {
    let callbacks = |species: u32| -> String {
        let entry = table.iter().find(|entry| entry.species == species).unwrap();
        [(Kind::Weapon, entry.weapon), (Kind::Special, entry.special)]
            .into_iter()
            .filter_map(|(kind, name)| name.filter(|_| keep(species, kind)))
            .map(|name| format!(" [hook {species} ships::races::{name}]"))
            .collect()
    };
    let mut trace = String::new();
    for species in 1..=25 {
        for opponent in [species % 25 + 1, (species + 1) % 25 + 1] {
            trace.push_str(&format!(
                "S 1 512 crc digest [spawn 0 {species}] [spawn 1 {opponent}] [hook {species} ships::races::fixture_native::intelligence] [hook {opponent} ships::races::fixture_native::intelligence]{}{}\n",
                callbacks(species),
                callbacks(opponent)
            ));
        }
    }
    trace
}

#[test]
fn bounded_engagements_require_both_live_races_and_allow_real_cleanup_without_deaths() {
    let mut trace = String::new();
    for species in 1..=25 {
        for opponent in [species % 25 + 1, (species + 1) % 25 + 1] {
            trace.push_str(&format!(
                "S 1 512 crc digest [spawn 0 {species}] [spawn 1 {opponent}]\n\
                 S 2 512 crc digest [hook {species} ships::races::arilou_native::arilou_intelligence] [hook {opponent} ships::races::human_native::initialize_missile]\n\
                 S 3 512 crc digest [free 0:1] [free 1:1] [drawable-free drawable:1]\n"
            ));
        }
    }
    let mut observed = coverage::Coverage::default();
    observed.observe(&trace).unwrap();
    observed.validate_bilateral().unwrap();
}

#[test]
fn spawns_deaths_one_sided_generic_stale_and_cross_bout_hooks_never_prove_engagement() {
    for hooks in [
        "[death 0 {a}] [death 1 {b}]",
        "[hook {a} ships::races::arilou_native::arilou_intelligence]",
        "[hook {a} battle::cyborg_native::ship_intelligence] [hook {b} battle::ship_native::ship_postprocess]",
        "[hook {a} ships::races::arilou_native::arilou_intelligence] [spawn 1 {b}] [hook {b} ships::races::human_native::initialize_missile]",
        "[death 0 {a}] [hook {a} ships::races::arilou_native::arilou_intelligence] [hook {b} ships::races::human_native::initialize_missile]",
        "[hook 26 ships::races::samatra_native::samatra_postprocess] [hook {b} ships::races::human_native::initialize_missile]",
    ] {
        let mut observed = coverage::Coverage::default();
        for species in 1..=25 {
            for opponent in [species % 25 + 1, (species + 1) % 25 + 1] {
                let events = hooks
                    .replace("{a}", &species.to_string())
                    .replace("{b}", &opponent.to_string());
                observed.observe(&format!(
                    "S 1 512 crc digest [spawn 0 {species}] [spawn 1 {opponent}] {events}\n"
                )).unwrap();
            }
        }
        assert!(observed.validate_bilateral().is_err(), "accepted {hooks}");
    }
}

#[test]
fn natural_deaths_must_match_live_species_and_reset_engagement_hooks() {
    for events in [
        "[death 0 1]",
        "[spawn 0 1] [death 0 2]",
        "[spawn 0 1] [death 0 1] [death 0 1]",
        "[spawn 0 1] [spawn 0 2] [death 0 1]",
    ] {
        assert!(
            coverage::Coverage::default()
                .observe(&format!("S 1 512 crc digest {events}\n"))
                .is_err(),
            "accepted {events}"
        );
    }
}
#[test]
fn journal_cleanup_events_preserve_spawn_and_death_coverage() {
    let mut observed = coverage::Coverage::default();
    for species in 1..=25 {
        for opponent in [species % 25 + 1, (species + 1) % 25 + 1] {
            observed
                .observe(&format!(
                    "I 1 512 menu=none;players=none\n\
                     S 1 512 00001234 abc [spawn 0 {species}] [spawn 1 {opponent}] [pool-full]\n\
                     S 2 512 00001234 def [hook {species} ships::races::arilou_native::arilou_intelligence] [hook {opponent} ships::races::human_native::initialize_missile] [hook {species} battle::ship_death] [death 0 {species}] [free 0:1] [free 0:2] [drawable-free drawable:0]\n\
                     S 3 512 00001234 ghi [hook {opponent} battle::ship_death] [death 1 {opponent}] [free 18446744073709551615:18446744073709551615] [free none] [drawable-free drawable:18446744073709551615] [drawable-free none]\n"
                ))
                .unwrap();
        }
    }
    observed.validate_bilateral().unwrap();
}

#[test]
fn journal_cleanup_does_not_count_as_death_or_opponent_evidence() {
    let mut observed = coverage::Coverage::default();
    for species in 1..=25 {
        for opponent in [species % 25 + 1, (species + 1) % 25 + 1] {
            observed
                .observe(&format!(
                    "S 1 512 00001234 abc [spawn 0 {species}] [spawn 1 {opponent}] [free 0:1] [drawable-free drawable:0]\n"
                ))
                .unwrap();
        }
    }
    assert!(observed.validate_bilateral().is_err());
    observed
        .observe("S 1 512 00001234 abc [spawn 0 1] [free 0:1] [spawn 0 2]\n")
        .unwrap();
    assert!(observed.validate_bilateral().is_err());
    assert!(observed
        .observe("S 1 512 00001234 abc [free 0:1] [death 0 1]\n")
        .is_err());
}

#[test]
fn malformed_cleanup_and_unknown_events_still_fail() {
    for event in [
        "free",
        "free 0",
        "free 0:0",
        "free :1",
        "free 0:",
        "free -1:1",
        "free 1:+1",
        "free 0:18446744073709551616",
        "free 1:2:3",
        "free drawable:1",
        "free 0:1 extra",
        "drawable-free",
        "drawable-free 0:1",
        "drawable-free drawable:",
        "drawable-free drawable:-1",
        "drawable-free drawable:18446744073709551616",
        "drawable-free drawable:1 extra",
        "drawable-free other:0",
        "unknown 0:1",
        "pool-full extra",
    ] {
        assert!(
            coverage::Coverage::default()
                .observe(&format!("S 1 512 00001234 abc [{event}]\n"))
                .is_err(),
            "accepted {event}"
        );
    }
    assert!(coverage::Coverage::default()
        .observe("S 1 512 00001234 abc [free 0:1\n")
        .is_err());
}

#[test]
fn species_callbacks_require_weapon_and_special_evidence_for_every_species() {
    let table = fixture(&[]);
    observed(&corpus(&table, |_, _| true))
        .validate_with(&table, &[])
        .unwrap();
    for kind in [Kind::Weapon, Kind::Special] {
        let trace = corpus(&table, |_, removed| removed != kind);
        assert!(
            observed(&trace).validate_with(&table, &[]).is_err(),
            "accepted corpus without any {kind:?} callback"
        );
    }
    for species in 1..=25 {
        for kind in [Kind::Weapon, Kind::Special] {
            let trace = corpus(&table, |id, removed| !(id == species && removed == kind));
            assert!(
                observed(&trace).validate_with(&table, &[]).is_err(),
                "accepted species {species} without {kind:?} callback"
            );
        }
    }
}

#[test]
fn intelligence_only_engagements_fail_the_callback_requirement() {
    let table = fixture(&[]);
    let trace = corpus(&table, |_, _| false);
    observed(&trace).validate_bilateral().unwrap();
    assert!(observed(&trace).validate_with(&table, &[]).is_err());
}

#[test]
fn only_plan_approved_absences_may_omit_a_special_callback() {
    let table = fixture(&[3]);
    let trace = corpus(&table, |_, _| true);
    assert!(
        observed(&trace).validate_with(&table, &[]).is_err(),
        "accepted an undeclared absence"
    );
    let approved = [Absence {
        species: 3,
        kind: Kind::Special,
        plan: "plan.md:113",
    }];
    observed(&trace).validate_with(&table, &approved).unwrap();
    let stale = [Absence {
        species: 4,
        kind: Kind::Special,
        plan: "plan.md:113",
    }];
    let full = fixture(&[]);
    assert!(
        observed(&corpus(&full, |_, _| true))
            .validate_with(&full, &stale)
            .is_err(),
        "accepted an approval for a species that declares its special"
    );
    let unknown = [Absence {
        species: 26,
        kind: Kind::Special,
        plan: "plan.md:113",
    }];
    assert!(observed(&trace).validate_with(&table, &unknown).is_err());
}

#[test]
fn melee_callback_table_names_each_species_once_in_order() {
    let species: Vec<u32> = MELEE_SPECIES.iter().map(|entry| entry.species).collect();
    assert_eq!(species, (1..=25).collect::<Vec<_>>());
}
