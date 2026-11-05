//! Legacy Super Melee team records, without gameplay validation or file access.

use std::io::{self, Read, Write};

pub const FLEET_SIZE: usize = 14;
pub const NAME_BYTES: usize = 55;

/// The name includes the legacy terminator and padding bytes. Importing a record
/// into gameplay may normalize ship IDs and terminate the displayed name, but the
/// byte codec preserves them so existing team files can round-trip unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Team {
    pub ships: [u8; FLEET_SIZE],
    pub name: [u8; NAME_BYTES],
}

impl Team {
    pub const SERIALIZED_LEN: usize = FLEET_SIZE + NAME_BYTES;

    pub fn from_bytes(bytes: &[u8; Self::SERIALIZED_LEN]) -> Self {
        let mut team = Self {
            ships: [0; FLEET_SIZE],
            name: [0; NAME_BYTES],
        };
        team.ships.copy_from_slice(&bytes[..FLEET_SIZE]);
        team.name.copy_from_slice(&bytes[FLEET_SIZE..]);
        team
    }

    pub fn to_bytes(&self) -> [u8; Self::SERIALIZED_LEN] {
        let mut bytes = [0; Self::SERIALIZED_LEN];
        bytes[..FLEET_SIZE].copy_from_slice(&self.ships);
        bytes[FLEET_SIZE..].copy_from_slice(&self.name);
        bytes
    }

    pub fn read_from(mut reader: impl Read) -> io::Result<Self> {
        let mut bytes = [0; Self::SERIALIZED_LEN];
        reader.read_exact(&mut bytes)?;
        Ok(Self::from_bytes(&bytes))
    }

    pub fn write_to(&self, mut writer: impl Write) -> io::Result<()> {
        writer.write_all(&self.to_bytes())
    }
}
