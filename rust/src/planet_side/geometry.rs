//! Pixel-mask collision geometry independent of the legacy C collision ABI.

use super::model::SurfacePoint;
use crate::graphics::tfb_draw::{Canvas, CanvasPixelFormat, TFImage};

/// Owned 1-bit collision mask with a signed hotspot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionMask {
    width: u16,
    height: u16,
    hotspot: SurfacePoint,
    bits: Vec<u8>,
}

impl CollisionMask {
    /// Build a mask from row-major occupancy bytes (`0` transparent, nonzero opaque).
    pub fn from_occupancy(
        width: u16,
        height: u16,
        hotspot: SurfacePoint,
        occupancy: &[u8],
    ) -> Result<Self, GeometryError> {
        let pixel_count = usize::from(width) * usize::from(height);
        if occupancy.len() != pixel_count {
            return Err(GeometryError::InvalidPixelCount {
                expected: pixel_count,
                actual: occupancy.len(),
            });
        }
        let mut bits = vec![0; pixel_count.div_ceil(8)];
        for (index, value) in occupancy.iter().enumerate() {
            if *value != 0 {
                bits[index / 8] |= 1 << (index % 8);
            }
        }
        Ok(Self {
            width,
            height,
            hotspot,
            bits,
        })
    }

    #[must_use]
    pub fn is_opaque(&self, x: u16, y: u16) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let index = usize::from(y) * usize::from(self.width) + usize::from(x);
        self.bits[index / 8] & (1 << (index % 8)) != 0
    }

    /// Extract opacity from a Rust graphics canvas.
    pub fn from_canvas(canvas: &Canvas, hotspot: SurfacePoint) -> Result<Self, GeometryError> {
        let extent = canvas.extent();
        let width = u16::try_from(extent.width).map_err(|_| GeometryError::InvalidExtent)?;
        let height = u16::try_from(extent.height).map_err(|_| GeometryError::InvalidExtent)?;
        if width == 0 || height == 0 {
            return Err(GeometryError::InvalidExtent);
        }
        let pixels = canvas.pixels();
        let format = canvas.format();
        let occupancy = match format.kind {
            CanvasPixelFormat::Rgba => pixels
                .chunks_exact(format.bytes_per_pixel as usize)
                .map(|pixel| u8::from(pixel.get(3).copied().unwrap_or(0) != 0))
                .collect::<Vec<_>>(),
            CanvasPixelFormat::Paletted => {
                let transparent = canvas.transparent_index();
                let palette = canvas.palette();
                pixels
                    .iter()
                    .map(|index| {
                        let visible_index = transparent != Some(*index);
                        let visible_alpha = palette
                            .as_ref()
                            .map(|colors| colors[usize::from(*index)].a != 0)
                            .unwrap_or(true);
                        u8::from(visible_index && visible_alpha)
                    })
                    .collect::<Vec<_>>()
            }
            CanvasPixelFormat::Rgb => vec![1; usize::from(width) * usize::from(height)],
        };
        Self::from_occupancy(width, height, hotspot, &occupancy)
    }

    /// Extract the normal canvas and hotspot from a Rust `TFImage`.
    pub fn from_image(image: &TFImage) -> Result<Self, GeometryError> {
        let canvas = image.normal().ok_or(GeometryError::MissingCanvas)?;
        let hotspot = image.normal_hot_spot();
        Self::from_canvas(
            &canvas,
            SurfacePoint {
                x: hotspot.x,
                y: hotspot.y,
            },
        )
    }

    #[must_use]
    pub const fn width(&self) -> u16 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u16 {
        self.height
    }
}

/// Collision-mask construction error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryError {
    InvalidPixelCount { expected: usize, actual: usize },
    InvalidExtent,
    MissingCanvas,
}

/// True when two opaque pixels overlap after applying each mask's hotspot.
#[must_use]
pub fn masks_intersect(
    left_position: SurfacePoint,
    left: &CollisionMask,
    right_position: SurfacePoint,
    right: &CollisionMask,
) -> bool {
    let left_x = left_position.x - left.hotspot.x;
    let left_y = left_position.y - left.hotspot.y;
    let right_x = right_position.x - right.hotspot.x;
    let right_y = right_position.y - right.hotspot.y;

    let overlap_left = left_x.max(right_x);
    let overlap_top = left_y.max(right_y);
    let overlap_right = (left_x + i32::from(left.width)).min(right_x + i32::from(right.width));
    let overlap_bottom = (left_y + i32::from(left.height)).min(right_y + i32::from(right.height));
    if overlap_left >= overlap_right || overlap_top >= overlap_bottom {
        return false;
    }

    for world_y in overlap_top..overlap_bottom {
        for world_x in overlap_left..overlap_right {
            let left_local_x = (world_x - left_x) as u16;
            let left_local_y = (world_y - left_y) as u16;
            let right_local_x = (world_x - right_x) as u16;
            let right_local_y = (world_y - right_y) as u16;
            if left.is_opaque(left_local_x, left_local_y)
                && right.is_opaque(right_local_x, right_local_y)
            {
                return true;
            }
        }
    }
    false
}

#[must_use]
pub fn masks_intersect_wrapped(
    left_position: SurfacePoint,
    left: &CollisionMask,
    right_position: SurfacePoint,
    right: &CollisionMask,
    world_width: i32,
) -> bool {
    // BuildObjectList offsets every entity by the shortest wrapped horizontal
    // displacement, so an overlap that visibly crosses the seam collides exactly
    // where it is drawn.  Raw world coordinates alone miss it.
    for right_x in [
        right_position.x,
        right_position.x - world_width,
        right_position.x + world_width,
    ] {
        for left_x in [
            left_position.x,
            left_position.x - world_width,
            left_position.x + world_width,
        ] {
            if masks_intersect(
                SurfacePoint {
                    x: left_x,
                    y: left_position.y,
                },
                left,
                SurfacePoint {
                    x: right_x,
                    y: right_position.y,
                },
                right,
            ) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(width: u16, height: u16, pixels: &[u8]) -> CollisionMask {
        CollisionMask::from_occupancy(width, height, SurfacePoint::default(), pixels).unwrap()
    }

    #[test]
    fn rejects_malformed_pixel_data() {
        assert_eq!(
            CollisionMask::from_occupancy(2, 2, SurfacePoint::default(), &[1]),
            Err(GeometryError::InvalidPixelCount {
                expected: 4,
                actual: 1
            })
        );
    }

    #[test]
    fn bounding_boxes_can_overlap_without_pixel_collision() {
        let left = mask(2, 2, &[1, 0, 0, 0]);
        let right = mask(2, 2, &[0, 0, 0, 1]);
        assert!(!masks_intersect(
            SurfacePoint::default(),
            &left,
            SurfacePoint::default(),
            &right
        ));
    }

    #[test]
    fn one_opaque_world_pixel_is_a_collision() {
        let left = mask(2, 2, &[0, 0, 0, 1]);
        let right = mask(1, 1, &[1]);
        assert!(masks_intersect(
            SurfacePoint::default(),
            &left,
            SurfacePoint { x: 1, y: 1 },
            &right
        ));
    }

    #[test]
    fn hotspots_shift_mask_origins() {
        let left = CollisionMask::from_occupancy(1, 1, SurfacePoint { x: 1, y: 1 }, &[1]).unwrap();
        let right = mask(1, 1, &[1]);
        assert!(masks_intersect(
            SurfacePoint { x: 1, y: 1 },
            &left,
            SurfacePoint::default(),
            &right
        ));
    }

    #[test]
    fn edge_touch_without_shared_pixel_is_not_collision() {
        let solid = mask(2, 2, &[1; 4]);
        assert!(!masks_intersect(
            SurfacePoint::default(),
            &solid,
            SurfacePoint { x: 2, y: 0 },
            &solid
        ));
    }

    #[test]
    fn wide_deposit_crossing_right_seam_collides_with_lander_near_left() {
        // A 3px deposit at world right edge covers ring pixels {W-1, 0, 1}.
        // The lander at world x=1 sits on ring pixel 1: visible overlap.
        let lander = mask(1, 1, &[1]);
        let deposit = mask(3, 1, &[1; 3]);
        assert!(masks_intersect_wrapped(
            SurfacePoint { x: 1, y: 0 },
            &lander,
            SurfacePoint { x: 99, y: 0 },
            &deposit,
            100,
        ));
        // Opposite argument order still collides at the same drawn pixel.
        assert!(masks_intersect_wrapped(
            SurfacePoint { x: 99, y: 0 },
            &deposit,
            SurfacePoint { x: 1, y: 0 },
            &lander,
            100,
        ));
    }

    #[test]
    fn wide_deposit_hotspot_left_of_origin_crosses_seam_only_wrapped() {
        // A 3px opaque deposit with hotspot (2,0) parked at raw x=0 spans
        // ring pixels {-2, -1, 0}: its raw extent ends one pixel left of the
        // lander at x=99, so raw coordinates alone never overlap.  Only the
        // +WORLD copy reaches the right edge and closes the seam, and the same
        // hit holds with the argument order reversed.
        let lander = mask(1, 1, &[1]);
        let deposit =
            CollisionMask::from_occupancy(3, 1, SurfacePoint { x: 2, y: 0 }, &[1; 3]).unwrap();
        assert!(!masks_intersect(
            SurfacePoint { x: 0, y: 0 },
            &deposit,
            SurfacePoint { x: 99, y: 0 },
            &lander
        ));
        assert!(masks_intersect_wrapped(
            SurfacePoint { x: 99, y: 0 },
            &lander,
            SurfacePoint { x: 0, y: 0 },
            &deposit,
            100,
        ));
        // Opposite argument order still collides at the same drawn pixel.
        assert!(masks_intersect_wrapped(
            SurfacePoint { x: 0, y: 0 },
            &deposit,
            SurfacePoint { x: 99, y: 0 },
            &lander,
            100,
        ));
    }

    #[test]
    fn rgba_canvas_uses_alpha_for_occupancy() {
        let mut canvas = Canvas::new_rgba(2, 1);
        canvas
            .with_pixels_mut(|pixels| {
                pixels.copy_from_slice(&[255, 0, 0, 0, 0, 0, 0, 255]);
            })
            .unwrap();
        let mask = CollisionMask::from_canvas(&canvas, SurfacePoint::default()).unwrap();
        assert!(!mask.is_opaque(0, 0));
        assert!(mask.is_opaque(1, 0));
    }

    #[test]

    fn image_extraction_preserves_hotspot() {
        let image = TFImage::new_rgba(1, 1);
        image.set_normal_hot_spot(crate::graphics::tfb_draw::HotSpot::new(2, 3));
        let image_mask = CollisionMask::from_image(&image).unwrap();
        let other = mask(1, 1, &[1]);
        assert!(!masks_intersect(
            SurfacePoint { x: 2, y: 3 },
            &image_mask,
            SurfacePoint::default(),
            &other
        ));
    }
}
