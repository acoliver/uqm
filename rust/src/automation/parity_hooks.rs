//! Callback lookup tables; raw addresses never leave the recorder.
#[cfg(not(test))]
pub(super) fn hooks() -> Vec<(usize, &'static str)> {
    [part0(), part1(), part2(), status()]
        .into_iter()
        .flatten()
        .collect()
}
#[cfg(not(test))]
fn part0() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::battle::tactrans_native::engine::FindAliveStarShip as *const () as usize,
            "battle::tactrans_native::FindAliveStarShip",
        ),
        (
            crate::battle::tactrans_native::engine::RecordShipDeath as *const () as usize,
            "battle::tactrans_native::RecordShipDeath",
        ),
        (
            crate::battle::tactrans_native::engine::StartShipExplosion as *const () as usize,
            "battle::tactrans_native::StartShipExplosion",
        ),
        (
            crate::battle::tactrans_native::engine::cleanup_dead_ship as *const () as usize,
            "battle::tactrans_native::cleanup_dead_ship",
        ),
        (
            crate::battle::tactrans_native::engine::cycle_ion_trail as *const () as usize,
            "battle::tactrans_native::cycle_ion_trail",
        ),
        (
            crate::battle::tactrans_native::engine::explosion_preprocess as *const () as usize,
            "battle::tactrans_native::explosion_preprocess",
        ),
        (
            crate::battle::tactrans_native::engine::flee_preprocess as *const () as usize,
            "battle::tactrans_native::flee_preprocess",
        ),
        (
            crate::battle::tactrans_native::engine::new_ship as *const () as usize,
            "battle::tactrans_native::new_ship",
        ),
        (
            crate::battle::tactrans_native::engine::preprocess_dead_ship as *const () as usize,
            "battle::tactrans_native::preprocess_dead_ship",
        ),
        (
            crate::battle::tactrans_native::engine::ship_death as *const () as usize,
            "battle::tactrans_native::ship_death",
        ),
        (
            crate::battle::tactrans_native::engine::ship_transition as *const () as usize,
            "battle::tactrans_native::ship_transition",
        ),
        (
            crate::battle::tactrans_native::engine::spawn_ion_trail as *const () as usize,
            "battle::tactrans_native::spawn_ion_trail",
        ),
    ]
}
#[cfg(not(test))]
fn part1() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::navigation::hyper_transition::hyper_preprocess as *const () as usize,
            "navigation::hyper_transition::hyper_preprocess",
        ),
        (
            crate::navigation::hyper_transition::hyper_transition as *const () as usize,
            "navigation::hyper_transition::hyper_transition",
        ),
        (
            crate::navigation::hyper_transition::portal_collision as *const () as usize,
            "navigation::hyper_transition::portal_collision",
        ),
        (
            crate::navigation::hyper_transition::portal_death as *const () as usize,
            "navigation::hyper_transition::portal_death",
        ),
        (
            crate::navigation::hyper_transition::star_collision as *const () as usize,
            "navigation::hyper_transition::star_collision",
        ),
        (
            crate::navigation::hyper_transition::star_death as *const () as usize,
            "navigation::hyper_transition::star_death",
        ),
        (
            crate::navigation::hyper_transition::unhyper as *const () as usize,
            "navigation::hyper_transition::unhyper",
        ),
        (
            crate::navigation::hyper_encounters::collision as *const () as usize,
            "navigation::hyper_encounters::collision",
        ),
        (
            crate::navigation::hyper_encounters::transition as *const () as usize,
            "navigation::hyper_encounters::transition",
        ),
        (
            crate::navigation::interplanetary::live::flagship_collision as *const () as usize,
            "navigation::interplanetary::flagship_collision",
        ),
        (
            crate::navigation::interplanetary::live::flagship_preprocess as *const () as usize,
            "navigation::interplanetary::flagship_preprocess",
        ),
        (
            crate::navigation::interplanetary::live::group_collision as *const () as usize,
            "navigation::interplanetary::group_collision",
        ),
    ]
}
#[cfg(not(test))]
fn part2() -> Vec<(usize, &'static str)> {
    vec![(
        crate::navigation::interplanetary::live::group_preprocess as *const () as usize,
        "navigation::interplanetary::group_preprocess",
    )]
}

#[cfg(not(test))]
fn status() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::battle::status_native::engine::DeltaCrew as *const () as usize,
            "battle::status_native::DeltaCrew",
        ),
        (
            crate::battle::status_native::engine::DeltaEnergy as *const () as usize,
            "battle::status_native::DeltaEnergy",
        ),
        (
            crate::battle::status_native::engine::PreProcessStatus as *const () as usize,
            "battle::status_native::PreProcessStatus",
        ),
        (
            crate::battle::status_native::engine::PostProcessStatus as *const () as usize,
            "battle::status_native::PostProcessStatus",
        ),
    ]
}
