use std::io::{self, Cursor, Read, Write};
use uqm_save::mle::{Team, FLEET_SIZE, NAME_BYTES};

fn legacy_bytes() -> [u8; 69] {
    let mut bytes = [0; 69];
    bytes[..14].fill(255);
    bytes[0] = 3;
    bytes[13] = 24;
    bytes[14..24].copy_from_slice(b"Test fleet");
    bytes
}

#[test]
fn legacy_team_has_the_original_field_offsets_and_size() {
    assert_eq!((FLEET_SIZE, NAME_BYTES, Team::SERIALIZED_LEN), (14, 55, 69));
    let bytes = legacy_bytes();
    let team = Team::from_bytes(&bytes);
    assert_eq!(
        team.ships,
        [3, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 24]
    );
    assert_eq!(&team.name[..11], b"Test fleet\0");
    assert_eq!(team.to_bytes(), bytes);
}

#[test]
fn raw_roundtrip_preserves_invalid_ships_and_every_name_byte() {
    let ships = [0, 1, 2, 3, 24, 25, 127, 128, 200, 253, 254, 255, 12, 13];
    let name = std::array::from_fn(|index| (index as u8).wrapping_add(201));
    let team = Team { ships, name };
    let bytes = team.to_bytes();
    assert_eq!(&bytes[..14], &ships);
    assert_eq!(&bytes[14..], &name);
    assert_eq!(Team::from_bytes(&bytes), team);
    assert_eq!(bytes[44], 231);
}

#[test]
fn reads_one_team_without_consuming_the_next_record() -> io::Result<()> {
    let bytes = legacy_bytes();
    let mut input = Cursor::new([bytes.as_slice(), &[7, 8, 9]].concat());
    assert_eq!(Team::read_from(&mut input)?.to_bytes(), bytes);
    assert_eq!(input.position(), 69);
    let mut tail = Vec::new();
    input.read_to_end(&mut tail)?;
    assert_eq!(tail, [7, 8, 9]);
    Ok(())
}

#[test]
fn every_truncated_record_is_rejected() {
    let bytes = legacy_bytes();
    for length in 0..bytes.len() {
        let Err(error) = Team::read_from(&bytes[..length]) else {
            panic!("accepted truncated team of length {length}");
        };
        assert_eq!(
            error.kind(),
            io::ErrorKind::UnexpectedEof,
            "length {length}"
        );
    }
}

struct Chunked<T>(T);

impl<T: Read> Read for Chunked<T> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let length = bytes.len().min(2);
        self.0.read(&mut bytes[..length])
    }
}

impl<T: Write> Write for Chunked<T> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(&bytes[..bytes.len().min(2)])
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[test]
fn partial_stream_operations_keep_the_same_bytes() -> io::Result<()> {
    let bytes = legacy_bytes();
    let team = Team::read_from(Chunked(bytes.as_slice()))?;
    let mut output = Chunked(Vec::new());
    team.write_to(&mut output)?;
    assert_eq!(output.0, bytes);
    Ok(())
}

#[test]
fn short_writes_and_underlying_io_failures_are_visible() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "team output closed",
            ))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let team = Team::from_bytes(&legacy_bytes());
    let mut short = [0; 68];
    let Err(error) = team.write_to(&mut short[..]) else {
        panic!("accepted short team output");
    };
    assert_eq!(error.kind(), io::ErrorKind::WriteZero);
    assert_eq!(short, legacy_bytes()[..68]);
    let Err(error) = team.write_to(Broken) else {
        panic!("accepted failed team output");
    };
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(error.to_string(), "team output closed");
}
