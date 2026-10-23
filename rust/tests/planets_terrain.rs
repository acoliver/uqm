//! Original compiled C, sealed full-buffer fixtures and the Rust terrain owner.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use uqm_rust::comm::locdata::{CPoint, CRect};
use uqm_rust::math::{random as TFB_Random, seed_random as TFB_SeedRandom, RandomContext};
use uqm_rust::planets::render::topo_gen::{make_crater, make_gas_giant, make_storms, MAP_HEIGHT};

#[derive(Deserialize, Debug)]
struct Case {
    op: u32,
    width: usize,
    count: u16,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    rim: i16,
    crater: i16,
    set_depth: u32,
    seed: u32,
    pattern: usize,
    global_seed: u32,
    offset: usize,
    length: usize,
    input_sha256: String,
    output_sha256: String,
}

fn input(c: &Case) -> Vec<i8> {
    let mut cells = vec![0xa5_u8 as i8; 32];
    for y in 0..MAP_HEIGHT {
        for x in 0..c.width {
            cells.push(match c.pattern {
                0 => 0,
                1 => -128,
                2 => 127,
                3 => -1,
                4 => (((x * 73 + y * 151 + (x ^ y) * 37) % 256) as i32 - 128) as i8,
                _ => unreachable!("sealed input pattern"),
            });
        }
    }
    cells.extend_from_slice(&[0x5a; 32]);
    cells
}

fn bytes(cells: &[i8]) -> Vec<u8> {
    cells.iter().map(|&v| v as u8).collect()
}

fn same(actual: &[u8], expected: &[u8], source: &str, case: &Case) {
    assert_eq!(actual.len(), expected.len(), "{source}: {case:?}");
    assert!(
        actual == expected,
        "{source}: first unequal byte {:?}; {case:?}",
        actual.iter().zip(expected).position(|(a, b)| a != b)
    );
}

#[test]
fn original_c_terrain_full_buffers_guards_rng_and_optional_live_oracle() {
    let fixture = include_bytes!("fixtures/planets/terrain.bin");
    let parameters = include_str!("fixtures/planets/terrain.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(fixture)),
        "9b396857902a452780d37a8499166340e256b26082302c3286e641aadca438cb"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(parameters.as_bytes())),
        "3e016aff310efbd8bc751804a9399c03f1dcdf443f094d7e4c7a08708b60f66c"
    );
    assert_eq!(
        compare_cases(fixture, parameters, "UQM_TERRAIN_ORIGINAL_C_BUFFERS"),
        [480, 516, 300]
    );
    let fixture = include_bytes!("fixtures/planets/terrain-band-zero.bin");
    let parameters = include_str!("fixtures/planets/terrain-band-zero.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(fixture)),
        "c42875fde1a31949fe833b20132216fe52b0660d8ab0d2739d8929d46a286c9a"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(parameters.as_bytes())),
        "dffb9a489dae830dae4223eb1601e84ae919abff98072736ee2ef0bf49b47997"
    );
    assert_eq!(
        compare_cases(fixture, parameters, "UQM_TERRAIN_BAND_ZERO_C_BUFFERS"),
        [0, 0, 6]
    );
    println!("1302 full guarded C == fixture == Rust records: 480 crater, 516 storm, 306 gas; system state + 8 RNG tail draws and independent global stream");
}

fn compare_cases(fixture: &[u8], parameters: &str, raw_var: &str) -> [usize; 3] {
    let cases: Vec<Case> = serde_json::from_str(parameters).expect("sealed C parameters");
    assert_eq!(MAP_HEIGHT, 75);
    let mut raw = std::env::var_os(raw_var)
        .map(|p| File::open(p).expect("open original C capture read-only"));
    let mut oracle = std::env::var_os("UQM_TERRAIN_ORIGINAL_C_ORACLE").map(|p| {
        Command::new(p)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start original compiled C oracle")
    });
    let mut offset = 0;
    let mut counts = [0; 3];
    for case in &cases {
        assert_eq!(case.offset, offset);
        let mut cells = input(case);
        let source = bytes(&cells);
        assert_eq!(format!("{:x}", Sha256::digest(&source)), case.input_sha256);
        let mut rng = RandomContext::with_seed(case.seed);
        TFB_SeedRandom(case.global_seed);
        let rect = CRect {
            corner: CPoint {
                x: case.x,
                y: case.y,
            },
            width: case.w,
            height: case.h,
        };
        let len = cells.len();
        let depth = &mut cells[32..len - 32];
        match case.op {
            0 => make_crater(
                depth,
                case.width,
                &rect,
                case.rim,
                case.crater,
                case.set_depth != 0,
            ),
            1 => make_storms(&mut rng, depth, case.width, case.count),
            2 => make_gas_giant(&mut rng, depth, case.width, case.count, case.rim),
            _ => unreachable!("sealed C operation"),
        }
        same(&bytes(&cells[..32]), &source[..32], "leading guard", case);
        same(
            &bytes(&cells[len - 32..]),
            &source[len - 32..],
            "trailing guard",
            case,
        );
        let mut actual = rng.seed().to_le_bytes().to_vec();
        actual.extend_from_slice(&TFB_Random().to_le_bytes());
        for _ in 0..8 {
            actual.extend_from_slice(&rng.next_random().to_le_bytes());
        }
        actual.extend_from_slice(&bytes(&cells));
        assert_eq!(actual.len(), case.length);
        let expected = &fixture[offset..offset + case.length];
        assert_eq!(
            format!("{:x}", Sha256::digest(expected)),
            case.output_sha256
        );
        same(&actual, expected, "Rust == immutable full C fixture", case);
        if let Some(raw) = raw.as_mut() {
            let mut original = vec![0; case.length];
            raw.read_exact(&mut original)
                .expect("complete retained C record");
            same(&original, expected, "raw C == fixture", case);
        }
        if let Some(child) = oracle.as_mut() {
            let stdin = child.stdin.as_mut().expect("oracle stdin");
            for value in [
                case.op,
                case.width as u32,
                case.seed,
                u32::from(case.count),
                case.x as u32,
                case.y as u32,
                case.w as u32,
                case.h as u32,
                case.rim as u32,
                case.crater as u32,
                case.set_depth,
                case.global_seed,
                32,
            ] {
                stdin
                    .write_all(&value.to_le_bytes())
                    .expect("oracle header");
            }
            stdin.write_all(&source).expect("original input");
            stdin.flush().expect("oracle flush");
            let mut original = vec![0; case.length];
            child
                .stdout
                .as_mut()
                .expect("oracle stdout")
                .read_exact(&mut original)
                .expect("original C full response");
            same(
                &original,
                expected,
                "live original C == immutable fixture",
                case,
            );
        }
        offset += case.length;
        counts[case.op as usize] += 1;
    }
    assert_eq!(offset, fixture.len());
    if let Some(mut raw) = raw {
        assert_eq!(raw.read(&mut [0]).expect("capture EOF"), 0);
    }
    if let Some(mut child) = oracle {
        drop(child.stdin.take());
        assert_eq!(
            child
                .stdout
                .as_mut()
                .expect("oracle stdout")
                .read(&mut [0])
                .expect("oracle EOF"),
            0
        );
        assert!(child.wait().expect("oracle exit").success());
    }
    counts
}

#[test]
fn invalid_geometry_and_counts_fail_before_map_or_rng_mutation() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    for (width, len, count, op) in [
        (242, 242 * MAP_HEIGHT, 9, 1),
        (69, 69 * MAP_HEIGHT, 1, 1),
        (242, 242 * MAP_HEIGHT - 1, 1, 1),
        (242, 242 * MAP_HEIGHT, 0, 2),
        (242, 242 * MAP_HEIGHT, 26, 2),
        (874, 874 * MAP_HEIGHT, 1, 2),
    ] {
        let mut depth = vec![127; len];
        let mut rng = RandomContext::with_seed(19);
        let result = catch_unwind(AssertUnwindSafe(|| {
            if op == 1 {
                make_storms(&mut rng, &mut depth, width, count);
            } else {
                make_gas_giant(&mut rng, &mut depth, width, count, 2);
            }
        }));
        assert!(result.is_err());
        assert!(depth.iter().all(|&v| v == 127));
        assert_eq!(rng.seed(), 19);
    }
    for (x, y, w, h) in [
        (-1, 0, 4, 4),
        (0, -1, 4, 4),
        (0, 0, -1, 4),
        (0, 0, 4, -1),
        (238, 0, 4, 4),
        (0, 71, 4, 4),
    ] {
        let rect = CRect {
            corner: CPoint { x, y },
            width: w,
            height: h,
        };
        let mut depth = vec![127; 242 * MAP_HEIGHT];
        assert!(catch_unwind(AssertUnwindSafe(|| make_crater(
            &mut depth, 242, &rect, 6, 6, true
        )))
        .is_err());
        assert!(depth.iter().all(|&v| v == 127));
    }
}
