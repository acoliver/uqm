//! Sound decoder integration tests
//!
//! These tests verify the Ogg decoder works with real audio files.

use std::path::PathBuf;

// Import from the crate using the lib name
use uqm_rust::sound::{AudioFormat, DecodeError, DecoderFormats, OggDecoder, SoundDecoder};

/// Get path to test audio file (if available)
fn get_test_ogg_path() -> Option<PathBuf> {
    // Try to find an ogg file in the content directory
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let content_path = project_root
        .parent()
        .unwrap()
        .join("sc2/content/addons/3domusic/starbase.ogg");

    if content_path.exists() {
        Some(content_path)
    } else {
        None
    }
}

#[test]
fn test_ogg_decoder_open_real_file() {
    let path = match get_test_ogg_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping test_ogg_decoder_open_real_file: no test ogg file available");
            return;
        }
    };

    let mut decoder = OggDecoder::new();
    let formats = DecoderFormats::default();

    // Initialize module and instance
    assert!(decoder.init_module(0, &formats));
    assert!(decoder.init());

    // Open the file
    let result = decoder.open(&path);
    assert!(result.is_ok(), "Failed to open {:?}: {:?}", path, result);

    // Check that frequency is reasonable (should be 22050, 44100, or 48000 typically)
    let freq = decoder.frequency();
    assert!(
        freq > 8000 && freq <= 48000,
        "Unexpected frequency: {}",
        freq
    );

    // Check format
    let format = decoder.format();
    assert!(
        format == AudioFormat::Mono16 || format == AudioFormat::Stereo16,
        "Unexpected format: {:?}",
        format
    );

    // Verify it's not a null decoder
    assert!(!decoder.is_null());

    // Clean up
    decoder.close();
    decoder.term();
}

#[test]
fn test_ogg_decoder_decode_real_file() {
    let path = match get_test_ogg_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping test_ogg_decoder_decode_real_file: no test ogg file available");
            return;
        }
    };

    let mut decoder = OggDecoder::new();
    let formats = DecoderFormats::default();

    decoder.init_module(0, &formats);
    decoder.init();
    decoder.open(&path).expect("Failed to open ogg file");

    // Decode some audio data
    let mut buffer = [0u8; 4096];
    let result = decoder.decode(&mut buffer);

    assert!(result.is_ok(), "Decode failed: {:?}", result);
    let bytes_decoded = result.unwrap();
    assert!(bytes_decoded > 0, "No bytes decoded");

    // Verify we got actual audio data (not all zeros)
    let non_zero_count = buffer[..bytes_decoded].iter().filter(|&&b| b != 0).count();
    assert!(
        non_zero_count > 0,
        "Decoded audio is all zeros - likely not real audio data"
    );

    decoder.close();
    decoder.term();
}

#[test]
fn test_ogg_decoder_multiple_decode_calls() {
    let path = match get_test_ogg_path() {
        Some(p) => p,
        None => {
            eprintln!(
                "Skipping test_ogg_decoder_multiple_decode_calls: no test ogg file available"
            );
            return;
        }
    };

    let mut decoder = OggDecoder::new();
    let formats = DecoderFormats::default();

    decoder.init_module(0, &formats);
    decoder.init();
    decoder.open(&path).expect("Failed to open ogg file");

    let mut total_bytes = 0usize;
    let mut buffer = [0u8; 4096];

    // Decode multiple chunks
    for _ in 0..10 {
        match decoder.decode(&mut buffer) {
            Ok(bytes) => {
                if bytes == 0 {
                    // End of file
                    break;
                }
                total_bytes += bytes;
            }
            Err(DecodeError::EndOfFile) => break,
            Err(e) => panic!("Unexpected decode error: {:?}", e),
        }
    }

    assert!(
        total_bytes > 4096,
        "Expected to decode more than one buffer"
    );

    decoder.close();
    decoder.term();
}

#[test]
fn test_ogg_decoder_seek_to_start() {
    let path = match get_test_ogg_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping test_ogg_decoder_seek_to_start: no test ogg file available");
            return;
        }
    };

    let mut decoder = OggDecoder::new();
    let formats = DecoderFormats::default();

    decoder.init_module(0, &formats);
    decoder.init();
    decoder.open(&path).expect("Failed to open ogg file");

    // Decode some data first
    let mut buffer = [0u8; 4096];
    decoder.decode(&mut buffer).expect("First decode failed");

    // Seek back to start
    let result = decoder.seek(0);
    assert!(result.is_ok(), "Seek to 0 failed: {:?}", result);

    decoder.close();
    decoder.term();
}

#[test]
fn test_ogg_decoder_error_recovery() {
    let mut decoder = OggDecoder::new();
    let formats = DecoderFormats::default();

    decoder.init_module(0, &formats);
    decoder.init();

    // Try to open a non-existent file
    let result = decoder.open(&PathBuf::from("/nonexistent/file.ogg"));
    assert!(result.is_err());

    // Decoder should still be in a good state - it's not a null decoder
    assert!(!decoder.is_null());

    // Should be able to try opening another file
    if let Some(path) = get_test_ogg_path() {
        let result = decoder.open(&path);
        assert!(
            result.is_ok(),
            "Should be able to open after error: {:?}",
            result
        );
        decoder.close();
    }

    decoder.term();
}

#[test]
fn test_ogg_decoder_format_detection() {
    let path = match get_test_ogg_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping test_ogg_decoder_format_detection: no test ogg file available");
            return;
        }
    };

    let mut decoder = OggDecoder::new();
    let formats = DecoderFormats::default();

    decoder.init_module(0, &formats);
    decoder.init();
    decoder.open(&path).expect("Failed to open ogg file");

    // The format should be detected from the file
    let format = decoder.format();
    let freq = decoder.frequency();

    println!("Detected format: {:?}, frequency: {} Hz", format, freq);

    // Verify bytes per sample makes sense for format
    let bytes_per_sample = format.bytes_per_sample();
    assert!(bytes_per_sample == 1 || bytes_per_sample == 2 || bytes_per_sample == 4);

    decoder.close();
    decoder.term();
}
