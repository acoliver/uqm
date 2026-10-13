//! The "Star" section: where in the star catalog the game is.
//!
//! `SaveGame` copies `*CurStarDescPtr` into a `STAR_DESC` (all zeroes when the
//! pointer is NULL, as in hyperspace between systems) and writes its eight
//! bytes. `LoadGame` reads the eight bytes back and sets `CurStarDescPtr =
//! FindStar (NULL, &SD.star_pt, 0, 0)`, which is the catalog entry at exactly
//! that point or NULL when there is none.
//!
//! The pointer itself is a C global and the catalog is `starmap_array`
//! (plandata.c), a static C table that `star_array` points at; neither has a
//! Rust owner yet. This module reads them as borrowed data: `catalog` is the
//! live table (`CATALOG_LEN` entries, both sentinels included), `current` is
//! `CurStarDescPtr`. Capture requires the pointer to lie in that table, so the
//! chunk always names a real catalog entry. Restore does not move the player:
//! it checks the saved descriptor against the catalog as `FindStar` would and
//! says which entry (or NULL) the game must point at; assigning
//! `CurStarDescPtr`, and everything a new system needs (the solar-system
//! state, the system's orbits), stays with the solar-system owner.
//!
//! The wire form is 8 bytes (`StarDesc`); the runtime form is a pointer to a
//! catalog entry of the same 8 bytes (`CStarDesc`). They are different types
//! on purpose.
//!
//! Where this differs from the C code on purpose: `LoadGame` took whatever
//! `FindStar` returned, so a saved descriptor whose point matched a star but
//! whose type, index or name numbers did not was loaded as that star. Here
//! that is `FieldsDisagree`, and a point with no catalog entry is
//! `NotInCatalog` unless the whole descriptor is zero, the NULL case.

use std::fmt;

use crate::comm::dispatch::CPoint;
use crate::savegame::types::{Point, StarDesc};

/// `NUM_SOLAR_SYSTEMS` (starmap.h).
pub const NUM_SOLAR_SYSTEMS: usize = 502;
/// `NUM_HYPER_VORTICES` (starmap.c, inside `FindStar`): the index of the last
/// quasispace portal, so the portals number one more.
pub const NUM_HYPER_VORTICES: usize = 15;
/// The systems, a sentinel, the quasispace portals and a second sentinel.
pub const CATALOG_LEN: usize = NUM_SOLAR_SYSTEMS + 1 + (NUM_HYPER_VORTICES + 1) + 1;

/// C: `STAR_DESC` (planets.h).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CStarDesc {
    pub star_pt: CPoint,
    pub star_type: u8,
    pub index: u8,
    pub prefix: u8,
    pub postfix: u8,
}

const _: () = {
    assert!(std::mem::size_of::<CStarDesc>() == 8);
    assert!(std::mem::align_of::<CStarDesc>() == 2);
};

/// Why the star section could not be captured or checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarError {
    /// The catalog is not `CATALOG_LEN` entries.
    WrongCatalogLength { found: usize },
    /// `CurStarDescPtr` points outside the catalog or between its entries.
    CurrentOutsideCatalog,
    /// No catalog entry sits at the saved point and the descriptor is not the
    /// all-zero NULL one.
    NotInCatalog { star_pt: Point },
    /// The catalog entry at the saved point differs from the saved descriptor.
    FieldsDisagree { catalog_index: usize },
}

impl fmt::Display for StarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for StarError {}

fn check_catalog(catalog: &[CStarDesc]) -> Result<(), StarError> {
    if catalog.len() != CATALOG_LEN {
        return Err(StarError::WrongCatalogLength {
            found: catalog.len(),
        });
    }
    Ok(())
}

fn wire(star: &CStarDesc) -> StarDesc {
    StarDesc {
        star_pt: Point {
            x: star.star_pt.x,
            y: star.star_pt.y,
        },
        star_type: star.star_type,
        index: star.index,
        prefix: star.prefix,
        postfix: star.postfix,
    }
}

/// The "Star" chunk of a save made while `current` is `CurStarDescPtr`.
///
/// NULL gives the all-zero descriptor, as `SaveGame`'s `memset` does.
pub fn capture_star(
    catalog: &[CStarDesc],
    current: *const CStarDesc,
) -> Result<StarDesc, StarError> {
    check_catalog(catalog)?;
    if current.is_null() {
        return Ok(StarDesc {
            star_pt: Point { x: 0, y: 0 },
            star_type: 0,
            index: 0,
            prefix: 0,
            postfix: 0,
        });
    }
    let start = catalog.as_ptr() as usize;
    let offset = (current as usize).wrapping_sub(start);
    let entry = std::mem::size_of::<CStarDesc>();
    if offset >= CATALOG_LEN * entry || !offset.is_multiple_of(entry) {
        return Err(StarError::CurrentOutsideCatalog);
    }
    Ok(wire(&catalog[offset / entry]))
}

/// `FindStar (NULL, &point, 0, 0)` (starmap.c): the index of the catalog entry
/// at exactly `point`, searching the systems, or the quasispace portals when
/// `ARILOU_SPACE_SIDE` is above 1.
///
/// The catalog is sorted by `y`, ends each section with a sentinel whose `y`
/// is past every real one, and the search is the original's, quirks included,
/// except that the sentinels themselves are never found.
pub fn find_star(
    catalog: &[CStarDesc],
    arilou_space_side: u8,
    point: Point,
) -> Result<Option<usize>, StarError> {
    check_catalog(catalog)?;
    let (base, mut hi) = if arilou_space_side <= 1 {
        (0usize, NUM_SOLAR_SYSTEMS as isize - 1)
    } else {
        (NUM_SOLAR_SYSTEMS + 1, NUM_HYPER_VORTICES as isize)
    };
    let mut lo = 0isize;
    while lo < hi {
        let mid = (lo + hi) >> 1;
        if catalog[base + mid as usize].star_pt.y >= point.y {
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
    }
    let mut at = base + lo as usize;
    if catalog[at].star_pt.y > point.y {
        return Ok(None);
    }
    loop {
        let star = &catalog[at].star_pt;
        if star.y >= point.y && star.x == point.x {
            // `FindStar` hands back a sentinel for the point it sits at; it is
            // not a star.
            let sentinel = at == NUM_SOLAR_SYSTEMS || at == CATALOG_LEN - 1;
            return Ok((!sentinel).then_some(at));
        }
        at += 1;
        if catalog[at].star_pt.y > point.y {
            return Ok(None);
        }
    }
}

/// What a loaded "Star" chunk means for `CurStarDescPtr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadedStar {
    /// `CurStarDescPtr` becomes NULL.
    None,
    /// `CurStarDescPtr` becomes `&catalog[index]`.
    Catalog(usize),
}

/// Check a saved descriptor against the live catalog, as `LoadGame`'s
/// `FindStar` call would resolve it, and say where the game must point.
/// Nothing is changed.
pub fn resolve_star(
    catalog: &[CStarDesc],
    arilou_space_side: u8,
    saved: &StarDesc,
) -> Result<LoadedStar, StarError> {
    match find_star(catalog, arilou_space_side, saved.star_pt)? {
        Some(index) => {
            if wire(&catalog[index]) != *saved {
                return Err(StarError::FieldsDisagree {
                    catalog_index: index,
                });
            }
            Ok(LoadedStar::Catalog(index))
        }
        None => {
            let zero = StarDesc {
                star_pt: Point { x: 0, y: 0 },
                star_type: 0,
                index: 0,
                prefix: 0,
                postfix: 0,
            };
            if *saved == zero {
                Ok(LoadedStar::None)
            } else {
                Err(StarError::NotInCatalog {
                    star_pt: saved.star_pt,
                })
            }
        }
    }
}
