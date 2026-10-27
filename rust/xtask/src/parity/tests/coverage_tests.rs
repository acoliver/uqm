use super::*;

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
    observed.validate().unwrap();
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
        assert!(observed.validate().is_err(), "accepted {hooks}");
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
    observed.validate().unwrap();
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
    assert!(observed.validate().is_err());
    observed
        .observe("S 1 512 00001234 abc [spawn 0 1] [free 0:1] [spawn 0 2]\n")
        .unwrap();
    assert!(observed.validate().is_err());
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
