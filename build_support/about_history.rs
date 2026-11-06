//! Strict, dependency-free reader for the small Markdown format used by About.
use std::{env, fs, path::PathBuf};

const LANGUAGES: [&str; 7] = ["zh-CN", "zh-TW", "en", "ja", "ko", "de", "fr"];

fn version_number(version: &str) -> Result<[u32; 3], String> {
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3 { return Err(format!("invalid version: {version}")); }
    let mut number = [0; 3];
    for (index, part) in parts.iter().enumerate() {
        number[index] = part.parse().map_err(|_| format!("invalid version: {version}"))?;
        if number[index].to_string() != *part { return Err(format!("noncanonical version: {version}")); }
    }
    Ok(number)
}

fn parse(version: &str, text: &str) -> Result<[Vec<String>; 7], String> {
    let mut rows: [Vec<String>; 7] = std::array::from_fn(|_| Vec::new());
    let mut seen = [false; 7];
    let mut language = None;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    if lines.next() != Some(format!("# {version}").as_str()) {
        return Err(format!("{version}: first heading must match filename"));
    }
    for line in lines {
        if let Some(code) = line.strip_prefix("## ") {
            let index = LANGUAGES.iter().position(|candidate| *candidate == code)
                .ok_or_else(|| format!("{version}: unknown language {code}"))?;
            if seen[index] { return Err(format!("{version}: duplicate language {code}")); }
            seen[index] = true;
            language = Some(index);
        } else {
            let index = language.ok_or_else(|| format!("{version}: text before language heading"))?;
            if line.starts_with('#') { return Err(format!("{version}: unsupported heading {line}")); }
            let text = match line.strip_prefix("- ") {
                Some(item) if !item.trim().is_empty() => format!("• {item}"),
                Some(_) => return Err(format!("{version}: empty list item")),
                None => line.to_owned(),
            };
            rows[index].push(text);
        }
    }
    if rows.iter().any(Vec::is_empty) {
        return Err(format!("{version}: all seven language sections must contain text"));
    }
    if rows.iter().any(|row| row.len() != rows[0].len()) {
        return Err(format!("{version}: language sections must have equal item counts"));
    }
    Ok(rows)
}

pub fn generate() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let directory = root.join("docs/about-history");
    println!("cargo:rerun-if-changed={}", directory.display());
    let index: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(directory.join("index.json")).expect("cannot read About history index")
    ).expect("invalid About history index JSON");
    let settings = index.as_object().expect("About history index must be an object");
    assert!(settings.keys().all(|key| matches!(key.as_str(),
        "initial_visible_versions" | "older_expanded_on_open" | "versions")),
        "unknown About history index setting");
    let visible = index["initial_visible_versions"].as_u64()
        .expect("initial_visible_versions must be a positive integer");
    let expanded = index["older_expanded_on_open"].as_bool()
        .expect("older_expanded_on_open must be a boolean");
    let listed = index["versions"].as_array().expect("versions must be an ordered list");
    assert!(visible > 0 && visible <= listed.len() as u64, "invalid initial_visible_versions");
    let mut versions = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for value in listed {
        let version = value.as_str().expect("version entries must be strings");
        version_number(version).unwrap_or_else(|error| panic!("{error}"));
        assert!(seen.insert(version.to_owned()), "duplicate indexed version: {version}");
        let path = directory.join(format!("{version}.md"));
        let text = fs::read_to_string(&path).expect("history must be UTF-8");
        let rows = parse(version, &text).unwrap_or_else(|error| panic!("{error}"));
        versions.push((version, rows));
    }
    for entry in fs::read_dir(&directory).expect("cannot read About history directory") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|value| value.to_str()) == Some("md") {
            let version = path.file_stem().unwrap().to_str().unwrap();
            assert!(version == "README" || seen.contains(version),
                "About history document missing from index: {version}");
        }
    }
    let current = env::var("CARGO_PKG_VERSION").unwrap();
    assert_eq!(versions.first().map(|item| item.0), Some(current.as_str()),
        "first indexed About history must match Cargo.toml");
    let mut output = format!("const INITIAL_VISIBLE_VERSIONS: usize = {visible};\nconst OLDER_EXPANDED_ON_OPEN: bool = {expanded};\nconst VERSIONS: &[Version] = &[\n");
    for (version, rows) in versions {
        output.push_str(&format!("Version {{ number: {version:?}, languages: [\n"));
        for row in rows {
            output.push_str("&[");
            for text in row { output.push_str(&format!("{text:?},")); }
            output.push_str("],\n");
        }
        output.push_str("] },\n");
    }
    output.push_str("];\n");
    let output_path = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("about_history.rs");
    fs::write(output_path, output).expect("cannot generate About history");
}
