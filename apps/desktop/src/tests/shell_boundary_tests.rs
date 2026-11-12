//! The shell's center names no discipline's window (plan/argon-tavsiye.md ADIM 06, docs/adr/8054 §6): `App`,
//! `Message` and `Dialog` reach the road, map, GIS and architecture windows only through each discipline's
//! `state.rs` (`crate::road`, `crate::map`, `crate::gis`, `crate::arch`), so a new window of one of them is added there
//! and `shell/app.rs` stays as it is.

use std::path::Path;

const DISCIPLINES: [&str; 4] = ["road", "map", "gis", "arch"];

/// The module a `mod` line declares, whatever its visibility.
fn declared(line: &str) -> Option<&str> {
    let line = line.trim();
    let line = line
        .strip_prefix("pub(crate) ")
        .or(line.strip_prefix("pub "))
        .unwrap_or(line);
    line.strip_prefix("mod ")
        .map(|name| name.trim_end_matches(';'))
}

#[test]
fn the_shell_reaches_a_disciplines_windows_only_through_its_state() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let lib = std::fs::read_to_string(src.join("lib.rs")).expect("lib.rs");
    let app = std::fs::read_to_string(src.join("shell/app.rs")).expect("shell/app.rs");
    // The modules in the four disciplines' folders, by the `#[path]` that loads them.
    let mut theirs = Vec::new();
    let mut folder = None;
    for line in lib.lines() {
        if let Some(path) = line.strip_prefix("#[path = \"") {
            folder = path.split('/').next().filter(|f| DISCIPLINES.contains(f));
        } else if let (Some(_), Some(name)) = (folder, declared(line)) {
            if !DISCIPLINES.contains(&name) {
                theirs.push(name.to_owned());
            }
            folder = None;
        }
    }
    assert!(
        theirs.len() > 100,
        "the disciplines' modules are found: {}",
        theirs.len()
    );
    let named: Vec<&String> = theirs
        .iter()
        .filter(|m| app.contains(&format!("crate::{m}::")))
        .collect();
    assert!(
        named.is_empty(),
        "shell/app.rs names these windows' modules; put them in their discipline's state.rs: {named:?}"
    );
}
