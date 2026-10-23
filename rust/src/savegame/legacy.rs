//! Pre-UMS save layout, including its 32-bit pointer padding and packed groups.
use super::format::*;
use super::{decode_save, decode_summary_from, SaveFile, Summary, GAME_STATE_BYTES};
use crate::state::game_state::{copy_state_bits_raw, get_state_32_raw, set_state_32_raw};
use std::io::{self, Read};

struct Reader<R> {
    input: R,
    left: u32,
}
impl<R: Read> Reader<R> {
    fn bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
        if n as u64 > u64::from(self.left) {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        let mut bytes = vec![0; n];
        self.input.read_exact(&mut bytes)?;
        self.left -= n as u32;
        Ok(bytes)
    }
    fn skip(&mut self, n: usize) -> io::Result<()> {
        self.bytes(n).map(|_| ())
    }
    fn byte(&mut self) -> io::Result<u8> {
        Ok(self.bytes(1)?[0])
    }
    fn count(&mut self) -> io::Result<u16> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }
    fn word(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn array(&mut self) -> io::Result<Vec<u8>> {
        let n = self.word()?;
        self.bytes(n as usize)
    }
}
fn chunk(wire: &mut Vec<u8>, tag: u32, body: &[u8]) {
    wire.extend(tag.to_le_bytes());
    wire.extend((body.len() as u32).to_le_bytes());
    wire.extend(body);
}

pub fn decode_legacy_summary(bytes: &[u8; 164]) -> Result<Summary, super::SaveFormatError> {
    let mut wire = SAVEFILE_TAG.to_le_bytes().to_vec();
    let mut body = bytes[..122].to_vec();
    body.extend(&bytes[124..162]);
    chunk(&mut wire, SUMMARY_TAG, &body);
    decode_summary_from(wire.as_slice(), wire.len() as u64)
}

pub fn transpose_state(old: &[u8]) -> [u8; GAME_STATE_BYTES] {
    let mut result = [0; GAME_STATE_BYTES];
    for (start, end, target) in [
        (0, 51, 0),
        (404, 450, 52),
        (483, 878, 99),
        (911, 930, 495),
        (963, 1237, 515),
    ] {
        copy_state_bits_raw(&mut result, target, old, start, end);
    }
    for (i, offset) in [
        52, 84, 116, 148, 180, 212, 244, 276, 308, 340, 372, 451, 879, 931,
    ]
    .into_iter()
    .enumerate()
    {
        set_state_32_raw(&mut result, 790 + 32 * i, get_state_32_raw(old, offset));
    }
    result
}

pub struct LegacySave {
    pub save: SaveFile,
    pub battle_group_ref: u32,
    /// STARINFO, DEFGRPINFO, RANDGRPINFO, in the original stream order.
    pub state_files: [Vec<u8>; 3],
}

fn ship<R: Read>(r: &mut Reader<R>, out: &mut Vec<u8>) -> io::Result<()> {
    r.skip(2)?;
    out.push(r.byte()?);
    r.skip(3)?;
    out.extend(r.bytes(2)?);
    out.extend(u16::from(r.byte()?).to_le_bytes());
    out.extend(u16::from(r.byte()?).to_le_bytes());
    out.extend(r.bytes(2)?);
    r.skip(4)
}
fn ships<R: Read>(r: &mut Reader<R>, wire: &mut Vec<u8>, tag: u32) -> io::Result<()> {
    let count = r.count()?;
    let mut out = Vec::new();
    for _ in 0..count {
        out.extend(r.bytes(2)?);
        ship(r, &mut out)?;
    }
    chunk(wire, tag, &out);
    Ok(())
}

pub fn decode_legacy_payload<R: Read>(
    summary: &Summary,
    input: R,
    len: u32,
) -> anyhow::Result<LegacySave> {
    let mut r = Reader { input, left: len };
    let mut wire = SAVEFILE_TAG.to_le_bytes().to_vec();
    // The existing encoder supplies the summary representation without another
    // copy of its field order in the legacy reader.
    let summary_wire = super::encode::encode_summary(summary);
    wire.extend(summary_wire);
    r.skip(1)?;
    let mut global = r.bytes(31)?;
    r.skip(4)?;
    let activity = r.count()?;
    global.extend(activity.to_le_bytes());
    r.skip(2)?;
    global.extend(r.bytes(8)?);
    r.skip(12 + 24)?;
    global.extend(r.bytes(34)?);
    r.skip(2)?;
    let battle_group_ref = r.word()?;
    r.skip(4 * 24)?;
    let state = transpose_state(&r.bytes(155)?);
    r.skip(1)?;
    chunk(&mut wire, GLOBAL_STATE_TAG, &global);
    chunk(&mut wire, GAME_STATE_TAG, &state);

    let count = r.count()?;
    let mut races = Vec::new();
    for _ in 0..count {
        races.extend(r.bytes(6)?);
        races.extend(u16::from(r.byte()?).to_le_bytes());
        races.extend(u16::from(r.byte()?).to_le_bytes());
        races.extend(r.bytes(20)?);
        r.skip(2)?;
    }
    chunk(&mut wire, RACE_Q_TAG, &races);
    if activity & 0x800 == 0 {
        if activity & 0x400 != 0 {
            ships(&mut r, &mut wire, NPC_SHIP_Q_TAG)?;
        } else if activity & 0xff == 4 {
            let count = r.count()?;
            let mut groups = Vec::new();
            for _ in 0..count {
                r.skip(6)?;
                groups.extend(r.bytes(3)?);
                let packed = r.byte()?;
                groups.extend([packed & 15, packed >> 4]);
                groups.push(r.byte()?);
                r.skip(1)?;
                let packed = r.byte()?;
                groups.extend([packed & 15, packed >> 4]);
                groups.push(r.byte()?);
                groups.extend(r.bytes(4)?);
            }
            chunk(&mut wire, IP_GRP_Q_TAG, &groups);
        } else if r.count()? != 0 {
            anyhow::bail!("legacy save's unused queue is not empty");
        }
    }
    ships(&mut r, &mut wire, SHIP_Q_TAG)?;
    let count = r.count()?;
    let mut events = Vec::new();
    for _ in 0..count {
        r.skip(8)?;
        events.extend(r.bytes(5)?);
        r.skip(3)?;
    }
    chunk(&mut wire, EVENTS_TAG, &events);
    let count = r.count()?;
    let mut encounters = Vec::new();
    for _ in 0..count {
        r.skip(12)?;
        encounters.extend(r.bytes(13)?);
        let packed = r.byte()?;
        encounters.extend([packed & 15, packed & 0xf0]);
        r.skip(2)?;
        for _ in 0..7 {
            r.skip(2)?;
            encounters.push(r.byte()?);
            r.skip(1)?;
            encounters.extend(u16::from(r.byte()?).to_le_bytes());
            encounters.extend(u16::from(r.byte()?).to_le_bytes());
            r.skip(1)?;
            encounters.push(r.byte()?);
            r.skip(16)?;
        }
        encounters.extend(r.bytes(8)?);
    }
    chunk(&mut wire, ENCOUNTERS_TAG, &encounters);
    let state_files = [r.array()?, r.array()?, r.array()?];
    chunk(&mut wire, STAR_TAG, &r.bytes(8)?);
    Ok(LegacySave {
        save: decode_save(&wire)?,
        battle_group_ref,
        state_files,
    })
}

#[cfg(not(test))]
mod live {
    use super::*;
    use crate::mainloop::dir_prep::saveDir;
    use crate::planets::c_api::GLOB_DATA;
    use crate::savegame::abi::SummaryDesc;
    use crate::savegame::lzh::{cclose, cfilelength, copen, cread, CodeDesc};
    use crate::savegame::slots::SaveSlots;

    struct Compressed(*mut CodeDesc);
    impl Read for Compressed {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            let count = bytes.len().min(u16::MAX as usize) as u16;
            Ok(unsafe { cread(bytes.as_mut_ptr().cast(), 1, count, self.0) } as usize)
        }
    }
    impl Drop for Compressed {
        fn drop(&mut self) {
            unsafe {
                cclose(self.0);
            }
        }
    }

    /// # Safety
    /// Game thread; summary is null for loading or a writable SUMMARY_DESC.
    #[no_mangle]
    pub unsafe extern "C" fn LoadLegacyGame(slot: u16, summary: *mut SummaryDesc) -> u32 {
        let result = (|| -> anyhow::Result<()> {
            let slots = SaveSlots::new(saveDir)?;
            let mut stream = slots
                .open(&format!("starcon2.{slot:02}"), c"rb")
                .ok_or_else(|| anyhow::anyhow!("no legacy save in slot {slot}"))?;
            if stream.is_directory() {
                anyhow::bail!("legacy slot is a directory");
            }
            let mut header = [0; 164];
            (&mut stream).read_exact(&mut header)?;
            let saved_summary = decode_legacy_summary(&header)?;
            if !summary.is_null() {
                summary.write(SummaryDesc::from(&saved_summary));
                return Ok(());
            }
            if !(2155..2186).contains(&saved_summary.year_index) {
                anyhow::bail!("legacy save is corrupt or from an incompatible platform");
            }
            let decoder = Compressed(copen(stream.fp, 0, 0));
            if decoder.0.is_null() {
                anyhow::bail!("invalid legacy compressed stream");
            }
            let mut len = 0;
            cfilelength(decoder.0, &mut len);
            let legacy = decode_legacy_payload(&saved_summary, decoder, len)?;
            let game = &mut (*std::ptr::addr_of_mut!(GLOB_DATA)).game_state;
            let save = legacy.save;
            super::super::runtime::restore_runtime(
                game,
                &save.global,
                &save.game_state,
                &save.events,
            )?;
            super::super::runtime::queues::restore_queues(
                game,
                &super::super::runtime::queues::QueueSections {
                    race_queue: save.race_queue,
                    npc_ship_queue: save.npc_ship_queue,
                    ship_queue: save.ship_queue,
                },
            )?;
            super::super::runtime::groups::restore_loaded_groups(
                game,
                &super::super::runtime::groups::GroupSections {
                    ip_group_queue: save.ip_group_queue,
                    encounters: save.encounters,
                },
            )?;
            crate::state::ffi::with_state_files(|files| {
                for (index, image) in [0, 2, 1].into_iter().zip(&legacy.state_files) {
                    files
                        .replace_contents(index, image)
                        .expect("three state files exist");
                }
            })?;
            game.battle_group_ref = legacy.battle_group_ref;
            GLOB_DATA.sis_state = (&save.summary.sis).into();
            super::super::live::finish_load(&save.star, save.global.current_activity);
            Ok(())
        })();
        match result {
            Ok(()) => 1,
            Err(error) => {
                log::error!("LoadLegacyGame: {error}");
                0
            }
        }
    }
}
#[cfg(not(test))]
pub use live::LoadLegacyGame;
