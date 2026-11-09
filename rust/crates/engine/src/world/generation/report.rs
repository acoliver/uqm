//! The report text and cursor retained by one solar-system session.

use crate::strings::owned_table::TextEntry;

use super::ReportDisposition;

#[derive(Debug, Default)]
pub struct ReportSlot {
    text: Option<TextEntry>,
}

impl ReportSlot {
    pub fn new(text: Option<TextEntry>) -> Self {
        Self { text }
    }

    pub fn current(&self) -> Option<&TextEntry> {
        self.text.as_ref()
    }

    pub fn clear(&mut self) {
        self.text = None;
    }

    pub fn present(
        &mut self,
        disposition: ReportDisposition,
        present: impl FnOnce(&TextEntry),
    ) -> bool {
        let Some(text) = &self.text else {
            return false;
        };
        present(text);
        self.text = match disposition {
            ReportDisposition::Once => None,
            ReportDisposition::Cycle => {
                let next = text.next_entry();
                (next.index() != 0).then_some(next)
            }
        };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{strings::owned_table::LoadedTextTable, world::generation::ReportDisposition};

    fn report(first: u16) -> ReportSlot {
        let table = LoadedTextTable::fixture(vec![
            c"first".to_bytes_with_nul().to_vec(),
            c"second".to_bytes_with_nul().to_vec(),
            c"third".to_bytes_with_nul().to_vec(),
        ]);
        ReportSlot::new(table.entry(first))
    }

    #[test]
    fn empty_report_does_not_present_or_advance() {
        let mut slot = ReportSlot::default();
        assert!(!slot.present(ReportDisposition::Once, |_| panic!("empty report")));
        assert!(slot.current().is_none());
    }

    #[test]
    fn once_consumes_only_its_own_slot_after_the_presenter_returns() {
        let mut first = report(1);
        let mut second = report(0);
        let mut observed = Vec::new();
        assert!(first.present(ReportDisposition::Once, |text| {
            observed.push(text.label(1).0.to_owned());
            assert_eq!(text.index(), 1);
        }));
        assert_eq!(observed, [c"second"]);
        assert!(first.current().is_none());
        assert_eq!(second.current().unwrap().index(), 0);
        assert!(second.present(ReportDisposition::Once, |_| {}));
    }

    #[test]
    fn cycle_clears_at_absolute_zero_not_at_its_initial_entry() {
        let mut slot = report(1);
        let mut indexes = Vec::new();
        while slot.present(ReportDisposition::Cycle, |text| indexes.push(text.index())) {}
        assert_eq!(indexes, [1, 2]);
        assert!(slot.current().is_none());
    }

    #[test]
    fn borrowed_text_retains_its_table_after_the_slot_is_cleared() {
        let mut slot = report(2);
        let retained = slot.current().unwrap().clone();
        slot.clear();
        assert!(slot.current().is_none());
        assert_eq!(retained.label(2).0, c"third");
    }

    #[test]
    fn failed_presentation_does_not_consume_the_report() {
        let mut slot = report(1);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            slot.present(ReportDisposition::Once, |_| panic!("presenter failed"));
        }));
        assert!(result.is_err());
        assert_eq!(slot.current().unwrap().index(), 1);
    }
}
