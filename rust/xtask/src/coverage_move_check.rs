use super::{
    capture,
    lcov::LineHits,
    moves::{self, Attribution, Debt},
    source, Lines,
};
use crate::quality::{
    baseline::{self, Baseline},
    metrics::Measurement,
    transfers,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

type Hits = BTreeMap<String, LineHits>;

fn active_owner(unit: &str, current: &Measurement) -> Result<(), String> {
    moves::owner(unit)?;
    let scopes: &[&str] = match unit {
        "U5-02" => &["uqm::planets::cargo::live"],
        "U6-03" => &[
            "uqm::ui::menu",
            "uqm::ui::util::live",
            "uqm::ui::starbase::cffi",
        ],
        "U6-04" => &[
            "uqm::mainloop::input_abi::live",
            "uqm::time::clock_abi::live",
            "uqm::navigation::events::live",
        ],
        _ => unreachable!(),
    };
    if !current.findings.iter().any(|f| {
        f.metric == "cfg_not_test_modules" && f.value > 0 && scopes.contains(&f.identity.as_str())
    }) {
        return Err(format!(
            "remove coverage debt: {unit}'s foreign test exclusions have retired"
        ));
    }
    Ok(())
}

fn source_text(
    root: &Path,
    reference: &str,
    function: &source::Function,
) -> Result<String, String> {
    let text = capture::git(
        root,
        &["show", &format!("{reference}:rust/{}", function.file)],
    )?;
    function.slice(&text)
}

fn validate_sources(
    root: &Path,
    reference: &str,
    baseline: &Baseline,
    prior: &Baseline,
    current: &Measurement,
    hits: &Hits,
    host: &str,
) -> Result<(), String> {
    let Some(sources) = baseline.coverage_moves.sources.get(host) else {
        return Ok(());
    };
    let commits: BTreeSet<_> = sources.values().map(|proof| &proof.commit).collect();
    for commit in commits {
        if capture::git(root, &["merge-base", reference, commit])?.trim() != commit {
            return Err(format!(
                "coverage source commit must be an ancestor of the comparison base: {commit}"
            ));
        }
    }
    for (identity, proof) in sources {
        proof.validate()?;
        if current
            .coverage_functions
            .iter()
            .any(|f| f.identity == *identity)
        {
            let function = source::unique(&current.coverage_functions, identity)?;
            if function.proof(root, hits)? != proof.resolve(root)? {
                return Err(format!(
                    "refresh measured {host} coverage source proof for {identity}"
                ));
            }
        } else if prior
            .coverage_moves
            .sources
            .get(host)
            .and_then(|p| p.get(identity))
            != Some(proof)
        {
            return Err(format!(
                "coverage move requires an unchanged archived source proof: {identity}"
            ));
        }
    }
    Ok(())
}

fn pending_sources(
    root: &Path,
    reference: &str,
    baseline: &Baseline,
    prior: &Baseline,
    current: &Measurement,
    host: &str,
) -> Result<BTreeMap<String, String>, String> {
    let pending = transfers::pending(&baseline.moved, Some(&prior.moved));
    if pending.is_empty() {
        return Ok(BTreeMap::new());
    }
    let (before, _) = baseline::archived_measurement(root, reference)?;
    transfers::validate(&prior.entries, current, &before, &pending)?;
    let mut targets = BTreeMap::new();
    for (identity, proof) in prior.coverage_moves.sources.get(host).into_iter().flatten() {
        let Some((old, new)) = transfers::matching(&pending, identity) else {
            continue;
        };
        let target = format!("{new}{}", &identity[old.len()..]);
        if !current
            .coverage_functions
            .iter()
            .any(|f| f.identity == target)
        {
            continue;
        }
        let function = source::unique(&before.coverage_functions, identity)?;
        let proof = proof.resolve(root)?;
        if function.file != proof.file
            || function.start.0 != proof.first
            || source_text(root, reference, function)? != proof.text
        {
            return Err(format!(
                "coverage source proof differs from base declaration: {identity}"
            ));
        }
        if targets.insert(target.clone(), identity.clone()).is_some() {
            return Err(format!("duplicate coverage target: {target}"));
        }
    }
    Ok(targets)
}

fn debt_limit(
    identity: &str,
    debt: &Debt,
    old: Option<&Debt>,
    attribution: &Attribution,
) -> Result<(), String> {
    moves::owner(&debt.owner)?;
    let actual = attribution.moved.len() as u64;
    if actual > debt.lines {
        return Err(format!(
            "coverage line debt grew for {identity}: {} -> {actual}",
            debt.lines
        ));
    }
    if let Some(old) = old {
        if old.source != debt.source || old.owner != debt.owner || debt.lines > old.lines {
            return Err(format!("coverage relocation may only shrink: {identity}"));
        }
    } else if actual != debt.lines {
        return Err(format!(
            "new coverage line debt must equal measured source correspondence: {identity}"
        ));
    }
    Ok(())
}

pub(crate) struct ResultSet {
    pub moved: BTreeMap<String, BTreeSet<usize>>,
    pub files: BTreeSet<String>,
    pub measured_debt: BTreeMap<String, u64>,
    pub sources: BTreeMap<String, String>,
}

pub(crate) fn analyze(
    root: &Path,
    reference: &str,
    baseline: &Baseline,
    current: &Measurement,
    hits: &Hits,
    enforce: bool,
) -> Result<ResultSet, String> {
    let prior = baseline::baseline_at_ref(root, reference)?.unwrap_or_default();
    let host = std::env::consts::OS;
    validate_sources(root, reference, baseline, &prior, current, hits, host)?;
    let mut sources = pending_sources(root, reference, baseline, &prior, current, host)?;
    let pending = transfers::pending(&baseline.moved, Some(&prior.moved));
    let previous = moves::relocated(&prior.coverage_moves.debt, &pending)?;
    for (target, debt) in &baseline.coverage_moves.debt {
        if let Some(previous) = previous.get(target) {
            if previous.source != debt.source {
                return Err(format!("coverage debt source changed: {target}"));
            }
        } else if sources.get(target) != Some(&debt.source) {
            return Err(format!(
                "coverage debt outside a validated declaration move: {target}"
            ));
        }
        active_owner(&debt.owner, current)?;
        sources.insert(target.clone(), debt.source.clone());
    }
    let mut result = ResultSet {
        moved: BTreeMap::new(),
        files: BTreeSet::new(),
        measured_debt: BTreeMap::new(),
        sources: BTreeMap::new(),
    };
    let proofs = prior.coverage_moves.sources.get(host);
    let mut spent = BTreeSet::new();
    for (target, identity) in sources {
        if !spent.insert(identity.clone()) {
            return Err(format!("coverage source reused: {identity}"));
        }
        let proof = proofs
            .and_then(|p| p.get(&identity))
            .ok_or_else(|| format!("missing archived {host} coverage proof: {identity}"))?
            .resolve(root)?;
        let function = source::unique(&current.coverage_functions, &target)?;
        let attribution = moves::attribute(&proof, &function.proof(root, hits)?)?;
        result
            .measured_debt
            .insert(target.clone(), attribution.moved.len() as u64);
        result.sources.insert(target.clone(), identity.clone());
        println!("relocated {target}: {} moved token lines ({} uncovered LLVM lines), {} new executable lines ({} covered)", attribution.moved.len(), attribution.uncovered, attribution.new.count, attribution.new.covered);
        if let Some(debt) = baseline.coverage_moves.debt.get(&target) {
            if enforce {
                debt_limit(&target, debt, previous.get(&target), &attribution)?;
            }
        } else if enforce && !attribution.moved.is_empty() {
            return Err(format!(
                "record an owned coverage relocation for {target}: {} uncovered moved lines",
                attribution.uncovered
            ));
        }
        result.files.insert(function.file.clone());
        result
            .moved
            .entry(function.file.clone())
            .or_default()
            .extend(
                attribution
                    .moved
                    .into_iter()
                    .map(|line| function.start.0 + line - 1),
            );
    }
    Ok(result)
}

pub(crate) fn new_lines(hits: &Hits, attributed: &ResultSet) -> BTreeMap<String, Lines> {
    let mut result = BTreeMap::new();
    for file in &attributed.files {
        let mut lines = Lines::default();
        for (line, covered) in hits.get(file).into_iter().flatten() {
            if !attributed
                .moved
                .get(file)
                .is_some_and(|moved| moved.contains(line))
            {
                lines.count += 1;
                lines.covered += u64::from(*covered);
            }
        }
        result.insert(file.clone(), lines);
    }
    result
}
