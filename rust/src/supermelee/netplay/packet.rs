//! Explicit protocol 0.4 encoding. Reserved bytes are emitted as zero.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    Init { protocol: [u8; 2], version: [u8; 3] },
    Ping(u32),
    Ack(u32),
    Ready,
    Fleet { side: u8, ships: Vec<(u8, u8)> },
    TeamName { side: u8, name: Vec<u8> },
    Handshake0,
    Handshake1,
    HandshakeCancel,
    HandshakeCancelAck,
    SeedRandom(u32),
    InputDelay(u32),
    SelectShip(u16),
    BattleInput(u8),
    FrameCount(u32),
    Checksum { frame: u32, checksum: u32 },
    Abort(u16),
    Reset(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Incomplete,
    UnknownType,
    InvalidLength,
}

const MIN_LENGTH: [usize; 18] = [12, 8, 8, 4, 8, 6, 4, 4, 4, 4, 8, 8, 8, 8, 8, 12, 8, 8];

impl Packet {
    pub fn kind(&self) -> usize {
        match self {
            Self::Init { .. } => 0,
            Self::Ping(_) => 1,
            Self::Ack(_) => 2,
            Self::Ready => 3,
            Self::Fleet { .. } => 4,
            Self::TeamName { .. } => 5,
            Self::Handshake0 => 6,
            Self::Handshake1 => 7,
            Self::HandshakeCancel => 8,
            Self::HandshakeCancelAck => 9,
            Self::SeedRandom(_) => 10,
            Self::InputDelay(_) => 11,
            Self::SelectShip(_) => 12,
            Self::BattleInput(_) => 13,
            Self::FrameCount(_) => 14,
            Self::Checksum { .. } => 15,
            Self::Abort(_) => 16,
            Self::Reset(_) => 17,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let extra = match self {
            Self::Fleet { ships, .. } => (ships.len() * 2 + 3) & !3,
            Self::TeamName { name, .. } => (name.len() + 4) & !3,
            _ => 0,
        };
        let len = MIN_LENGTH[self.kind()] + extra;
        let wire_len = match u16::try_from(len) {
            Ok(len) => len,
            Err(_) => panic!("packet exceeds the wire length field"),
        };
        let mut bytes = vec![0; len];
        bytes[..2].copy_from_slice(&wire_len.to_be_bytes());
        bytes[2..4].copy_from_slice(&(self.kind() as u16).to_be_bytes());
        match self {
            Self::Init { protocol, version } => {
                bytes[4..6].copy_from_slice(protocol);
                bytes[8..11].copy_from_slice(version);
            }
            Self::Ping(value)
            | Self::Ack(value)
            | Self::SeedRandom(value)
            | Self::InputDelay(value)
            | Self::FrameCount(value) => {
                bytes[4..8].copy_from_slice(&value.to_be_bytes());
            }
            Self::Fleet { side, ships } => {
                bytes[4] = *side;
                bytes[6..8].copy_from_slice(&(ships.len() as u16).to_be_bytes());
                for (dest, &(index, ship)) in
                    bytes[8..].as_chunks_mut::<2>().0.iter_mut().zip(ships)
                {
                    dest.copy_from_slice(&[index, ship]);
                }
            }
            Self::TeamName { side, name } => {
                bytes[4] = *side;
                bytes[6..6 + name.len()].copy_from_slice(name);
            }
            Self::SelectShip(value) | Self::Abort(value) | Self::Reset(value) => {
                bytes[4..6].copy_from_slice(&value.to_be_bytes());
            }
            Self::BattleInput(value) => bytes[4] = *value,
            Self::Checksum { frame, checksum } => {
                bytes[4..8].copy_from_slice(&frame.to_be_bytes());
                bytes[8..12].copy_from_slice(&checksum.to_be_bytes());
            }
            Self::Ready
            | Self::Handshake0
            | Self::Handshake1
            | Self::HandshakeCancel
            | Self::HandshakeCancelAck => {}
        }
        bytes
    }

    fn decode_fleet(bytes: &[u8]) -> Result<Self, DecodeError> {
        let count = usize::from(u16::from_be_bytes([bytes[6], bytes[7]]));
        let end = 8 + count * 2;
        if end > bytes.len() {
            return Err(DecodeError::InvalidLength);
        }
        Ok(Self::Fleet {
            side: bytes[4],
            ships: bytes[8..end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| (pair[0], pair[1]))
                .collect(),
        })
    }

    fn decode_team_name(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() == 6 {
            return Err(DecodeError::InvalidLength);
        }
        let end = bytes[6..bytes.len() - 1]
            .iter()
            .position(|&b| b == 0)
            .map_or(bytes.len() - 1, |n| 6 + n);
        Ok(Self::TeamName {
            side: bytes[4],
            name: bytes[6..end].to_vec(),
        })
    }

    /// Decode one frame; return its consumed length so stream callers retain the suffix.
    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), DecodeError> {
        if bytes.len() < 4 {
            return Err(DecodeError::Incomplete);
        }
        let len = usize::from(u16::from_be_bytes([bytes[0], bytes[1]]));
        let kind = usize::from(u16::from_be_bytes([bytes[2], bytes[3]]));
        let minimum = *MIN_LENGTH.get(kind).ok_or(DecodeError::UnknownType)?;
        if len < minimum {
            return Err(DecodeError::InvalidLength);
        }
        if bytes.len() < len {
            return Err(DecodeError::Incomplete);
        }
        Ok((Self::decode_payload(kind, &bytes[..len])?, len))
    }

    fn decode_payload(kind: usize, bytes: &[u8]) -> Result<Self, DecodeError> {
        let short = || u16::from_be_bytes([bytes[4], bytes[5]]);
        let word = || u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let packet = match kind {
            0 => Self::Init {
                protocol: [bytes[4], bytes[5]],
                version: [bytes[8], bytes[9], bytes[10]],
            },
            1 => Self::Ping(word()),
            2 => Self::Ack(word()),
            3 => Self::Ready,
            4 => Self::decode_fleet(bytes)?,
            5 => Self::decode_team_name(bytes)?,
            6 => Self::Handshake0,
            7 => Self::Handshake1,
            8 => Self::HandshakeCancel,
            9 => Self::HandshakeCancelAck,
            10 => Self::SeedRandom(word()),
            11 => Self::InputDelay(word()),
            12 => Self::SelectShip(short()),
            13 => Self::BattleInput(bytes[4]),
            14 => Self::FrameCount(word()),
            15 => Self::Checksum {
                frame: word(),
                checksum: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            },
            16 => Self::Abort(short()),
            17 => Self::Reset(short()),
            _ => unreachable!(),
        };
        Ok(packet)
    }
}

/// # Safety
/// Consumes the uniquely owned packet allocated by queue_packet.
pub(crate) unsafe fn packet_delete(packet: *mut Packet) {
    drop(Box::from_raw(packet));
}
