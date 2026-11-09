//! Star catalog lookup and cluster naming, ported from starmap.c.

use crate::comm::dispatch::CPoint;
use crate::savegame::runtime::star::{CStarDesc, NUM_HYPER_VORTICES, NUM_SOLAR_SYSTEMS};

pub(crate) fn catalog_star_count(catalog: &[CStarDesc]) -> usize {
    catalog
        .iter()
        .take_while(|star| star.star_pt.x <= 9999 && star.star_pt.y <= 9999)
        .count()
}

fn find_star_index(
    catalog: &[CStarDesc],
    last: Option<usize>,
    point: CPoint,
    xb: i16,
    yb: i16,
    mut hi: usize,
) -> Option<usize> {
    let mut lo = last.map_or(0, |last| last + 1);
    if last.is_some() {
        if lo > hi {
            return None;
        }
        hi = lo;
    }
    let (min_y, max_y) = if yb <= 0 {
        (point.y, point.y)
    } else {
        (point.y.wrapping_sub(yb), point.y.wrapping_add(yb))
    };
    while lo < hi {
        let mid = (lo + hi) >> 1;
        if catalog[mid].star_pt.y >= min_y {
            // The original signed search can stop at -1 only when mid is zero.
            if mid == 0 {
                break;
            }
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
    }
    let mut at = lo;
    if yb < 0 || catalog[at].star_pt.y <= max_y {
        let (min_x, max_x) = if xb <= 0 {
            (point.x, point.x)
        } else {
            (point.x.wrapping_sub(xb), point.x.wrapping_add(xb))
        };
        loop {
            let star = &catalog[at];
            if (yb < 0 || star.star_pt.y >= min_y)
                && (xb < 0 || (star.star_pt.x >= min_x && star.star_pt.x <= max_x))
            {
                return Some(at);
            }
            at += 1;
            if catalog[at].star_pt.y > max_y {
                break;
            }
        }
    }
    None
}

pub fn find_star_id(
    world: &crate::engine::World,
    catalog: &[CStarDesc; crate::engine::world::CATALOG_LEN],
    last: Option<crate::engine::StarId>,
    point: CPoint,
    xb: i16,
    yb: i16,
) -> Option<crate::engine::StarId> {
    let (base, hi) = if world.story_state(crate::engine::StoryKey::ARILOU_SPACE_SIDE) <= 1 {
        (0, NUM_SOLAR_SYSTEMS - 1)
    } else {
        (NUM_SOLAR_SYSTEMS + 1, NUM_HYPER_VORTICES)
    };
    let last = last.map(|id| {
        id.0.checked_sub(base)
            .unwrap_or_else(|| panic!("star search resumed outside its space"))
    });
    find_star_index(&catalog[base..], last, point, xb, yb, hi)
        .map(|index| crate::engine::StarId(base + index))
}

pub(crate) mod naming {
    use super::*;

    pub(crate) fn name_near(
        kernel: &crate::engine::Kernel,
        catalog: &[CStarDesc; crate::engine::world::CATALOG_LEN],
        point: CPoint,
    ) -> Option<(std::ffi::CString, CPoint)> {
        let index = find_star_index(catalog, None, point, 1, 1, NUM_SOLAR_SYSTEMS - 1)?;
        let star = &catalog[index];
        Some((
            cluster_name(
                kernel,
                star,
                crate::starbase::ids::STAR_NUMBER_BASE as usize,
            ),
            star.star_pt,
        ))
    }

    pub(crate) fn cluster_name(
        kernel: &crate::engine::Kernel,
        star: &CStarDesc,
        prefix_base: usize,
    ) -> std::ffi::CString {
        let mut text = Vec::new();
        if star.prefix != 0 {
            let index = prefix_base + usize::from(star.prefix) - 1;
            if let Some(prefix) = kernel.game_text_entry(index) {
                text.extend_from_slice(prefix.to_bytes());
                text.push(b' ');
            }
        }
        text.extend_from_slice(kernel.game_text(usize::from(star.postfix)).to_bytes());
        std::ffi::CString::new(text)
            .unwrap_or_else(|error| panic!("cluster name contains NUL: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universe_catalog_is_one_immutable_complete_table() {
        let stars: &'static [CStarDesc; crate::engine::world::CATALOG_LEN] =
            &crate::planets::tables::STARMAP_ARRAY;
        assert_eq!(catalog_star_count(stars), NUM_SOLAR_SYSTEMS);
    }

    fn star(x: i16, y: i16) -> CStarDesc {
        CStarDesc {
            star_pt: CPoint { x, y },
            star_type: 0,
            index: 0,
            prefix: 0,
            postfix: 0,
        }
    }

    #[test]
    fn catalog_count_stops_at_the_first_out_of_bounds_coordinate() {
        let mut stars = [star(0, 0); 4];
        stars[1] = star(9999, 9999);
        stars[2] = star(10000, 0);
        stars[3] = star(1, 1);
        assert_eq!(catalog_star_count(&stars), 2);
        stars[2] = star(0, 10000);
        assert_eq!(catalog_star_count(&stars), 2);
        stars[0] = star(0, 10000);
        assert_eq!(catalog_star_count(&stars), 0);
    }

    #[test]
    fn bounds_are_inclusive_and_iteration_resumes_after_last_match() {
        let stars = [star(5, 10), star(8, 10), star(9, 20), star(9999, 32767)];
        let point = CPoint { x: 7, y: 10 };
        assert_eq!(find_star_index(&stars, None, point, 2, 0, 2), Some(0));
        assert_eq!(find_star_index(&stars, Some(0), point, 2, 0, 2), Some(1));
        assert_eq!(find_star_index(&stars, Some(1), point, 2, 0, 2), None);
        assert_eq!(find_star_index(&stars, Some(2), point, -1, -1, 2), None);
        assert_eq!(
            find_star_index(&stars, None, CPoint { x: 9, y: 20 }, 0, 0, 2),
            Some(2)
        );
        assert_eq!(
            find_star_index(&stars, None, CPoint { x: 6, y: 10 }, 0, 0, 2),
            None
        );
    }
}
