//! @plan PLAN-20260314-SOUND.P34
//! Decoder regressions against installed libopenmpt, not player-generated expectations.
use uqm_rust::sound::decoder::{DecodeError, SoundDecoder};
use uqm_rust::sound::mod_decoder::ModDecoder;

fn fixture(name: &str, extension: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tracker")
            .join(format!("{name}.{extension}")),
    )
    .unwrap()
}

fn decoder(name: &str) -> ModDecoder {
    let mut decoder = ModDecoder::new();
    assert!(decoder.init());
    decoder
        .open_from_bytes(&fixture(name, "mod"), name)
        .unwrap();
    decoder
}

#[test]
fn non_looping_sample_end_matches_reference_click_removal() {
    let reference = fixture("finite", "f32le");
    let mut pcm = vec![0; reference.len() / 2];
    assert_eq!(decoder("finite").decode(&mut pcm).unwrap(), pcm.len());
    for frame in [1360, 1362, 1363, 1500, 2000, 3000] {
        for channel in 0..2 {
            let sample = frame * 2 + channel;
            let actual = f32::from(i16::from_le_bytes(
                pcm[sample * 2..sample * 2 + 2].try_into().unwrap(),
            )) / 32767.0;
            let expected =
                f32::from_le_bytes(reference[sample * 4..sample * 4 + 4].try_into().unwrap());
            assert!(
                (actual - expected).abs() < 1.0 / 32767.0,
                "frame {frame} channel {channel}: actual={actual}, reference={expected}"
            );
        }
    }
}

#[test]
fn reference_control_duration_and_complete_frames() {
    let manifest: serde_json::Value =
        serde_json::from_slice(&fixture("reference", "json")).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let seconds = case["duration"].as_f64().unwrap();
        let mut decoder = decoder(name);
        assert!(
            (f64::from(decoder.length()) - seconds).abs() < 0.00001,
            "{name}"
        );
        let expected_frames = (seconds * 44100.0).round() as usize;
        let mut pcm = vec![0; (expected_frames + 1) * 4];
        assert_eq!(
            decoder.decode(&mut pcm).unwrap(),
            expected_frames * 4,
            "{name}"
        );
        assert_eq!(
            decoder.decode(&mut pcm),
            Err(DecodeError::EndOfFile),
            "{name}"
        );
    }
}
#[test]
fn reference_initial_pcm_gain_and_clock() {
    let reference = fixture("mix", "f32le");
    let mut pcm = vec![0; reference.len() / 2];
    decoder("mix").decode(&mut pcm).unwrap();
    let actual_peak = pcm
        .chunks_exact(2)
        .map(|v| f32::from(i16::from_le_bytes(v.try_into().unwrap())).abs() / 32767.0)
        .fold(0.0_f32, f32::max);
    let reference_peak = reference
        .chunks_exact(4)
        .map(|v| f32::from_le_bytes(v.try_into().unwrap()).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        (actual_peak - reference_peak).abs() < 1.0 / 32767.0,
        "actual={actual_peak}, reference={reference_peak}"
    );
}

#[test]
fn reference_pcm_pitch_gain_pan_and_effects() {
    for name in [
        "low-b",
        "short-periods",
        "extended-slide",
        "pcm",
        "mix",
        "finite",
        "off-grid",
        "empty-word",
        "delay",
        "loop",
        "jump",
        "tempo",
        "fractional-tempo",
    ] {
        let reference = fixture(name, "f32le");
        let mut pcm = vec![0; reference.len() / 2];
        let bytes = decoder(name).decode(&mut pcm).unwrap();
        assert_eq!(bytes, pcm.len(), "{name}");
        let mut absolute_error = 0.0_f64;
        let mut peak = 0.0_f32;
        let mut reference_peak = 0.0_f32;
        for (actual, expected) in pcm.chunks_exact(2).zip(reference.chunks_exact(4)) {
            let actual = f32::from(i16::from_le_bytes(actual.try_into().unwrap())) / 32767.0;
            let expected = f32::from_le_bytes(expected.try_into().unwrap());
            absolute_error += f64::from((actual - expected).abs());
            peak = peak.max(actual.abs());
            reference_peak = reference_peak.max(expected.abs());
        }
        let mean_absolute_error = absolute_error / (pcm.len() / 2) as f64;
        eprintln!(
            "{name}: MAE={mean_absolute_error}, peak={peak}, reference_peak={reference_peak}"
        );
        // One i16 quantization step; the fixture has no interpolation or volume ramping.
        for (row, (actual, expected)) in pcm
            .chunks(5292 * 4)
            .zip(reference.chunks(5292 * 8))
            .enumerate()
        {
            let error: f64 = actual
                .chunks_exact(2)
                .zip(expected.chunks_exact(4))
                .map(|(a, e)| {
                    (f64::from(i16::from_le_bytes(a.try_into().unwrap())) / 32767.0
                        - f64::from(f32::from_le_bytes(e.try_into().unwrap())))
                    .abs()
                })
                .sum();
            eprintln!(
                "{name} row {row}: MAE={}",
                error / (actual.len() / 2) as f64
            );
        }
        assert!(
            (peak - reference_peak).abs() < 1.0 / 32767.0,
            "{name}: gain"
        );
        // Nearest-neighbor transitions can differ by one source sample from fixed-point rounding.
        assert!(mean_absolute_error < 0.003, "{name}: pitch/effects");
    }
}

#[test]
fn seek_pcm_position_replays_exact_audio_and_clamps_at_eof() {
    let mut decoder = decoder("pcm");
    let mut contiguous = vec![0; 30000 * 4];
    decoder.decode(&mut contiguous).unwrap();
    for position in [1024, 7000, 22000, 0] {
        assert_eq!(decoder.seek(position).unwrap(), position);
        let mut chunk = vec![0; 1024];
        assert_eq!(decoder.decode(&mut chunk).unwrap(), chunk.len());
        assert_eq!(
            chunk,
            contiguous[position as usize * 4..position as usize * 4 + 1024]
        );
    }
    assert_eq!(decoder.seek(u32::MAX).unwrap(), 37044);
    assert_eq!(decoder.decode(&mut [0; 4]), Err(DecodeError::EndOfFile));
    decoder.seek(0).unwrap();
    assert_eq!(decoder.decode(&mut [0; 4]).unwrap(), 4);
    decoder.close();
    assert_eq!(decoder.length(), 0.0);
    assert!(decoder.seek(0).is_err());
}

#[test]
fn unsupported_extended_semantics_are_explicit_not_silent() {
    for argument in [0x00, 0x31, 0xf1] {
        let mut bytes = fixture("finite", "mod");
        bytes[1086] = 0x1e;
        bytes[1087] = argument;
        let mut decoder = ModDecoder::new();
        assert!(
            matches!(
                decoder.open_from_bytes(&bytes, "unsupported"),
                Err(DecodeError::UnsupportedFormat(_))
            ),
            "E{argument:02X}"
        );
    }
}

#[test]
fn malformed_signature_and_active_order_are_rejected() {
    for (offset, value) in [(1080, b'X'), (952, 255)] {
        let mut bytes = fixture("finite", "mod");
        bytes[offset] = value;
        assert!(matches!(
            ModDecoder::new().open_from_bytes(&bytes, "malformed"),
            Err(DecodeError::InvalidData(_))
        ));
    }
}

#[test]
#[ignore = "writes requested decoder comparison evidence"]
fn emit_reference_comparison_pcm() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tmp/issue34-final-20261008/continuation-commfix/decoder");
    for name in [
        "low-b",
        "short-periods",
        "extended-slide",
        "pcm",
        "mix",
        "finite",
        "off-grid",
        "empty-word",
        "delay",
        "loop",
        "jump",
        "tempo",
        "fractional-tempo",
    ] {
        let mut data = vec![0; fixture(name, "f32le").len() / 2];
        let written = decoder(name).decode(&mut data).unwrap();
        std::fs::write(root.join(format!("{name}.rust.i16le")), &data[..written]).unwrap();
    }
}

#[test]
#[ignore = "requires installed shipped UQM content; writes decoder comparison evidence"]
fn shipped_durations_and_pcm_prefixes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let evidence = root.join("tmp/issue34-final-20261008/continuation-commfix/decoder");
    let cases: serde_json::Value =
        serde_json::from_slice(&fixture("shipped-durations", "json")).unwrap();
    let mut failures = Vec::new();
    for case in cases.as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        let bytes = std::fs::read(root.join(path)).unwrap();
        let mut decoder = ModDecoder::new();
        decoder.init();
        decoder.open_from_bytes(&bytes, path).unwrap();
        let expected = case["duration"].as_f64().unwrap();
        let actual = f64::from(decoder.length());
        eprintln!("{path}: reference={expected}, rust={actual}");
        if (actual - expected).abs() > 1.0 / 44100.0 {
            failures.push(path.to_string());
        }
        let mut pcm = vec![0; 44100 * 4 * 3];
        let count = decoder.decode(&mut pcm).unwrap();
        let filename = path
            .strip_prefix("sc2/content/base/")
            .unwrap()
            .replace('/', "_");
        std::fs::write(
            evidence.join(format!("{filename}.rust.i16le")),
            &pcm[..count],
        )
        .unwrap();
    }
    assert!(failures.is_empty(), "duration mismatches: {failures:?}");
}

#[test]
#[ignore = "requires installed shipped UQM content; renders each song to completion"]
fn shipped_full_playback_ends_at_reference_duration() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cases: serde_json::Value =
        serde_json::from_slice(&fixture("shipped-durations", "json")).unwrap();
    for case in cases.as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        let mut decoder = ModDecoder::new();
        decoder.init();
        decoder
            .open_from_bytes(&std::fs::read(root.join(path)).unwrap(), path)
            .unwrap();
        let expected_frames = (case["duration"].as_f64().unwrap() * 44100.0).round() as u64;
        let mut frames = 0;
        let mut pcm = [0; 16384];
        loop {
            match decoder.decode(&mut pcm) {
                Ok(bytes) => {
                    assert!(bytes > 0 && bytes % 4 == 0, "{path}");
                    frames += bytes as u64 / 4;
                    assert!(frames <= expected_frames, "{path}: playback did not stop");
                }
                Err(DecodeError::EndOfFile) => break,
                Err(error) => panic!("{path}: {error}"),
            }
        }
        assert_eq!(frames, expected_frames, "{path}");
        assert_eq!(decoder.decode(&mut pcm), Err(DecodeError::EndOfFile));
        assert_eq!(decoder.seek(0).unwrap(), 0);
        assert_eq!(decoder.decode(&mut pcm).unwrap(), pcm.len());
        eprintln!("{path}: completed {frames} frames at 44100 Hz and restarted");
    }
}
