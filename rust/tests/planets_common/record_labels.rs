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
        ("star_size:", "StarSize:"),
        ("star_intensity:", "StarIntensity:"),
        ("star_energy:", "StarEnergy:"),
        ("planet_info:", "PlanetInfo:"),
        ("num_planets:", "NumPlanets:"),
        ("next_index:", "NextIndex:"),
        ("prev_desc:", "pPrevDesc:"),
        ("Point {", "point {"),
        ("Stamp {", "stamp {"),
        ("frame: FrameId(None)", "frame: 0x0"),
    ] {
        text = text.replace(rust, captured);
    }
    // Generation captures contain no presentation assets. Preserve the retired
    // C columns, which follow the scan masks, here.
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(at) = rest.find("ScanRetrieveMask: [") {
        let end = at
            + rest[at..]
                .find(']')
                .unwrap_or_else(|| panic!("unterminated scan mask"))
            + 1;
        out.push_str(&rest[..end]);
        out.push_str(", DiscoveryString: 0x0, LanderFont: 0x0, LanderFontEff: 0x0");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}
