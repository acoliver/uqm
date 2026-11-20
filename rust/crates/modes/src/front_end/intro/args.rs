pub(in crate::front_end) struct Arguments<'a> {
    bytes: &'a [u8],
}

impl<'a> Arguments<'a> {
    pub(in crate::front_end) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub(in crate::front_end) fn remaining(&self) -> &'a [u8] {
        self.bytes
    }

    pub(in crate::front_end) fn whitespace(&mut self) {
        let count = self
            .bytes
            .iter()
            .take_while(|&&byte| matches!(byte, 9..=13 | 32))
            .count();
        self.bytes = &self.bytes[count..];
    }

    pub(in crate::front_end) fn word(&mut self, limit: usize) -> Option<&'a [u8]> {
        self.whitespace();
        let count = self
            .bytes
            .iter()
            .take(limit)
            .take_while(|&&byte| !matches!(byte, 9..=13 | 32))
            .count();
        let word = &self.bytes[..count];
        self.bytes = &self.bytes[count..];
        (!word.is_empty()).then_some(word)
    }

    pub(in crate::front_end) fn integer(&mut self) -> Option<i32> {
        self.whitespace();
        let sign = usize::from(matches!(self.bytes.first(), Some(b'+' | b'-')));
        let digits = self.bytes[sign..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits == 0 {
            return None;
        }
        let end = sign + digits;
        let value = std::str::from_utf8(&self.bytes[..end]).ok()?.parse().ok()?;
        self.bytes = &self.bytes[end..];
        Some(value)
    }

    pub(in crate::front_end) fn line(&mut self, limit: usize) -> Option<&'a [u8]> {
        self.whitespace();
        let count = self
            .bytes
            .iter()
            .take(limit)
            .take_while(|&&byte| byte != b'\n')
            .count();
        let line = &self.bytes[..count];
        self.bytes = &self.bytes[count..];
        (!line.is_empty()).then_some(line)
    }
}

pub(in crate::front_end) fn rgb(bytes: &[u8]) -> Option<crate::graphics::geometry::Color> {
    let mut args = Arguments::new(bytes);
    args.whitespace();
    let negative = args.bytes.first() == Some(&b'-');
    if matches!(args.bytes.first(), Some(b'+' | b'-')) {
        args.bytes = &args.bytes[1..];
    }
    if args.bytes.starts_with(b"0x") || args.bytes.starts_with(b"0X") {
        args.bytes = &args.bytes[2..];
    }
    let mut value = 0u32;
    let mut count = 0;
    for &byte in args.bytes {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => break,
        };
        value = value.wrapping_mul(16).wrapping_add(u32::from(digit));
        count += 1;
    }
    if count == 0 {
        return None;
    }
    if negative {
        value = value.wrapping_neg();
    }
    Some(crate::graphics::geometry::Color {
        r: (value >> 16) as u8,
        g: (value >> 8) as u8,
        b: value as u8,
        a: 0,
    })
}
