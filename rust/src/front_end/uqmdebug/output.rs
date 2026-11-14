//! Diagnostic bytes are written to the host's supplied streams.

use super::*;
use crate::engine::world::planet::PlanetFrame;
use crate::engine::{clock::CEvent, world::CStarDesc};
use crate::planets::tables::PLANET_ARRAY;
use crate::starbase::ids::STAR_NUMBER_BASE;
use std::io::{self, Write};

pub fn dump_event(out: &mut impl Write, event: &CEvent) -> io::Result<()> {
    writeln!(
        out,
        "{:4}/{:02}/{:02}: {}",
        event.year_index,
        event.month_index,
        event.day_index,
        event_name(event.func_index)
    )
}

pub fn dump_events(world: &World, out: &mut impl Write) -> io::Result<()> {
    let events = world.clock.events_ref();
    let mut next = events.head();
    while let Some(id) = next {
        next = events.succ(id);
        dump_event(out, events.get(id))?;
    }
    Ok(())
}

fn padded(out: &mut impl Write, bytes: &[u8], width: usize) -> io::Result<()> {
    out.write_all(bytes)?;
    for _ in bytes.len()..width {
        out.write_all(b" ")?;
    }
    Ok(())
}

pub fn dump_system(
    kernel: &Kernel,
    out: &mut impl Write,
    star: &CStarDesc,
    _: &SolarSystem,
) -> io::Result<()> {
    let name =
        crate::planets::starmap::naming::cluster_name(kernel, star, STAR_NUMBER_BASE as usize);
    padded(out, name.as_bytes(), 22)?;
    let color = body_color_string(crate::planets::generate::consts::star_color(star.star_type));
    let kind = star_type_string(crate::planets::generate::consts::star_type(star.star_type));
    write!(
        out,
        "  ({:3}.{}, {:3}.{}) ",
        star.star_pt.x / 10,
        star.star_pt.x % 10,
        star.star_pt.y / 10,
        star.star_pt.y % 10
    )?;
    let mut description = format!("{color} {kind}").into_bytes();
    description.truncate(39);
    padded(out, &description, 19)?;
    writeln!(out, "  {}", star_presence_string(star.index))
}

pub fn planet_type_string(kernel: &Kernel, index: i32) -> Vec<u8> {
    let mut bytes = if index >= FIRST_GAS_GIANT as i32 {
        kernel
            .game_text(SCAN_STRING_BASE as usize + 55)
            .to_bytes()
            .to_vec()
    } else {
        let mut bytes = kernel
            .game_text((SCAN_STRING_BASE as i32 + 4 + index) as usize)
            .to_bytes()
            .to_vec();
        bytes.push(b' ');
        bytes.extend_from_slice(kernel.game_text(SCAN_STRING_BASE as usize + 54).to_bytes());
        bytes
    };
    bytes.truncate(39);
    bytes
}

pub fn tectonics_string(value: u8) -> String {
    match u32::from(value) {
        NO_TECTONICS => "none".into(),
        LOW_TECTONICS => "low".into(),
        MED_TECTONICS => "medium".into(),
        HIGH_TECTONICS => "high".into(),
        SUPER_TECTONICS => "super".into(),
        _ => value.to_string(),
    }
}

pub fn dump_planet<G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    out: &mut impl Write,
    solar: &mut SolarSystem,
    body: Body,
    generation: &mut G,
) -> io::Result<()> {
    generation.invoke(cx, io, solar, |g, s, h| g.generate_name(s, body, h));
    out.write_all(b"- ")?;
    let name = &cx.world.data.flagship.planet_name;
    let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    padded(out, &name[..end], 37)?;
    out.write_all(b"  ")?;
    out.write_all(&planet_type_string(
        cx.kernel,
        i32::from(solar.sys.descriptor(body).data_index & !0x80),
    ))?;
    out.write_all(b"\n")?;
    dump_world(cx, io, out, solar, body, generation)
}

pub fn dump_moon<G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    out: &mut impl Write,
    solar: &mut SolarSystem,
    body: Body,
    generation: &mut G,
) -> io::Result<()> {
    let Body::Moon { moon, .. } = body else {
        panic!("moon visitor requires a moon")
    };
    write!(out, "  - Moon {:<30}  ", char::from(b'a' + moon))?;
    let kind = solar.sys.descriptor(body).data_index;
    match u32::from(kind) {
        HIERARCHY_STARBASE => out.write_all(b"StarBase")?,
        SA_MATRA => out.write_all(b"Sa-Matra")?,
        _ => out.write_all(&planet_type_string(cx.kernel, i32::from(kind & !0x80)))?,
    }
    out.write_all(b"\n")?;
    dump_world(cx, io, out, solar, body, generation)
}

fn dump_world<G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    out: &mut impl Write,
    solar: &mut SolarSystem,
    body: Body,
    generation: &mut G,
) -> io::Result<()> {
    let kind = solar.sys.descriptor(body).data_index;
    if u32::from(kind) == HIERARCHY_STARBASE || u32::from(kind) == SA_MATRA {
        return Ok(());
    }
    let info = solar.sys.info.planet_info;
    for (name, value) in [
        ("AxialTilt: ", i32::from(info.axial_tilt)),
        ("Tectonics: ", i32::from(info.tectonics)),
        ("Weather:   ", i32::from(info.weather)),
        ("Density:   ", i32::from(info.planet_density)),
        ("Radius:    ", i32::from(info.planet_radius)),
        ("Gravity:   ", i32::from(info.gravity)),
        ("Temp:      ", i32::from(info.temperature)),
        ("Day:       ", i32::from(info.rotation_period)),
        ("Atmosphere:", i32::from(info.atmo_density)),
        ("LifeChance:", i32::from(info.life_chance)),
        ("DistToSun: ", i32::from(info.planet_to_sun_dist)),
    ] {
        writeln!(out, "          {name} {value}")?;
    }
    if kind & 0x80 == 0 {
        let bio = calculate_bio_value(cx, io, solar, body, generation);
        let mineral = calculate_mineral_value(cx, io, solar, body, generation);
        writeln!(out, "          Bio: {bio:4}    Min: {mineral:4}")?;
    }
    Ok(())
}

pub fn dump_universe<G: Generation, W: Write>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    generation: &mut G,
    out: &mut W,
) -> io::Result<()> {
    universe_recurse(
        cx,
        io,
        generation,
        &mut UniverseRecurseArg {
            system_func_pre: Some(|kernel, star, solar, out| dump_system(kernel, out, star, solar)),
            system_func_post: None,
            planet_func_pre: Some(|cx, io, solar, body, generation, out| {
                dump_planet(cx, io, out, solar, body, generation)
            }),
            planet_func_post: None,
            moon_func: Some(|cx, io, solar, body, generation, out| {
                dump_moon(cx, io, out, solar, body, generation)
            }),
            arg: out,
        },
    )
}

struct ResourceTally<'a, W> {
    out: &'a mut W,
    minerals: u16,
    bio: u16,
}

pub fn tally_resources<G: Generation, W: Write>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    generation: &mut G,
    out: &mut W,
) -> io::Result<()> {
    fn tally<G: Generation, W: Write>(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        solar: &mut SolarSystem,
        body: Body,
        generation: &mut G,
        tally: &mut ResourceTally<'_, W>,
    ) -> io::Result<()> {
        let kind = solar.sys.descriptor(body).data_index;
        if u32::from(kind) != HIERARCHY_STARBASE && u32::from(kind) != SA_MATRA {
            tally.bio = tally
                .bio
                .wrapping_add(calculate_bio_value(cx, io, solar, body, generation));
            tally.minerals = tally
                .minerals
                .wrapping_add(calculate_mineral_value(cx, io, solar, body, generation));
        }
        Ok(())
    }
    let mut state = ResourceTally {
        out,
        minerals: 0,
        bio: 0,
    };
    universe_recurse(
        cx,
        io,
        generation,
        &mut UniverseRecurseArg {
            system_func_pre: Some(|_, _, _, tally: &mut ResourceTally<'_, W>| {
                tally.minerals = 0;
                tally.bio = 0;
                Ok(())
            }),
            system_func_post: Some(|kernel, star, _, tally| {
                let name = crate::planets::starmap::naming::cluster_name(
                    kernel,
                    star,
                    STAR_NUMBER_BASE as usize,
                );
                tally.out.write_all(name.as_bytes())?;
                writeln!(tally.out, "\t{}\t{}", tally.minerals, tally.bio)
            }),
            planet_func_pre: Some(tally::<G, W>),
            planet_func_post: None,
            moon_func: Some(tally::<G, W>),
            arg: &mut state,
        },
    )
}

pub fn dump_planet_type(
    kernel: &Kernel,
    out: &mut impl Write,
    index: i32,
    planet: &PlanetFrame,
) -> io::Result<()> {
    out.write_all(&planet_type_string(kernel, index))?;
    writeln!(out, "\n\tType: {}\n\tColor: {}\n\tSurface generation algoritm: {}\n\tTectonics: {}\n\tAtmosphere: {}\n\tDensity: {}\n\tElements:",
        world_size_string(planet.type_ & 3), body_color_string(planet.type_ >> 4), world_gen_algo_string(planet.type_ & 12),
        tectonics_string(planet.base_tectonics), atmosphere_string(planet.atmo_and_density >> 4), density_string(planet.atmo_and_density & 15))?;
    for entry in &planet.useful_elements {
        if entry.density == 0 {
            continue;
        }
        let quality = entry.density >> 4;
        write!(
            out,
            "\t\t0 to {} {}-quality (+{}) deposits of ",
            entry.density & 15,
            deposit_quality_string(quality),
            u32::from(quality) * 5
        )?;
        out.write_all(
            kernel
                .game_text(ELEMENTS_STRING_BASE as usize + entry.element_type as usize)
                .to_bytes(),
        )?;
        out.write_all(b" (")?;
        out.write_all(
            kernel
                .game_text(
                    CARGO_STRING_BASE as usize
                        + 2
                        + traversal::element_category(entry.element_type as usize),
                )
                .to_bytes(),
        )?;
        out.write_all(b")\n")?;
    }
    out.write_all(b"\n")
}

pub fn dump_planet_types(kernel: &Kernel, out: &mut impl Write) -> io::Result<()> {
    for (index, planet) in PLANET_ARRAY.iter().enumerate() {
        dump_planet_type(kernel, out, index as i32, planet)?;
    }
    Ok(())
}

pub fn dump_strings(
    kernel: &Kernel,
    out: &mut impl Write,
    errors: &mut impl Write,
) -> io::Result<()> {
    for pair in STRING_CATEGORIES.windows(2) {
        if pair[0].1 + pair[0].2 != pair[1].1 {
            errors
                .write_all(b"Error: String category list in dumpStrings() is not up to date.\n")?;
            return Ok(());
        }
    }
    let count = kernel.game_text_table().len();
    if GAMESTR_COUNT as usize != count {
        writeln!(
            errors,
            "Warning: GAMESTR_COUNT is {}, but GameStrings contains {} strings.",
            GAMESTR_COUNT, count
        )?;
    }
    let mut category = 0;
    for index in 0..count {
        let text = kernel.game_text(index);
        while category + 1 < STRING_CATEGORIES.len() && index >= STRING_CATEGORIES[category + 1].1 {
            category += 1;
        }
        write!(
            out,
            "[ {} + {} ]  ",
            STRING_CATEGORIES[category].0,
            index - STRING_CATEGORIES[category].1
        )?;
        out.write_all(text.to_bytes())?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

macro_rules! category {
    ($name:literal,$base:ident,$count:ident) => {
        ($name, $base as usize, $count as usize)
    };
}
const STRING_CATEGORIES: &[(&str, usize, usize)] = &[
    ("0", 0, 0),
    category!("STAR_STRING_BASE", STAR_STRING_BASE, STAR_STRING_COUNT),
    category!(
        "DEVICE_STRING_BASE",
        DEVICE_STRING_BASE,
        DEVICE_STRING_COUNT
    ),
    category!("CARGO_STRING_BASE", CARGO_STRING_BASE, CARGO_STRING_COUNT),
    category!(
        "ELEMENTS_STRING_BASE",
        ELEMENTS_STRING_BASE,
        ELEMENTS_STRING_COUNT
    ),
    category!("SCAN_STRING_BASE", SCAN_STRING_BASE, SCAN_STRING_COUNT),
    category!("STAR_NUMBER_BASE", STAR_NUMBER_BASE, STAR_NUMBER_COUNT),
    category!(
        "PLANET_NUMBER_BASE",
        PLANET_NUMBER_BASE,
        PLANET_NUMBER_COUNT
    ),
    category!(
        "MONTHS_STRING_BASE",
        MONTHS_STRING_BASE,
        MONTHS_STRING_COUNT
    ),
    category!(
        "FEEDBACK_STRING_BASE",
        FEEDBACK_STRING_BASE,
        FEEDBACK_STRING_COUNT
    ),
    category!(
        "STARBASE_STRING_BASE",
        STARBASE_STRING_BASE,
        STARBASE_STRING_COUNT
    ),
    category!(
        "ENCOUNTER_STRING_BASE",
        ENCOUNTER_STRING_BASE,
        ENCOUNTER_STRING_COUNT
    ),
    category!(
        "NAVIGATION_STRING_BASE",
        NAVIGATION_STRING_BASE,
        NAVIGATION_STRING_COUNT
    ),
    category!(
        "NAMING_STRING_BASE",
        NAMING_STRING_BASE,
        NAMING_STRING_COUNT
    ),
    category!("MELEE_STRING_BASE", MELEE_STRING_BASE, MELEE_STRING_COUNT),
    category!(
        "SAVEGAME_STRING_BASE",
        SAVEGAME_STRING_BASE,
        SAVEGAME_STRING_COUNT
    ),
    category!(
        "OPTION_STRING_BASE",
        OPTION_STRING_BASE,
        OPTION_STRING_COUNT
    ),
    category!(
        "QUITMENU_STRING_BASE",
        QUITMENU_STRING_BASE,
        QUITMENU_STRING_COUNT
    ),
    category!(
        "STATUS_STRING_BASE",
        STATUS_STRING_BASE,
        STATUS_STRING_COUNT
    ),
    category!(
        "FLAGSHIP_STRING_BASE",
        FLAGSHIP_STRING_BASE,
        FLAGSHIP_STRING_COUNT
    ),
    category!(
        "ORBITSCAN_STRING_BASE",
        ORBITSCAN_STRING_BASE,
        ORBITSCAN_STRING_COUNT
    ),
    category!(
        "MAINMENU_STRING_BASE",
        MAINMENU_STRING_BASE,
        MAINMENU_STRING_COUNT
    ),
    category!(
        "NETMELEE_STRING_BASE",
        NETMELEE_STRING_BASE,
        NETMELEE_STRING_COUNT
    ),
    ("GAMESTR_COUNT", GAMESTR_COUNT as usize, usize::MAX),
];
