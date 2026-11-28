//! No other program's name and no ADR cite reaches the person (CLAUDE.md, “Başka uygulamaya atıf yok”).
//!
//! Three places show text to the person or to a script author and are scanned here:
//! - the catalog's texts (`docs/inventory/web.json`, `catalog/v1/additions.json`): titles, descriptions, aliases;
//! - the string literals of the desktop programs' sources, the catalog's and the contracts' (comments are not
//!   scanned: the rule cleans them as files are touched). ADR cites are checked only outside test code (inline
//!   `#[cfg(test)]` modules, `tests/` folders, `tests.rs`, `*_tests.rs` and `*_scenes.rs`); names are checked in all of it;
//! - the Python SDK's text (`python/kentos`), which `help()` shows. ADR cites there are the contracts' doc comments
//!   and are not scanned yet; names are.
//!
//! A name that is a protocol, a service type or a format's own signature is allowed explicitly in `ALLOWED`, with
//! the reason (owner decision F4 of todos-new.md is to confirm that list).

use std::path::{Path, PathBuf};

/// Other programs' names, as written.
const NAMES: [&str; 16] = [
    "Netcad",
    "NETCAD",
    "AutoCAD",
    "Civil 3D",
    "Civil3D",
    "ArcGIS",
    "QGIS",
    "OpenRoads",
    "MicroStation",
    "NETPRO",
    "Netpro",
    "Revit",
    "Google Earth",
    "Acrobat",
    "Inkscape",
    "LibreOffice",
];

/// (file ending, name, why): a name kept because it is a protocol, a service type or a file signature.
const ALLOWED: [(&str, &str, &str); 10] = [
    ("contracts/src/core/shape_resource.rs", "AutoCAD-86", "the first bytes of an SHX shape file, read to recognise it"),
    ("contracts/src/gis/service.rs", "ArcGIS", "the ArcGIS REST service kind, its token kind and the parameters it asks for"),
    ("commands/src/catalog/annotation.rs", "ArcGIS REST", "the service kind a layer is added from"),
    ("shell/services/feed_window.rs", "ArcGIS", "the service kind's name in the feed window and the `where` text its query takes"),
    ("shell/services/window.rs", "ArcGIS", "the service kind's name in the service window"),
    ("shell/services/info.rs", "ArcGIS", "the service kind the identify tool reads"),
    ("python/kentos/services.py", "ArcGIS", "the service kind and its query words"),
    ("python/kentos/cad/layers.py", "ArcGIS REST", "the service kind a layer is added from"),
    ("python/kentos/cad/types.py", "ArcGIS", "the service kind and its query words, generated from the contracts"),
    ("web.json", "ArcGIS", "the service kind, its token kind and its identify records"),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(folder: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "__pycache__" || n == "target") {
                continue;
            }
            files(&path, extensions, out);
        } else if path.extension().is_some_and(|e| extensions.iter().any(|x| e == *x)) {
            out.push(path);
        }
    }
}

/// Whether `text`, found in `file`, names something the rule bans and `ALLOWED` does not allow.
fn offence(file: &str, text: &str, cites: bool) -> Option<String> {
    for name in NAMES {
        let banned = text.match_indices(name).any(|(at, _)| {
            !ALLOWED
                .iter()
                .any(|(end, allowed, _)| file.ends_with(end) && allowed.starts_with(name) && text[at..].starts_with(allowed))
        });
        if banned {
            return Some(format!("names {name}"));
        }
    }
    if cites {
        if text.contains("docs/adr/") {
            return Some("cites an ADR file (docs/adr/…)".to_owned());
        }
        if text.split("ADR ").skip(1).any(|rest| rest.starts_with(|c: char| c.is_ascii_digit())) {
            return Some("cites an ADR (ADR nnnn)".to_owned());
        }
    }
    None
}

/// The string literals of a Rust source with their lines: comments and character literals are skipped.
fn literals(source: &str) -> Vec<(usize, String)> {
    let c: Vec<char> = source.chars().collect();
    let (mut i, mut line, mut out) = (0, 1, Vec::new());
    while i < c.len() {
        match c[i] {
            '\n' => {
                line += 1;
                i += 1;
            }
            '/' if c.get(i + 1) == Some(&'/') => {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
            }
            '/' if c.get(i + 1) == Some(&'*') => {
                let mut depth = 1;
                i += 2;
                while i < c.len() && depth > 0 {
                    if c[i] == '/' && c.get(i + 1) == Some(&'*') {
                        depth += 1;
                        i += 2;
                    } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        i += 2;
                    } else {
                        if c[i] == '\n' {
                            line += 1;
                        }
                        i += 1;
                    }
                }
            }
            'r' if (i == 0 || !(c[i - 1].is_alphanumeric() || c[i - 1] == '_') || c[i - 1] == 'b')
                && raw_start(&c, i).is_some() =>
            {
                let hashes = raw_start(&c, i).unwrap_or(0);
                let begin = i + 2 + hashes;
                let mut end = begin;
                while end < c.len() && !(c[end] == '"' && (0..hashes).all(|h| c.get(end + 1 + h) == Some(&'#'))) {
                    end += 1;
                }
                let text: String = c[begin..end.min(c.len())].iter().collect();
                line += text.matches('\n').count();
                out.push((line - text.matches('\n').count(), text));
                i = end + 1 + hashes;
            }
            '"' => {
                let (begin, start_line) = (i + 1, line);
                i += 1;
                while i < c.len() && c[i] != '"' {
                    if c[i] == '\\' {
                        i += 1;
                    }
                    if i < c.len() && c[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
                out.push((start_line, c[begin..i.min(c.len())].iter().collect()));
                i += 1;
            }
            '\'' => {
                // A character literal ('x', '\n', '"') or a lifetime ('a).
                if c.get(i + 1) == Some(&'\\') {
                    i += 2;
                    while i < c.len() && c[i] != '\'' {
                        i += 1;
                    }
                    i += 1;
                } else if c.get(i + 2) == Some(&'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// The line where an inline `#[cfg(test)] mod name {` starts, or `usize::MAX` when the file has none.
fn inline_tests_from(source: &str) -> usize {
    let lines: Vec<&str> = source.lines().collect();
    (0..lines.len())
        .find(|&i| {
            lines[i].trim() == "#[cfg(test)]"
                && lines.get(i + 1).is_some_and(|n| n.trim_start().starts_with("mod ") && n.trim_end().ends_with('{'))
        })
        .map_or(usize::MAX, |i| i + 1)
}

/// The number of `#` of a raw string starting at `c[i] == 'r'`, if one starts there.
fn raw_start(c: &[char], i: usize) -> Option<usize> {
    let mut j = i + 1;
    while c.get(j) == Some(&'#') {
        j += 1;
    }
    (c.get(j) == Some(&'"') && j - i - 1 <= 8).then_some(j - i - 1)
}

#[test]
fn rust_string_literals_name_no_other_program_and_cite_no_adr() {
    let root = root();
    let mut folders = vec![
        root.join("apps/kentos-cad/src"),
        root.join("crates/shared/commands/src"),
        root.join("crates/shared/contracts/src"),
        // The interface kits: their texts reach the person as the programs' own do.
        root.join("crates/ui/src"),
        root.join("crates/sheet-ui/src"),
        root.join("crates/model-view-ui/src"),
    ];
    for entry in std::fs::read_dir(root.join("apps")).expect("the apps folder").flatten() {
        if entry.file_name().to_string_lossy().starts_with("desktop") {
            folders.push(entry.path().join("src"));
        }
    }
    let mut sources = Vec::new();
    for folder in &folders {
        files(folder, &["rs"], &mut sources);
    }
    assert!(sources.len() > 1_000, "the sources are found: {}", sources.len());
    let mut found = Vec::new();
    for path in &sources {
        let name = path.to_string_lossy().replace('\\', "/");
        if name.ends_with("tests/other_names_tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("a readable source");
        // ADR cites are banned in what the person reads; a test's message or note may cite one. Names are banned
        // everywhere.
        let test_file = name.contains("/tests/") || name.ends_with("_tests.rs") || name.ends_with("/tests.rs") || name.ends_with("_scenes.rs");
        let test_from = inline_tests_from(&text);
        for (line, literal) in literals(&text) {
            let cites = !test_file && line < test_from;
            if let Some(why) = offence(&name, &literal, cites) {
                let at = path.strip_prefix(&root).unwrap_or(path).display();
                found.push(format!("{at}:{line} {why}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a text the person reads names another program or cites an ADR; say what it does instead (a protocol or a \
         format signature goes into ALLOWED with its reason):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_literal_reader_skips_comments_and_reads_raw_strings() {
    let src = "// \"AutoCAD\" in a comment\nlet a = \"one\"; /* \"Civil 3D\" */ let b = r#\"two \"quoted\"\"#; let c = '\"'; let d = \"x\\\"y\";";
    let read: Vec<String> = literals(src).into_iter().map(|(_, s)| s).collect();
    assert_eq!(read, vec!["one", "two \"quoted\"", "x\\\"y"]);
}

#[test]
fn catalog_texts_name_no_other_program_and_cite_no_adr() {
    let root = root();
    let mut found = Vec::new();
    for file in ["docs/inventory/web.json", "catalog/v1/additions.json"] {
        let text = std::fs::read_to_string(root.join(file)).expect("a readable catalog");
        let json: serde_json::Value = serde_json::from_str(&text).expect("a JSON catalog");
        // Notes, reasons and acceptance lines are the inventory's own bookkeeping, not what the person reads.
        walk(&json, file, &mut Vec::new(), &mut found);
    }
    assert!(
        found.is_empty(),
        "a catalog text names another program or cites an ADR (tooltips and Komut ara show it):\n{}",
        found.join("\n")
    );
}

fn walk(value: &serde_json::Value, file: &str, path: &mut Vec<String>, found: &mut Vec<String>) {
    const BOOKKEEPING: [&str; 6] = ["note", "reason", "acceptance", "desktopNote", "desktopWhere", "generated"];
    match value {
        serde_json::Value::Object(map) => {
            for (key, inner) in map {
                if BOOKKEEPING.contains(&key.as_str()) {
                    continue;
                }
                path.push(key.clone());
                walk(inner, file, path, found);
                path.pop();
            }
        }
        serde_json::Value::Array(items) => {
            for (i, inner) in items.iter().enumerate() {
                path.push(i.to_string());
                walk(inner, file, path, found);
                path.pop();
            }
        }
        serde_json::Value::String(text) => {
            if let Some(why) = offence(file, text, true) {
                found.push(format!("{file} {} {why}", path.join("/")));
            }
        }
        _ => {}
    }
}

#[test]
fn the_python_sdk_names_no_other_program() {
    let root = root();
    let mut sources = Vec::new();
    files(&root.join("python/kentos"), &["py", "pyi"], &mut sources);
    assert!(sources.len() > 50, "the SDK is found: {}", sources.len());
    let mut found = Vec::new();
    for path in &sources {
        let name = path.strip_prefix(&root).unwrap_or(path).to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(path).expect("a readable Python file");
        for (i, line) in text.lines().enumerate() {
            if let Some(why) = offence(&name, line, false) {
                found.push(format!("{name}:{} {why}", i + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "the SDK's text (generated from the contracts' doc comments: fix it there and run scripts/python/sdk.py) names \
         another program:\n{}",
        found.join("\n")
    );
}
