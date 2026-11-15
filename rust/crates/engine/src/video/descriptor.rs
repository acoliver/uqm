use std::ffi::{c_long, CString};

pub(super) struct Descriptor {
    pub video: CString,
    pub audio: Option<CString>,
    pub speech: Option<CString>,
    pub loop_frame: u32,
}

fn terminated(bytes: &[u8]) -> CString {
    CString::new(bytes)
        .unwrap_or_else(|error| panic!("terminated video descriptor contains NUL: {error}"))
}

fn components(path: &[u8]) -> Vec<&[u8]> {
    path[..path.len().min(1023)]
        .splitn(4, |&c| c == b':')
        .collect()
}

impl Descriptor {
    pub(super) fn parse(path: &[u8]) -> Self {
        let parts = components(path);
        Self {
            video: terminated(parts[0]),
            audio: parts.get(1).map(|bytes| terminated(bytes)),
            speech: parts.get(2).map(|bytes| terminated(bytes)),
            loop_frame: parts.get(3).map_or(u32::MAX, |bytes| loop_frame(bytes)),
        }
    }
}

fn loop_frame(bytes: &[u8]) -> u32 {
    let whitespace = bytes
        .iter()
        .take_while(|&&byte| matches!(byte, 9..=13 | 32))
        .count();
    let mut end = whitespace;
    let negative = bytes.get(end) == Some(&b'-');
    if matches!(bytes.get(end), Some(b'+' | b'-')) {
        end += 1;
    }
    let start = end;
    let bound = if negative {
        c_long::MAX as u64 + 1
    } else {
        c_long::MAX as u64
    };
    let mut value = 0u64;
    while let Some(byte @ b'0'..=b'9') = bytes.get(end) {
        value = value
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'))
            .min(bound);
        end += 1;
    }
    if end == start {
        end = 0;
    }
    // strtol leaves end at the entry start if it consumes no digits.
    let next = bytes.get(end).copied().unwrap_or(0) as std::ffi::c_char;
    if next > 32 {
        return u32::MAX;
    }
    if negative {
        value.wrapping_neg() as u32
    } else {
        value as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_components_preserve_empty_fields_and_limit_splitting() {
        assert_eq!(components(b"video.duk"), vec![b"video.duk".as_slice()]);
        assert_eq!(
            components(b"v::speech:12:extra"),
            vec![b"v".as_slice(), b"", b"speech", b"12:extra"]
        );
        assert_eq!(components(&[b'x'; 1024])[0].len(), 1023);
    }

    #[test]
    fn loop_numbers_saturate_like_long_and_keep_empty_scan_rules() {
        let limit = c_long::MAX as u64;
        assert_eq!(loop_frame((limit + 1).to_string().as_bytes()), limit as u32);
        assert_eq!(
            loop_frame(format!("-{}", limit + 2).as_bytes()),
            (limit + 1).wrapping_neg() as u32
        );
        assert_eq!(loop_frame(b"  invalid"), 0);
        assert_eq!(loop_frame(b"+"), u32::MAX);
        assert_eq!(loop_frame(b"invalid"), u32::MAX);
        assert_eq!(loop_frame(b"-0 rest"), 0);
        assert_eq!(
            loop_frame(b"99999999999999999999999999999999999999"),
            limit as u32
        );
    }

    #[test]
    fn resource_loop_parsing_matches_strtol_and_legacy_whitespace_rule() {
        for (path, expected) in [
            (c"v", u32::MAX),
            (c"v:::12", 12),
            (c"v:::-2", u32::MAX - 1),
            (c"v:::12x", u32::MAX),
            (c"v:::12 x", 12),
            (c"v:::", 0),
        ] {
            let descriptor = Descriptor::parse(path.to_bytes());
            assert_eq!(descriptor.loop_frame, expected);
            assert_eq!(descriptor.video.as_c_str(), c"v");
        }
    }
}
