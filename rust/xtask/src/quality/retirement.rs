use std::collections::{BTreeMap, BTreeSet};

use super::{budgets::Budget, metrics::Measurement, transfers};

type Budgets = BTreeMap<String, Budget>;

pub struct TransferBase {
    pub measurement: Measurement,
    pub budgets: Budgets,
    pub retired: Budgets,
}

impl TransferBase {
    pub fn limits(
        &self,
        entries: &BTreeMap<String, u64>,
        current: &Measurement,
        moves: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<String, u64>, String> {
        transfers::limits(entries, current, &self.measurement, moves, &self.budgets)
    }
}

pub fn transfer_base(
    base: &Measurement,
    current: &Measurement,
    recorded: &Budgets,
    held: &Budgets,
    moves: &BTreeMap<String, String>,
) -> Result<TransferBase, String> {
    let retired: Budgets = recorded
        .iter()
        .filter(|(name, _)| !held.contains_key(*name))
        .map(|(name, budget)| (name.clone(), budget.clone()))
        .collect();
    for (name, budget) in &retired {
        let scopes = budget
            .scopes
            .keys()
            .map(|key| relocated(key, moves))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let remaining: u64 = current
            .findings
            .iter()
            .filter(|finding| {
                if budget.scopes.is_empty() {
                    finding.metric == *name
                } else {
                    scopes.contains(&finding.key())
                }
            })
            .map(|finding| finding.value)
            .sum();
        if remaining != 0 {
            return Err(format!(
                "cannot retire {name}: {remaining} remaining sites after declaration moves"
            ));
        }
    }
    let mut measurement = base.clone();
    // Retired debt supplies no declaration or subsystem allowance at its destination.
    measurement.findings.retain(|finding| {
        !retired.iter().any(|(name, budget)| {
            if budget.scopes.is_empty() {
                finding.metric == *name
            } else {
                budget.scopes.contains_key(&finding.key())
            }
        })
    });
    let budgets = recorded
        .iter()
        .filter(|(name, _)| held.contains_key(*name))
        .map(|(name, budget)| (name.clone(), budget.clone()))
        .collect();
    Ok(TransferBase {
        measurement,
        budgets,
        retired,
    })
}

fn relocated(key: &str, moves: &BTreeMap<String, String>) -> Result<String, String> {
    let (metric, identity) = key
        .rsplit_once('|')
        .ok_or("missing budget scope identity")?;
    Ok(match transfers::matching(moves, identity) {
        Some((old, new)) => format!("{metric}|{new}{}", &identity[old.len()..]),
        None => key.into(),
    })
}
