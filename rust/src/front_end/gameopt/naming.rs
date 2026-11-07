//! Byte-bounded labels used by the save-name editor.
use std::ffi::{CStr, CString};

pub(super) struct SaveLabel {
    pub text: CString,
    pub cursor_prefix: usize,
}

pub(super) fn save_label(date: &CStr, name: &CStr) -> SaveLabel {
    let mut bytes = date.to_bytes().iter().take(79).copied().collect::<Vec<_>>();
    bytes.extend(b": ".iter().take(79 - bytes.len()));
    let cursor_prefix = bytes.len() - 1;
    bytes.extend(name.to_bytes());
    bytes.truncate(255);
    SaveLabel {
        text: CString::new(bytes).unwrap_or_else(|error| panic!("invalid save label: {error}")),
        cursor_prefix,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_labels_preserve_the_date_prefix_and_byte_buffer_boundaries() {
        let label = save_label(c"November 07·2155", c"Earth");
        assert_eq!(label.text.to_bytes(), "November 07·2155: Earth".as_bytes());
        assert_eq!(label.cursor_prefix, "November 07·2155: ".len() - 1);
        let date = CString::new(format!("{} 07·2155", "é".repeat(40)))
            .unwrap_or_else(|error| panic!("{error}"));
        let bounded = save_label(&date, c"B");
        assert_eq!(bounded.cursor_prefix, 78);
        assert_eq!(bounded.text.to_bytes().len(), 80);
        assert_eq!(&bounded.text.to_bytes()[78..], &[0xc3, b'B']);
        let name = CString::new("A".repeat(300)).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            save_label(c"November 07·2155", &name).text.to_bytes().len(),
            255
        );
    }
}
