//! Descriptor for the noncombat Ur-Quan drone.

pub(crate) mod engine {

    fn descriptor(_world: &mut uqm_engine::battle::World) -> uqm_engine::battle::RaceDesc {
        let d = uqm_engine::battle::template::RaceTemplate {
            info: uqm_engine::battle::views::Info {
                crew_level: 1,
                max_crew: 1,
                energy_level: 1,
                max_energy: 1,
                ..Default::default()
            },
            resources: uqm_engine::battle::template::RaceResources {
                melee_icon: Some(c"ship.drone.meleeicons"),
                ..Default::default()
            },
            ..Default::default()
        };
        d.descriptor(&PROBE)
    }

    pub static PROBE: uqm_engine::battle::Race = uqm_engine::battle::Race {
        descriptor,
        preprocess: None,
        postprocess: None,
        init_weapon: None,
        intelligence: None,
        uninit: None,
    };
}
