use super::*;

impl Walker {
    pub(super) fn finish_allow_items(&mut self, excluded: &BTreeMap<String, metrics::Excluded>) {
        for (item, file, tokens, scopes) in std::mem::take(&mut self.allow_items) {
            let lines = metrics::filtered_lines(tokens, &excluded[&file]);
            for scope in scopes {
                let covered = &self.allow_lines[&scope][&file];
                self.measurement
                    .allow_coverage
                    .push(metrics::AllowCoverage {
                        item: item.clone(),
                        scope,
                        file: file.clone(),
                        lines: lines.intersection(covered).copied().collect(),
                    });
            }
        }
    }

    pub(super) fn finish_allows(&mut self, active_lines: &BTreeMap<String, BTreeSet<usize>>) {
        for (key, files) in std::mem::take(&mut self.allow_lines) {
            let value = files
                .iter()
                .map(|(file, lines)| {
                    lines
                        .iter()
                        .filter(|line| active_lines.get(file).is_some_and(|e| e.contains(line)))
                        .count() as u64
                })
                .sum();
            if let Some(f) = self
                .measurement
                .findings
                .iter_mut()
                .find(|f| f.key() == key)
            {
                f.value = value;
            }
        }
    }
}
