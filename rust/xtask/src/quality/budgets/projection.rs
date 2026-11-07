//! The retiring unit owns each measured legacy caller, never a workspace ceiling.

pub(super) fn ends(name: &str) -> Option<&'static str> {
    match name {
        "u2_10a_projection" | "u2_10b_projection" => Some("phase-2"),
        "u3_02_world_projection" => Some("U3-02"),
        "u4_02_world_projection" => Some("U4-02"),
        "u4_03_world_projection" => Some("U4-03"),
        "u4_04_world_projection" => Some("U4-04"),
        "u5_01_world_projection" => Some("U5-01"),
        "u5_02_world_projection" => Some("U5-02"),
        "u5_03_world_projection" => Some("U5-03"),
        "u5_04_world_projection" => Some("U5-04"),
        "u6_01_world_projection" => Some("U6-01"),
        "u6_02_world_projection" => Some("U6-02"),
        "u6_03_world_projection" => Some("U6-03"),
        "u6_04_world_projection" | "u6_04_content_edges" => Some("U6-04"),
        _ => None,
    }
}

pub(super) fn identity(name: &str, identity: &str) -> bool {
    let Some(scope) = identity.strip_prefix("uqm::") else {
        return false;
    };
    if [
        "state::",
        "game_init::",
        "navigation::globals::",
        "c_bindings::",
    ]
    .iter()
    .any(|prefix| scope.starts_with(prefix))
        || !scope.contains("::")
        || scope.ends_with("::")
    {
        return false;
    }
    let prefixes: &[&str] = match name {
        "u2_10a_projection" => &["battle::"],
        "u2_10b_projection" => &["ships::"],
        "u3_02_world_projection" => &["supermelee::native::", "supermelee::setup::", "pickship::"],
        "u4_02_world_projection" => &[
            "navigation::groups::",
            "time::clock_abi::",
            "time::clock_bridge::",
            "savegame::runtime::queues::",
        ],
        "u4_03_world_projection" => &["automation::"],
        "u4_04_world_projection" => &["mainloop::", "savegame::", "engine::"],
        "u5_01_world_projection" => &["planets::generate::", "planets::generate_all::"],
        "u5_02_world_projection" => &["planets::"],
        "u5_03_world_projection" => &["planet_side::"],
        "u5_04_world_projection" => &["navigation::"],
        "u6_01_world_projection" => &["comm::"],
        "u6_02_world_projection" => &["comm::races::"],
        "u6_03_world_projection" => &["starbase::", "ui::"],
        "u6_04_content_edges" => &["battle::", "ships::races::"],
        "u6_04_world_projection" => &[
            "front_end::",
            "mainloop::restart_menu::",
            "mainloop::state_abi::",
            "mainloop::input_abi::",
            "planets::cargo::",
            "planets::report::",
            "planets::sphere::",
            "comm::glue::",
            "comm::speech_graphics::",
        ],
        _ => &[],
    };
    prefixes.iter().any(|prefix| scope.starts_with(prefix))
}
