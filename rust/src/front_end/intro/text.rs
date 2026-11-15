use crate::graphics::native_abi::{POINT, RECT};
use std::ffi::CString;

pub(in crate::front_end) fn owned_lines(bytes: &[u8], capacity: usize) -> Vec<CString> {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    let mut bytes = &bytes[..end];
    let mut lines = Vec::new();
    while !bytes.is_empty() && lines.len() < capacity {
        let end = bytes
            .iter()
            .position(|&byte| byte == b'\n')
            .unwrap_or(bytes.len());
        lines.push(
            CString::new(&bytes[..end])
                .unwrap_or_else(|error| panic!("bounded presentation text contains NUL: {error}")),
        );
        bytes = &bytes[(end + 1).min(bytes.len())..];
    }
    lines
}

pub(super) struct TextLine {
    pub text: CString,
    pub baseline: POINT,
}

pub(super) fn position_lines(
    lines: &mut [TextLine],
    clip: RECT,
    leading: i16,
    alignment: i32,
    width: i32,
) -> RECT {
    let (fade, mut y) = layout(clip, lines.len() as i32, leading, alignment);
    for line in lines {
        line.baseline = POINT {
            x: (width / 2) as i16,
            y,
        };
        y = (i32::from(y) + i32::from(leading)) as i16;
    }
    fade
}

pub(super) fn layout(mut clip: RECT, count: i32, leading: i16, alignment: i32) -> (RECT, i16) {
    let height = count * i32::from(leading);
    let first = match alignment {
        84 => leading,
        77 => ((i32::from(clip.extent.height) - height) / 2) as i16,
        _ => (i32::from(clip.extent.height) - height) as i16,
    };
    clip.corner.y = (i32::from(clip.corner.y) + i32::from(first) - i32::from(leading)) as i16;
    clip.extent.height = ((count + 1) * i32::from(leading)) as i16;
    (clip, first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::native_abi::EXTENT;

    #[test]
    fn position_lines_updates_baselines_without_changing_the_text() {
        let clip = RECT {
            corner: POINT { x: 4, y: 10 },
            extent: EXTENT {
                width: 160,
                height: 100,
            },
        };
        let mut lines: [_; 2] = std::array::from_fn(|_| TextLine {
            text: c"hello".to_owned(),
            baseline: POINT { x: 0, y: 0 },
        });
        let fade = position_lines(&mut lines, clip, 8, 77, 320);
        assert_eq!((lines[0].baseline.x, lines[0].baseline.y), (160, 42));
        assert_eq!((lines[1].baseline.x, lines[1].baseline.y), (160, 50));
        assert_eq!((fade.corner.y, fade.extent.height), (44, 24));
        assert!(lines.iter().all(|line| line.text.as_c_str() == c"hello"));
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
