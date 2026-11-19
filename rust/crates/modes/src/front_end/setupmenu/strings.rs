//! Owned setup text and bounded parsing of the external string table.
use std::ffi::{CStr, CString};
use uqm_engine::strings::owned_table::LoadedTextTable;

pub(super) struct ChoiceText {
    pub category: CString,
    pub options: Vec<OptionText>,
}
pub(super) struct OptionText {
    pub name: CString,
    pub tips: [CString; 3],
}
pub(super) struct EntryText {
    pub name: CString,
    pub tips: [CString; 3],
}
pub(super) struct SetupStrings {
    pub title: CString,
    pub subtitles: [CString; 8],
    pub choices: [ChoiceText; 24],
    pub sliders: [EntryText; 4],
    pub buttons: [EntryText; 10],
    pub labels: [Vec<CString>; 4],
    pub text_category: CString,
    pub control_categories: [CString; 7],
}

pub(super) fn split_lines(text: &CStr, limit: usize) -> Vec<CString> {
    text.to_bytes()
        .splitn(limit, |byte| *byte == b'\n')
        .map(|line| {
            CString::new(line).unwrap_or_else(|_| unreachable!("CStr contains no interior NUL"))
        })
        .collect()
}

struct Reader<'a> {
    table: &'a LoadedTextTable,
    index: usize,
}
impl Reader<'_> {
    fn next(&mut self) -> Vec<CString> {
        assert!(
            self.index < self.table.len(),
            "setup string table cut short at {}",
            self.index
        );
        let text = self.table.text(self.index).unwrap_or(c"");
        self.index += 1;
        split_lines(text, 100)
    }

    fn names<const N: usize>(&mut self) -> [CString; N] {
        self.next().try_into().unwrap_or_else(|rows: Vec<CString>| {
            panic!("expected {N} setup names, found {}", rows.len())
        })
    }

    fn tips(&mut self) -> [CString; 3] {
        let mut lines = self.next().into_iter();
        std::array::from_fn(|_| lines.next().unwrap_or_default())
    }

    fn entries<const N: usize>(&mut self) -> [EntryText; N] {
        let names = self.names::<N>();
        names.map(|name| EntryText {
            name,
            tips: self.tips(),
        })
    }
}

impl SetupStrings {
    pub(super) fn parse(table: &LoadedTextTable) -> Self {
        assert!(table.len() >= 3, "setup string table has no indices");
        let title = table.text(0).unwrap_or(c"").to_owned();
        let mut reader = Reader { table, index: 1 };
        let subtitles = reader.names();
        let categories = reader.names::<24>();
        let choices = categories.map(|category| {
            let names = reader.next();
            assert!(!names.is_empty(), "setup choice has no options");
            let options = names
                .into_iter()
                .map(|name| OptionText {
                    name,
                    tips: reader.tips(),
                })
                .collect();
            ChoiceText { category, options }
        });
        let sliders = reader.entries();
        let buttons = reader.entries();
        let _label_names = reader.names::<4>();
        let labels = std::array::from_fn(|_| reader.next());
        let [text_category] = reader.names::<1>();
        let _initial_text = reader.names::<1>();
        let control_categories = reader.names();
        if reader.index < table.len() {
            log::warn!(
                "Setup strings had {} garbage entries at the end.",
                table.len() - reader.index
            );
        }
        Self {
            title,
            subtitles,
            choices,
            sliders,
            buttons,
            labels,
            text_category,
            control_categories,
        }
    }
}
