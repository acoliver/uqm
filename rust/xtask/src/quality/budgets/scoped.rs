use std::collections::BTreeMap;

use super::{Budget, Measurement};

pub(super) fn parse(
    name: &str,
    value: Option<&toml_edit::Item>,
    limit: u64,
) -> Result<BTreeMap<String, u64>, String> {
    let mut scopes = BTreeMap::new();
    if let Some(value) = value {
        let table = value
            .as_inline_table()
            .ok_or("budget scopes must be a table")?;
        for (key, value) in table {
            let value = value
                .as_integer()
                .and_then(|n| u64::try_from(n).ok())
                .ok_or_else(|| format!("invalid scope limit: {key}"))?;
            scopes.insert(key.into(), value);
        }
    }
    validate(name, &scopes, limit)?;
    Ok(scopes)
}

pub(super) fn validate(
    name: &str,
    scopes: &BTreeMap<String, u64>,
    limit: u64,
) -> Result<(), String> {
    if name != "stage_b_raw" {
        return if scopes.is_empty() {
            Ok(())
        } else {
            Err("only stage_b_raw accepts identity scopes".into())
        };
    }
    if scopes.is_empty() {
        return Err("stage_b_raw requires exact raw identity scopes".into());
    }
    let mut sum = 0u64;
    for (key, value) in scopes {
        let fields: Vec<_> = key.split('|').collect();
        if fields.len() != 3
            || fields[0] != "ffi"
            || !["raw_records", "raw_pointer_parameters"].contains(&fields[1])
            || !fields[2].contains("::")
            || fields[2].contains('*')
        {
            return Err(format!("invalid stage_b_raw identity: {key}"));
        }
        sum = sum.checked_add(*value).ok_or("scope limits overflow")?;
    }
    if sum != limit {
        return Err(format!(
            "stage_b_raw limit {limit} must equal scope limits {sum}"
        ));
    }
    Ok(())
}

pub(super) fn write(scopes: &BTreeMap<String, u64>) -> Result<toml_edit::Value, String> {
    let mut table = toml_edit::InlineTable::new();
    for (key, value) in scopes {
        table.insert(
            key,
            i64::try_from(*value).map_err(|e| e.to_string())?.into(),
        );
    }
    Ok(table.into())
}

pub(super) fn expanded(current: &BTreeMap<String, u64>, base: &BTreeMap<String, u64>) -> bool {
    current
        .iter()
        .any(|(key, value)| !base.contains_key(key) || *value > base[key])
}

pub(super) fn check(measured: &Measurement, budget: &Budget) -> Result<(), String> {
    for (key, limit) in &budget.scopes {
        let value: u64 = measured
            .findings
            .iter()
            .filter(|f| f.key() == *key)
            .map(|f| f.value)
            .sum();
        if value > *limit {
            return Err(format!(
                "budget stage_b_raw {key}: limit {limit}, measured {value}"
            ));
        }
    }
    Ok(())
}
