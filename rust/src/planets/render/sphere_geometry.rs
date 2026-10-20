//! Pure sphere lookup geometry from `plangen.c`.
//!
//! `units.h` fixes SAFE_Y=0, so MAP_HEIGHT=75 and SPHERE_SPAN_X=MAP_HEIGHT,
//! independently of ScreenWidth and output/window scaling. No scaled sphere
//! array is needed. Each routine preserves the C-inactive fields.

use crate::graphics::cmap::Color;
use crate::graphics::drawable::Point;

pub const RADIUS: i32 = 37;
pub const DIAMETER: usize = (RADIUS * 2 + 1) as usize;
pub const DIFFUSE_BITS: u32 = 16;
pub const AA_WEIGHT_BITS: u32 = 16;
pub const MAP_HEIGHT: usize = crate::planets::render::topo_gen::MAP_HEIGHT;
const SPHERE_SPAN_X: usize = MAP_HEIGHT;
const RADIUS_2: i32 = RADIUS * RADIUS;
const RADIUS_THRES: i32 = (RADIUS + 1) * (RADIUS + 1);
const M_DEG2RAD: f64 = std::f64::consts::TAU / 360.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SphereMapPoint {
    pub points: [Point; 4],
    pub weights: [u32; 4],
}

impl Default for SphereMapPoint {
    fn default() -> Self {
        Self {
            points: [Point::new(0, 0); 4],
            weights: [0; 4],
        }
    }
}

pub type SphereTiltMap = [[SphereMapPoint; DIAMETER]; DIAMETER];
pub type SphereMask = [[u32; DIAMETER]; DIAMETER];

pub fn p3_mult(result: &mut Point3, vector: Point3, constant: f64) {
    result.x = vector.x * constant;
    result.y = vector.y * constant;
    result.z = vector.z * constant;
}

pub fn p3_sub(result: &mut Point3, first: Point3, second: Point3) {
    result.x = first.x - second.x;
    result.y = first.y - second.y;
    result.z = first.z - second.z;
}

pub fn p3_dot(first: Point3, second: Point3) -> f64 {
    // The clang -O2 oracle fuses the x product into the rounded y product,
    // then the z product into that sum. Cancellation cases distinguish both.
    let xy = first.x.mul_add(second.x, first.y * second.y);
    first.z.mul_add(second.z, xy)
}

/// Normalize a vector with a finite, positive computed squared norm.
pub fn p3_norm(result: &mut Point3, vector: Point3) {
    let magnitude = p3_dot(vector, vector).sqrt();
    p3_mult(result, vector, 1.0 / magnitude);
}

/// Rebuild the diffuse sphere mask for a nonzero light location whose squared
/// coordinates sum to at most i32::MAX, matching the defined C input domain.
pub fn generate_sphere_mask(location: Point, output: &mut SphereMask) {
    let (lx, ly) = (i32::from(location.x), i32::from(location.y));
    let radial = ((lx * lx + ly * ly) as f64).sqrt();
    let mut light = Point3 {
        x: -(lx as f64),
        y: -(ly as f64),
        z: 1.2 * radial,
    };
    let light_vector = light;
    p3_norm(&mut light, light_vector);
    for (row, y) in (-RADIUS..=RADIUS).enumerate() {
        let y2 = y * y;
        for (column, x) in (-RADIUS..=RADIUS).enumerate() {
            let x2 = x * x;
            let radius2 = x2 + y2;
            let mut intensity = 0_u32;
            if radius2 < RADIUS_THRES {
                let mut normal = Point3 {
                    x: x as f64,
                    y: y as f64,
                    z: (((RADIUS_2 - x2) as f64).sqrt() * ((RADIUS_2 - y2) as f64).sqrt())
                        / RADIUS as f64,
                };
                let normal_vector = normal;
                p3_norm(&mut normal, normal_vector);
                let mut diffuse = p3_dot(normal, light);
                if diffuse < 0.0 {
                    diffuse = 0.0;
                }
                if diffuse < 0.1 {
                    diffuse = 0.1;
                }
                if radius2 > RADIUS_2 {
                    diffuse *= 1.0 - ((radius2 as f64).sqrt() - RADIUS as f64);
                    if diffuse < 0.0 {
                        diffuse = 0.0;
                    }
                }
                intensity = (diffuse * (1_u32 << DIFFUSE_BITS) as f64) as u32;
            }
            output[row][column] = intensity;
        }
    }
}

/// Populate the four bilinear-distance weights, preserving C's early return:
/// when the clamped location is integral, fields 1..3 are left unchanged.
pub fn create_aa_points(point: &mut SphereMapPoint, mut x: f64, mut y: f64) {
    if x < 0.0 {
        x = 0.0;
    } else if x >= SPHERE_SPAN_X as f64 {
        x = (SPHERE_SPAN_X - 1) as f64;
    }
    if y < 0.0 {
        y = 0.0;
    } else if y >= MAP_HEIGHT as f64 {
        y = (MAP_HEIGHT - 1) as f64;
    }
    point.points[0] = Point::new(x as i16, y as i16);
    let dx = x - f64::from(point.points[0].x);
    let dy = y - f64::from(point.points[0].y);
    if dx == 0.0 && dy == 0.0 {
        point.weights[0] = 0;
        return;
    }
    let next_x = point.points[0].x + i16::from(dx != 0.0);
    let next_y = point.points[0].y + i16::from(dy != 0.0);
    point.points[1] = Point::new(next_x, point.points[0].y);
    point.points[2] = Point::new(point.points[0].x, next_y);
    point.points[3] = Point::new(next_x, next_y);
    let inverse_x = 1.0 - dx.abs();
    let inverse_x = inverse_x * inverse_x;
    let inverse_y = 1.0 - dy.abs();
    let inverse_y = inverse_y * inverse_y;
    let dx2 = dx * dx;
    let dy2 = dy * dy;
    let d1 = (dx2 + dy2).sqrt();
    let d2 = (inverse_x + dy2).sqrt();
    let d3 = (dx2 + inverse_y).sqrt();
    let d4 = (inverse_x + inverse_y).sqrt();
    let first = 1.0 / (1.0 + d1 * (1.0 / d2 + 1.0 / d3 + 1.0 / d4));
    let weights = [first, first * d1 / d2, first * d1 / d3, first * d1 / d4];
    for (out, weight) in point.weights.iter_mut().zip(weights) {
        *out = (weight * (1_u32 << AA_WEIGHT_BITS) as f64 + 0.5) as u32;
    }
}

pub fn get_color_channel(color: Color, channel: i32) -> u8 {
    match channel {
        0 => color.r,
        1 => color.g,
        2 => color.b,
        _ => 0,
    }
}

/// Sum in wrapping DWORD arithmetic before shifting and saturating as C does.
pub fn get_avg_channel(colors: [Color; 4], multipliers: [u32; 4], channel: i32) -> u8 {
    let sum = colors
        .iter()
        .zip(multipliers)
        .fold(0_u32, |sum, (color, multiplier)| {
            sum.wrapping_add(u32::from(get_color_channel(*color, channel)).wrapping_mul(multiplier))
        });
    (sum >> AA_WEIGHT_BITS).min(255) as u8
}

/// Create the topography-to-sphere map. Integral/AA paths preserve exactly
/// the fields that the original routine does not write.
pub fn create_sphere_tilt_map(angle: i32, map: &mut SphereTiltMap) {
    let mult_x = SPHERE_SPAN_X as f64 / std::f64::consts::PI;
    let mult_y = MAP_HEIGHT as f64 / std::f64::consts::PI;
    let x_adjust = SPHERE_SPAN_X as f64 / 2.0;
    for y in -RADIUS..=RADIUS {
        let y2 = y * y;
        for x in -RADIUS..=RADIUS {
            let entry = &mut map[(y + RADIUS) as usize][(x + RADIUS) as usize];
            let radius2 = x * x + y2;
            if radius2 >= RADIUS_THRES {
                entry.points[0] = Point::new((x + RADIUS) as i16, (y + RADIUS) as i16);
                entry.weights[0] = 0;
                continue;
            }
            let mut radius = (radius2 as f64).sqrt();
            if radius >= RADIUS as f64 {
                radius = RADIUS as f64 - 0.1;
            }
            // The C oracle emits fmadd here; separate rounding changes INT_MAX maps.
            let angle_at = M_DEG2RAD.mul_add(angle as f64, (y as f64).atan2(x as f64));
            let dx = radius * angle_at.cos();
            let dy = radius * angle_at.sin();
            let xa = (-dx / RADIUS as f64).acos();
            let ya = (-dy / RADIUS as f64).acos();
            let new_y = mult_y * ya;
            let new_x = if ya <= 0.05 || ya >= 3.1 {
                x_adjust
            } else {
                x_adjust + ((mult_x * xa - x_adjust) / ya.sin())
            };
            create_aa_points(entry, new_x, new_y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super as geometry;
    use super::*;
    mod cases {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/planets_render/sphere_cases.rs"
        ));
    }

    #[test]
    fn aa_integral_shortcut_preserves_inactive_fields_and_clamps_edges() {
        let mut p = SphereMapPoint {
            points: [Point::new(9, 8); 4],
            weights: [7; 4],
        };
        create_aa_points(&mut p, -2.0, MAP_HEIGHT as f64);
        assert_eq!(
            p.points,
            [
                Point::new(0, 74),
                Point::new(9, 8),
                Point::new(9, 8),
                Point::new(9, 8)
            ]
        );
        assert_eq!(p.weights, [0, 7, 7, 7]);
        create_aa_points(&mut p, 3.25, 4.5);
        assert_eq!(
            p.points,
            [
                Point::new(3, 4),
                Point::new(4, 4),
                Point::new(3, 5),
                Point::new(4, 5)
            ]
        );
        assert_eq!(p.weights.iter().copied().sum::<u32>(), 65_536);
    }

    #[test]
    fn channels_use_rgb_selection_wrap_saturation_and_invalid_zero() {
        let colors = [Color::with_alpha(255, 3, 4, 0); 4];
        assert_eq!(get_color_channel(colors[0], -1), 0);
        assert_eq!(get_color_channel(colors[0], 2), 4);
        assert_eq!(get_avg_channel(colors, [u32::MAX; 4], 0), 255);
        assert_eq!(get_avg_channel(colors, [0; 4], 9), 0);
    }

    #[test]
    fn complete_outputs_match_sealed_original_c_fixture() {
        cases::check_fixture(&cases::rust_output());
    }

    #[test]
    fn supplied_original_c_oracle_replays_the_immutable_fixture() {
        let original = cases::original_output();
        let live_c = original.is_some();
        if let Some(output) = original {
            cases::check_fixture(&output);
        }
        cases::report_coverage(live_c);
    }

    #[test]
    fn sphere_mask_and_tilt_map_write_only_their_c_domains() {
        #[repr(C)]
        struct Guarded<T> {
            before: [u32; 16],
            map: T,
            after: [u32; 16],
        }
        let mut mask = Guarded {
            before: [0x51a7_cafe; 16],
            map: [[0xdead_beef; DIAMETER]; DIAMETER],
            after: [0x51a7_cafe; 16],
        };
        generate_sphere_mask(Point::new(11, -19), &mut mask.map);
        assert_eq!(mask.before, [0x51a7_cafe; 16]);
        assert_eq!(mask.after, [0x51a7_cafe; 16]);
        assert_eq!(mask.map[0][0], 0);
        assert!(mask.map[RADIUS as usize][RADIUS as usize] > 0);
        let sentinel = SphereMapPoint {
            points: [Point::new(-16657, -16657); 4],
            weights: [0xdead_beef; 4],
        };
        let mut tilt = Box::new(Guarded {
            before: [0x51a7_cafe; 16],
            map: [[sentinel; DIAMETER]; DIAMETER],
            after: [0x51a7_cafe; 16],
        });
        create_sphere_tilt_map(37, &mut tilt.map);
        assert_eq!(tilt.before, [0x51a7_cafe; 16]);
        assert_eq!(tilt.after, [0x51a7_cafe; 16]);
        assert_eq!(tilt.map[0][0].points[0], Point::new(0, 0));
        assert_eq!(tilt.map[0][0].points[1], Point::new(-16657, -16657));
        assert_eq!(tilt.map[0][0].weights[1], 0xdead_beef);
    }
}
