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

/// Borrowed current-system pointers supplied only at the temporary ABI edge.
pub struct CffiSurfaceGenerator {
    #[cfg_attr(not(feature = "linked_c_archive"), allow(dead_code))]
    solar_system: *mut c_void,
    #[cfg_attr(not(feature = "linked_c_archive"), allow(dead_code))]
    world: *mut c_void,
}

impl CffiSurfaceGenerator {
    pub fn new(solar_system: *mut c_void, world: *mut c_void) -> Result<Self, AdapterError> {
        if solar_system.is_null() || world.is_null() {
            Err(AdapterError::new("surface_generator_context"))
        } else {
            Ok(Self {
                solar_system,
                world,
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
                        amount: info.density >> 8,
                    }
                }
                ScanType::Energy => GeneratedNodeKind::Energy,
                ScanType::Biological => {
                    let creature_index = u8::try_from(info.node_type)
                        .map_err(|_| AdapterError::new("creature_type"))?;
                    let creature = CreatureKind::new(creature_index)
                        .ok_or(AdapterError::new("creature_type"))?;
                    GeneratedNodeKind::Biological {
                        creature,
                        hit_points: CreatureCatalog::stats(creature).hit_points,
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
            super::batch_guard::preserving_batch_depth("surface_generator_pickup_batch", || {
                // SAFETY: borrowed current-system pointers validated at
                // construction; the callback runs synchronously on this thread.
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
            CffiSurfaceGenerator::new(std::ptr::null_mut(), std::ptr::null_mut()),
            Err(AdapterError {
                operation: "surface_generator_context"
            })
        ));
    }

    #[test]
    fn unlinked_generator_fails_without_fallback() {
        let pointer = std::ptr::NonNull::<u8>::dangling().as_ptr().cast();
        let mut generator = CffiSurfaceGenerator::new(pointer, pointer).unwrap();
        assert_eq!(
            generator.node_count(ScanType::Mineral),
            Err(AdapterError::new("surface_generator_unlinked"))
        );
    }
}
