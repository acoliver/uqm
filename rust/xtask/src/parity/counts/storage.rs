use super::*;

pub(in super::super) fn write_directory(
    directory: &Path,
    rows: &BTreeMap<String, Counts>,
) -> Result<()> {
    for (name, row) in rows {
        if !manifest::identifier(name) {
            return Err(format!("invalid counts scenario filename {name}"));
        }
        let text = encode(&BTreeMap::from([(name.clone(), row.clone())]))?;
        write(&directory.join(format!("{name}.toml")), text.as_bytes())?;
    }
    Ok(())
}

pub(in super::super) fn read_directory(directory: &Path) -> Result<BTreeMap<String, Counts>> {
    let mut rows = BTreeMap::new();
    for entry in
        fs::read_dir(directory).map_err(|error| format!("read {}: {error}", directory.display()))?
    {
        let path = entry.map_err(|error| error.to_string())?.path();
        if !fs::symlink_metadata(&path)
            .map_err(|error| error.to_string())?
            .is_file()
        {
            return Err(format!("counts require regular files: {}", path.display()));
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let mut decoded = decode(&text)?;
        if decoded.len() != 1 {
            return Err(format!(
                "counts file must contain one scenario: {}",
                path.display()
            ));
        }
        let (name, row) = decoded.pop_first().ok_or("missing counts scenario")?;
        if !manifest::identifier(&name)
            || path.file_name().and_then(|name| name.to_str()) != Some(&format!("{name}.toml"))
        {
            return Err(format!(
                "counts scenario {name} disagrees with filename {}",
                path.display()
            ));
        }
        rows.insert(name, row);
    }
    if rows.is_empty() {
        return Err("empty counts inventory".into());
    }
    Ok(rows)
}

pub(in super::super) fn compare_directories(actual: &Path, expected: &Path) -> Result<()> {
    let actual_rows = read_directory(actual)?;
    let expected_rows = read_directory(expected)?;
    if actual_rows != expected_rows {
        return Err(difference::first(&actual_rows, &expected_rows));
    }
    for name in actual_rows.keys() {
        let file = format!("{name}.toml");
        if read(&actual.join(&file))? != read(&expected.join(&file))? {
            return Err(format!(
                "counts {name}: bytes differ despite equal decoded samples"
            ));
        }
    }
    Ok(())
}
