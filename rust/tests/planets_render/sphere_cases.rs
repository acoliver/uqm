// Shared by the pure unit tests and the serialized linked parity owner.
use super::geometry::*;
use super::{Color, Point};
use sha2::{Digest, Sha256};
use std::ffi::{c_void, CString};

const GUARD: [u32; 16] = [0x51a7_cafe; 16];
const INITIAL_BYTES: usize = 607_765;
const INITIAL_SHA: &str = "b6b8a81f17952218cbb506bb21b9cfc788af1e9af3371a0551c82f91bcfa0044";
const FIXTURE_SHA: &str = "913bedd679cefcce48026df05ef53964474427944417bb37e16ef9fb9008bfbf";
pub const FIXTURE: &[u8] = include_bytes!("../fixtures/planets/sphere-geometry.bin");

#[derive(Clone, Copy)]
enum Case {
    Vector(Point3, Point3, f64, u8),
    Mask(Point),
    Tilt(i32),
    Aa(f64, f64),
    Average([u32; 4]),
}
const COLORS: [Color; 4] = [
    Color::with_alpha(255, 1, 2, 3),
    Color::with_alpha(17, 230, 31, 4),
    Color::with_alpha(91, 44, 178, 5),
    Color::with_alpha(0, 255, 128, 6),
];
const WRAP: [u32; 4] = [0xffff_ffff, 0x8000_0001, 0x7fff_ffff, 0x1234_5678];
fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3 { x, y, z }
}

fn cases() -> Vec<Case> {
    use Case::*;
    // The original capture remains an identical prefix, including all sentinels.
    let mut cases = vec![Vector(
        p(0.125, -17.75, 1024.5),
        p(-9.25, 3.5, 0.0625),
        -3.125,
        0,
    )];
    cases.extend([(1, 0), (17, -23), (1000, -2000)].map(|(x, y)| Mask(Point::new(x, y))));
    cases.extend([-45, 0, 37].map(Tilt));
    cases.extend(
        [
            (0.0, 0.0),
            (3.25, 4.5),
            (-4.5, 81.0),
            (74.25, 73.75),
            (12.0, 8.375),
        ]
        .map(|(x, y)| Aa(x, y)),
    );
    cases.push(Average(WRAP));

    let vectors = [
        (p(0.1, -0.2, 0.3), p(0.7, 1.1, -0.9), 0.1),
        (p(1.0e16, 1.0, -1.0e16), p(1.0, 1.0, 1.0), -0.7),
        (p(1.0e16, -1.0e16, 1.0), p(1.0, 1.0, 1.0), 1.1),
        (
            p(1.0000000000000002, 1.0, -1.0),
            p(1.0000000000000002, -1.0, 0.1),
            0.3,
        ),
        (
            p(1.0000000000000002, 1.0, 0.0),
            p(1.0000000000000002, -1.0000000000000004, 0.0),
            0.7,
        ),
        (
            p(1.0, 1.0000000000000002, 0.0),
            p(-1.0000000000000004, 1.0000000000000002, 0.0),
            0.7,
        ),
        (
            p(1.0, 0.0, 1.0000000000000002),
            p(-1.0000000000000004, 0.0, 1.0000000000000002),
            0.7,
        ),
        (p(1.0, 0.0, 0.0), p(-0.0, 0.1, -0.3), -0.0),
        (p(0.0, -1.0, 0.0), p(0.7, -0.0, 0.2), 0.7),
        (p(0.0, 0.0, 1.0), p(0.1, 0.3, 0.0), -0.7),
        (p(1.0e-150, -2.0e-150, 3.0e-150), p(0.1, -0.3, 0.7), 0.3),
        (p(1.0e150, -2.0e150, 3.0e150), p(0.1, -0.3, 0.7), 0.3),
        (p(-0.0, 0.1, -0.0), p(0.0, 0.10000000000000002, -0.0), -1.1),
    ];
    for (first, second, scalar) in vectors {
        for alias in 0..=2 {
            cases.push(Vector(first, second, scalar, alias));
        }
    }
    cases.extend(
        [
            (-1, 0),
            (0, 1),
            (0, -1),
            (17, 23),
            (-17, 23),
            (-17, -23),
            (i16::MIN, 0),
            (i16::MAX, 0),
            (0, i16::MIN),
            (0, i16::MAX),
            (i16::MAX, i16::MAX),
            (i16::MIN, i16::MAX),
            (i16::MAX, i16::MIN),
        ]
        .map(|(x, y)| Mask(Point::new(x, y))),
    );
    cases.extend([-180, -90, -1, 1, 90, 179, 180, 360, i32::MIN, i32::MAX].map(Tilt));
    let below = |x: f64| f64::from_bits(x.to_bits() - 1);
    let above = |x: f64| f64::from_bits(x.to_bits() + 1);
    cases.extend(
        [
            (-0.0, 0.0),
            (74.0, 74.0),
            (75.0, 75.0),
            (-1.0, 0.0),
            (0.0, -1.0),
            (below(75.0), below(75.0)),
            (above(75.0), above(75.0)),
            (below(1.0), above(1.0)),
            (above(1.0), below(1.0)),
            (below(74.0), above(74.0)),
            (f64::MIN_POSITIVE, 0.0),
            (0.0, 0.25),
            (0.25, 0.0),
            (0.0, below(1.0)),
            (below(1.0), 0.0),
            (1.0, 0.75),
            (0.75, 1.0),
            (0.1, 0.9),
            (0.9, 0.1),
            (19.125, 53.875),
            (f64::MAX, -f64::MAX),
        ]
        .map(|(x, y)| Aa(x, y)),
    );
    cases.extend(
        [
            [0; 4],
            [65536, 0, 0, 0],
            [0, 65536, 0, 0],
            [0, 0, 65536, 0],
            [0, 0, 0, 65536],
            [16384; 4],
            [32768, 16384, 8192, 8192],
            [65535, 1, 0, 0],
            [1, 65535, 0, 0],
            [32767, 32769, 0, 0],
            [65537, 0, 0, 0],
            [131072; 4],
            [u32::MAX; 4],
        ]
        .map(Average),
    );
    let counts = cases.iter().fold([0; 5], |mut counts, case| {
        counts[match case {
            Vector(..) => 0,
            Mask(..) => 1,
            Tilt(..) => 2,
            Aa(..) => 3,
            Average(..) => 4,
        }] += 1;
        counts
    });
    assert_eq!(counts, [40, 16, 13, 26, 14]);
    cases
}
fn append_p3(out: &mut Vec<u8>, point: Point3) {
    for value in [point.x, point.y, point.z] {
        out.extend(value.to_le_bytes());
    }
}
fn append_point(out: &mut Vec<u8>, point: SphereMapPoint) {
    for Point { x, y } in point.points {
        out.extend(x.to_le_bytes());
        out.extend(y.to_le_bytes());
    }
    for weight in point.weights {
        out.extend(weight.to_le_bytes());
    }
}
fn input() -> Vec<u8> {
    let mut out = Vec::new();
    for case in cases() {
        match case {
            Case::Vector(a, b, k, alias) => {
                out.extend([1, alias]);
                append_p3(&mut out, a);
                append_p3(&mut out, b);
                out.extend(k.to_le_bytes());
            }
            Case::Mask(Point { x, y }) => {
                out.push(2);
                out.extend(x.to_le_bytes());
                out.extend(y.to_le_bytes());
            }
            Case::Tilt(angle) => {
                out.push(3);
                out.extend(angle.to_le_bytes());
            }
            Case::Aa(x, y) => {
                out.push(4);
                out.extend(x.to_le_bytes());
                out.extend(y.to_le_bytes());
            }
            Case::Average(weights) => {
                out.push(5);
                for w in weights {
                    out.extend(w.to_le_bytes());
                }
            }
        }
    }
    out
}
#[repr(C)]
struct Guarded<T> {
    before: [u32; 16],
    value: T,
    after: [u32; 16],
}
impl<T> Guarded<T> {
    fn new(value: T) -> Self {
        Self {
            before: GUARD,
            value,
            after: GUARD,
        }
    }
    fn guards(&self, out: &mut Vec<u8>) {
        assert_eq!(self.before, GUARD);
        assert_eq!(self.after, GUARD);
        for word in self.before.into_iter().chain(self.after) {
            out.extend(word.to_le_bytes());
        }
    }
}
fn sentinel(byte: u8) -> SphereMapPoint {
    SphereMapPoint {
        points: [Point::new(i16::from_le_bytes([byte; 2]), i16::from_le_bytes([byte; 2])); 4],
        weights: [u32::from_le_bytes([byte; 4]); 4],
    }
}
pub fn rust_output() -> Vec<u8> {
    let mut out = Vec::new();
    let mut guards = Vec::new();
    for case in cases() {
        match case {
            Case::Vector(a, b, k, alias) => {
                let mut result = Point3::default();
                if alias == 0 {
                    p3_mult(&mut result, a, k);
                } else {
                    result = a;
                    let source = result;
                    p3_mult(&mut result, source, k);
                }
                append_p3(&mut out, result);
                match alias {
                    0 => p3_sub(&mut result, a, b),
                    1 => {
                        result = a;
                        let first = result;
                        p3_sub(&mut result, first, b);
                    }
                    2 => {
                        result = b;
                        let second = result;
                        p3_sub(&mut result, a, second);
                    }
                    _ => unreachable!(),
                }
                append_p3(&mut out, result);
                out.extend(p3_dot(a, b).to_le_bytes());
                if alias == 0 {
                    p3_norm(&mut result, a);
                } else {
                    result = a;
                    let source = result;
                    p3_norm(&mut result, source);
                }
                append_p3(&mut out, result);
            }
            Case::Mask(location) => {
                let mut value = Guarded::new([[0xa5a5_a5a5; DIAMETER]; DIAMETER]);
                generate_sphere_mask(location, &mut value.value);
                for row in &value.value {
                    for word in row {
                        out.extend(word.to_le_bytes());
                    }
                }
                value.guards(&mut guards);
            }
            Case::Tilt(angle) => {
                let mut value = Box::new(Guarded::new([[sentinel(0xa5); DIAMETER]; DIAMETER]));
                create_sphere_tilt_map(angle, &mut value.value);
                for row in &value.value {
                    for point in row {
                        append_point(&mut out, *point);
                    }
                }
                value.guards(&mut guards);
            }
            Case::Aa(x, y) => {
                let mut value = Guarded::new(sentinel(0x6d));
                create_aa_points(&mut value.value, x, y);
                append_point(&mut out, value.value);
                value.guards(&mut guards);
            }
            Case::Average(weights) => {
                for channel in -1..=3 {
                    for color in COLORS {
                        out.push(get_color_channel(color, channel));
                    }
                    out.push(get_avg_channel(COLORS, weights, channel));
                }
            }
        }
    }
    out.extend(guards);
    // The C capture buffer's two canaries, followed by the unchanged input stream.
    for word in GUARD.into_iter().chain(GUARD) {
        out.extend(word.to_le_bytes());
    }
    out.extend(input());
    out
}
pub fn check_fixture(output: &[u8]) {
    let source = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sc2/src/uqm/planets/plangen.c"
    ))
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(source)),
        "0b1605af93c4c5a2c4b99200ad03a027255e3e5d8b7479c744c5d1afba153ec0"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&FIXTURE[..INITIAL_BYTES])),
        INITIAL_SHA
    );
    assert_eq!(format!("{:x}", Sha256::digest(FIXTURE)), FIXTURE_SHA);
    assert_eq!(output.len(), FIXTURE.len());
    if let Some(index) = output.iter().zip(FIXTURE).position(|(a, b)| a != b) {
        panic!(
            "sphere fixture mismatch at byte {index}: actual {:?}, expected {:?}",
            &output[index.saturating_sub(8)..(index + 16).min(output.len())],
            &FIXTURE[index.saturating_sub(8)..(index + 16).min(FIXTURE.len())]
        );
    }
}
pub fn report_coverage(live_c: bool) {
    println!(
        "SphereGeometry: cases={} bytes={} live_C={live_c}",
        cases().len(),
        FIXTURE.len()
    );
}

pub fn original_output() -> Option<Vec<u8>> {
    type Run = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
    let path = std::env::var_os("UQM_SPHERE_GEOMETRY_ORACLE")?;
    let path = CString::new(path.as_encoded_bytes()).unwrap();
    let input = input();
    let input_before = input.clone();
    // Guard the caller's capture allocation as well as the C internal buffer.
    let mut output = vec![0xcd; FIXTURE.len() + 128];
    unsafe {
        let handle = libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
        assert!(
            !handle.is_null(),
            "load original C sphere oracle: {:?}",
            std::ffi::CStr::from_ptr(libc::dlerror())
        );
        let symbol = libc::dlsym(handle, c"original_sphere_geometry".as_ptr());
        assert!(!symbol.is_null());
        let run = std::mem::transmute::<*mut c_void, Run>(symbol);
        assert_eq!(
            run(
                output.as_mut_ptr().add(64),
                FIXTURE.len(),
                input.as_ptr(),
                input.len()
            ),
            FIXTURE.len(),
            "C result length"
        );
        assert_eq!(libc::dlclose(handle), 0);
    }
    assert_eq!(input, input_before, "C changed its inputs");
    assert_eq!(&output[..64], &[0xcd; 64]);
    assert_eq!(&output[output.len() - 64..], &[0xcd; 64]);
    Some(output[64..output.len() - 64].to_vec())
}
