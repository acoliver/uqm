//! Tests that decode the Ogg Vorbis files shipped in the content tree and
//! check the PCM the decoder hands out, including seeking and refusals.

use super::*;
use std::io::Cursor;

const VOICE: &str = concat!(
    env!("UQM_RUST_ROOT"),
    "/../sc2/content/addons/3dovoice/talkingpet/talkingpet-039.ogg"
);
const MUSIC: &str = concat!(
    env!("UQM_RUST_ROOT"),
    "/../sc2/content/addons/3domusic/starbase.ogg"
);

fn read_fixture(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("fixture {path}: {e}"))
}

/// Keeps the first `pages` Ogg pages so a long recording becomes a short,
/// still valid stream whose last page carries a real granule position.
fn first_pages(data: &[u8], pages: usize) -> Vec<u8> {
    let mut starts: Vec<usize> = (0..data.len().saturating_sub(4))
        .filter(|&i| &data[i..i + 4] == b"OggS" && data[i + 4] == 0)
        .collect();
    starts.push(data.len());
    data[..starts[pages]].to_vec()
}

fn bytes_decoder(data: &[u8]) -> OggDecoder {
    let mut decoder = OggDecoder::new();
    assert!(decoder.init());
    decoder.open_from_bytes(data, "fixture.ogg").unwrap();
    decoder
}

fn file_decoder(path: &str) -> OggDecoder {
    let mut decoder = OggDecoder::new();
    assert!(decoder.init());
    decoder.open(Path::new(path)).unwrap();
    decoder
}

fn decode_all(decoder: &mut OggDecoder, chunk: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = vec![0u8; chunk];
    loop {
        match decoder.decode(&mut buf) {
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(DecodeError::EndOfFile) => return out,
            Err(other) => panic!("unexpected decode error {other:?}"),
        }
    }
}

fn ogg_page(granule: u64, payload: usize) -> Vec<u8> {
    let mut page = b"OggS".to_vec();
    page.push(0);
    page.push(0);
    page.extend_from_slice(&granule.to_le_bytes());
    page.resize(27 + payload, 0);
    page
}

#[test]
fn duration_comes_from_the_last_page_granule_position() {
    let mut stream = ogg_page(100, 40);
    stream.extend(ogg_page(48_000, 40));
    let seconds = calculate_ogg_duration(&mut Cursor::new(stream), 48_000);
    assert!((seconds - 1.0).abs() < 1e-6, "got {seconds}");
}

#[test]
fn duration_skips_trailing_pages_without_a_granule_position() {
    let mut stream = ogg_page(24_000, 40);
    stream.extend(ogg_page(u64::MAX, 40));
    let seconds = calculate_ogg_duration(&mut Cursor::new(stream), 48_000);
    assert!((seconds - 0.5).abs() < 1e-6, "got {seconds}");
}

#[test]
fn duration_is_zero_without_pages_or_a_sample_rate() {
    assert_eq!(
        calculate_ogg_duration(&mut Cursor::new(vec![0u8; 500]), 44_100),
        0.0
    );
    assert_eq!(
        calculate_ogg_duration(&mut Cursor::new(Vec::new()), 44_100),
        0.0
    );
    assert_eq!(
        calculate_ogg_duration(&mut Cursor::new(ogg_page(9, 40)), 0),
        0.0
    );
    assert_eq!(
        calculate_ogg_duration(&mut Cursor::new(ogg_page(u64::MAX, 40)), 44_100),
        0.0
    );
}

#[test]
fn duration_rewinds_the_reader_for_normal_reading() {
    let mut cursor = Cursor::new(ogg_page(44_100, 40));
    calculate_ogg_duration(&mut cursor, 44_100);
    assert_eq!(cursor.position(), 0);
}

#[test]
fn voice_clip_opens_as_mono_at_its_recorded_rate() {
    let decoder = bytes_decoder(&read_fixture(VOICE));
    assert_eq!(decoder.frequency(), 11_025);
    assert_eq!(decoder.format(), AudioFormat::Mono16);
    assert!(decoder.length() > 0.1, "length {}", decoder.length());
}

#[test]
fn music_opens_as_stereo() {
    let decoder = bytes_decoder(&first_pages(&read_fixture(MUSIC), 6));
    assert_eq!(decoder.frequency(), 44_100);
    assert_eq!(decoder.format(), AudioFormat::Stereo16);
}

#[test]
fn decoding_to_the_end_yields_the_advertised_number_of_frames() {
    let mut decoder = bytes_decoder(&read_fixture(VOICE));
    let advertised = (decoder.length() * decoder.frequency() as f32) as usize;
    let pcm = decode_all(&mut decoder, 4096);
    let frames = pcm.len() / 2;
    assert!(frames > 1000);
    assert!(
        frames.abs_diff(advertised) <= 2048,
        "decoded {frames} frames, header says {advertised}"
    );
    assert!(pcm.iter().any(|&byte| byte != 0), "clip is not silence");
}

#[test]
fn chunk_size_does_not_change_the_decoded_samples() {
    let data = read_fixture(VOICE);
    let whole = decode_all(&mut bytes_decoder(&data), 1 << 20);
    let small = decode_all(&mut bytes_decoder(&data), 100);
    assert_eq!(small, whole);
}

#[test]
fn bytes_and_file_modes_decode_identical_samples() {
    let from_bytes = decode_all(&mut bytes_decoder(&read_fixture(VOICE)), 2048);
    let from_file = decode_all(&mut file_decoder(VOICE), 2048);
    assert_eq!(from_file, from_bytes);
}

#[test]
fn file_mode_reports_the_clip_length_like_bytes_mode() {
    let from_bytes = bytes_decoder(&read_fixture(VOICE));
    let from_file = file_decoder(VOICE);
    assert_eq!(from_file.length(), from_bytes.length());
    assert_eq!(from_file.frequency(), 11_025);
    assert_eq!(from_file.format(), AudioFormat::Mono16);
}

#[test]
fn decoding_past_the_end_keeps_reporting_end_of_file() {
    let mut decoder = bytes_decoder(&read_fixture(VOICE));
    decode_all(&mut decoder, 4096);
    let mut buf = [0u8; 64];
    assert!(matches!(
        decoder.decode(&mut buf),
        Err(DecodeError::EndOfFile)
    ));
    assert!(matches!(
        decoder.decode(&mut buf),
        Err(DecodeError::EndOfFile)
    ));
}

#[test]
fn an_odd_sized_buffer_gets_whole_samples_and_no_hang() {
    let mut decoder = bytes_decoder(&read_fixture(VOICE));
    let mut buf = [0xEEu8; 7];
    let written = decoder.decode(&mut buf).unwrap();
    assert_eq!(written, 6, "three whole 16-bit samples fit in seven bytes");
    assert_eq!(buf[6], 0xEE, "the spare byte stays untouched");
}

#[test]
fn a_one_byte_buffer_receives_nothing_instead_of_spinning() {
    let mut decoder = bytes_decoder(&read_fixture(VOICE));
    let mut buf = [0xEEu8; 1];
    assert_eq!(decoder.decode(&mut buf).unwrap(), 0);
}

#[test]
fn decode_requires_init_and_an_open_stream() {
    let mut decoder = OggDecoder::new();
    decoder
        .open_from_bytes(&read_fixture(VOICE), "v.ogg")
        .unwrap();
    let mut buf = [0u8; 16];
    assert!(matches!(
        decoder.decode(&mut buf),
        Err(DecodeError::NotInitialized)
    ));

    let mut decoder = OggDecoder::new();
    decoder.init();
    assert!(matches!(
        decoder.decode(&mut buf),
        Err(DecodeError::NotInitialized)
    ));
}

#[test]
fn closing_discards_the_stream_and_term_clears_init() {
    let mut decoder = bytes_decoder(&read_fixture(VOICE));
    let mut buf = [0u8; 64];
    decoder.decode(&mut buf).unwrap();
    decoder.close();
    assert!(matches!(
        decoder.decode(&mut buf),
        Err(DecodeError::NotInitialized)
    ));
    assert_eq!(decoder.get_frame(), 0);
    decoder.term();
    assert!(!decoder.initialized);
}

#[test]
fn opening_garbage_is_refused_without_changing_the_decoder() {
    let mut decoder = OggDecoder::new();
    decoder.init();
    let error = decoder.open_from_bytes(b"this is not an ogg stream", "bad.ogg");
    assert!(
        matches!(error, Err(DecodeError::InvalidData(_))),
        "{error:?}"
    );
    let error = decoder.open_from_bytes(&[], "empty.ogg");
    assert!(
        matches!(error, Err(DecodeError::InvalidData(_))),
        "{error:?}"
    );
    let mut buf = [0u8; 16];
    assert!(matches!(
        decoder.decode(&mut buf),
        Err(DecodeError::NotInitialized)
    ));
}

#[test]
fn opening_a_text_file_by_path_is_refused() {
    let manifest = concat!(env!("UQM_RUST_ROOT"), "/Cargo.toml");
    let mut decoder = OggDecoder::new();
    decoder.init();
    assert!(matches!(
        decoder.open(Path::new(manifest)),
        Err(DecodeError::InvalidData(_))
    ));
}

#[test]
fn a_truncated_header_is_refused() {
    let data = read_fixture(VOICE);
    let mut decoder = OggDecoder::new();
    decoder.init();
    let error = decoder.open_from_bytes(&data[..40], "cut.ogg");
    assert!(
        matches!(error, Err(DecodeError::InvalidData(_))),
        "{error:?}"
    );
}

#[test]
fn damaged_audio_data_surfaces_a_decoder_error() {
    let mut data = read_fixture(VOICE);
    let middle = data.len() / 2;
    for byte in &mut data[middle..middle + 64] {
        *byte ^= 0xFF;
    }
    let mut decoder = bytes_decoder(&data);
    let mut buf = vec![0u8; 4096];
    let outcome = loop {
        match decoder.decode(&mut buf) {
            Ok(_) => continue,
            Err(error) => break error,
        }
    };
    assert!(
        matches!(outcome, DecodeError::DecoderError(_)),
        "{outcome:?}"
    );
    assert_eq!(decoder.get_error(), -1);
    assert_eq!(decoder.get_error(), 0);
}

#[test]
fn seeking_to_zero_replays_the_clip_from_the_start() {
    let mut decoder = bytes_decoder(&read_fixture(VOICE));
    let full = decode_all(&mut decoder, 4096);
    assert_eq!(decoder.seek(0).unwrap(), 0);
    assert_eq!(decode_all(&mut decoder, 4096), full);
}

#[test]
fn file_mode_rewinds_to_the_start_too() {
    let mut decoder = file_decoder(VOICE);
    let full = decode_all(&mut decoder, 4096);
    assert_eq!(decoder.seek(0).unwrap(), 0);
    assert_eq!(decode_all(&mut decoder, 4096), full);
}

#[test]
fn seeking_forward_skips_exactly_that_many_frames() {
    let data = read_fixture(VOICE);
    let full = decode_all(&mut bytes_decoder(&data), 4096);
    for target in [1usize, 777, 1500, 3000] {
        let mut decoder = bytes_decoder(&data);
        assert_eq!(decoder.seek(target as u32).unwrap(), target as u32);
        let rest = decode_all(&mut decoder, 4096);
        assert_eq!(rest, full[target * 2..], "seek to frame {target}");
    }
}

#[test]
fn seeking_forward_from_the_middle_of_a_packet_counts_the_unread_remainder() {
    let data = read_fixture(VOICE);
    let full = decode_all(&mut bytes_decoder(&data), 4096);
    let mut decoder = bytes_decoder(&data);
    let mut head = [0u8; 200];
    decoder.decode(&mut head).unwrap();
    assert_eq!(decoder.seek(2500).unwrap(), 2500);
    let rest = decode_all(&mut decoder, 4096);
    assert_eq!(rest, full[2500 * 2..]);
}

#[test]
fn seeking_backwards_replays_from_the_requested_frame() {
    let data = read_fixture(VOICE);
    let full = decode_all(&mut bytes_decoder(&data), 4096);
    let mut decoder = bytes_decoder(&data);
    let mut skipped = vec![0u8; 8000];
    decoder.decode(&mut skipped).unwrap();
    assert_eq!(decoder.seek(1200).unwrap(), 1200);
    assert_eq!(decode_all(&mut decoder, 4096), full[1200 * 2..]);
}

#[test]
fn seeking_beyond_the_end_stops_at_the_last_frame() {
    let data = read_fixture(VOICE);
    let total = decode_all(&mut bytes_decoder(&data), 4096).len() / 2;
    let mut decoder = bytes_decoder(&data);
    let reached = decoder.seek(u32::MAX).unwrap() as usize;
    assert_eq!(reached, total);
}

#[test]
fn stereo_seek_counts_frames_not_samples() {
    let data = first_pages(&read_fixture(MUSIC), 12);
    let full = decode_all(&mut bytes_decoder(&data), 8192);
    assert!(full.len() / 4 > 5000, "fixture too short: {}", full.len());
    let mut decoder = bytes_decoder(&data);
    assert_eq!(decoder.seek(4321).unwrap(), 4321);
    let rest = decode_all(&mut decoder, 8192);
    assert_eq!(rest, full[4321 * 4..]);
}

#[test]
fn get_frame_follows_the_decode_position() {
    let mut decoder = bytes_decoder(&read_fixture(VOICE));
    assert_eq!(decoder.get_frame(), 0);
    let mut buf = vec![0u8; 2048 * 2 * 2];
    let written = decoder.decode(&mut buf).unwrap();
    assert_eq!(written, buf.len());
    assert_eq!(decoder.get_frame(), 4);
}

#[test]
fn reopening_resets_the_position() {
    let data = read_fixture(VOICE);
    let mut decoder = bytes_decoder(&data);
    let mut buf = vec![0u8; 4096];
    decoder.decode(&mut buf).unwrap();
    decoder.open_from_bytes(&data, "again.ogg").unwrap();
    assert_eq!(decoder.get_frame(), 0);
    let first = decode_all(&mut decoder, 4096);
    assert_eq!(first, decode_all(&mut bytes_decoder(&data), 4096));
}
