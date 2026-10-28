use super::*;
use crate::sound::null::NullDecoder;

fn silent() -> Box<dyn SoundDecoder> {
    let mut decoder = NullDecoder::with_duration(2.0);
    assert!(decoder.init());
    Box::new(decoder)
}

#[test]
fn limited_decoder_seeks_to_start_and_stops_at_the_requested_duration() {
    let mut decoder = LimitedDecoder::new(silent(), 500, 250);
    assert_eq!(decoder.name(), "Null");
    assert_eq!(decoder.frequency(), 11025);
    assert_eq!(decoder.format(), AudioFormat::Mono16);
    assert_eq!(decoder.length(), 0.25);
    assert!(decoder.is_null());
    assert!(!decoder.needs_swap());
    assert_eq!(decoder.get_frame(), 0);
    assert_eq!(decoder.get_error(), 0);
    let mut buffer = vec![0xff; 11025];
    let size = decoder.decode(&mut buffer).unwrap();
    assert_eq!(size, 2756 * 2);
    assert!(buffer[..size].iter().all(|byte| *byte == 0));
    assert!(buffer[size..].iter().all(|byte| *byte == 0xff));
    assert_eq!(decoder.decode(&mut buffer), Err(DecodeError::EndOfFile));
    assert_eq!(decoder.seek(100), Ok(100));
    assert_eq!(decoder.decode(&mut buffer), Ok((2756 - 100) * 2));
}

#[test]
fn limited_decoder_clamps_duration_and_preserves_inner_lifecycle() {
    let mut decoder = LimitedDecoder::new(silent(), 500, 0);
    assert_eq!(decoder.length(), 1.5);
    assert_eq!(decoder.decode(&mut []), Err(DecodeError::EndOfFile));
    assert!(decoder.init_module(0, &DecoderFormats::default()));
    assert!(decoder.init());
    decoder.open(Path::new("silent")).unwrap();
    decoder.open_from_bytes(&[], "silent").unwrap();
    assert_eq!(decoder.decode(&mut [0xff; 4]), Ok(4));
    decoder.close();
    decoder.term();
    assert_eq!(
        decoder.decode(&mut [0; 4]),
        Err(DecodeError::NotInitialized)
    );
    decoder.term_module();
    let mut beyond_end = LimitedDecoder::new(silent(), 3000, 500);
    assert_eq!(beyond_end.length(), 0.0);
    assert_eq!(beyond_end.decode(&mut [0; 4]), Err(DecodeError::EndOfFile));
}

#[test]
fn decode_errors_include_operation_and_detail() {
    for (error, message) in [
        (DecodeError::NotFound("a".into()), "File not found: a"),
        (
            DecodeError::InvalidData("a".into()),
            "Invalid audio data: a",
        ),
        (
            DecodeError::UnsupportedFormat("a".into()),
            "Unsupported format: a",
        ),
        (DecodeError::IoError("a".into()), "I/O error: a"),
        (DecodeError::NotInitialized, "Decoder not initialized"),
        (DecodeError::EndOfFile, "End of file"),
        (DecodeError::SeekFailed("a".into()), "Seek failed: a"),
        (DecodeError::DecoderError("a".into()), "Decoder error: a"),
    ] {
        assert_eq!(error.to_string(), message);
    }
}
