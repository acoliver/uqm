//! Stable callback entries.
#[cfg(not(test))]
pub(super) fn hooks() -> Vec<(usize, &'static str)> {
    [part0(), part1(), part2()].into_iter().flatten().collect()
}
#[cfg(not(test))]
fn part0() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::battle::ship_native::engine::animation_preprocess as *const () as usize,
            "battle::ship_native::animation_preprocess",
        ),
        (
            crate::battle::ship_native::engine::collision as *const () as usize,
            "battle::ship_native::collision",
        ),
        (
            crate::battle::ship_native::engine::inertial_thrust as *const () as usize,
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
            crate::battle::world_elements::engine::abandon_ship as *const () as usize,
            "battle::world_elements::AbandonShip",
        ),
    ]
}
#[cfg(not(test))]
fn part1() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::battle::world_elements::engine::asteroid_preprocess as *const () as usize,
            "battle::world_elements::asteroid_preprocess",
        ),
        (
            crate::battle::world_elements::engine::crew_collision as *const () as usize,
            "battle::world_elements::crew_collision",
        ),
        (
            crate::battle::world_elements::engine::crew_preprocess as *const () as usize,
            "battle::world_elements::crew_preprocess",
        ),
        (
            crate::battle::world_elements::engine::gravity_postprocess as *const () as usize,
            "battle::world_elements::gravity_postprocess",
        ),
        (
            crate::battle::world_elements::engine::spawn_asteroid as *const () as usize,
            "battle::world_elements::spawn_asteroid",
        ),
        (
            crate::battle::world_elements::engine::spawn_rubble as *const () as usize,
            "battle::world_elements::spawn_rubble",
        ),
        (
            crate::battle::weapon_native::engine::modify_silhouette as *const () as usize,
            "battle::weapon_native::ModifySilhouette",
        ),
        (
            crate::battle::weapon_native::engine::track_ship as *const () as usize,
            "battle::weapon_native::track_ship",
        ),
        (
            crate::battle::weapon_native::engine::collision_cb as *const () as usize,
            "battle::weapon_native::collision_cb",
        ),
    ]
}
#[cfg(not(test))]
fn part2() -> Vec<(usize, &'static str)> {
    vec![(
        crate::battle::weapon_native::engine::weapon_collision as *const () as usize,
        "battle::weapon_native::weapon_collision",
    )]
}
