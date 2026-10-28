//! In-memory ProTracker parsing and playback. No decoder scratch files are needed.

use std::collections::HashSet;

use super::decoder::{DecodeError, DecodeResult};

const PERIODS: [u16; 72] = [
    1712, 1616, 1524, 1440, 1356, 1280, 1208, 1140, 1076, 1016, 960, 907, 856, 808, 762, 720, 678,
    640, 604, 570, 538, 508, 480, 453, 428, 404, 381, 360, 339, 320, 302, 285, 269, 254, 240, 226,
    214, 202, 190, 180, 170, 160, 151, 143, 135, 127, 120, 113, 107, 101, 95, 90, 85, 80, 75, 71,
    67, 63, 60, 56, 53, 50, 47, 45, 42, 40, 37, 35, 33, 31, 30, 28,
];

fn note_period(period: u16) -> f64 {
    let closest = PERIODS
        .iter()
        .min_by_key(|value| (value.abs_diff(period), **value));
    f64::from(closest.copied().unwrap_or(period))
}

fn validate_effect(effect: u8, argument: u8) -> DecodeResult<()> {
    if effect == 14 {
        let command = argument >> 4;
        let value = argument & 15;
        if (command == 0 && value != 1)
            || (matches!(command, 3 | 15) && value != 0)
            || (matches!(command, 4 | 7) && value & 3 == 3)
        {
            return Err(DecodeError::UnsupportedFormat(format!(
                "MOD extended effect E{argument:02X} is not supported"
            )));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Note {
    sample: usize,
    period: u16,
    effect: u8,
    argument: u8,
}

struct Sample {
    data: Vec<i8>,
    volume: u8,
    fine_tune: i8,
    loop_start: usize,
    loop_length: usize,
}

pub(super) struct Song {
    samples: Vec<Sample>,
    patterns: Vec<Vec<Note>>,
    orders: Vec<usize>,
    pub(super) channels: usize,
}

fn invalid(message: &str) -> DecodeError {
    DecodeError::InvalidData(message.to_string())
}

fn slice(data: &[u8], offset: usize, length: usize) -> DecodeResult<&[u8]> {
    data.get(offset..offset + length)
        .ok_or_else(|| invalid("truncated MOD"))
}

fn words(data: &[u8], offset: usize) -> usize {
    usize::from(u16::from_be_bytes([data[offset], data[offset + 1]])) * 2
}

impl Song {
    pub(super) fn parse(data: &[u8]) -> DecodeResult<Self> {
        let (channels, sample_count, header_size) = match data.get(1080..1084) {
            Some(b"M.K." | b"M!K!" | b"FLT4" | b"4CHN") => (4, 31, 1084),
            Some(b"6CHN" | b"CD61") => (6, 31, 1084),
            Some(b"8CHN" | b"CD81") => (8, 31, 1084),
            Some(b"12CH") => (12, 31, 1084),
            _ => (4, 15, 600),
        };
        slice(data, 0, header_size)?;
        let order_offset = 20 + sample_count * 30;
        let order_count = usize::from(data[order_offset]);
        if !(1..=128).contains(&order_count) {
            return Err(invalid("MOD order count must be 1..=128"));
        }
        let orders: Vec<usize> = data[order_offset + 2..order_offset + 2 + order_count]
            .iter()
            .map(|order| usize::from(*order))
            .collect();
        let pattern_count = orders.iter().copied().max().unwrap_or(0) + 1;
        let pattern_bytes = pattern_count * channels * 64 * 4;
        let patterns = slice(data, header_size, pattern_bytes)?
            .chunks_exact(channels * 64 * 4)
            .map(|pattern| {
                pattern
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|bytes| {
                        let sample = usize::from((bytes[0] & 0xf0) | (bytes[2] >> 4));
                        if sample > sample_count {
                            return Err(invalid("MOD note references an absent sample"));
                        }
                        validate_effect(bytes[2] & 15, bytes[3])?;
                        Ok(Note {
                            sample,
                            period: (u16::from(bytes[0] & 15) << 8) | u16::from(bytes[1]),
                            effect: bytes[2] & 15,
                            argument: bytes[3],
                        })
                    })
                    .collect::<DecodeResult<Vec<_>>>()
            })
            .collect::<DecodeResult<Vec<_>>>()?;
        let mut offset = header_size + pattern_bytes;
        let mut samples = Vec::with_capacity(sample_count);
        for index in 0..sample_count {
            let header = &data[20 + index * 30..50 + index * 30];
            // A one-word sample is the MOD empty-instrument sentinel and has no payload.
            let declared_length = words(header, 22);
            let length = if declared_length > 2 {
                declared_length
            } else {
                0
            };
            let volume = header[25];
            if volume > 64 || header[24] > 15 {
                return Err(invalid("invalid MOD sample volume or fine tune"));
            }
            let loop_start = words(header, 26);
            let loop_length = words(header, 28);
            if loop_length > 2 && loop_start + loop_length > length {
                return Err(invalid("MOD sample loop exceeds sample"));
            }
            let fine_tune = if header[24] < 8 {
                header[24] as i8
            } else {
                header[24] as i8 - 16
            };
            samples.push(Sample {
                data: slice(data, offset, length)?
                    .iter()
                    .map(|value| *value as i8)
                    .collect(),
                volume,
                fine_tune,
                loop_start,
                loop_length,
            });
            offset += length;
        }
        Ok(Self {
            samples,
            patterns,
            orders,
            channels,
        })
    }

    pub(super) fn length_frames(&self, frequency: u32) -> u64 {
        let mut player = Player::new(self.channels, frequency);
        let mut frames = 0;
        while !player.ended {
            player.begin_tick(self);
            if player.ended {
                break;
            }
            frames += u64::from(player.remaining_samples);
            player.remaining_samples = 0;
            player.advance_tick(self);
        }
        frames
    }
}

#[derive(Default)]
struct Channel {
    sample: usize,
    position: f64,
    period: f64,
    target: f64,
    fine_tune: i8,
    volume: f64,
    pan: f64,
    effect: u8,
    argument: u8,
    porta_speed: u8,
    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_phase: u8,
    vibrato_wave: u8,
    tremolo_speed: u8,
    tremolo_depth: u8,
    tremolo_phase: u8,
    tremolo_wave: u8,
    sample_offset: u8,
    loop_row: usize,
    loop_count: u8,
    delayed_note: Option<Note>,
    last_voice: (f64, f64),
    end_tail: (f64, f64),
}

impl Channel {
    fn trigger(&mut self, note: Note, song: &Song, portamento: bool) {
        if note.sample != 0 {
            self.sample = note.sample;
            let sample = &song.samples[note.sample - 1];
            self.volume = f64::from(sample.volume);
            self.fine_tune = sample.fine_tune;
        }
        if note.period != 0 {
            if portamento {
                self.target = note_period(note.period);
            } else {
                self.period = note_period(note.period);
                self.position = 0.0;
                if self.vibrato_wave < 4 {
                    self.vibrato_phase = 0;
                }
                if self.tremolo_wave < 4 {
                    self.tremolo_phase = 0;
                }
            }
        }
    }

    fn tone_portamento(&mut self) {
        let distance = f64::from(self.porta_speed);
        if self.period < self.target {
            self.period = (self.period + distance).min(self.target);
        } else if self.target > 0.0 {
            self.period = (self.period - distance).max(self.target);
        }
    }

    fn volume_slide(&mut self) {
        let up = self.argument >> 4;
        let change = if up != 0 {
            f64::from(up)
        } else {
            -f64::from(self.argument & 15)
        };
        self.volume = (self.volume + change).clamp(0.0, 64.0);
    }

    fn tick(&mut self, tick: u8, song: &Song) {
        if tick == 0 {
            return;
        }
        match self.effect {
            1 => self.period = (self.period - f64::from(self.argument)).max(28.0),
            2 => self.period = (self.period + f64::from(self.argument)).min(3424.0),
            3 => self.tone_portamento(),
            4 => self.vibrato_phase = self.vibrato_phase.wrapping_add(self.vibrato_speed),
            5 => {
                self.tone_portamento();
                self.volume_slide();
            }
            6 => {
                self.vibrato_phase = self.vibrato_phase.wrapping_add(self.vibrato_speed);
                self.volume_slide();
            }
            7 => self.tremolo_phase = self.tremolo_phase.wrapping_add(self.tremolo_speed),
            10 => self.volume_slide(),
            14 => self.extended_tick(tick, song),
            _ => {}
        }
    }

    fn extended_tick(&mut self, tick: u8, song: &Song) {
        let argument = self.argument & 15;
        match self.argument >> 4 {
            9 if argument != 0 && tick.is_multiple_of(argument) => self.position = 0.0,
            12 if tick == argument => self.volume = 0.0,
            13 if tick == argument => {
                if let Some(note) = self.delayed_note.take() {
                    self.trigger(note, song, false);
                }
            }
            _ => {}
        }
    }

    fn rendered_period(&self, tick: u8) -> f64 {
        let mut period = self.period;
        if self.effect == 0 && self.argument != 0 {
            let semitones = match tick % 3 {
                1 => self.argument >> 4,
                2 => self.argument & 15,
                _ => 0,
            };
            let index = PERIODS.iter().position(|value| f64::from(*value) <= period);
            if let Some(index) = index {
                period =
                    f64::from(PERIODS[(index + usize::from(semitones)).min(PERIODS.len() - 1)]);
            }
        }
        if matches!(self.effect, 4 | 6) {
            period += waveform(self.vibrato_phase, self.vibrato_wave)
                * f64::from(self.vibrato_depth)
                * 2.0;
        }
        period / 2.0_f64.powf(f64::from(self.fine_tune) / 96.0)
    }

    fn mix(&mut self, song: &Song, tick: u8, frequency: u32) -> (f32, f32) {
        let voice = self.voice_value(song, tick, frequency);
        if let Some(value) = voice {
            self.last_voice = (value * (1.0 - self.pan), value * self.pan);
        } else {
            self.end_tail.0 += self.last_voice.0;
            self.end_tail.1 += self.last_voice.1;
            self.last_voice = (0.0, 0.0);
        }
        // Preserve the mixer offset when a non-looping voice ends, then decay
        // it independently of later notes so sample endings do not click.
        self.end_tail.0 *= 255.0 / 256.0;
        self.end_tail.1 *= 255.0 / 256.0;
        (
            (self.last_voice.0 + self.end_tail.0) as f32,
            (self.last_voice.1 + self.end_tail.1) as f32,
        )
    }

    fn voice_value(&mut self, song: &Song, tick: u8, frequency: u32) -> Option<f64> {
        if self.sample == 0 || self.period <= 0.0 {
            return None;
        }
        let sample = &song.samples[self.sample - 1];
        let limit = if sample.loop_length > 2 {
            sample.loop_start + sample.loop_length
        } else {
            sample.data.len()
        };
        if self.position >= limit as f64 {
            if sample.loop_length <= 2 {
                return None;
            }
            self.position = sample.loop_start as f64
                + (self.position - sample.loop_start as f64) % sample.loop_length as f64;
        }
        let volume = if self.effect == 7 {
            (self.volume
                + waveform(self.tremolo_phase, self.tremolo_wave)
                    * f64::from(self.tremolo_depth)
                    * 4.0)
                .clamp(0.0, 64.0)
        } else {
            self.volume
        };
        // Half gain leaves room for the two full-volume Amiga voices on each side.
        let mut sample_index = self.position.round() as usize;
        if sample_index >= limit {
            sample_index = if sample.loop_length > 2 {
                sample.loop_start
            } else {
                limit - 1
            };
        }
        let value = f64::from(sample.data[sample_index]) * volume / (256.0 * 64.0);
        self.position += 3_546_895.0 / self.rendered_period(tick).max(1.0) / f64::from(frequency);
        Some(value)
    }
}

fn waveform(phase: u8, wave: u8) -> f64 {
    let phase = phase & 63;
    match wave & 3 {
        1 => 1.0 - f64::from(phase) / 32.0,
        2 => {
            if phase < 32 {
                1.0
            } else {
                -1.0
            }
        }
        3 => f64::from((phase.wrapping_mul(37) & 63) as i8 - 32) / 32.0,
        _ => (f64::from(phase) * std::f64::consts::TAU / 64.0).sin(),
    }
}

#[derive(Hash, PartialEq, Eq)]
struct RowState {
    order: usize,
    row: usize,
    speed: u8,
    bpm: u8,
    loops: Vec<(usize, u8)>,
}

pub(super) struct Player {
    channels: Vec<Channel>,
    order: usize,
    row: usize,
    tick: u8,
    speed: u8,
    bpm: u8,
    remaining_samples: u32,
    frequency: u32,
    next_order: Option<usize>,
    next_row: Option<usize>,
    loop_row: Option<usize>,
    delay: u8,
    new_row: bool,
    visited: HashSet<RowState>,
    pub(super) ended: bool,
}

impl Player {
    pub(super) fn new(channels: usize, frequency: u32) -> Self {
        Self {
            channels: (0..channels)
                .map(|index| Channel {
                    pan: if matches!(index % 4, 0 | 3) { 0.0 } else { 1.0 },
                    ..Channel::default()
                })
                .collect(),
            order: 0,
            row: 0,
            tick: 0,
            speed: 6,
            bpm: 125,
            remaining_samples: 0,
            frequency,
            next_order: None,
            next_row: None,
            loop_row: None,
            delay: 0,
            new_row: true,
            visited: HashSet::new(),
            ended: false,
        }
    }

    fn row_effect(&mut self, index: usize, note: Note) {
        let channel = &mut self.channels[index];
        let hi = note.argument >> 4;
        let lo = note.argument & 15;
        match note.effect {
            3 if note.argument != 0 => channel.porta_speed = note.argument,
            4 => {
                if hi != 0 {
                    channel.vibrato_speed = hi;
                }
                if lo != 0 {
                    channel.vibrato_depth = lo;
                }
            }
            7 => {
                if hi != 0 {
                    channel.tremolo_speed = hi;
                }
                if lo != 0 {
                    channel.tremolo_depth = lo;
                }
            }
            8 => channel.pan = f64::from(note.argument) / 255.0,
            9 => {
                if note.argument != 0 {
                    channel.sample_offset = note.argument;
                }
                if note.period != 0 {
                    channel.position = f64::from(channel.sample_offset) * 256.0;
                }
            }
            11 => self.next_order = Some(usize::from(note.argument)),
            12 => channel.volume = f64::from(note.argument.min(64)),
            13 => self.next_row = Some((usize::from(hi) * 10 + usize::from(lo)).min(63)),
            14 => self.extended_row(index, hi, lo),
            15 => match note.argument {
                0 => self.ended = true,
                1..=31 => self.speed = note.argument,
                _ => self.bpm = note.argument,
            },
            _ => {}
        }
    }

    fn extended_row(&mut self, index: usize, effect: u8, argument: u8) {
        let channel = &mut self.channels[index];
        match effect {
            1 => channel.period = (channel.period - f64::from(argument)).max(113.0),
            2 => channel.period = (channel.period + f64::from(argument)).min(856.0),
            4 => channel.vibrato_wave = argument,
            5 => {
                channel.fine_tune = if argument < 8 {
                    argument as i8
                } else {
                    argument as i8 - 16
                }
            }
            6 => {
                if argument == 0 {
                    channel.loop_row = self.row;
                } else {
                    if channel.loop_count == 0 {
                        channel.loop_count = argument;
                    } else {
                        channel.loop_count -= 1;
                    }
                    if channel.loop_count != 0 {
                        self.loop_row = Some(channel.loop_row);
                    }
                }
            }
            7 => channel.tremolo_wave = argument,
            8 => channel.pan = f64::from(argument) / 15.0,
            10 => channel.volume = (channel.volume + f64::from(argument)).min(64.0),
            11 => channel.volume = (channel.volume - f64::from(argument)).max(0.0),
            12 if argument == 0 => channel.volume = 0.0,
            14 => self.delay = argument,
            _ => {}
        }
    }

    fn begin_row(&mut self, song: &Song) {
        let pattern = &song.patterns[song.orders[self.order]];
        for index in 0..song.channels {
            let note = pattern[self.row * song.channels + index];
            let channel = &mut self.channels[index];
            channel.effect = note.effect;
            channel.argument = note.argument;
            if note.effect == 14 && note.argument >> 4 == 13 && note.argument & 15 != 0 {
                channel.delayed_note = Some(note);
            } else {
                channel.trigger(note, song, matches!(note.effect, 3 | 5));
            }
            self.row_effect(index, note);
        }
        // Pattern loops are finite control states; revisiting an identical state
        // instead marks the end of a song programmed to jump back indefinitely.
        let state = RowState {
            order: self.order,
            row: self.row,
            speed: self.speed,
            bpm: self.bpm,
            loops: self
                .channels
                .iter()
                .map(|channel| (channel.loop_row, channel.loop_count))
                .collect(),
        };
        self.ended |= !self.visited.insert(state);
        self.new_row = false;
    }

    fn advance_row(&mut self, song: &Song) {
        if self.delay != 0 {
            self.delay -= 1;
            return;
        }
        self.new_row = true;
        if let Some(row) = self.loop_row.take() {
            self.row = row;
            return;
        }
        match (self.next_order.take(), self.next_row.take()) {
            (Some(order), row) => {
                self.order = order;
                self.row = row.unwrap_or(0);
            }
            (None, Some(row)) => {
                self.order += 1;
                self.row = row;
            }
            (None, None) => {
                self.row += 1;
                if self.row == 64 {
                    self.row = 0;
                    self.order += 1;
                }
            }
        }
        self.ended = self.order >= song.orders.len();
    }

    fn begin_tick(&mut self, song: &Song) {
        // MOD tempo changes take effect on the next tick. The mixer truncates
        // each tick independently rather than carrying fractional frames forward.
        self.remaining_samples = (u64::from(self.frequency) * 5 / (u64::from(self.bpm) * 2)) as u32;
        if self.tick == 0 {
            if self.new_row {
                self.begin_row(song);
            }
        } else {
            for channel in &mut self.channels {
                channel.tick(self.tick, song);
            }
        }
    }

    fn advance_tick(&mut self, song: &Song) {
        self.tick += 1;
        if self.tick == self.speed {
            self.tick = 0;
            self.advance_row(song);
        }
    }

    pub(super) fn next_sample(&mut self, song: &Song) -> (f32, f32) {
        if self.ended {
            return (0.0, 0.0);
        }
        if self.remaining_samples == 0 {
            self.begin_tick(song);
            if self.ended {
                return (0.0, 0.0);
            }
        }
        let mut output = (0.0, 0.0);
        for channel in &mut self.channels {
            let (left, right) = channel.mix(song, self.tick, self.frequency);
            output.0 += left;
            output.1 += right;
        }
        self.remaining_samples -= 1;
        if self.remaining_samples == 0 {
            self.advance_tick(song);
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module() -> Vec<u8> {
        let mut bytes = vec![0; 1084 + 1024 + 32];
        bytes[42..44].copy_from_slice(&16_u16.to_be_bytes());
        bytes[45] = 64;
        bytes[48..50].copy_from_slice(&16_u16.to_be_bytes());
        bytes[950] = 1;
        bytes[1080..1084].copy_from_slice(b"M.K.");
        bytes[1084..1088].copy_from_slice(&[1, 172, 16, 0]);
        bytes[2108..].fill(100);
        bytes
    }

    #[test]
    fn sample_order_pattern_and_loop_are_validated_before_playback() {
        for (offset, value) in [
            (950, 0),
            (950, 129),
            (45, 65),
            (44, 16),
            (1084, 0x21),
            (49, 255),
        ] {
            let mut bytes = module();
            bytes[offset] = value;
            assert!(
                matches!(Song::parse(&bytes), Err(DecodeError::InvalidData(_))),
                "{offset}"
            );
        }
        let bytes = module();
        for length in [0, 599, 1083, 2107, 2139] {
            assert!(matches!(
                Song::parse(&bytes[..length]),
                Err(DecodeError::InvalidData(_))
            ));
        }
    }

    #[test]
    fn tick_effects_change_sample_pitch_volume_and_position() {
        let song = Song::parse(&module()).unwrap();
        let mut channel = Channel {
            sample: 1,
            period: 428.0,
            volume: 64.0,
            ..Channel::default()
        };
        channel.effect = 1;
        channel.argument = 3;
        channel.tick(1, &song);
        assert_eq!(channel.period, 425.0);
        channel.effect = 2;
        channel.tick(1, &song);
        assert_eq!(channel.period, 428.0);
        channel.effect = 10;
        channel.argument = 4;
        channel.tick(1, &song);
        assert_eq!(channel.volume, 60.0);
        channel.effect = 0;
        channel.argument = 0x37;
        assert!(channel.rendered_period(1) < channel.rendered_period(0));
        assert!(channel.rendered_period(2) < channel.rendered_period(1));
        channel.effect = 14;
        channel.argument = 0x92;
        channel.position = 10.0;
        channel.tick(2, &song);
        assert_eq!(channel.position, 0.0);
        channel.argument = 0xc3;
        channel.tick(3, &song);
        assert_eq!(channel.volume, 0.0);
    }

    #[test]
    fn flow_effects_and_tempo_advance_orders_and_end_without_panics() {
        let song = Song::parse(&module()).unwrap();
        let mut player = Player::new(4, 44100);
        player.row_effect(
            0,
            Note {
                effect: 15,
                argument: 3,
                ..Note::default()
            },
        );
        assert_eq!(player.speed, 3);
        player.row_effect(
            0,
            Note {
                effect: 15,
                argument: 150,
                ..Note::default()
            },
        );
        assert_eq!(player.bpm, 150);
        player.row_effect(
            0,
            Note {
                effect: 13,
                argument: 0x12,
                ..Note::default()
            },
        );
        player.advance_row(&song);
        assert_eq!((player.order, player.row), (1, 12));
        assert!(player.ended);
        assert_eq!(player.next_sample(&song), (0.0, 0.0));
        let mut player = Player::new(4, 44100);
        player.row_effect(
            0,
            Note {
                effect: 11,
                argument: 200,
                ..Note::default()
            },
        );
        player.advance_row(&song);
        assert!(player.ended);
    }

    #[test]
    #[ignore = "requires the pinned external UQM content package"]
    fn shipped_mod_resources_parse_and_render_in_memory() {
        use std::io::Read;
        let path = std::path::Path::new(env!("UQM_RUST_ROOT"))
            .join("../sc2/content/packages/uqm-0.8.0-content.uqm");
        let file = std::fs::File::open(path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut count = 0;
        for index in 0..archive.len() {
            let mut resource = archive.by_index(index).unwrap();
            let name = resource.name().unwrap().into_owned();
            if !name.ends_with(".mod") {
                continue;
            }
            let mut bytes = Vec::new();
            resource.read_to_end(&mut bytes).unwrap();
            let song = Song::parse(&bytes).unwrap_or_else(|error| panic!("{name}: {error:?}"));
            let mut player = Player::new(song.channels, 8000);
            let mut audible = false;
            for _ in 0..8000 * 3 {
                let (left, right) = player.next_sample(&song);
                assert!(left.is_finite() && right.is_finite(), "{name}");
                audible |= left != 0.0 || right != 0.0;
            }
            assert!(audible, "{name}");
            count += 1;
        }
        assert!(count > 25);
        eprintln!("shipped MOD resources decoded: {count}");
    }
}
