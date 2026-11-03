use crate::navigation::abi::{COORD, RECT, SIZE, TEXT};

pub(super) fn split_lines(lines: &mut [TEXT], buffer: &mut [u8]) -> usize {
    let end = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or_else(|| panic!("presentation text requires a terminating NUL"));
    let mut start = 0;
    let mut count = 0;
    for line in lines {
        if start >= end {
            break;
        }
        let term = buffer[start..end]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(end, |offset| start + offset);
        buffer[term] = 0;
        line.pStr = buffer.as_mut_ptr().wrapping_add(start).cast();
        line.CharCount = crate::navigation::abi::COUNT::MAX;
        start = term + 1;
        count += 1;
    }
    count
}

pub(super) fn position_lines(
    lines: &mut [TEXT],
    clip: RECT,
    leading: SIZE,
    alignment: i32,
    width: i32,
) -> RECT {
    let (fade, mut y) = layout(clip, lines.len() as i32, leading, alignment);
    for line in lines {
        line.align = crate::navigation::abi::TEXT_ALIGN_ALIGN_CENTER;
        line.baseline.x = (width / 2) as COORD;
        line.baseline.y = y;
        y = (y as i32 + leading as i32) as COORD;
    }
    fade
}

pub(super) fn layout(mut clip: RECT, count: i32, leading: SIZE, alignment: i32) -> (RECT, COORD) {
    let height = count * leading as i32;
    let first = match alignment {
        84 => leading,
        77 => ((clip.extent.height as i32 - height) / 2) as COORD,
        _ => (clip.extent.height as i32 - height) as COORD,
    };
    clip.corner.y = (clip.corner.y as i32 + first as i32 - leading as i32) as COORD;
    clip.extent.height = ((count + 1) * leading as i32) as COORD;
    (clip, first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::abi::{EXTENT, POINT};

    #[test]
    fn position_lines_updates_baselines_without_changing_the_text() {
        let clip = RECT {
            corner: POINT { x: 4, y: 10 },
            extent: EXTENT {
                width: 160,
                height: 100,
            },
        };
        let mut lines = [TEXT {
            baseline: POINT { x: 0, y: 0 },
            pStr: std::ptr::null(),
            align: 0,
            CharCount: 5,
        }; 2];
        let fade = position_lines(&mut lines, clip, 8, 77, 320);
        assert_eq!((lines[0].baseline.x, lines[0].baseline.y), (160, 42));
        assert_eq!((lines[1].baseline.x, lines[1].baseline.y), (160, 50));
        assert_eq!((fade.corner.y, fade.extent.height), (44, 24));
        assert!(lines
            .iter()
            .all(|line| line.pStr.is_null() && line.CharCount == 5));
    }

    #[test]
    fn vertical_layout_preserves_script_alignment_and_fade_padding() {
        let clip = RECT {
            corner: POINT { x: 4, y: 10 },
            extent: EXTENT {
                width: 160,
                height: 100,
            },
        };
        for (alignment, baseline, top) in [(84, 8, 10), (77, 42, 44), (0, 84, 86)] {
            let (fade, first) = layout(clip, 2, 8, alignment);
            assert_eq!(first, baseline);
            assert_eq!((fade.corner.x, fade.corner.y), (4, top));
            assert_eq!((fade.extent.width, fade.extent.height), (160, 24));
        }
        let (fade, first) = layout(clip, 0, 8, 77);
        assert_eq!(first, 50);
        assert_eq!(fade.extent.height, 8);
    }
}
