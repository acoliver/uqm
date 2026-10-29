//! Descriptor for the noncombat Ur-Quan drone.

#[cfg(not(test))]
pub(crate) mod engine {

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let d = crate::engine::battle::template::RaceTemplate {
            info: crate::engine::battle::views::Info {
                crew_level: 1,
                max_crew: 1,
                energy_level: 1,
                max_energy: 1,
                ..Default::default()
            },
            resources: crate::engine::battle::template::RaceResources {
                melee_icon: Some(c"ship.drone.meleeicons"),
                ..Default::default()
            },
            ..Default::default()
        };
        d.descriptor(&PROBE)
    }

    pub static PROBE: crate::engine::battle::Race = crate::engine::battle::Race {
        descriptor,
        preprocess: None,
        postprocess: None,
        init_weapon: None,
        intelligence: None,
        uninit: None,
    };
}
