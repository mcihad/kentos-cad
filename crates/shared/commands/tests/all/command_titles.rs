//! The commands' titles by themselves (`catalog/v1/command-titles.json`): what a program shows of a command without
//! building the catalog (its schemas take seconds in a debug build; the desktop's history reads a title for every
//! row). The file is written with the catalog, `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-commands catalog`, and
//! held to it here: a title is written once, in the command's descriptor.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// The file's text: every command once (its first version's title), in the catalog's order.
fn text() -> String {
    let catalog = kentos_commands::catalog::catalog();
    let mut seen = BTreeSet::new();
    let rows: Vec<String> = catalog
        .commands
        .iter()
        .filter(|c| seen.insert(c.id.clone()))
        .map(|c| {
            format!(
                "  {}: {}",
                serde_json::to_string(&c.id).expect("an id"),
                serde_json::to_string(&c.title).expect("a title")
            )
        })
        .collect();
    format!(
        "{{\n \"format\": \"kentos.command-titles\",\n \"version\": 1,\n \"titles\": {{\n{}\n }}\n}}\n",
        rows.join(",\n")
    )
}

#[test]
fn the_catalog_s_titles_file_is_up_to_date() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../catalog/v1/command-titles.json");
    let text = text();
    if std::env::var("KENTOS_WRITE_CATALOG").as_deref() == Ok("1") {
        std::fs::write(&path, &text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        return;
    }
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        on_disk == text,
        "{} is out of date: run `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-commands catalog`, read the diff, commit it",
        path.display()
    );
}
