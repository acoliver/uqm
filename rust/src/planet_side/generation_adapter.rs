//! Transitional concrete adapter for selected-system generation hooks.
//!
//! The gameplay world and persistence are Rust-owned. Until every special
//! system generator is ported, this adapter calls only the selected generator
//! hook and immediately converts its output into typed Rust data.

use std::ffi::c_void;

#[cfg(feature = "linked_c_archive")]
use super::creatures::{CreatureCatalog, CreatureKind};
#[cfg(feature = "linked_c_archive")]
use super::generation::GeneratedNodeKind;
use super::generation::{GeneratedNode, ScanNodeId, ScanType, SurfaceGenerator};
#[cfg(feature = "linked_c_archive")]
use super::model::SurfacePoint;
use super::runtime::AdapterError;

#[cfg(feature = "linked_c_archive")]
const GENERATE_ALL: u16 = u16::MAX;

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
#[derive(Default)]
struct CNodeInfo {
    x: i16,
    y: i16,
    density: u16,
    node_type: u16,
}

#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn callGenerateForScanType(
        solar_system: *const c_void,
        world: *const c_void,
        node: u16,
        scan_type: u8,
        info: *mut CNodeInfo,
    ) -> u16;
    fn callPickupForScanType(
        solar_system: *mut c_void,
        world: *mut c_void,
        node: u16,
        scan_type: u8,
    ) -> bool;
    static Elements: *const u8;
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
#[cfg_attr(not(feature = "linked_c_archive"), allow(dead_code))]
fn select_initial_frame(role: GeneratorRole, draw: impl FnOnce() -> u32) -> u8 {
    match role {
        GeneratorRole::PlanetSide => (draw() % 4) as u8,
        GeneratorRole::OrbitScan => ORBIT_SCAN_INITIAL_FRAME,
    }
}

/// Deterministic creature frame the orbit-scan renderer ignores, chosen without a
/// gameplay random draw.
#[cfg_attr(not(feature = "linked_c_archive"), allow(dead_code))]
const ORBIT_SCAN_INITIAL_FRAME: u8 = 0;

/// Borrowed current-system pointers supplied only at the temporary ABI edge.
pub struct CffiSurfaceGenerator {
    #[cfg_attr(not(feature = "linked_c_archive"), allow(dead_code))]
    solar_system: *mut c_void,
    #[cfg_attr(not(feature = "linked_c_archive"), allow(dead_code))]
    world: *mut c_void,
    #[cfg_attr(not(feature = "linked_c_archive"), allow(dead_code))]
    role: GeneratorRole,
}

impl CffiSurfaceGenerator {
    /// Adapter for a real PlanetSide lander session.  Biological node generation
    /// consumes exactly one gameplay random value per node for the typed 0..3 life
    /// frame, and mining/energy consume nothing.
    pub fn for_planet_side(
        solar_system: *mut c_void,
        world: *mut c_void,
    ) -> Result<Self, AdapterError> {
        Self::with_role(solar_system, world, GeneratorRole::PlanetSide)
    }

    /// Adapter for orbit-scan map rendering.  Generation never touches the
    /// gameplay random stream; biological nodes carry a deterministic placeholder
    /// initial frame that the scan display list ignores.
    pub fn for_orbit_scan(
        solar_system: *mut c_void,
        world: *mut c_void,
    ) -> Result<Self, AdapterError> {
        Self::with_role(solar_system, world, GeneratorRole::OrbitScan)
    }

    fn with_role(
        solar_system: *mut c_void,
        world: *mut c_void,
        role: GeneratorRole,
    ) -> Result<Self, AdapterError> {
        if solar_system.is_null() || world.is_null() {
            Err(AdapterError::new("surface_generator_context"))
        } else {
            Ok(Self {
                solar_system,
                world,
                role,
            })
        }
    }
}

impl SurfaceGenerator for CffiSurfaceGenerator {
    fn node_count(&mut self, scan: ScanType) -> Result<u8, AdapterError> {
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            let count = callGenerateForScanType(
                self.solar_system,
                self.world,
                GENERATE_ALL,
                scan as u8,
                std::ptr::null_mut(),
            );
            u8::try_from(count).map_err(|_| AdapterError::new("surface_node_count"))
        }
        #[cfg(not(feature = "linked_c_archive"))]
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
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            let mut info = CNodeInfo::default();
            let _ = callGenerateForScanType(
                self.solar_system,
                self.world,
                u16::from(node.get()),
                scan as u8,
                &mut info,
            );
            let kind = match scan {
                ScanType::Mineral => {
                    if Elements.is_null() {
                        return Err(AdapterError::new("element_catalog"));
                    }
                    GeneratedNodeKind::Mineral {
                        category: usize::from(*Elements.add(usize::from(info.node_type)) & 0x07),
                        gross_size: info.density & 0x07,
                        fine_quantity: info.density >> 8,
                    }
                }
                ScanType::Energy => GeneratedNodeKind::Energy,
                ScanType::Biological => {
                    let creature_index = u8::try_from(info.node_type)
                        .map_err(|_| AdapterError::new("creature_type"))?;
                    let creature = CreatureKind::new(creature_index)
                        .ok_or(AdapterError::new("creature_type"))?;
                    // Legacy `generateBioNode` is only embedded in a real
                    // `GeneratePlanetSide` lander population, never in the orbit
                    // scan render, so only that role draws a random 0..3 frame.
                    let initial_frame =
                        select_initial_frame(self.role, || crate::math::TFB_Random());
                    GeneratedNodeKind::Biological {
                        creature,
                        hit_points: CreatureCatalog::stats(creature).hit_points,
                        initial_frame,
                    }
                }
            };
            Ok(GeneratedNode {
                position: SurfacePoint {
                    x: i32::from(info.x) << 2,
                    y: i32::from(info.y) << 2,
                },
                kind,
            })
        }
        #[cfg(not(feature = "linked_c_archive"))]
        {
            let _ = (scan, node);
            Err(AdapterError::new("surface_generator_unlinked"))
        }
    }

    fn pickup(&mut self, scan: ScanType, node: ScanNodeId) -> Result<bool, AdapterError> {
        #[cfg(feature = "linked_c_archive")]
        {
            // A pickup callback may present a discovery report, and the native
            // report helpers were written for the retired lander loop's ambient
            // batch. Rust holds no such batch, so it owns restoring the depth
            // rather than trusting the callback to leave it untouched.
            let solar_system = self.solar_system;
            let world = self.world;
            super::batch_guard::calling_native_unbatched("surface_generator_pickup_batch", || {
                // SAFETY: the outer PlanetSide ABI contract guarantees these
                // borrowed current-system pointers are valid, correctly typed
                // and exclusively ours for the duration of the call; the
                // callback runs synchronously on this thread.
                unsafe {
                    callPickupForScanType(solar_system, world, u16::from(node.get()), scan as u8)
                }
            })
        }
        #[cfg(not(feature = "linked_c_archive"))]
        {
            let _ = (scan, node);
            Err(AdapterError::new("surface_generator_unlinked"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_generation_context_is_rejected() {
        assert!(matches!(
            CffiSurfaceGenerator::for_planet_side(std::ptr::null_mut(), std::ptr::null_mut()),
            Err(AdapterError {
                operation: "surface_generator_context"
            })
        ));
        assert!(matches!(
            CffiSurfaceGenerator::for_orbit_scan(std::ptr::null_mut(), std::ptr::null_mut()),
            Err(AdapterError {
                operation: "surface_generator_context"
            })
        ));
    }

    #[test]
    fn unlinked_generator_fails_without_fallback() {
        let pointer = std::ptr::NonNull::<u8>::dangling().as_ptr().cast();
        let mut generator = CffiSurfaceGenerator::for_planet_side(pointer, pointer).unwrap();
        assert_eq!(
            generator.node_count(ScanType::Mineral),
            Err(AdapterError::new("surface_generator_unlinked"))
        );
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
