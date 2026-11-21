//! Callback lookup tables; raw addresses never leave the recorder.
pub(super) fn hooks() -> Vec<(usize, &'static str)> {
    [part0(), part1(), part2(), status()]
        .into_iter()
        .flatten()
        .collect()
}
fn part0() -> Vec<(usize, &'static str)> {
    vec![
        (
            uqm_modes::battle::tactrans_native::engine::find_alive_ship as *const () as usize,
            "battle::tactrans_native::FindAliveStarShip",
        ),
        (
            uqm_modes::battle::tactrans_native::engine::start_ship_explosion as *const () as usize,
            "battle::tactrans_native::StartShipExplosion",
        ),
        (
            uqm_modes::battle::tactrans_native::engine::cleanup_dead_ship as *const () as usize,
            "battle::tactrans_native::cleanup_dead_ship",
        ),
        (
            uqm_modes::battle::tactrans_native::engine::cycle_ion_trail as *const () as usize,
            "battle::tactrans_native::cycle_ion_trail",
        ),
        (
            uqm_modes::battle::tactrans_native::engine::explosion_preprocess as *const () as usize,
            "battle::tactrans_native::explosion_preprocess",
        ),
        (
            uqm_modes::battle::tactrans_native::engine::flee_preprocess as *const () as usize,
            "battle::tactrans_native::flee_preprocess",
        ),
        (
            uqm_modes::battle::tactrans_native::engine::new_ship as *const () as usize,
            "battle::tactrans_native::new_ship",
        ),
        (
            uqm_modes::battle::tactrans_native::engine::preprocess_dead_ship as *const () as usize,
            "battle::tactrans_native::preprocess_dead_ship",
        ),
        (
            uqm_modes::battle::tactrans_native::engine::ship_death as *const () as usize,
            "battle::tactrans_native::ship_death",
        ),
        (
            uqm_modes::battle::tactrans_native::engine::ship_transition as *const () as usize,
            "battle::tactrans_native::ship_transition",
        ),
        (
            uqm_engine::battle::Cx::spawn_ion_trail as *const () as usize,
            "battle::tactrans_native::spawn_ion_trail",
        ),
    ]
}
fn part1() -> Vec<(usize, &'static str)> {
    vec![
        (
            uqm_modes::navigation::hyper_transition::hyper_preprocess as *const () as usize,
            "navigation::hyper_transition::hyper_preprocess",
        ),
        (
            uqm_modes::navigation::hyper_transition::hyper_transition as *const () as usize,
            "navigation::hyper_transition::hyper_transition",
        ),
        (
            uqm_modes::navigation::hyper_transition::portal_collision as *const () as usize,
            "navigation::hyper_transition::portal_collision",
        ),
        (
            uqm_modes::navigation::hyper_transition::portal_death as *const () as usize,
            "navigation::hyper_transition::portal_death",
        ),
        (
            uqm_modes::navigation::hyper_transition::star_collision as *const () as usize,
            "navigation::hyper_transition::star_collision",
        ),
        (
            uqm_modes::navigation::hyper_transition::star_death as *const () as usize,
            "navigation::hyper_transition::star_death",
        ),
        (
            uqm_modes::navigation::hyper_transition::unhyper as *const () as usize,
            "navigation::hyper_transition::unhyper",
        ),
        (
            uqm_modes::navigation::hyper_encounters::collision as *const () as usize,
            "navigation::hyper_encounters::collision",
        ),
        (
            uqm_modes::navigation::hyper_encounters::transition as *const () as usize,
            "navigation::hyper_encounters::transition",
        ),
        (
            uqm_modes::navigation::interplanetary::live::flagship_collision as *const () as usize,
            "navigation::interplanetary::flagship_collision",
        ),
        (
            uqm_modes::navigation::interplanetary::live::flagship_preprocess as *const () as usize,
            "navigation::interplanetary::flagship_preprocess",
        ),
        (
            uqm_modes::navigation::interplanetary::live::group_collision as *const () as usize,
            "navigation::interplanetary::group_collision",
        ),
    ]
}
fn part2() -> Vec<(usize, &'static str)> {
    vec![(
        uqm_modes::navigation::interplanetary::live::group_preprocess as *const () as usize,
        "navigation::interplanetary::group_preprocess",
    )]
}

fn status() -> Vec<(usize, &'static str)> {
    vec![
        (
            uqm_engine::battle::statistics::delta_crew as *const () as usize,
            "battle::status_native::DeltaCrew",
        ),
        (
            uqm_engine::battle::statistics::delta_energy as *const () as usize,
            "battle::status_native::DeltaEnergy",
        ),
        (
            uqm_modes::battle::status_native::engine::preprocess_status as *const () as usize,
            "battle::status_native::PreProcessStatus",
        ),
        (
            uqm_modes::battle::status_native::engine::post_process_status as *const () as usize,
            "battle::status_native::PostProcessStatus",
        ),
    ]
}
