use std::collections::BTreeMap;

pub(crate) type LineHits = BTreeMap<usize, bool>;

pub(super) struct File {
    pub lines: LineHits,
    pub summary: super::Lines,
}

#[derive(Default)]
struct Record {
    file: String,
    lines: LineHits,
    found: Option<u64>,
    hit: Option<u64>,
}

impl Record {
    fn total(slot: &mut Option<u64>, count: &str, name: &str) -> Result<(), String> {
        let count = count.parse::<u64>().map_err(|e| e.to_string())?;
        if slot.replace(count).is_some() {
            return Err(format!("duplicate LCOV {name}"));
        }
        Ok(())
    }

    fn line(&mut self, data: &str) -> Result<(), String> {
        let fields: Vec<_> = data.split(',').collect();
        if !(2..=3).contains(&fields.len()) {
            return Err("invalid LCOV DA entry".into());
        }
        let number = fields[0]
            .parse::<usize>()
            .map_err(|e| format!("LCOV line: {e}"))?;
        let count = fields[1]
            .parse::<u64>()
            .map_err(|e| format!("LCOV count: {e}"))?;
        if number == 0 || self.lines.insert(number, count > 0).is_some() {
            return Err(format!("zero or duplicate LCOV line in {}", self.file));
        }
        Ok(())
    }

    fn finish(self) -> Result<(String, File), String> {
        let summary = super::Lines {
            count: self.found.ok_or("LCOV record is missing LF")?,
            covered: self.hit.ok_or("LCOV record is missing LH")?,
        };
        if summary.covered > summary.count {
            return Err(format!("invalid LCOV line totals: {}", self.file));
        }
        Ok((
            self.file,
            File {
                lines: self.lines,
                summary,
            },
        ))
    }
}

pub(super) fn parse(text: &str) -> Result<BTreeMap<String, File>, String> {
    let mut result = BTreeMap::new();
    let mut record: Option<Record> = None;
    for line in text.lines().filter(|line| !line.is_empty()) {
        if let Some(file) = line.strip_prefix("SF:") {
            if record.is_some() || file.is_empty() {
                return Err("LCOV has an empty or unfinished source record".into());
            }
            record = Some(Record {
                file: file.into(),
                ..Record::default()
            });
        } else if line == "end_of_record" {
            let (file, lines) = record.take().ok_or("LCOV record has no source")?.finish()?;
            if result.insert(file.clone(), lines).is_some() {
                return Err(format!("duplicate LCOV source: {file}"));
            }
        } else if let Some(data) = line.strip_prefix("DA:") {
            record.as_mut().ok_or("LCOV DA has no source")?.line(data)?;
        } else if let Some(count) = line.strip_prefix("LF:") {
            let current = record.as_mut().ok_or("LCOV LF has no source")?;
            Record::total(&mut current.found, count, "LF")?;
        } else if let Some(count) = line.strip_prefix("LH:") {
            let current = record.as_mut().ok_or("LCOV LH has no source")?;
            Record::total(&mut current.hit, count, "LH")?;
        } else if ![
            "TN:", "FN:", "FNDA:", "FNF:", "FNH:", "BRDA:", "BRF:", "BRH:",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
        {
            return Err(format!("unrecognized LCOV entry: {line}"));
        }
    }
    if record.is_some() {
        return Err("unterminated LCOV source record".into());
    }
    if result.is_empty() {
        return Err("LCOV contains no source records".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_closure_summary_can_count_one_physical_line_twice() -> Result<(), String> {
        let report = parse("SF:closure.rs\nFN:2,closure\nFN:1,main\nFNDA:1,closure\nFNDA:1,main\nFNF:2\nFNH:2\nDA:1,1\nDA:2,1\nDA:3,1\nDA:4,1\nLF:5\nLH:5\nend_of_record\n")?;
        let file = &report["closure.rs"];
        assert_eq!(file.lines.len(), 4);
        assert!(file.lines.values().all(|covered| *covered));
        assert_eq!(file.summary.count, 5);
        assert_eq!(file.summary.covered, 5);
        Ok(())
    }

    #[test]
    fn sparse_execution_lines_preserve_covered_and_uncovered_rows() -> Result<(), String> {
        let text = "TN:\nSF:/repo/rust/src/a.rs\nFN:2,8,callback\nFNDA:1,callback\nFNF:1\nFNH:1\nDA:2,1\nDA:5,0\nDA:8,99,checksum\nBRDA:5,0,0,-\nBRF:1\nBRH:0\nLF:3\nLH:2\nend_of_record\n";
        let report = parse(text)?;
        assert_eq!(
            report["/repo/rust/src/a.rs"].lines,
            [(2, true), (5, false), (8, true)].into()
        );
        Ok(())
    }

    #[test]
    fn empty_files_do_not_fabricate_executable_lines() -> Result<(), String> {
        let report = parse("SF:empty.rs\nLF:0\nLH:0\nend_of_record\n")?;
        assert!(report["empty.rs"].lines.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_or_inconsistent_records_fail() {
        for text in [
            "",
            "DA:1,1\n",
            "SF:\nLF:0\nLH:0\nend_of_record\n",
            "SF:a.rs\nDA:1,1\nLF:1\nLH:1\n",
            "SF:a.rs\nDA:1,1\nLF:1\nLH:2\nend_of_record\n",
            "SF:a.rs\nDA:1,1\nLF:2\nend_of_record\n",
            "SF:a.rs\nDA:1,1\nLH:1\nend_of_record\n",
            "SF:a.rs\nDA:0,1\nLF:1\nLH:1\nend_of_record\n",
            "SF:a.rs\nDA:1,-1\nLF:1\nLH:0\nend_of_record\n",
            "SF:a.rs\nDA:1,1\nDA:1,0\nLF:1\nLH:1\nend_of_record\n",
            "SF:a.rs\nDA:1,1,checksum,extra\nLF:1\nLH:1\nend_of_record\n",
            "SF:a.rs\nLF:0\nLH:0\nend_of_record\nSF:a.rs\nLF:0\nLH:0\nend_of_record\n",
            "SF:a.rs\nLF:0\nLH:0\nLF:0\nend_of_record\n",
            "SF:a.rs\nSF:b.rs\n",
            "end_of_record\n",
        ] {
            assert!(parse(text).is_err(), "accepted {text:?}");
        }
    }
}
