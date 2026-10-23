//! Planet scanning, scan-node callbacks, and lander dispatch from `scan.c`.

fn scan_value(value: i64, suffix: &str) -> String {
    if value >= 1000 {
        format!("{}.{}{}", value / 100, (value / 10) % 10, suffix)
    } else {
        format!("{}.{:02}{}", value / 100, value % 100, suffix)
    }
}

fn planet_mass(radius: u16, density: u16) -> i32 {
    let radius = u32::from(radius);
    let cube = radius.wrapping_mul(radius).wrapping_mul(radius);
    let mass = (i64::from(cube) / 100 * i64::from(density) + 5000) / 10000;
    if mass == 0 {
        1
    } else {
        mass as i32
    }
}

fn landing_fuel(gravity: u16) -> u16 {
    gravity.wrapping_shl(1).min(300)
}

fn move_cursor(position: (i16, i16), dx: i16, dy: i16, width: i16) -> (i16, i16) {
    let (mut x, mut y) = position;
    if dx != 0 {
        x += dx << 2;
        if x < 0 {
            x += width << 2;
        } else if x >= width << 2 {
            x -= width << 2;
        }
    }
    if dy != 0 {
        y += dy << 2;
        if !(0..75 << 2).contains(&y) {
            y = position.1;
        }
    }
    (x, y)
}

use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, PLANET_INFO, SOLARSYS_STATE};

#[no_mangle]
unsafe extern "C" fn callGenerateForScanType(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    node: u16,
    scan: u8,
    info: *mut NODE_INFO,
) -> u16 {
    let functions = &*(*system).genFuncs;
    let callback = match scan {
        0 => functions.generateMinerals,
        1 => functions.generateEnergy,
        2 => functions.generateLife,
        _ => panic!("invalid scan type {scan}"),
    };
    callback.expect("scan generator")(system, world, node, info)
}

#[no_mangle]
unsafe extern "C" fn callPickupForScanType(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    node: u16,
    scan: u8,
) -> bool {
    let functions = &*(*system).genFuncs;
    let callback = match scan {
        0 => functions.pickupMinerals,
        1 => functions.pickupEnergy,
        2 => functions.pickupLife,
        _ => panic!("invalid scan type {scan}"),
    };
    callback.expect("scan pickup")(system, world, node)
}

#[no_mangle]
unsafe extern "C" fn isNodeRetrieved(info: *mut PLANET_INFO, scan: u8, node: u8) -> bool {
    (*info).ScanRetrieveMask[usize::from(scan)] & (1_u32 << node) != 0
}
#[no_mangle]
unsafe extern "C" fn countNodesRetrieved(info: *mut PLANET_INFO, scan: u8) -> u16 {
    (*info).ScanRetrieveMask[usize::from(scan)].count_ones() as u16
}
#[no_mangle]
unsafe extern "C" fn setNodeRetrieved(info: *mut PLANET_INFO, scan: u8, node: u8) {
    (*info).ScanRetrieveMask[usize::from(scan)] |= 1_u32 << node;
}
#[no_mangle]
unsafe extern "C" fn setNodeNotRetrieved(info: *mut PLANET_INFO, scan: u8, node: u8) {
    (*info).ScanRetrieveMask[usize::from(scan)] &= !(1_u32 << node);
}

#[cfg(not(test))]
mod live;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_masks_keep_scan_types_and_all_32_nodes_independent() {
        let mut info: PLANET_INFO = unsafe { std::mem::zeroed() };
        unsafe {
            for scan in 0..3 {
                for node in 0..32 {
                    assert!(!isNodeRetrieved(&mut info, scan, node));
                    setNodeRetrieved(&mut info, scan, node);
                    assert!(isNodeRetrieved(&mut info, scan, node));
                    assert_eq!(countNodesRetrieved(&mut info, scan), u16::from(node) + 1);
                }
            }
            for scan in 0..3 {
                setNodeNotRetrieved(&mut info, scan, 31);
                setNodeNotRetrieved(&mut info, scan, 0);
                assert_eq!(countNodesRetrieved(&mut info, scan), 30);
                assert_eq!(info.ScanRetrieveMask[usize::from(scan)], 0x7fff_fffe);
            }
        }
    }

    #[test]
    fn scan_generation_and_pickup_dispatch_preserve_callback_arguments() {
        use crate::c_bindings::global_state::GenerateFunctions;
        unsafe extern "C" fn mineral(
            _: *const SOLARSYS_STATE,
            world: *const PLANET_DESC,
            node: u16,
            _: *mut NODE_INFO,
        ) -> u16 {
            node + u16::from((*world).data_index)
        }
        unsafe extern "C" fn energy(
            _: *const SOLARSYS_STATE,
            world: *const PLANET_DESC,
            node: u16,
            _: *mut NODE_INFO,
        ) -> u16 {
            mineral(std::ptr::null(), world, node, std::ptr::null_mut()) + 100
        }
        unsafe extern "C" fn life(
            _: *const SOLARSYS_STATE,
            world: *const PLANET_DESC,
            node: u16,
            _: *mut NODE_INFO,
        ) -> u16 {
            mineral(std::ptr::null(), world, node, std::ptr::null_mut()) + 200
        }
        unsafe extern "C" fn pickup_mineral(
            _: *mut SOLARSYS_STATE,
            world: *mut PLANET_DESC,
            node: u16,
        ) -> bool {
            node == u16::from((*world).data_index)
        }
        unsafe extern "C" fn pickup_energy(
            _: *mut SOLARSYS_STATE,
            world: *mut PLANET_DESC,
            node: u16,
        ) -> bool {
            node == u16::from((*world).data_index) + 1
        }
        unsafe extern "C" fn pickup_life(
            _: *mut SOLARSYS_STATE,
            world: *mut PLANET_DESC,
            node: u16,
        ) -> bool {
            node == u16::from((*world).data_index) + 2
        }
        unsafe {
            let mut functions: GenerateFunctions = std::mem::zeroed();
            functions.generateMinerals = Some(mineral);
            functions.generateEnergy = Some(energy);
            functions.generateLife = Some(life);
            functions.pickupMinerals = Some(pickup_mineral);
            functions.pickupEnergy = Some(pickup_energy);
            functions.pickupLife = Some(pickup_life);
            let mut system: SOLARSYS_STATE = std::mem::zeroed();
            let mut world: PLANET_DESC = std::mem::zeroed();
            system.genFuncs = &functions;
            world.data_index = 17;
            for scan in 0..3 {
                assert_eq!(
                    callGenerateForScanType(&system, &world, 5, scan, std::ptr::null_mut()),
                    22 + u16::from(scan) * 100
                );
                assert!(callPickupForScanType(
                    &mut system,
                    &mut world,
                    17 + u16::from(scan),
                    scan
                ));
                assert!(!callPickupForScanType(&mut system, &mut world, 30, scan));
            }
        }
    }

    #[test]
    fn scan_values_preserve_c_decimal_precision_boundary() {
        for (value, expected) in [
            (0, "0.00"),
            (1, "0.01"),
            (99, "0.99"),
            (100, "1.00"),
            (999, "9.99"),
            (1000, "10.0"),
            (1059, "10.5"),
        ] {
            assert_eq!(scan_value(value, ""), expected);
        }
        assert_eq!(scan_value(125, " e.s."), "1.25 e.s.");
    }
    #[test]
    fn mass_uses_original_integer_order_and_minimum() {
        assert_eq!(planet_mass(100, 100), 100);
        assert_eq!(planet_mass(1, 1), 1);
        assert_eq!(planet_mass(120, 80), 138);
        assert_eq!(planet_mass(200, 100), 800);
    }
    #[test]
    fn landing_fuel_doubles_gravity_before_clamping() {
        assert_eq!(landing_fuel(0), 0);
        assert_eq!(landing_fuel(99), 198);
        assert_eq!(landing_fuel(150), 300);
        assert_eq!(landing_fuel(151), 300);
        assert_eq!(landing_fuel(32768), 0);
    }
    #[test]
    fn cursor_wraps_longitude_but_stops_at_poles() {
        assert_eq!(move_cursor((0, 0), -1, -1, 242), (964, 0));
        assert_eq!(move_cursor((964, 296), 1, 1, 242), (0, 296));
        assert_eq!(move_cursor((100, 100), 1, -1, 242), (104, 96));
    }
}
