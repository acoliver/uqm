//! Read-only full-map oracle for the elevation subset of original plangen.c.

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::render::topo_gen::{
    dither_map, generate_light_map, validate_map, MAP_HEIGHT,
};

fn input(width: usize, pattern: usize) -> Vec<i8> {
    let mut cells = Vec::with_capacity(width * MAP_HEIGHT);
    for y in 0..MAP_HEIGHT {
        for x in 0..width {
            let i = y * width + x;
            let value = match pattern {
                0..=5 => [-128, -127, -1, 0, 1, 127][pattern],
                6..=13 => {
                    let (a, b) = [
                        (0, 18),
                        (0, 20),
                        (0, 58),
                        (0, 60),
                        (-19, 0),
                        (-20, 0),
                        (-59, 0),
                        (-60, 0),
                    ][pattern - 6];
                    if i.is_multiple_of(2) {
                        a
                    } else {
                        b
                    }
                }
                14 => (i % 256) as i32 - 128,
                15 => {
                    if (x + y).is_multiple_of(2) {
                        -128
                    } else {
                        127
                    }
                }
                16 => {
                    if y == 0 {
                        if x < width / 2 {
                            -128
                        } else {
                            127
                        }
                    } else {
                        0
                    }
                }
                17 => {
                    if (x == 0 || x == width - 1) && (y == 0 || y == 74) {
                        127
                    } else {
                        -128
                    }
                }
                18 => ((x + y * 3) % 17) as i32 - 8,
                19 => ((x * x + 17 * y + 13 * x * y) % 256) as i32 - 128,
                20 => [-128, 0, 127, -1][x % 4],
                21 => (x % 256) as i32 - 128,
                22 => {
                    if [7, 8, 66, 67].contains(&y) || [7, 8, width - 8].contains(&x) {
                        127
                    } else {
                        -128
                    }
                }
                23 => ((x * 73 + y * 151 + ((x ^ y) * 37)) % 256) as i32 - 128,
                _ => unreachable!("fixture pattern"),
            };
            cells.push(value as i8);
        }
    }
    cells
}

fn bytes(cells: &[i8]) -> Vec<u8> {
    cells.iter().map(|&v| v as u8).collect()
}

#[test]
fn original_c_elevation_fixture_and_optional_live_oracle() {
    let fixture = include_str!("fixtures/planets/elevation.txt");
    assert_eq!(
        format!("{:x}", Sha256::digest(fixture.as_bytes())),
        "88badb0e3e0ebce4440af58b6ecdb9cc64fd68009e8390e0ae384052c0db947f"
    );
    assert_eq!(MAP_HEIGHT, 75);
    let mut expected = fixture.lines().skip(1);
    let mut raw = std::env::var_os("UQM_PLANGEN_ORIGINAL_C_BUFFERS")
        .map(|path| File::open(path).expect("open retained original-C buffers read-only"));
    let mut raw_hash = Sha256::new();
    let mut oracle = std::env::var_os("UQM_PLANGEN_ORIGINAL_C_ORACLE").map(|path| {
        Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("launch retained original-C oracle")
    });
    let mut records = 0;
    for width in [242, 243, 244, 245, 562] {
        for pattern in 0..24 {
            for seed in [1_u32, 0x1234_5678] {
                let source = input(width, pattern);
                let source_bytes = bytes(&source);
                for op in 0..4_u32 {
                    let mut actual = source.clone();
                    let mut rng = RandomContext::with_seed(seed);
                    match op {
                        0 => dither_map(&mut rng, &mut actual, width),
                        1 => validate_map(&mut actual, width),
                        2 => generate_light_map(&mut actual, width),
                        3 => {
                            dither_map(&mut rng, &mut actual, width);
                            validate_map(&mut actual, width);
                            generate_light_map(&mut actual, width);
                        }
                        _ => unreachable!(),
                    }
                    let result = bytes(&actual);
                    let line = format!(
                        "{op} {width} {pattern} {seed} {} {} {:x} {:x}",
                        rng.seed(),
                        result.len(),
                        Sha256::digest(&source_bytes),
                        Sha256::digest(&result),
                    );
                    assert_eq!(
                        Some(line.as_str()),
                        expected.next(),
                        "op={op} width={width} pattern={pattern} seed={seed}"
                    );
                    let mut response = rng.seed().to_le_bytes().to_vec();
                    response.extend_from_slice(&result);
                    if let Some(raw) = raw.as_mut() {
                        let mut original = vec![0; response.len()];
                        raw.read_exact(&mut original)
                            .expect("complete original C record");
                        assert_eq!(response, original, "full C buffer and RNG record {records}");
                        raw_hash.update(&original);
                    }
                    if let Some(child) = oracle.as_mut() {
                        let stdin = child.stdin.as_mut().expect("oracle stdin");
                        for value in [op, width as u32, seed, source_bytes.len() as u32] {
                            stdin
                                .write_all(&value.to_le_bytes())
                                .expect("oracle request");
                        }
                        stdin.write_all(&source_bytes).expect("oracle input");
                        stdin.flush().expect("flush oracle request");
                        let mut original = vec![0; response.len()];
                        child
                            .stdout
                            .as_mut()
                            .expect("oracle stdout")
                            .read_exact(&mut original)
                            .expect("original C response");
                        assert_eq!(
                            response, original,
                            "live original C full buffer and RNG record {records}"
                        );
                    }
                    records += 1;
                }
            }
        }
    }
    assert_eq!(records, 960);
    assert_eq!(expected.next(), None, "no untested fixture records");
    if let Some(mut raw) = raw {
        assert_eq!(raw.read(&mut [0]).expect("raw EOF"), 0);
        assert_eq!(
            format!("{:x}", raw_hash.finalize()),
            "aa7f5f2709ab3c862e0f658a36295b72b47faec5b11c506fa6a357468991db8e"
        );
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
}

#[test]
fn dither_consumes_ceil_one_draw_per_four_cells() {
    for width in [242, 243, 244, 245] {
        let mut actual = RandomContext::with_seed(19);
        let mut expected = RandomContext::with_seed(19);
        let mut cells = vec![0; width * MAP_HEIGHT];
        dither_map(&mut actual, &mut cells, width);
        for _ in 0..cells.len().div_ceil(4) {
            expected.next_random();
        }
        assert_eq!(actual.seed(), expected.seed());
        assert!(cells.iter().all(|v| (-3..=4).contains(v)));
    }
}

#[test]
fn smooth_maps_are_levelled_and_wrapped_discontinuities_removed() {
    for value in [-128, -1, 0, 1, 127] {
        let mut cells = vec![value; 242 * MAP_HEIGHT];
        generate_light_map(&mut cells, 242);
        assert!(cells.iter().all(|v| *v == 0));
    }
    let mut cells = vec![-128; 242 * MAP_HEIGHT];
    cells[241] = 127;
    validate_map(&mut cells, 242);
    assert!(cells.iter().all(|v| *v == -128));
}
