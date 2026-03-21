// Netplay Integration — CRC-32 Checksums & Synchronization Types
// @plan PLAN-20260320-BATTLE.P15
// @requirement REQ-NET-001 through REQ-NET-015 — Netplay checksum system

use super::element::{Element, ElementFlags, Point};
use super::velocity::{Extent, VelocityDesc};

// ---------------------------------------------------------------------------
// CRC-32 Core
// ---------------------------------------------------------------------------

/// CRC-32 table for polynomial 0x04c11db7 (IEEE 802.3)
/// Bit-identical to C's crcTable from crc.c
const CRC_TABLE: [u32; 256] = [
    0x00000000, 0x77073096, 0xee0e612c, 0x990951ba, 0x076dc419, 0x706af48f, 0xe963a535, 0x9e6495a3,
    0x0edb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988, 0x09b64c2b, 0x7eb17cbd, 0xe7b82d07, 0x90bf1d91,
    0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de, 0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7,
    0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec, 0x14015c4f, 0x63066cd9, 0xfa0f3d63, 0x8d080df5,
    0x3b6e20c8, 0x4c69105e, 0xd56041e4, 0xa2677172, 0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b,
    0x35b5a8fa, 0x42b2986c, 0xdbbbc9d6, 0xacbcf940, 0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59,
    0x26d930ac, 0x51de003a, 0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423, 0xcfba9599, 0xb8bda50f,
    0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924, 0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d,
    0x76dc4190, 0x01db7106, 0x98d220bc, 0xefd5102a, 0x71b18589, 0x06b6b51f, 0x9fbfe4a5, 0xe8b8d433,
    0x7807c9a2, 0x0f00f934, 0x9609a88e, 0xe10e9818, 0x7f6a0dbb, 0x086d3d2d, 0x91646c97, 0xe6635c01,
    0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e, 0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457,
    0x65b0d9c6, 0x12b7e950, 0x8bbeb8ea, 0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65,
    0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2, 0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb,
    0x4369e96a, 0x346ed9fc, 0xad678846, 0xda60b8d0, 0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9,
    0x5005713c, 0x270241aa, 0xbe0b1010, 0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
    0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17, 0x2eb40d81, 0xb7bd5c3b, 0xc0ba6cad,
    0xedb88320, 0x9abfb3b6, 0x03b6e20c, 0x74b1d29a, 0xead54739, 0x9dd277af, 0x04db2615, 0x73dc1683,
    0xe3630b12, 0x94643b84, 0x0d6d6a3e, 0x7a6a5aa8, 0xe40ecf0b, 0x9309ff9d, 0x0a00ae27, 0x7d079eb1,
    0xf00f9344, 0x8708a3d2, 0x1e01f268, 0x6906c2fe, 0xf762575d, 0x806567cb, 0x196c3671, 0x6e6b06e7,
    0xfed41b76, 0x89d32be0, 0x10da7a5a, 0x67dd4acc, 0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5,
    0xd6d6a3e8, 0xa1d1937e, 0x38d8c2c4, 0x4fdff252, 0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
    0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55, 0x316e8eef, 0x4669be79,
    0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236, 0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f,
    0xc5ba3bbe, 0xb2bd0b28, 0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7, 0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d,
    0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x026d930a, 0x9c0906a9, 0xeb0e363f, 0x72076785, 0x05005713,
    0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0x0cb61b38, 0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0x0bdbdf21,
    0x86d3d2d4, 0xf1d4e242, 0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777,
    0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c, 0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45,
    0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2, 0xa7672661, 0xd06016f7, 0x4969474d, 0x3e6e77db,
    0xaed16a4a, 0xd9d65adc, 0x40df0b66, 0x37d83bf0, 0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
    0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605, 0xcdd70693, 0x54de5729, 0x23d967bf,
    0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94, 0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d,
];

/// CRC-32 state accumulator
/// Matches C's crc_State from crc.h
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrcState {
    crc: u32,
}

impl CrcState {
    /// Initializes CRC state to 0xFFFFFFFF
    /// Matches C's crc_init()
    pub fn new() -> Self {
        CrcState { crc: 0xFFFFFFFF }
    }

    /// Resets CRC state to initial value
    pub fn init(&mut self) {
        self.crc = 0xFFFFFFFF;
    }

    /// Processes a single byte into the CRC
    /// Matches C's crc_processUint8()
    pub fn update(&mut self, byte: u8) {
        self.crc = (self.crc >> 8) ^ CRC_TABLE[((self.crc ^ (byte as u32)) & 0xFF) as usize];
    }

    /// Finalizes the CRC (XOR with 0xFFFFFFFF)
    /// Matches C's crc_finish()
    pub fn finish(&self) -> u32 {
        !self.crc
    }

    /// Processes a u16 value (little-endian)
    /// Matches C's crc_processUint16()
    pub fn process_u16(&mut self, val: u16) {
        self.update((val & 0xFF) as u8);
        self.update((val >> 8) as u8);
    }

    /// Processes a u32 value (little-endian)
    /// Matches C's crc_processUint32()
    pub fn process_u32(&mut self, val: u32) {
        self.update((val & 0xFF) as u8);
        self.update(((val >> 8) & 0xFF) as u8);
        self.update(((val >> 16) & 0xFF) as u8);
        self.update((val >> 24) as u8);
    }

    /// Processes an i16 value (little-endian, reinterpreted as u16)
    /// Used for COORD, SIZE, and other signed 16-bit types
    pub fn process_i16(&mut self, val: i16) {
        self.process_u16(val as u16);
    }

    /// Processes an Extent (width, height)
    /// Matches C's crc_processEXTENT()
    pub fn process_extent(&mut self, extent: &Extent) {
        self.process_i16(extent.width);
        self.process_i16(extent.height);
    }

    /// Processes a Point (x, y)
    /// Matches C's crc_processPOINT()
    pub fn process_point(&mut self, point: &Point) {
        self.process_i16(point.x);
        self.process_i16(point.y);
    }

    /// Processes a VelocityDesc (TravelAngle, vector, fract, error, incr)
    /// Matches C's crc_processVELOCITY_DESC()
    pub fn process_velocity_desc(&mut self, vel: &VelocityDesc) {
        self.process_u16(vel.travel_angle);
        self.process_extent(&vel.vector);
        self.process_extent(&vel.fract);
        self.process_extent(&vel.error);
        self.process_extent(&vel.incr);
    }

    /// Processes an Element state (location only)
    /// Matches C's crc_processSTATE()
    /// The C version only processes location, not frame/farray
    pub fn process_element_state(&mut self, location: &Point) {
        self.process_point(location);
    }

    /// Processes an Element for checksum
    /// Matches C's crc_processELEMENT() exactly
    ///
    /// CRITICAL: Field order must match C implementation:
    /// 1. state_flags (u16)
    /// 2. life_span (u16)
    /// 3. crew_level (u16)
    /// 4. mass_points (u8)
    /// 5. turn_wait (u8)
    /// 6. thrust_wait (u8)
    /// 7. velocity (VelocityDesc)
    /// 8. current.location (Point)
    /// 9. next.location (Point)
    ///
    /// BACKGROUND_OBJECT elements are EXCLUDED (skipped entirely)
    pub fn process_element(&mut self, element: &Element) {
        // Skip BACKGROUND_OBJECT elements
        if element
            .state_flags
            .contains(ElementFlags::BACKGROUND_OBJECT)
        {
            return;
        }

        // Process fields in exact order as C
        self.process_u16(element.state_flags.bits());
        self.process_u16(element.life_span);
        self.process_u16(element.crew_or_hp);
        self.update(element.mass_points);
        self.update(element.turn_wait);
        self.update(element.thrust_or_blast);
        self.process_velocity_desc(&element.velocity);
        self.process_element_state(&element.current.location);
        self.process_element_state(&element.next.location);
    }
}

impl Default for CrcState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Netplay Types (Type-Only Definitions)
// ---------------------------------------------------------------------------

/// Battle frame counter
/// Tracks the current frame number in netplay synchronization
pub type BattleFrameCounter = u32;

/// Checksum value (CRC-32)
pub type Checksum = u32;

/// Battle end protocol phases
/// Matches the 4-phase battle termination handshake in netplay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BattleEndPhase {
    /// Phase 0: Battle ongoing, no end signaled
    Ongoing = 0,
    /// Phase 1: Local side detects battle end, sends BATTLEEND signal
    LocalEnd = 1,
    /// Phase 2: Remote side acknowledges, sends BATTLEEND_ACK
    RemoteAck = 2,
    /// Phase 3: Both sides synchronized, battle cleanup proceeds
    Synchronized = 3,
}

/// Input buffer hook types (placeholder for future implementation)
/// These will be used for netplay input synchronization
pub mod input_buffer {
    use super::BattleFrameCounter;

    /// Input command buffer entry
    #[derive(Debug, Clone, Copy)]
    pub struct InputBufferEntry {
        pub frame: BattleFrameCounter,
        pub command: u16, // Ship control flags
    }

    /// Input buffer for a single player
    pub struct InputBuffer {
        entries: Vec<InputBufferEntry>,
        capacity: usize,
    }

    impl InputBuffer {
        pub fn new(capacity: usize) -> Self {
            InputBuffer {
                entries: Vec::with_capacity(capacity),
                capacity,
            }
        }

        pub fn push(&mut self, entry: InputBufferEntry) -> Result<(), &'static str> {
            if self.entries.len() >= self.capacity {
                return Err("Input buffer full");
            }
            self.entries.push(entry);
            Ok(())
        }

        pub fn get(&self, frame: BattleFrameCounter) -> Option<u16> {
            self.entries
                .iter()
                .find(|e| e.frame == frame)
                .map(|e| e.command)
        }

        pub fn clear(&mut self) {
            self.entries.clear();
        }
    }
}

/// Frame synchronization types (placeholder for future implementation)
pub mod frame_sync {
    use super::{BattleFrameCounter, Checksum};

    /// Synchronization state for a single frame
    #[derive(Debug, Clone, Copy)]
    pub struct FrameSyncState {
        pub frame: BattleFrameCounter,
        pub local_checksum: Option<Checksum>,
        pub remote_checksum: Option<Checksum>,
        pub verified: bool,
    }

    impl FrameSyncState {
        pub fn new(frame: BattleFrameCounter) -> Self {
            FrameSyncState {
                frame,
                local_checksum: None,
                remote_checksum: None,
                verified: false,
            }
        }

        pub fn set_local(&mut self, checksum: Checksum) {
            self.local_checksum = Some(checksum);
        }

        pub fn set_remote(&mut self, checksum: Checksum) {
            self.remote_checksum = Some(checksum);
        }

        pub fn verify(&mut self) -> bool {
            if let (Some(local), Some(remote)) = (self.local_checksum, self.remote_checksum) {
                self.verified = local == remote;
                self.verified
            } else {
                false
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- CRC-32 Table Verification --

    #[test]
    fn crc_table_first_entries_match_c() {
        // Verify first 6 entries match C's crcTable
        assert_eq!(CRC_TABLE[0], 0x00000000);
        assert_eq!(CRC_TABLE[1], 0x77073096);
        assert_eq!(CRC_TABLE[2], 0xee0e612c);
        assert_eq!(CRC_TABLE[3], 0x990951ba);
        assert_eq!(CRC_TABLE[4], 0x076dc419);
        assert_eq!(CRC_TABLE[5], 0x706af48f);
    }

    #[test]
    fn crc_table_complete() {
        // Verify table has 256 entries
        assert_eq!(CRC_TABLE.len(), 256);
        // Spot-check last entry
        assert_eq!(CRC_TABLE[255], 0x2d02ef8d);
    }

    // -- CRC-32 Core Operations --

    #[test]
    fn crc_init_sets_initial_value() {
        let mut crc = CrcState::new();
        assert_eq!(crc.crc, 0xFFFFFFFF);

        crc.crc = 0x12345678;
        crc.init();
        assert_eq!(crc.crc, 0xFFFFFFFF);
    }

    #[test]
    fn crc_finish_inverts_state() {
        let crc = CrcState { crc: 0x12345678 };
        assert_eq!(crc.finish(), !0x12345678);

        let crc_initial = CrcState::new();
        assert_eq!(crc_initial.finish(), 0x00000000);
    }

    #[test]
    fn crc_update_single_byte() {
        let mut crc = CrcState::new();
        crc.update(0x00);
        // After processing 0x00 with init value 0xFFFFFFFF:
        // newCrc = (0xFFFFFFFF >> 8) ^ CRC_TABLE[(0xFFFFFFFF ^ 0x00) & 0xFF]
        //        = 0x00FFFFFF ^ CRC_TABLE[0xFF]
        //        = 0x00FFFFFF ^ 0x2d02ef8d
        //        = 0x2dfd1072
        assert_eq!(crc.crc, 0x2dfd1072);
    }

    #[test]
    fn crc_of_empty_data() {
        let crc = CrcState::new();
        assert_eq!(crc.finish(), 0x00000000);
    }

    #[test]
    fn crc_of_known_sequence() {
        // Test with the byte sequence "123456789" (common CRC test vector)
        // Expected CRC-32: 0xCBF43926
        let mut crc = CrcState::new();
        for &byte in b"123456789" {
            crc.update(byte);
        }
        assert_eq!(crc.finish(), 0xCBF43926);
    }

    // -- CRC-32 Typed Operations --

    #[test]
    fn crc_process_u16_little_endian() {
        let mut crc = CrcState::new();
        crc.process_u16(0x1234);

        // Should process low byte first (0x34), then high byte (0x12)
        let mut expected = CrcState::new();
        expected.update(0x34);
        expected.update(0x12);

        assert_eq!(crc.crc, expected.crc);
    }

    #[test]
    fn crc_process_i16_reinterprets_as_u16() {
        let mut crc_signed = CrcState::new();
        crc_signed.process_i16(-1);

        let mut crc_unsigned = CrcState::new();
        crc_unsigned.process_u16(0xFFFF);

        assert_eq!(crc_signed.crc, crc_unsigned.crc);
    }

    #[test]
    fn crc_process_extent() {
        let extent = Extent::new(100, 200);

        let mut crc = CrcState::new();
        crc.process_extent(&extent);

        let mut expected = CrcState::new();
        expected.process_i16(100);
        expected.process_i16(200);

        assert_eq!(crc.crc, expected.crc);
    }

    #[test]
    fn crc_process_point() {
        let point = Point::new(-50, 75);

        let mut crc = CrcState::new();
        crc.process_point(&point);

        let mut expected = CrcState::new();
        expected.process_i16(-50);
        expected.process_i16(75);

        assert_eq!(crc.crc, expected.crc);
    }

    #[test]
    fn crc_process_velocity_desc() {
        let mut vel = VelocityDesc::new();
        vel.travel_angle = 32;
        vel.vector = Extent::new(10, 20);
        vel.fract = Extent::new(5, 7);
        vel.error = Extent::new(1, 2);
        vel.incr = Extent::new(8, 9);

        let mut crc = CrcState::new();
        crc.process_velocity_desc(&vel);

        let mut expected = CrcState::new();
        expected.process_u16(32);
        expected.process_extent(&Extent::new(10, 20));
        expected.process_extent(&Extent::new(5, 7));
        expected.process_extent(&Extent::new(1, 2));
        expected.process_extent(&Extent::new(8, 9));

        assert_eq!(crc.crc, expected.crc);
    }

    // -- Element CRC Processing --

    #[test]
    fn crc_process_element_field_order() {
        let mut elem = Element::new();
        elem.state_flags = ElementFlags::APPEARING | ElementFlags::PLAYER_SHIP;
        elem.life_span = 100;
        elem.crew_or_hp = 42;
        elem.mass_points = 10;
        elem.turn_wait = 3;
        elem.thrust_or_blast = 5;
        elem.current.location = Point::new(1000, 2000);
        elem.next.location = Point::new(1100, 2100);

        let mut crc = CrcState::new();
        crc.process_element(&elem);

        // Verify field order by manual CRC computation
        let mut expected = CrcState::new();
        expected.process_u16(elem.state_flags.bits());
        expected.process_u16(elem.life_span);
        expected.process_u16(elem.crew_or_hp);
        expected.update(elem.mass_points);
        expected.update(elem.turn_wait);
        expected.update(elem.thrust_or_blast);
        expected.process_velocity_desc(&elem.velocity);
        expected.process_point(&elem.current.location);
        expected.process_point(&elem.next.location);

        assert_eq!(crc.crc, expected.crc);
    }

    #[test]
    fn crc_process_element_background_object_excluded() {
        let mut elem = Element::new();
        elem.state_flags = ElementFlags::BACKGROUND_OBJECT;
        elem.mass_points = 100;
        elem.life_span = 50;

        let mut crc = CrcState::new();
        crc.process_element(&elem);

        // Should not modify CRC at all
        let expected = CrcState::new();
        assert_eq!(crc.crc, expected.crc);
    }

    #[test]
    fn crc_process_element_normal_vs_background() {
        let mut normal_elem = Element::new();
        normal_elem.mass_points = 10;
        normal_elem.life_span = 100;

        let mut background_elem = Element::new();
        background_elem.state_flags = ElementFlags::BACKGROUND_OBJECT;
        background_elem.mass_points = 10;
        background_elem.life_span = 100;

        let mut crc_normal = CrcState::new();
        crc_normal.process_element(&normal_elem);

        let mut crc_background = CrcState::new();
        crc_background.process_element(&background_elem);

        // Normal element should produce non-initial CRC
        // Background element should produce initial CRC
        assert_ne!(crc_normal.crc, CrcState::new().crc);
        assert_eq!(crc_background.crc, CrcState::new().crc);
    }

    // -- Netplay Types --

    #[test]
    fn battle_end_phase_enum_values() {
        assert_eq!(BattleEndPhase::Ongoing as u8, 0);
        assert_eq!(BattleEndPhase::LocalEnd as u8, 1);
        assert_eq!(BattleEndPhase::RemoteAck as u8, 2);
        assert_eq!(BattleEndPhase::Synchronized as u8, 3);
    }

    #[test]
    fn input_buffer_push_and_get() {
        let mut buffer = input_buffer::InputBuffer::new(10);

        let entry1 = input_buffer::InputBufferEntry {
            frame: 100,
            command: 0x0001,
        };
        let entry2 = input_buffer::InputBufferEntry {
            frame: 101,
            command: 0x0002,
        };

        assert!(buffer.push(entry1).is_ok());
        assert!(buffer.push(entry2).is_ok());

        assert_eq!(buffer.get(100), Some(0x0001));
        assert_eq!(buffer.get(101), Some(0x0002));
        assert_eq!(buffer.get(102), None);
    }

    #[test]
    fn input_buffer_full() {
        let mut buffer = input_buffer::InputBuffer::new(2);

        let entry1 = input_buffer::InputBufferEntry {
            frame: 1,
            command: 0x0001,
        };
        let entry2 = input_buffer::InputBufferEntry {
            frame: 2,
            command: 0x0002,
        };
        let entry3 = input_buffer::InputBufferEntry {
            frame: 3,
            command: 0x0003,
        };

        assert!(buffer.push(entry1).is_ok());
        assert!(buffer.push(entry2).is_ok());
        assert_eq!(buffer.push(entry3), Err("Input buffer full"));
    }

    #[test]
    fn frame_sync_state_verify() {
        let mut sync = frame_sync::FrameSyncState::new(1000);

        assert!(!sync.verify()); // No checksums set

        sync.set_local(0x12345678);
        assert!(!sync.verify()); // Only local set

        sync.set_remote(0x12345678);
        assert!(sync.verify()); // Both match

        let mut sync_mismatch = frame_sync::FrameSyncState::new(1001);
        sync_mismatch.set_local(0x11111111);
        sync_mismatch.set_remote(0x22222222);
        assert!(!sync_mismatch.verify()); // Mismatch
    }
}
