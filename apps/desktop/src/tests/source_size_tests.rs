//! The desktop's sources stay readable (plan/argon-tavsiye.md ADIM 07, docs/adr/8054 §7): no file under `src` passes
//! 2 000 lines. One that grows past it is split by its subjects into child files, as the twenty big ones were.

use std::path::{Path, PathBuf};

/// The most lines a source file may have.
const MOST_LINES: usize = 2_000;

fn sources(folder: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(folder).expect("a readable folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_source_file_passes_two_thousand_lines() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);
    assert!(
        files.len() > 1_000,
        "the sources are found: {}",
        files.len()
    );
    let mut long: Vec<String> = files
        .iter()
        .filter_map(|file| {
            let lines = std::fs::read_to_string(file)
                .expect("a readable source")
                .lines()
                .count();
            (lines > MOST_LINES).then(|| {
                format!(
                    "{} ({lines})",
                    file.strip_prefix(&root).unwrap_or(file).display()
                )
            })
        })
        .collect();
    long.sort();
    assert!(
        long.is_empty(),
        "split these by their subjects into child files (docs/adr/8054 §7): {}",
        long.join(", ")
    );
}
