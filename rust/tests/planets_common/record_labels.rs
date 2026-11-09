//! Preserve the C-captured fixture schema independently of Rust record names.

pub fn captured_labels(mut text: String) -> String {
    for (rust, captured) in [
        ("ElementEntry {", "ELEMENT_ENTRY {"),
        ("PlanetInfo {", "PLANET_INFO {"),
        ("SystemInfo {", "SYSTEM_INFO {"),
        ("PlanetDescriptor {", "planet_desc {"),
        ("NodeRecord {", "node_info {"),
        ("element_type:", "ElementType:"),
        ("base_tectonics:", "BaseTectonics:"),
        ("atmo_and_density:", "AtmoAndDensity:"),
        ("useful_elements:", "UsefulElements:"),
        ("cmap_instance:", "CMapInstance:"),
        ("xlat_tab_instance:", "XlatTabInstance:"),
        ("axial_tilt:", "AxialTilt:"),
        ("tectonics:", "Tectonics:"),
        ("weather:", "Weather:"),
        ("planet_density:", "PlanetDensity:"),
        ("planet_radius:", "PlanetRadius:"),
        ("gravity:", "SurfaceGravity:"),
        ("temperature:", "SurfaceTemperature:"),
        ("rotation_period:", "RotationPeriod:"),
        ("atmo_density:", "AtmoDensity:"),
        ("life_chance:", "LifeChance:"),
        ("planet_to_sun_dist:", "PlanetToSunDist:"),
        ("plan_data_ptr:", "PlanDataPtr:"),
        ("scan_seed:", "ScanSeed:"),
        ("scan_retrieve_mask:", "ScanRetrieveMask:"),
        ("discovery_string:", "DiscoveryString:"),
        ("star_size:", "StarSize:"),
        ("star_intensity:", "StarIntensity:"),
        ("star_energy:", "StarEnergy:"),
        ("planet_info:", "PlanetInfo:"),
        ("num_planets:", "NumPlanets:"),
        ("next_index:", "NextIndex:"),
    ] {
        text = text.replace(rust, captured);
    }
    // Generation captures contain no presentation assets. Preserve the retired C columns here.
    text.replace(
        "DiscoveryString: 0x0",
        "DiscoveryString: 0x0, LanderFont: 0x0, LanderFontEff: 0x0",
    )
}
