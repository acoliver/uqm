//! Planet scanning, scan-node callbacks, and lander dispatch from `scan.c`.

use crate::engine::world::planet::{NodeInfo, PlanetInfo};
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

use crate::engine::world::generation::{Body, SolarSys};
use crate::planets::generate::{SiteHost, SystemGenerator};

pub fn generate(
    generator: &dyn SystemGenerator,
    system: &mut SolarSys<'_>,
    body: Body,
    node: u16,
    scan: u8,
    info: &mut NodeInfo,
    host: &dyn SiteHost,
) -> u16 {
    match scan {
        0 => generator.generate_minerals(system, body, node, info, host),
        1 => generator.generate_energy(system, body, node, info, host),
        2 => generator.generate_life(system, body, node, info, host),
        _ => panic!("invalid scan type {scan}"),
    }
}

pub fn pickup(
    generator: &dyn SystemGenerator,
    system: &mut SolarSys<'_>,
    body: Body,
    node: u16,
    scan: u8,
    host: &mut dyn SiteHost,
) -> bool {
    match scan {
        0 => generator.pickup_minerals(system, body, node, host),
        1 => generator.pickup_energy(system, body, node, host),
        2 => generator.pickup_life(system, body, node, host),
        _ => panic!("invalid scan type {scan}"),
    }
}

pub(crate) unsafe fn is_node_retrieved(info: *const PlanetInfo, scan: u8, node: u8) -> bool {
    (*info).scan_retrieve_mask[usize::from(scan)] & (1_u32 << node) != 0
}
#[cfg(test)]
unsafe fn count_nodes_retrieved(info: *mut PlanetInfo, scan: u8) -> u16 {
    (*info).scan_retrieve_mask[usize::from(scan)].count_ones() as u16
}
pub(crate) unsafe fn set_node_retrieved(info: *mut PlanetInfo, scan: u8, node: u8) {
    (*info).scan_retrieve_mask[usize::from(scan)] |= 1_u32 << node;
}
#[cfg(test)]
unsafe fn set_node_not_retrieved(info: *mut PlanetInfo, scan: u8, node: u8) {
    (*info).scan_retrieve_mask[usize::from(scan)] &= !(1_u32 << node);
}

#[cfg(not(test))]
pub(crate) mod live;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_masks_keep_scan_types_and_all_32_nodes_independent() {
        let mut info: PlanetInfo = unsafe { std::mem::zeroed() };
        unsafe {
            for scan in 0..3 {
                for node in 0..32 {
                    assert!(!is_node_retrieved(&info, scan, node));
                    set_node_retrieved(&mut info, scan, node);
                    assert!(is_node_retrieved(&info, scan, node));
                    assert_eq!(count_nodes_retrieved(&mut info, scan), u16::from(node) + 1);
                }
            }
            for scan in 0..3 {
                set_node_not_retrieved(&mut info, scan, 31);
                set_node_not_retrieved(&mut info, scan, 0);
                assert_eq!(count_nodes_retrieved(&mut info, scan), 30);
                assert_eq!(info.scan_retrieve_mask[usize::from(scan)], 0x7fff_fffe);
            }
        }
    }

    #[test]
    fn scan_generation_and_pickup_dispatch_preserve_callback_arguments() {
        struct Generator;
        impl SystemGenerator for Generator {
            fn generate_minerals(
                &self,
                system: &mut SolarSys<'_>,
                body: Body,
                node: u16,
                _: &mut NodeInfo,
                _: &dyn SiteHost,
            ) -> u16 {
                node + u16::from(system.body_data_index(body))
            }
            fn generate_energy(
                &self,
                system: &mut SolarSys<'_>,
                body: Body,
                node: u16,
                info: &mut NodeInfo,
                host: &dyn SiteHost,
            ) -> u16 {
                self.generate_minerals(system, body, node, info, host) + 100
            }
            fn generate_life(
                &self,
                system: &mut SolarSys<'_>,
                body: Body,
                node: u16,
                info: &mut NodeInfo,
                host: &dyn SiteHost,
            ) -> u16 {
                self.generate_minerals(system, body, node, info, host) + 200
            }
            fn pickup_minerals(
                &self,
                system: &mut SolarSys<'_>,
                body: Body,
                node: u16,
                _: &mut dyn SiteHost,
            ) -> bool {
                node == u16::from(system.body_data_index(body))
            }
            fn pickup_energy(
                &self,
                system: &mut SolarSys<'_>,
                body: Body,
                node: u16,
                _: &mut dyn SiteHost,
            ) -> bool {
                node == u16::from(system.body_data_index(body)) + 1
            }
            fn pickup_life(
                &self,
                system: &mut SolarSys<'_>,
                body: Body,
                node: u16,
                _: &mut dyn SiteHost,
            ) -> bool {
                node == u16::from(system.body_data_index(body)) + 2
            }
        }
        crate::planets::generate::testing::with_empty(|system| {
            system.layout(|_, _, _, _, planets| {
                planets[0].data_index = 17;
                1
            });
            let mut host = crate::planets::generate::testing::NoEffects;
            for scan in 0..3 {
                assert_eq!(
                    generate(
                        &Generator,
                        system,
                        Body::Planet(0),
                        5,
                        scan,
                        &mut NodeInfo::default(),
                        &host
                    ),
                    22 + u16::from(scan) * 100
                );
                assert!(pickup(
                    &Generator,
                    system,
                    Body::Planet(0),
                    17 + u16::from(scan),
                    scan,
                    &mut host
                ));
                assert!(!pickup(
                    &Generator,
                    system,
                    Body::Planet(0),
                    30,
                    scan,
                    &mut host
                ));
            }
        });
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
