//! Transitional concrete adapter for selected-system generation hooks.
//!
//! The gameplay world and persistence are Rust-owned. Until every special
//! system generator is ported, this adapter calls only the selected generator
//! hook and immediately converts its output into typed Rust data.

#[cfg(not(test))]
use crate::engine::world::planet::NodeInfo;
use crate::engine::{battle::Cx, observe::Io, StoryKey};
#[cfg(not(test))]
use crate::planets::solarsys::site::{dispatch::body, Site};
use std::cell::RefCell;
use std::ffi::c_void;

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

#[cfg(not(test))]
extern "C" {
    static mut Elements: *const u8;
}

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
pub struct CffiSurfaceGenerator<'a, 'game, 'observer> {
    game: &'a RefCell<Cx<'game>>,
    io: &'a RefCell<Io<'observer>>,
    #[cfg_attr(test, allow(dead_code))]
    solar_system: *mut c_void,
    #[cfg_attr(test, allow(dead_code))]
    world: *mut c_void,
    #[cfg_attr(test, allow(dead_code))]
    role: GeneratorRole,
    report_context: Option<crate::planets::report::typography::ReportContext>,
}

impl<'a, 'game, 'observer> CffiSurfaceGenerator<'a, 'game, 'observer> {
    /// Adapter for a real PlanetSide lander session.  Biological node generation
    /// consumes exactly one gameplay random value per node for the typed 0..3 life
    /// frame, and mining/energy consume nothing.
    pub fn for_planet_side(
        game: &'a RefCell<Cx<'game>>,
        io: &'a RefCell<Io<'observer>>,
        context: &super::ffi::PlanetSideRunContext,
        report_context: crate::planets::report::typography::ReportContext,
    ) -> Result<Self, AdapterError> {
        let mut generator = Self::with_role(
            game,
            io,
            context.solar_system,
            context.world,
            GeneratorRole::PlanetSide,
        )?;
        generator.report_context = Some(report_context);
        Ok(generator)
    }

    /// Adapter for orbit-scan map rendering.  Generation never touches the
    /// gameplay random stream; biological nodes carry a deterministic placeholder
    /// initial frame that the scan display list ignores.
    pub fn for_orbit_scan(
        game: &'a RefCell<Cx<'game>>,
        io: &'a RefCell<Io<'observer>>,
        solar_system: *mut c_void,
        world: *mut c_void,
        report_context: Option<crate::planets::report::typography::ReportContext>,
    ) -> Result<Self, AdapterError> {
        let mut generator =
            Self::with_role(game, io, solar_system, world, GeneratorRole::OrbitScan)?;
        generator.report_context = report_context;
        Ok(generator)
    }

    #[cfg(not(test))]
    unsafe fn generate_node(&mut self, scan_type: ScanType, node: u16, info: &mut NodeInfo) -> u16 {
        // SAFETY: the adapter retains this system and orbital descriptor throughout the selected callback.
        unsafe {
            let record = self.solar_system.cast();
            let target = body(record, self.world.cast());
            let mut pickup = PickupContext::NoLander;
            let report = self.report_context.clone();
            let mut site = Site::new(self.game, self.io, record, &mut pickup, report);
            site.with_system(|generator, system, host| {
                scan::generate(generator, system, target, node, scan_type as u8, info, host)
            })
        }
    }

    #[cfg(not(test))]
    unsafe fn pickup_node(
        &mut self,
        scan_type: ScanType,
        node: u16,
        effects: &mut PickupContext,
    ) -> bool {
        // SAFETY: the adapter retains this system and orbital descriptor throughout the selected callback.
        unsafe {
            let record = self.solar_system.cast();
            let target = body(record, self.world.cast());
            let report = self.report_context.clone();
            let mut site = Site::new(self.game, self.io, record, effects, report);
            site.with_system(|generator, system, host| {
                crate::planets::scan::pickup(generator, system, target, node, scan_type as u8, host)
            })
        }
    }

    #[cfg(not(test))]
    fn node_key(&self, node: ScanNodeId) -> (u16, u8, u8, u8) {
        // SAFETY: the adapter retains the selected system while its owners are borrowed.
        let (star, planet, moon) = unsafe {
            {
                let game = self.game.borrow();
                let world = &*game.world;
                crate::state::c_api::planets::location(world, self.solar_system.cast())
            }
        };
        (star as u16, planet as u8, moon as u8, node.get())
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
            let solar_system = self.solar_system;
            let retrieved = super::batch_guard::calling_native_unbatched(
                "surface_generator_pickup_batch",
                || callback(self),
            )?;
            if retrieved {
                // SAFETY: the adapter owns the retained solar record during pickup.
                unsafe {
                    let system =
                        &mut *solar_system.cast::<crate::ships::native::abi::SOLARSYS_STATE>();
                    crate::planets::scan::set_node_retrieved(
                        &mut system.SysInfo.planet_info,
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
    fn with_role(
        game: &'a RefCell<Cx<'game>>,
        io: &'a RefCell<Io<'observer>>,
        solar_system: *mut c_void,
        world: *mut c_void,
        role: GeneratorRole,
    ) -> Result<Self, AdapterError> {
        if solar_system.is_null() || world.is_null() {
            Err(AdapterError::new("surface_generator_context"))
        } else {
            Ok(Self {
                game,
                io,
                solar_system,
                world,
                role,
                report_context: None,
            })
        }
    }
}

impl SurfaceGenerator for CffiSurfaceGenerator<'_, '_, '_> {
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
                ScanType::Mineral => {
                    if Elements.is_null() {
                        return Err(AdapterError::new("element_catalog"));
                    }
                    GeneratedNodeKind::Mineral {
                        category: usize::from(*Elements.add(usize::from(info.type_)) & 0x07),
                        gross_size: info.density & 0x07,
                        fine_quantity: info.density >> 8,
                    }
                }
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
            self.pickup_with(scan, node, |this| unsafe {
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
            // SAFETY: all scalar and pointer fields admit zero before fixture initialization.
            let mut system: Box<crate::ships::native::abi::SOLARSYS_STATE> =
                Box::new(unsafe { std::mem::zeroed() });
            let world = &raw mut system.PlanetDesc[0];
            let mut generator = CffiSurfaceGenerator::for_planet_side(
                game,
                io,
                &super::super::ffi::PlanetSideRunContext {
                    solar_system: std::ptr::from_mut(&mut *system).cast(),
                    world: world.cast(),
                    ..Default::default()
                },
                report_context,
            )
            .unwrap();
            assert!(std::ptr::eq(generator.io, io));
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
                    system.SysInfo.planet_info.scan_retrieve_mask[scan as usize],
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
            assert_eq!(system.SysInfo.planet_info.scan_retrieve_mask[1], 1);
            assert_eq!(
                game.borrow().world.story_state(StoryKey::PLANETARY_CHANGE),
                0
            );
        });
    }

    #[test]
    #[serial_test::serial]
    fn null_generation_context_is_rejected() {
        with_context(|game, io, report_context| {
            assert!(matches!(
                CffiSurfaceGenerator::for_planet_side(
                    game,
                    io,
                    &super::super::ffi::PlanetSideRunContext::default(),
                    report_context,
                ),
                Err(AdapterError {
                    operation: "surface_generator_context",
                    batch_depth_corrections: 0
                })
            ));
            assert!(matches!(
                CffiSurfaceGenerator::for_orbit_scan(
                    game,
                    io,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    None
                ),
                Err(AdapterError {
                    operation: "surface_generator_context",
                    batch_depth_corrections: 0
                })
            ));
        });
    }

    #[test]
    #[serial_test::serial]
    fn unlinked_generator_fails_without_fallback() {
        with_context(|game, io, report_context| {
            let pointer = std::ptr::NonNull::<u8>::dangling().as_ptr().cast();
            let mut generator = CffiSurfaceGenerator::for_planet_side(
                game,
                io,
                &super::super::ffi::PlanetSideRunContext {
                    solar_system: pointer,
                    world: pointer,
                    ..Default::default()
                },
                report_context,
            )
            .unwrap();
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
