use uqm_rust::supermelee::netplay::packet::{DecodeError, Packet};

#[test]
fn codec_roundtrips_frozen_original_layouts_for_every_packet_type() {
    let fixtures = include_str!("fixtures/netplay/packets.txt")
        .lines()
        .filter(|line| !line.starts_with('#'));
    let mut count = 0;
    for fixture in fixtures {
        let (name, hex) = fixture.split_once(' ').unwrap();
        let wire: Vec<u8> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect();
        let (packet, consumed) = Packet::decode(&wire).unwrap();
        assert_eq!(packet.kind(), count, "{name}");
        assert_eq!(consumed, wire.len(), "{name}");
        assert_eq!(packet.encode(), wire, "{name}");
        for prefix in 0..wire.len() {
            assert_eq!(
                Packet::decode(&wire[..prefix]),
                Err(DecodeError::Incomplete),
                "{name}, prefix {prefix}"
            );
        }
        let mut stream = vec![0xaa];
        stream.extend_from_slice(&wire);
        stream.extend_from_slice(&Packet::Ready.encode());
        assert_eq!(Packet::decode(&stream[1..]).unwrap(), (packet, wire.len()));
        count += 1;
    }
    assert_eq!(count, 18);
}

#[test]
fn variable_payloads_preserve_original_padding_lengths() {
    for size in 0..9 {
        let name = b"abcdefgh";
        let packet = Packet::TeamName {
            side: 1,
            name: name[..size].to_vec(),
        };
        let wire = packet.encode();
        assert_eq!(wire.len(), 6 + ((size + 4) & !3));
        assert_eq!(&wire[6..6 + size], &name[..size]);
        assert!(wire[6 + size..].iter().all(|&byte| byte == 0));
        assert_eq!(Packet::decode(&wire).unwrap().0, packet);
        let packet = Packet::Fleet {
            side: 0,
            ships: (0..size).map(|index| (index as u8, 255)).collect(),
        };
        let wire = packet.encode();
        assert_eq!(wire.len(), 8 + ((size * 2 + 3) & !3));
        assert!(wire[8 + size * 2..].iter().all(|&byte| byte == 0));
        assert_eq!(Packet::decode(&wire).unwrap().0, packet);
    }
}

#[test]
fn malformed_frames_are_rejected_before_dispatch() {
    for wire in [
        &[0, 4, 0, 18][..],
        &[0, 4, 0, 11],
        &[0, 6, 0, 5, 0, 0],
        &[0, 8, 0, 4, 0, 0, 0, 1],
    ] {
        assert!(matches!(
            Packet::decode(wire),
            Err(DecodeError::UnknownType | DecodeError::InvalidLength)
        ));
    }
}
