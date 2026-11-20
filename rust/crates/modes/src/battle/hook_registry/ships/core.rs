//! Stable callback entries.
pub(super) fn hooks() -> Vec<(usize, &'static str)> {
    [part0(), part1(), part2()].into_iter().flatten().collect()
}
fn part0() -> Vec<(usize, &'static str)> {
    vec![
        (
            uqm_engine::battle::hooks::animation_preprocess as *const () as usize,
            "battle::ship_native::animation_preprocess",
        ),
        (
            uqm_engine::battle::motion::collision as *const () as usize,
            "battle::ship_native::collision",
        ),
        (
            uqm_engine::battle::motion::inertial_thrust as *const () as usize,
            "battle::ship_native::inertial_thrust",
        ),
        (
            crate::battle::ship_native::engine::ship_postprocess as *const () as usize,
            "battle::ship_native::ship_postprocess",
        ),
        (
            crate::battle::ship_native::engine::ship_preprocess as *const () as usize,
            "battle::ship_native::ship_preprocess",
        ),
        (
            uqm_engine::battle::world_elements::abandon_ship as *const () as usize,
            "battle::world_elements::AbandonShip",
        ),
    ]
}
fn part1() -> Vec<(usize, &'static str)> {
    vec![
        (
            uqm_engine::battle::world_elements::asteroid_preprocess as *const () as usize,
            "battle::world_elements::asteroid_preprocess",
        ),
        (
            uqm_engine::battle::world_elements::crew_collision as *const () as usize,
            "battle::world_elements::crew_collision",
        ),
        (
            uqm_engine::battle::world_elements::crew_preprocess as *const () as usize,
            "battle::world_elements::crew_preprocess",
        ),
        (
            uqm_engine::battle::world_elements::gravity_postprocess as *const () as usize,
            "battle::world_elements::gravity_postprocess",
        ),
        (
            uqm_engine::battle::world_elements::spawn_asteroid as *const () as usize,
            "battle::world_elements::spawn_asteroid",
        ),
        (
            uqm_engine::battle::world_elements::spawn_rubble as *const () as usize,
            "battle::world_elements::spawn_rubble",
        ),
        (
            crate::battle::weapon_native::engine::modify_silhouette as *const () as usize,
            "battle::weapon_native::ModifySilhouette",
        ),
        (
            uqm_engine::battle::tracking::track_ship as *const () as usize,
            "battle::weapon_native::track_ship",
        ),
        (
            uqm_engine::battle::weapon::collision_cb as *const () as usize,
            "battle::weapon_native::collision_cb",
        ),
    ]
}
fn part2() -> Vec<(usize, &'static str)> {
    vec![(
        uqm_engine::battle::weapon::weapon_collision as *const () as usize,
        "battle::weapon_native::weapon_collision",
    )]
}
