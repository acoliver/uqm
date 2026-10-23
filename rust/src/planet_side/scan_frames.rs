//! Frame selection rules for scan dots and mineral sprites in `miscdata.ani`.
//!
//! These mirror `generate_planet_side` in the original C `scan.c`
//! (`NUM_SCANDOT_TRANSITIONS` is 4 in `planets.h`).  Both the orbit-scan
//! display list and the lander surface visuals select frames here so the two
//! cannot drift apart.
//!
//! Layout of `miscdata.ani`: frames 0..=3 are the energy dot, 4..=7 the
//! biological dot, and from frame 8 each of the eight mineral categories owns a
//! cluster of five frames: the cluster base (the "current" frame), three
//! scan-dot frames (one per gross size), then three surface sprites that
//! overlap the dot frames shifted by one.

/// Number of animation frames in one scan-dot transition.
pub const NUM_SCANDOT_TRANSITIONS: u16 = 4;
/// First frame of the energy-dot transition.
pub const ENERGY_FRAME_BASE: u16 = 0;
/// First frame of the biological-dot transition.
pub const BIO_FRAME_BASE: u16 = NUM_SCANDOT_TRANSITIONS;
/// First frame of the first mineral category cluster.
pub const MINERAL_FRAME_BASE: u16 = 2 * NUM_SCANDOT_TRANSITIONS;
/// Frames owned by each mineral category cluster.
pub const MINERAL_FRAMES_PER_CATEGORY: u16 = 5;
/// Number of mineral categories.
pub const MINERAL_CATEGORY_COUNT: usize = 8;
/// Number of gross deposit sizes the surface generator produces (0..=2).
pub const MINERAL_GROSS_SIZE_COUNT: u16 = 3;

/// Rejected scan-frame inputs.  Callers never clamp: out-of-range values mean
/// the generator broke its contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanFrameError {
    /// Mineral category is not in `0..8`.
    Category,
    /// Gross deposit size is not in `0..3`.
    GrossSize,
}

/// The `current` and `next` frames of a scan element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanDotFrames {
    pub current: u16,
    pub next: u16,
}

/// Energy blips use frames 0 (current) and 3 (next).
pub const ENERGY_SCAN_DOT: ScanDotFrames = ScanDotFrames {
    current: ENERGY_FRAME_BASE,
    next: ENERGY_FRAME_BASE + NUM_SCANDOT_TRANSITIONS - 1,
};

/// Biological blips use frames 4 (current) and 7 (next).
pub const BIO_SCAN_DOT: ScanDotFrames = ScanDotFrames {
    current: BIO_FRAME_BASE,
    next: BIO_FRAME_BASE + NUM_SCANDOT_TRANSITIONS - 1,
};

fn mineral_cluster_base(category: usize, gross: u16) -> Result<u16, ScanFrameError> {
    if category >= MINERAL_CATEGORY_COUNT {
        return Err(ScanFrameError::Category);
    }
    if gross >= MINERAL_GROSS_SIZE_COUNT {
        return Err(ScanFrameError::GrossSize);
    }
    // `category` is below 8, so the narrowing cannot truncate.
    Ok(MINERAL_FRAME_BASE + category as u16 * MINERAL_FRAMES_PER_CATEGORY)
}

/// Scan and minimap dot for a mineral deposit: `current` is the cluster base,
/// `next` is `base + gross + 1`.
pub fn mineral_scan_dot(category: usize, gross: u16) -> Result<ScanDotFrames, ScanFrameError> {
    let current = mineral_cluster_base(category, gross)?;
    Ok(ScanDotFrames {
        current,
        next: current + gross + 1,
    })
}

/// Lander surface sprite for a mineral deposit: `base + gross + 2`.
pub fn mineral_surface_sprite(category: usize, gross: u16) -> Result<u16, ScanFrameError> {
    Ok(mineral_scan_dot(category, gross)?.next + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mineral_scan_dot_matches_c_literals() {
        assert_eq!(mineral_scan_dot(0, 0).unwrap().next, 9);
        assert_eq!(mineral_scan_dot(2, 0).unwrap().next, 19);
        assert_eq!(mineral_scan_dot(2, 2).unwrap().next, 21);
        assert_eq!(mineral_scan_dot(7, 2).unwrap().next, 46);
    }

    #[test]
    fn mineral_scan_dot_current_is_cluster_base() {
        assert_eq!(mineral_scan_dot(0, 1).unwrap().current, 8);
        assert_eq!(mineral_scan_dot(2, 0).unwrap().current, 18);
        assert_eq!(mineral_scan_dot(7, 2).unwrap().current, 43);
    }

    #[test]
    fn mineral_surface_sprite_matches_c_literals() {
        assert_eq!(mineral_surface_sprite(2, 0), Ok(20));
        assert_eq!(mineral_surface_sprite(7, 2), Ok(47));
        assert_eq!(mineral_surface_sprite(0, 0), Ok(10));
    }

    #[test]
    fn energy_and_bio_dots_match_c_literals() {
        assert_eq!((ENERGY_SCAN_DOT.current, ENERGY_SCAN_DOT.next), (0, 3));
        assert_eq!((BIO_SCAN_DOT.current, BIO_SCAN_DOT.next), (4, 7));
    }

    #[test]
    fn out_of_range_inputs_are_rejected() {
        assert_eq!(mineral_scan_dot(8, 0), Err(ScanFrameError::Category));
        assert_eq!(mineral_surface_sprite(8, 0), Err(ScanFrameError::Category));
        assert_eq!(mineral_scan_dot(2, 3), Err(ScanFrameError::GrossSize));
        assert_eq!(mineral_surface_sprite(2, 3), Err(ScanFrameError::GrossSize));
        assert_eq!(
            mineral_scan_dot(usize::MAX, 0),
            Err(ScanFrameError::Category)
        );
    }

    #[test]
    fn every_valid_selection_stays_inside_its_category_cluster() {
        // Cluster ranges written out for the C layout: 8 + 5*category ..= +4.
        let clusters: [(usize, u16, u16); 8] = [
            (0, 8, 12),
            (1, 13, 17),
            (2, 18, 22),
            (3, 23, 27),
            (4, 28, 32),
            (5, 33, 37),
            (6, 38, 42),
            (7, 43, 47),
        ];
        for (category, first, last) in clusters {
            for gross in 0..3u16 {
                let dot = mineral_scan_dot(category, gross).unwrap();
                let sprite = mineral_surface_sprite(category, gross).unwrap();
                for frame in [dot.current, dot.next, sprite] {
                    assert!(
                        (first..=last).contains(&frame),
                        "{category}/{gross}: {frame}"
                    );
                }
            }
        }
    }
}
