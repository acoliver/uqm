//! Ordered universe diagnostics over the same supplied generation calls as play.

use super::*;
use crate::engine::{
    observe::CheckpointView,
    world::{generation::SolarSys, CStarDesc},
};
use crate::planets::generate::{SiteHost, SystemGenerator};
use crate::planets::solar_coords::star_seed;
use std::io;

/// The visit owner supplies each generation admission, including its host effects.
pub trait Generation {
    fn invoke<R>(
        &mut self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        solar: &mut SolarSystem,
        run: impl FnOnce(&dyn SystemGenerator, &mut SolarSys<'_>, &mut dyn SiteHost) -> R,
    ) -> R;
}

type SystemVisitor<T> = fn(&Kernel, &CStarDesc, &SolarSystem, &mut T) -> io::Result<()>;
type WorldVisitor<T, G> =
    fn(&mut Cx<'_>, &mut Io<'_>, &mut SolarSystem, Body, &mut G, &mut T) -> io::Result<()>;

pub struct UniverseRecurseArg<'a, T, G> {
    pub system_func_pre: Option<SystemVisitor<T>>,
    pub system_func_post: Option<SystemVisitor<T>>,
    pub planet_func_pre: Option<WorldVisitor<T, G>>,
    pub planet_func_post: Option<WorldVisitor<T, G>>,
    pub moon_func: Option<WorldVisitor<T, G>>,
    pub arg: &'a mut T,
}

pub fn universe_recurse<T, G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    generation: &mut G,
    arg: &mut UniverseRecurseArg<'_, T, G>,
) -> io::Result<()> {
    if arg.system_func_pre.is_none()
        && arg.system_func_post.is_none()
        && arg.planet_func_pre.is_none()
        && arg.planet_func_post.is_none()
        && arg.moon_func.is_none()
    {
        return Ok(());
    }
    let saved_activity = cx.world.activity();
    let _quiet = cx.kernel.diagnostics.suppress_interaction();
    let result = (|| {
        for index in 0..NUM_SOLAR_SYSTEMS as usize {
            star_recurse(cx, io, generation, crate::engine::StarId(index), arg)?;
        }
        Ok(())
    })();
    cx.world.set_activity(saved_activity);
    result
}

fn star_recurse<T, G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    generation: &mut G,
    star_id: crate::engine::StarId,
    recurse: &mut UniverseRecurseArg<'_, T, G>,
) -> io::Result<()> {
    let old_star = cx.world.current_star;
    let old_frame = cx.solar_frame;
    let star = cx.world.catalog()[star_id.0];
    cx.world.current_star = Some(star_id);
    let mut solar = SolarSystem::new(cx.world, star_id);
    solar.sys.rng.reseed(star_seed(star.star_pt));
    solar.sys.sun.rand_seed = solar.sys.rng.next_random();
    solar.sys.sun.data_index = crate::planets::generate::consts::star_type(star.star_type);
    let result = (|| {
        generation.invoke(cx, io, &mut solar, |g, s, h| g.generate_planets(s, h));
        if let Some(callback) = recurse.system_func_pre {
            callback(cx.kernel, &star, &solar, recurse.arg)?;
        }
        if recurse.planet_func_pre.is_some()
            || recurse.planet_func_post.is_some()
            || recurse.moon_func.is_some()
        {
            for planet in 0..solar.sys.sun.num_planets {
                planet_recurse(cx, io, generation, &mut solar, planet, recurse)?;
            }
        }
        if let Some(callback) = recurse.system_func_post {
            callback(cx.kernel, &star, &solar, recurse.arg)?;
        }
        Ok(())
    })();
    cx.world.current_star = old_star;
    cx.solar_frame = old_frame;
    io.observer
        .checkpoint(&CheckpointView::Solar(old_frame.as_ref()));
    result
}

fn planet_recurse<T, G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    generation: &mut G,
    solar: &mut SolarSystem,
    planet: u8,
    recurse: &mut UniverseRecurseArg<'_, T, G>,
) -> io::Result<()> {
    let target = Body::Planet(planet);
    solar.enter_inner(planet);
    if let Some(callback) = recurse.planet_func_pre {
        generate_orbital(cx, io, generation, solar, target, true);
        callback(cx, io, solar, target, generation, recurse.arg)?;
    }
    if let Some(callback) = recurse.moon_func {
        solar
            .sys
            .rng
            .reseed(solar.sys.planets[usize::from(planet)].rand_seed);
        generation.invoke(cx, io, solar, |g, s, h| g.generate_moons(s, planet, h));
        for moon in 0..solar.sys.planets[usize::from(planet)].num_planets {
            let target = Body::Moon { planet, moon };
            solar.select_body(target);
            let data = solar.sys.descriptor(target).data_index;
            generate_orbital(
                cx,
                io,
                generation,
                solar,
                target,
                data != HIERARCHY_STARBASE as u8 && data != SA_MATRA as u8,
            );
            callback(cx, io, solar, target, generation, recurse.arg)?;
        }
    }
    if let Some(callback) = recurse.planet_func_post {
        solar.select_body(target);
        generate_orbital(cx, io, generation, solar, target, true);
        callback(cx, io, solar, target, generation, recurse.arg)?;
    }
    Ok(())
}

fn generate_orbital<G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    generation: &mut G,
    solar: &mut SolarSystem,
    body: Body,
    analyse: bool,
) {
    generation.invoke(cx, io, solar, |g, s, h| {
        if analyse {
            crate::planets::generate::calc::analyse_body(s, body);
        }
        g.generate_orbital(s, body, h)
    });
}

fn scan_node<G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    generation: &mut G,
    solar: &mut SolarSystem,
    body: Body,
    node: u16,
    scan: u8,
) -> (u16, crate::engine::world::planet::NodeInfo) {
    assert_eq!(solar.selected_body(), Some(body));
    let mut info = crate::engine::world::planet::NodeInfo::default();
    let count = generation.invoke(cx, io, solar, |g, s, h| {
        crate::planets::scan::generate(g, s, body, node, scan, &mut info, h)
    });
    (count, info)
}

pub fn calculate_bio_value<G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
    body: Body,
    generation: &mut G,
) -> u16 {
    let count = scan_node(cx, io, generation, solar, body, u16::MAX, 2).0;
    let mut value = 0u16;
    for node in 0..count {
        let info = scan_node(cx, io, generation, solar, body, node, 2).1;
        let bio = uqm_debug_creature_bio_value(info.type_ as u8);
        assert!(bio >= 0, "invalid generated creature {}", info.type_);
        value = value.wrapping_add((BIO_CREDIT_VALUE as i32 * bio) as u16);
    }
    value
}

pub fn calculate_mineral_value<G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
    body: Body,
    generation: &mut G,
) -> u16 {
    let count = scan_node(cx, io, generation, solar, body, u16::MAX, 0).0;
    let mut value = 0u16;
    for node in 0..count {
        let info = scan_node(cx, io, generation, solar, body, node, 0).1;
        value = value.wrapping_add(
            (u32::from(info.density >> 8)
                * u32::from(
                    cx.world.data.game.element_worth[element_category(info.type_ as usize)],
                )) as u16,
        );
    }
    value
}

pub fn generate_bio_index<G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
    body: Body,
    generation: &mut G,
) -> Vec<u16> {
    let count = scan_node(cx, io, generation, solar, body, u16::MAX, 2).0;
    let mut values = vec![0u16; (NUM_CREATURE_TYPES + NUM_SPECIAL_CREATURE_TYPES) as usize];
    for node in 0..count {
        let info = scan_node(cx, io, generation, solar, body, node, 2).1;
        values[info.type_ as usize] = values[info.type_ as usize].wrapping_add(1);
    }
    values
}

pub fn generate_mineral_index<G: Generation>(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
    body: Body,
    generation: &mut G,
) -> [u16; 8] {
    let count = scan_node(cx, io, generation, solar, body, u16::MAX, 0).0;
    let mut values = [0u16; 8];
    for node in 0..count {
        let info = scan_node(cx, io, generation, solar, body, node, 0).1;
        let category = element_category(info.type_ as usize);
        values[category] = values[category].wrapping_add(info.density >> 8);
    }
    values
}

pub(super) fn element_category(kind: usize) -> usize {
    usize::from(crate::planets::tables::ELEMENT_ARRAY[kind] & 7)
}
