//! Reads a UMS version 1 file, following `sc2/src/uqm/load.c`.
//!
//! The input is external data. Every size the file declares is checked
//! against the bytes that actually remain before anything is allocated or
//! skipped, and queue and group counts are checked against what the writer
//! can express, so a hostile size cannot cause a large allocation.
//!
//! Where `load.c` is lenient the same leniency is kept: unknown tags are
//! skipped by their declared size, a "GmSt" chunk of any size is accepted,
//! and a save name longer than 63 bytes is cut. Where `load.c` would read
//! past a chunk or ignore leftover bytes (a queue chunk that is not a whole
//! number of entries, a short fixed-size chunk) it carries on with a
//! desynchronised stream and no error; this decoder returns
//! [`SaveFormatError::ChunkSize`] instead, because the stream cannot be
//! trusted after that.

use std::io::{self, Read};

use super::error::{SaveFormatError, Section};
use super::format::*;
use super::types::*;

type Result<T> = std::result::Result<T, SaveFormatError>;

/// The largest "BtGp" body the format can express: the header plus 255
/// fleets of 255 ships each.
const MAX_BATTLE_GROUP_BYTES: u64 = BATTLE_GROUP_HEADER_BYTES as u64
    + MAX_BYTE_COUNT
        * (BATTLE_FLEET_HEADER_BYTES as u64 + MAX_BYTE_COUNT * BATTLE_SHIP_BYTES as u64);

/// Decode a whole save held in memory.
pub fn decode_save(bytes: &[u8]) -> Result<SaveFile> {
    decode_save_from(bytes, bytes.len() as u64)
}

/// Decode a whole save from `reader`, which holds `len` bytes.
pub fn decode_save_from<R: Read>(reader: R, len: u64) -> Result<SaveFile> {
    let mut src = Source {
        reader,
        remaining: len,
    };
    let summary = read_summary(&mut src)?;
    let global = read_global_state(&mut src)?;
    let game_state = read_game_state(&mut src)?;
    read_chunks(&mut src, summary, global, game_state)
}

/// Decode only the leading "Summ" chunk, as the slot list does.
pub fn decode_summary_from<R: Read>(reader: R, len: u64) -> Result<Summary> {
    read_summary(&mut Source {
        reader,
        remaining: len,
    })
}

struct Source<R> {
    reader: R,
    remaining: u64,
}

impl<R: Read> Source<R> {
    fn fill(&mut self, buf: &mut [u8], at: Section) -> Result<()> {
        if buf.len() as u64 > self.remaining {
            return Err(SaveFormatError::Truncated { at });
        }
        self.reader.read_exact(buf).map_err(|e| match e.kind() {
            io::ErrorKind::UnexpectedEof => SaveFormatError::Truncated { at },
            kind => SaveFormatError::Io { kind },
        })?;
        self.remaining -= buf.len() as u64;
        Ok(())
    }

    fn u32(&mut self, at: Section) -> Result<u32> {
        let mut b = [0u8; 4];
        self.fill(&mut b, at)?;
        Ok(u32::from_le_bytes(b))
    }

    fn bytes(&mut self, len: usize, at: Section) -> Result<Vec<u8>> {
        // Checked before allocating so that a declared size beyond the
        // input costs nothing.
        if len as u64 > self.remaining {
            return Err(SaveFormatError::Truncated { at });
        }
        let mut v = vec![0u8; len];
        self.fill(&mut v, at)?;
        Ok(v)
    }

    fn skip(&mut self, mut len: u64, at: Section) -> Result<()> {
        if len > self.remaining {
            return Err(SaveFormatError::Truncated { at });
        }
        let mut scratch = [0u8; 4096];
        while len > 0 {
            let n = len.min(scratch.len() as u64) as usize;
            self.fill(&mut scratch[..n], at)?;
            len -= n as u64;
        }
        Ok(())
    }

    /// Tag and size of the next chunk, or `None` at a clean end of input.
    fn chunk_header(&mut self) -> Result<Option<(u32, u32)>> {
        if self.remaining == 0 {
            return Ok(None);
        }
        let tag = self.u32(Section::ChunkHeader)?;
        let size = self.u32(Section::ChunkHeader)?;
        Ok(Some((tag, size)))
    }

    /// The header of a chunk that must have `expected` as its tag.
    fn expect_header(&mut self, expected: u32) -> Result<u32> {
        let tag = self.u32(Section::ChunkHeader)?;
        if tag != expected {
            return Err(SaveFormatError::UnexpectedTag {
                expected,
                found: tag,
            });
        }
        self.u32(Section::ChunkHeader)
    }
}

/// A chunk body held in memory, read field by field. A read past the end
/// means the declared size and the layout disagree.
struct Body<'a> {
    tag: u32,
    size: u32,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Body<'a> {
    fn new(tag: u32, size: u32, bytes: &'a [u8]) -> Self {
        Body {
            tag,
            size,
            bytes,
            pos: 0,
        }
    }

    fn mismatch(&self) -> SaveFormatError {
        SaveFormatError::ChunkSize {
            tag: self.tag,
            size: self.size,
        }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or_else(|| self.mismatch())?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| self.mismatch())?;
        self.pos = end;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut a = [0u8; N];
        a.copy_from_slice(self.take(N)?);
        Ok(a)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn i16(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    fn point(&mut self) -> Result<Point> {
        Ok(Point {
            x: self.i16()?,
            y: self.i16()?,
        })
    }

    fn extent(&mut self) -> Result<Extent> {
        Ok(Extent {
            width: self.i16()?,
            height: self.i16()?,
        })
    }

    fn finish(self) -> Result<()> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(self.mismatch())
        }
    }
}

fn read_summary<R: Read>(src: &mut Source<R>) -> Result<Summary> {
    let magic = src.u32(Section::FileMagic)?;
    if magic != SAVEFILE_TAG {
        return Err(SaveFormatError::BadMagic { found: magic });
    }
    let size = src.expect_header(SUMMARY_TAG)?;
    if size < SUMMARY_FIXED_BYTES {
        return Err(SaveFormatError::ChunkSize {
            tag: SUMMARY_TAG,
            size,
        });
    }
    let at = Section::Chunk(SUMMARY_TAG);
    let fixed = src.bytes(SUMMARY_FIXED_BYTES as usize, at)?;
    let name_len = u64::from(size - SUMMARY_FIXED_BYTES);
    let kept = name_len.min(SaveName::MAX_BYTES as u64);
    let mut name = src.bytes(kept as usize, at)?;
    src.skip(name_len - kept, at)?;
    // The game keeps the name as a C string, which ends at the first NUL.
    if let Some(nul) = name.iter().position(|&b| b == 0) {
        name.truncate(nul);
    }

    let mut body = Body::new(SUMMARY_TAG, size, &fixed);
    let sis = SisState {
        log_x: body.i32()?,
        log_y: body.i32()?,
        res_units: body.u32()?,
        fuel_on_board: body.u32()?,
        crew_enlisted: body.u16()?,
        total_element_mass: body.u16()?,
        total_bio_mass: body.u16()?,
        module_slots: body.array()?,
        drive_slots: body.array()?,
        jet_slots: body.array()?,
        num_landers: body.u8()?,
        element_amounts: {
            let mut amounts = [0u16; NUM_ELEMENT_CATEGORIES];
            for amount in &mut amounts {
                *amount = body.u16()?;
            }
            amounts
        },
        ship_name: body.array()?,
        commander_name: body.array()?,
        planet_name: body.array()?,
    };
    let summary = Summary {
        sis,
        activity: body.u8()?,
        flags: body.u8()?,
        day_index: body.u8()?,
        month_index: body.u8()?,
        year_index: body.u16()?,
        mcredit_lo: body.u8()?,
        mcredit_hi: body.u8()?,
        num_ships: body.u8()?,
        num_devices: body.u8()?,
        ship_list: body.array()?,
        device_list: body.array()?,
        save_name: SaveName::new(name).expect("name cut to 63 bytes before the first NUL"),
    };
    body.finish()?;
    Ok(summary)
}

fn read_global_state<R: Read>(src: &mut Source<R>) -> Result<GlobalState> {
    let size = src.expect_header(GLOBAL_STATE_TAG)?;
    if size != GLOBAL_STATE_BYTES {
        return Err(SaveFormatError::ChunkSize {
            tag: GLOBAL_STATE_TAG,
            size,
        });
    }
    let bytes = src.bytes(size as usize, Section::Chunk(GLOBAL_STATE_TAG))?;
    let mut b = Body::new(GLOBAL_STATE_TAG, size, &bytes);
    let global = GlobalState {
        glob_flags: b.u8()?,
        crew_cost: b.u8()?,
        fuel_cost: b.u8()?,
        module_cost: b.array()?,
        element_worth: b.array()?,
        current_activity: b.u16()?,
        clock: ClockState {
            day_index: b.u8()?,
            month_index: b.u8()?,
            year_index: b.u16()?,
            tick_count: b.i16()?,
            day_in_ticks: b.i16()?,
        },
        autopilot: b.point()?,
        ip_location: b.point()?,
        ship_stamp_origin: b.point()?,
        ship_facing: b.u16()?,
        ip_planet: b.u8()?,
        in_orbit: b.u8()?,
        velocity: Velocity {
            travel_angle: b.u16()?,
            vector: b.extent()?,
            fract: b.extent()?,
            error: b.extent()?,
            incr: b.extent()?,
        },
    };
    b.finish()?;
    Ok(global)
}

/// "GmSt" may be longer than this build's game state (modded saves) or
/// shorter (older saves); `LoadGameState` zero-fills and cuts to fit.
fn read_game_state<R: Read>(src: &mut Source<R>) -> Result<[u8; GAME_STATE_BYTES]> {
    let size = src.expect_header(GAME_STATE_TAG)?;
    let at = Section::Chunk(GAME_STATE_TAG);
    let kept = (size as usize).min(GAME_STATE_BYTES);
    let mut state = [0u8; GAME_STATE_BYTES];
    if u64::from(size) > src.remaining {
        return Err(SaveFormatError::Truncated { at });
    }
    src.fill(&mut state[..kept], at)?;
    src.skip(u64::from(size) - kept as u64, at)?;
    Ok(state)
}

fn read_chunks<R: Read>(
    src: &mut Source<R>,
    summary: Summary,
    global: GlobalState,
    game_state: [u8; GAME_STATE_BYTES],
) -> Result<SaveFile> {
    let mut save = SaveFile {
        summary,
        global,
        game_state,
        race_queue: Vec::new(),
        ip_group_queue: Vec::new(),
        npc_ship_queue: Vec::new(),
        ship_queue: Vec::new(),
        events: Vec::new(),
        encounters: Vec::new(),
        scan: None,
        group_list: None,
        battle_groups: Vec::new(),
        star: StarDesc {
            star_pt: Point::default(),
            star_type: 0,
            index: 0,
            prefix: 0,
            postfix: 0,
        },
    };
    let mut star = None;

    while let Some((tag, size)) = src.chunk_header()? {
        match tag {
            RACE_Q_TAG => {
                save.race_queue
                    .extend(read_entries(src, tag, size, RACE_ENTRY_BYTES, race_entry)?)
            }
            IP_GRP_Q_TAG => save.ip_group_queue.extend(read_entries(
                src,
                tag,
                size,
                IP_GROUP_ENTRY_BYTES,
                ip_group,
            )?),
            NPC_SHIP_Q_TAG => save.npc_ship_queue.extend(read_entries(
                src,
                tag,
                size,
                SHIP_FRAGMENT_ENTRY_BYTES,
                ship_entry,
            )?),
            SHIP_Q_TAG => save.ship_queue.extend(read_entries(
                src,
                tag,
                size,
                SHIP_FRAGMENT_ENTRY_BYTES,
                ship_entry,
            )?),
            EVENTS_TAG => save.events.extend(read_entries(
                src,
                tag,
                size,
                EVENT_ENTRY_BYTES,
                event_entry,
            )?),
            ENCOUNTERS_TAG => save.encounters.extend(read_entries(
                src,
                tag,
                size,
                ENCOUNTER_ENTRY_BYTES,
                encounter_entry,
            )?),
            STAR_TAG => star = Some(read_star(src, size)?),
            SCAN_TAG => save.scan = Some(read_scan(src, size)?),
            GROUP_LIST_TAG => save.group_list = Some(read_group_list(src, size)?),
            BATTLE_GROUP_TAG => save.battle_groups.push(read_battle_group(src, size)?),
            // Includes Summ, GlSt and GmSt repeated after their place, which
            // the loader's switch also does not know.
            _ => src.skip(u64::from(size), Section::Chunk(tag))?,
        }
    }

    save.star = star.ok_or(SaveFormatError::MissingChunk { tag: STAR_TAG })?;
    Ok(save)
}

fn read_entries<R: Read, T>(
    src: &mut Source<R>,
    tag: u32,
    size: u32,
    entry_bytes: u32,
    parse: fn(&mut Body<'_>) -> Result<T>,
) -> Result<Vec<T>> {
    if !size.is_multiple_of(entry_bytes) {
        return Err(SaveFormatError::ChunkSize { tag, size });
    }
    let count = u64::from(size / entry_bytes);
    if count > MAX_QUEUE_ENTRIES {
        return Err(SaveFormatError::CountOverflow { tag, count });
    }
    let bytes = src.bytes(size as usize, Section::Chunk(tag))?;
    let mut body = Body::new(tag, size, &bytes);
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        entries.push(parse(&mut body)?);
    }
    body.finish()?;
    Ok(entries)
}

fn race_entry(b: &mut Body<'_>) -> Result<RaceEntry> {
    Ok(RaceEntry {
        index: b.u16()?,
        allied_state: b.u16()?,
        days_left: b.u8()?,
        growth_fract: b.u8()?,
        crew_level: b.u16()?,
        max_crew: b.u16()?,
        growth: b.u8()?,
        max_energy: b.u8()?,
        loc: b.point()?,
        actual_strength: b.u16()?,
        known_strength: b.u16()?,
        known_loc: b.point()?,
        growth_err_term: b.u8()?,
        func_index: b.u8()?,
        dest_loc: b.point()?,
    })
}

fn ip_group(b: &mut Body<'_>) -> Result<IpGroup> {
    Ok(IpGroup {
        group_counter: b.u16()?,
        race_id: b.u8()?,
        sys_loc: b.u8()?,
        task: b.u8()?,
        in_system: b.u8()?,
        dest_loc: b.u8()?,
        orbit_pos: b.u8()?,
        group_id: b.u8()?,
        loc: b.point()?,
    })
}

fn ship_entry(b: &mut Body<'_>) -> Result<ShipFragmentEntry> {
    Ok(ShipFragmentEntry {
        race_index: b.u16()?,
        captains_name_index: b.u8()?,
        race_id: b.u8()?,
        index: b.u8()?,
        crew_level: b.u16()?,
        max_crew: b.u16()?,
        energy_level: b.u8()?,
        max_energy: b.u8()?,
    })
}

fn event_entry(b: &mut Body<'_>) -> Result<EventEntry> {
    Ok(EventEntry {
        day_index: b.u8()?,
        month_index: b.u8()?,
        year_index: b.u16()?,
        func_index: b.u8()?,
    })
}

fn encounter_entry(b: &mut Body<'_>) -> Result<EncounterEntry> {
    let transition_state = b.i16()?;
    let origin = b.point()?;
    let radius = b.u16()?;
    let loc_pt = b.point()?;
    let race_id = b.u8()?;
    let num_ships = b.u8()?;
    let flags = b.u8()?;
    let mut ship_list = [BriefShipInfo::default(); MAX_HYPER_SHIPS];
    for ship in &mut ship_list {
        *ship = BriefShipInfo {
            race_id: b.u8()?,
            crew_level: b.u16()?,
            max_crew: b.u16()?,
            max_energy: b.u8()?,
        };
    }
    Ok(EncounterEntry {
        transition_state,
        origin,
        radius,
        loc_pt,
        race_id,
        num_ships,
        flags,
        ship_list,
        log_x: b.i32()?,
        log_y: b.i32()?,
    })
}

fn read_star<R: Read>(src: &mut Source<R>, size: u32) -> Result<StarDesc> {
    if size != STAR_BYTES {
        return Err(SaveFormatError::ChunkSize {
            tag: STAR_TAG,
            size,
        });
    }
    let bytes = src.bytes(size as usize, Section::Chunk(STAR_TAG))?;
    let mut b = Body::new(STAR_TAG, size, &bytes);
    let star = StarDesc {
        star_pt: b.point()?,
        star_type: b.u8()?,
        index: b.u8()?,
        prefix: b.u8()?,
        postfix: b.u8()?,
    };
    b.finish()?;
    Ok(star)
}

fn read_scan<R: Read>(src: &mut Source<R>, size: u32) -> Result<Vec<u32>> {
    if !size.is_multiple_of(4) {
        return Err(SaveFormatError::ChunkSize {
            tag: SCAN_TAG,
            size,
        });
    }
    let at = Section::Chunk(SCAN_TAG);
    if u64::from(size) > src.remaining {
        return Err(SaveFormatError::Truncated { at });
    }
    let mut words = Vec::with_capacity((size / 4) as usize);
    for _ in 0..size / 4 {
        words.push(src.u32(at)?);
    }
    Ok(words)
}

fn read_group_list<R: Read>(src: &mut Source<R>, size: u32) -> Result<GroupList> {
    if size < 1 || !(size - 1).is_multiple_of(GROUP_LIST_ENTRY_BYTES) {
        return Err(SaveFormatError::ChunkSize {
            tag: GROUP_LIST_TAG,
            size,
        });
    }
    let count = u64::from((size - 1) / GROUP_LIST_ENTRY_BYTES);
    if count > MAX_BYTE_COUNT {
        return Err(SaveFormatError::CountOverflow {
            tag: GROUP_LIST_TAG,
            count,
        });
    }
    let bytes = src.bytes(size as usize, Section::Chunk(GROUP_LIST_TAG))?;
    let mut b = Body::new(GROUP_LIST_TAG, size, &bytes);
    let last_encounter = b.u8()?;
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        entries.push(GroupListEntry {
            race_outer: b.u8()?,
            group: ip_group(&mut b)?,
        });
    }
    b.finish()?;
    Ok(GroupList {
        last_encounter,
        entries,
    })
}

fn read_battle_group<R: Read>(src: &mut Source<R>, size: u32) -> Result<BattleGroup> {
    if size < BATTLE_GROUP_HEADER_BYTES || u64::from(size) > MAX_BATTLE_GROUP_BYTES {
        return Err(SaveFormatError::ChunkSize {
            tag: BATTLE_GROUP_TAG,
            size,
        });
    }
    let bytes = src.bytes(size as usize, Section::Chunk(BATTLE_GROUP_TAG))?;
    let mut b = Body::new(BATTLE_GROUP_TAG, size, &bytes);
    let encounter_id = b.u32()?;
    let current = b.u8()? != 0;
    let star_index = b.u16()?;
    let day_index = b.u8()?;
    let month_index = b.u8()?;
    let year_index = b.u16()?;
    let fleet_count = b.u8()?;
    let mut fleets = Vec::with_capacity(usize::from(fleet_count));
    for _ in 0..fleet_count {
        let icon = b.u8()?;
        let ship_count = b.u8()?;
        let mut ships = Vec::with_capacity(usize::from(ship_count));
        for _ in 0..ship_count {
            ships.push(BattleShip {
                race_outer: b.u8()?,
                captains_name_index: b.u8()?,
                race_id: b.u8()?,
                index: b.u8()?,
                crew_level: b.u16()?,
                max_crew: b.u16()?,
                energy_level: b.u8()?,
                max_energy: b.u8()?,
            });
        }
        fleets.push(BattleFleet { icon, ships });
    }
    b.finish()?;
    Ok(BattleGroup {
        encounter_id,
        current,
        star_index,
        day_index,
        month_index,
        year_index,
        fleets,
    })
}
