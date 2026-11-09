//! Transitional concrete adapter for selected-system generation hooks.
//!
//! The gameplay world and persistence are Rust-owned. Until every special
//! system generator is ported, this adapter calls only the selected generator
//! hook and immediately converts its output into typed Rust data.

#[cfg(not(test))]
use crate::engine::world::planet::NodeInfo;
use crate::engine::world::{
    generation::{Body, SolarSys},
    StarId,
};
use crate::engine::{battle::Cx, observe::Io, StoryKey};
use crate::planets::solar_system::SolarSystem;
#[cfg(not(test))]
use crate::planets::solarsys::site::Site;
use std::cell::RefCell;

#[cfg(not(test))]
use super::creatures::{CreatureCatalog, CreatureKind};
#[cfg(not(test))]
use super::generation::GeneratedNodeKind;
use super::generation::{GeneratedNode, ScanNodeId, ScanType, SurfaceGenerator};
#[cfg(not(test))]
use super::model::SurfacePoint;
use super::runtime::AdapterError;
use super::special_effects::PickupContext;

#[cfg(not(test))]
const GENERATE_ALL: u16 = u16::MAX;

#[cfg(not(test))]
use crate::planets::scan;

/// Why a [`CffiSurfaceGenerator`] exists, fixed by its typed constructor.
///
/// Both callers drive the same native node generator, but only a real PlanetSide
/// population may advance the gameplay random stream.  Orbit-scan rendering shares
/// this adapter purely to build its display list, and that is observable, so it
/// must never consume a random value.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GeneratorRole {
    /// Real PlanetSide lander population.  Legacy `generateBioNode` stamps each
    /// freshly generated creature on a `TFB_Random()` 0..3 frame, so this
    /// mode draws exactly one gameplay random value per biological node and never for
    /// mineral or energy nodes.
    PlanetSide,
    /// Orbit-scan map rendering.  The scan display list never reads the returned
    /// creature initial frame, so this mode consumes no gameplay random value and
    /// returns a documented deterministic placeholder.
    OrbitScan,
}

/// Frame of the 4-frame life animation a fresh creature starts on.
///
/// Planet-side population invokes `draw` exactly once and normalizes the result to
/// 0..3, at the same semantic point as the legacy `generateBioNode` stamp
/// `SetAbsFrameIndex(life_form, (COUNT)TFB_Random())`.  Orbit-scan
/// rendering never invokes `draw`, so scanning a map from orbit cannot perturb the
/// gameplay random stream.
#[cfg_attr(test, allow(dead_code))]
fn select_initial_frame(role: GeneratorRole, draw: impl FnOnce() -> u32) -> u8 {
    match role {
        GeneratorRole::PlanetSide => (draw() % 4) as u8,
        GeneratorRole::OrbitScan => ORBIT_SCAN_INITIAL_FRAME,
    }
}

/// Deterministic creature frame the orbit-scan renderer ignores, chosen without a
/// gameplay random draw.
#[cfg_attr(test, allow(dead_code))]
const ORBIT_SCAN_INITIAL_FRAME: u8 = 0;

/// Borrowed current-system pointers supplied only at the temporary ABI edge.
pub struct CffiSurfaceGenerator<'a, 'game, 'observer, 'tables> {
    game: &'a RefCell<Cx<'game>>,
    io: &'a RefCell<Io<'observer>>,
    #[cfg_attr(test, allow(dead_code))]
    system: &'a mut SolarSys<'tables>,
    star: StarId,
    body: Body,
    in_orbit: bool,
    #[cfg_attr(test, allow(dead_code))]
    role: GeneratorRole,
    report_context: Option<crate::planets::report::typography::ReportContext>,
}

impl<'a, 'game, 'observer, 'tables> CffiSurfaceGenerator<'a, 'game, 'observer, 'tables> {
    /// Adapter for a real PlanetSide lander session.  Biological node generation
    /// consumes exactly one gameplay random value per node for the typed 0..3 life
    /// frame, and mining/energy consume nothing.
    pub fn for_planet_side(
        game: &'a RefCell<Cx<'game>>,
        io: &'a RefCell<Io<'observer>>,
        solar: &'a mut SolarSystem,
        report_context: crate::planets::report::typography::ReportContext,
    ) -> Self
    where
        'tables: 'static,
    {
        let star = solar.star();
        let body = solar
            .selected_body()
            .expect("lander requires a selected body");
        let in_orbit = solar.in_orbit();
        Self {
            game,
            io,
            system: &mut solar.sys,
            star,
            body,
            in_orbit,
            role: GeneratorRole::PlanetSide,
            report_context: Some(report_context),
        }
    }

    /// Adapter for orbit-scan map rendering.  Generation never touches the
    /// gameplay random stream; biological nodes carry a deterministic placeholder
    /// initial frame that the scan display list ignores.
    pub fn for_orbit_scan(
        game: &'a RefCell<Cx<'game>>,
        io: &'a RefCell<Io<'observer>>,
        system: &'a mut SolarSys<'tables>,
        star: StarId,
        body: Body,
        report_context: Option<crate::planets::report::typography::ReportContext>,
    ) -> Self {
        Self {
            game,
            io,
            system,
            star,
            body,
            in_orbit: true,
            role: GeneratorRole::OrbitScan,
            report_context,
        }
    }

    #[cfg(not(test))]
    unsafe fn generate_node(&mut self, scan_type: ScanType, node: u16, info: &mut NodeInfo) -> u16 {
        let mut pickup = PickupContext::NoLander;
        let mut site = Site::new(
            self.game,
            self.io,
            self.star,
            self.system.star_index(),
            self.in_orbit,
            &mut pickup,
            self.report_context.clone(),
        );
        let target = self.body;
        site.invoke(self.system, |generator, system, host| {
            scan::generate(generator, system, target, node, scan_type as u8, info, host)
        })
    }

    #[cfg(not(test))]
    fn pickup_node(&mut self, scan_type: ScanType, node: u16, effects: &mut PickupContext) -> bool {
        let mut site = Site::new(
            self.game,
            self.io,
            self.star,
            self.system.star_index(),
            self.in_orbit,
            effects,
            self.report_context.clone(),
        );
        let target = self.body;
        site.invoke(self.system, |generator, system, host| {
            crate::planets::scan::pickup(generator, system, target, node, scan_type as u8, host)
        })
    }

    #[cfg(not(test))]
    fn node_key(&self, node: ScanNodeId) -> (u16, u8, u8, u8) {
        crate::planets::solar_system::BodyLocation::new(self.star, self.body).node_key(node.get())
    }

    fn pickup_with(
        &mut self,
        scan: ScanType,
        node: ScanNodeId,
        callback: impl FnOnce(&mut Self) -> bool,
    ) -> Result<bool, AdapterError> {
        {
            // A pickup callback may present a discovery report, and the native
            // report helpers were written for the retired lander loop's ambient
            // batch. Rust holds no such batch, so it owns restoring the depth
            // rather than trusting the callback to leave it untouched.
            let retrieved = super::batch_guard::calling_native_unbatched(
                "surface_generator_pickup_batch",
                || callback(self),
            )?;
            if retrieved {
                unsafe {
                    crate::planets::scan::set_node_retrieved(
                        &mut self.system.info.planet_info,
                        scan as u8,
                        node.get(),
                    );
                }
                #[cfg(not(test))]
                if scan == ScanType::Biological {
                    let key = self.node_key(node);
                    {
                        let mut game = self.game.borrow_mut();
                        let world = &mut *game.world;
                        crate::state::ffi::with_state_files(world, |files| {
                            files.remove_canned_node(key)
                        })
                    }
                    .map_err(|_| AdapterError::new("canned_node_state"))?;
                }
                self.game
                    .borrow_mut()
                    .world
                    .set_story_state(StoryKey::PLANETARY_CHANGE, 1);
            }
            Ok(retrieved)
        }
    }
}

impl SurfaceGenerator for CffiSurfaceGenerator<'_, '_, '_, '_> {
    fn node_count(&mut self, scan: ScanType) -> Result<u8, AdapterError> {
        #[cfg(not(test))]
        // SAFETY: this adapter retains the initialized system, descriptor and RNG for the callback.
        unsafe {
            let count = self.generate_node(scan, GENERATE_ALL, &mut NodeInfo::default());
            u8::try_from(count).map_err(|_| AdapterError::new("surface_node_count"))
        }
        #[cfg(test)]
        {
            let _ = scan;
            Err(AdapterError::new("surface_generator_unlinked"))
        }
    }

    fn generate(
        &mut self,
        scan: ScanType,
        node: ScanNodeId,
    ) -> Result<GeneratedNode, AdapterError> {
        #[cfg(not(test))]
        // SAFETY: this adapter retains the initialized system, descriptor and RNG for the callback.
        unsafe {
            if scan == ScanType::Biological {
                let key = self.node_key(node);
                if let Some(canned) = {
                    let mut game = self.game.borrow_mut();
                    let world = &mut *game.world;
                    crate::state::ffi::with_state_files(world, |files| files.canned_node(key))
                }
                .map_err(|_| AdapterError::new("canned_node_state"))?
                {
                    return Ok(GeneratedNode::from_canister(canned));
                }
            }
            let mut info: NodeInfo = std::mem::zeroed();
            let _ = self.generate_node(scan, u16::from(node.get()), &mut info);
            let kind = match scan {
                ScanType::Mineral => GeneratedNodeKind::Mineral {
                    category: usize::from(
                        crate::planets::tables::ELEMENT_ARRAY[usize::from(info.type_)] & 0x07,
                    ),
                    gross_size: info.density & 0x07,
                    fine_quantity: info.density >> 8,
                },
                ScanType::Energy => GeneratedNodeKind::Energy,
                ScanType::Biological => {
                    let creature_index =
                        u8::try_from(info.type_).map_err(|_| AdapterError::new("creature_type"))?;
                    let creature = CreatureKind::new(creature_index)
                        .ok_or(AdapterError::new("creature_type"))?;
                    // Legacy `generateBioNode` is only embedded in a real
                    // `generate_planet_side` lander population, never in the orbit
                    // scan render, so only that role draws a random 0..3 frame.
                    let initial_frame = select_initial_frame(self.role, crate::math::random);
                    GeneratedNodeKind::Biological {
                        creature,
                        hit_points: CreatureCatalog::stats(creature).hit_points,
                        initial_frame,
                    }
                }
            };
            Ok(GeneratedNode {
                position: SurfacePoint {
                    x: i32::from(info.loc_pt.x) << 2,
                    y: i32::from(info.loc_pt.y) << 2,
                },
                kind,
            })
        }
        #[cfg(test)]
        {
            let _ = (scan, node);
            Err(AdapterError::new("surface_generator_unlinked"))
        }
    }

    #[cfg(not(test))]
    fn can_node(
        &mut self,
        node: ScanNodeId,
        position: SurfacePoint,
        value: u16,
    ) -> Result<(), AdapterError> {
        let (star, planet, moon, node) = self.node_key(node);
        let canned = crate::engine::world::planet_info::CannedNode {
            star,
            planet,
            moon,
            node,
            value: value as u8,
            x: position.x as i16,
            y: position.y as i16,
        };
        {
            let mut game = self.game.borrow_mut();
            let world = &mut *game.world;
            crate::state::ffi::with_state_files(world, |files| files.can_node(canned))
        }
        .map_err(|_| AdapterError::new("canned_node_state"))
    }

    #[cfg(test)]
    fn can_node(
        &mut self,
        _: ScanNodeId,
        _: super::model::SurfacePoint,
        _: u16,
    ) -> Result<(), AdapterError> {
        Err(AdapterError::new("surface_generator_unlinked"))
    }

    fn pickup(
        &mut self,
        scan: ScanType,
        node: ScanNodeId,
        effects: &mut PickupContext,
    ) -> Result<bool, AdapterError> {
        #[cfg(not(test))]
        {
            self.pickup_with(scan, node, |this| {
                this.pickup_node(scan, u16::from(node.get()), effects)
            })
        }
        #[cfg(test)]
        {
            let _ = (scan, node, effects);
            Err(AdapterError::new("surface_generator_unlinked"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_context(
        run: impl FnOnce(
            &RefCell<Cx<'_>>,
            &RefCell<Io<'_>>,
            crate::planets::report::typography::ReportContext,
        ),
    ) {
        let mut owners = crate::engine::Game::new();
        owners.world.set_catalog(std::borrow::Cow::Borrowed(
            &crate::planets::tables::STARMAP_ARRAY,
        ));
        let mut battle = crate::engine::battle::Battle::new();
        let game = RefCell::new(Cx::new(
            &mut battle,
            &mut owners.world,
            &mut owners.kernel,
            crate::math::random,
        ));
        let mut observer = crate::engine::observe::Unobserved;
        let io = RefCell::new(Io {
            observer: &mut observer,
        });
        let scan =
            crate::engine::graphics::context_abi::LoadedContext::create(c"generation-test-scan");
        run(
            &game,
            &io,
            crate::planets::report::typography::ReportContext {
                context: scan.lease(),
                resources: Default::default(),
            },
        );
    }
    #[test]
    #[serial_test::serial]
    fn accepted_live_pickups_update_the_planet_mask_and_dirty_flag() {
        with_context(|game, io, report_context| {
            crate::state::ffi::rust_init_game_state(game.borrow_mut().world);
            game.borrow_mut().world.data.flagship.crew_enlisted = 23;
            let mut system = SolarSystem::new(game.borrow().world, StarId(0));
            system.sys.sun.num_planets = 1;
            system.enter_inner(0);
            system.enter_orbit(Body::Planet(0));
            let report_color = crate::graphics::native_abi::Color {
                r: 3,
                g: 5,
                b: 7,
                a: 11,
            };
            report_context.context.set_foreground(report_color);
            let mut generator =
                CffiSurfaceGenerator::for_planet_side(game, io, &mut system, report_context);
            assert!(std::ptr::eq(generator.io, io));
            assert_eq!(generator.star, StarId(0));
            assert_eq!(generator.body, Body::Planet(0));
            assert!(generator.in_orbit);
            let actual = generator
                .report_context
                .as_ref()
                .unwrap()
                .context
                .foreground();
            assert_eq!((actual.r, actual.g, actual.b, actual.a), (3, 5, 7, 11));
            for (scan, index) in [
                (ScanType::Mineral, 31),
                (ScanType::Energy, 0),
                (ScanType::Biological, 9),
            ] {
                game.borrow_mut()
                    .world
                    .set_story_state(StoryKey::PLANETARY_CHANGE, 0);
                assert!(generator
                    .pickup_with(scan, ScanNodeId::new(index).unwrap(), |generator| {
                        assert_eq!(
                            generator.game.borrow().world.data.flagship.crew_enlisted,
                            23
                        );
                        true
                    })
                    .unwrap());
                assert_eq!(
                    generator.system.info.planet_info.scan_retrieve_mask[scan as usize],
                    1 << index
                );
                assert_eq!(
                    game.borrow().world.story_state(StoryKey::PLANETARY_CHANGE),
                    1
                );
            }
            game.borrow_mut()
                .world
                .set_story_state(StoryKey::PLANETARY_CHANGE, 0);
            assert!(!generator
                .pickup_with(ScanType::Energy, ScanNodeId::new(1).unwrap(), |_| false)
                .unwrap());
            assert_eq!(generator.system.info.planet_info.scan_retrieve_mask[1], 1);
            assert_eq!(
                game.borrow().world.story_state(StoryKey::PLANETARY_CHANGE),
                0
            );
        });
    }

    #[test]
    #[serial_test::serial]
    fn missing_selected_body_is_rejected_before_generation() {
        with_context(|game, io, report| {
            let mut solar = SolarSystem::new(game.borrow().world, StarId(0));
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = CffiSurfaceGenerator::for_planet_side(game, io, &mut solar, report);
            }))
            .is_err());
        });
    }

    #[test]
    #[serial_test::serial]
    fn unlinked_generator_fails_without_fallback() {
        with_context(|game, io, report_context| {
            let mut solar = SolarSystem::new(game.borrow().world, StarId(0));
            solar.sys.sun.num_planets = 1;
            solar.enter_inner(0);
            solar.enter_orbit(Body::Planet(0));
            let mut generator =
                CffiSurfaceGenerator::for_planet_side(game, io, &mut solar, report_context);
            assert_eq!(
                generator.node_count(ScanType::Mineral),
                Err(AdapterError::new("surface_generator_unlinked"))
            );
        });
    }

    #[test]
    fn planet_side_selects_initial_frame_once_and_normalizes_to_0_3() {
        for raw in [0, 1, 2, 3, 4, 5] {
            let draws = std::cell::Cell::new(0);
            let frame = select_initial_frame(GeneratorRole::PlanetSide, || {
                draws.set(draws.get() + 1);
                raw as u32
            });
            assert_eq!(draws.get(), 1, "exactly one draw for generator value {raw}");
            assert_eq!(frame, raw % 4, "initial frame normalized to 0..3");
        }
    }

    #[test]
    fn orbit_scan_never_draws_and_uses_placeholder_frame() {
        let draw = || -> u32 {
            panic!("orbit-scan biological frame must not draw gameplay random");
        };
        let frame = select_initial_frame(GeneratorRole::OrbitScan, draw);
        assert_eq!(frame, ORBIT_SCAN_INITIAL_FRAME);
    }
}
