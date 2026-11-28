//! The published description is the generated one, and it names every route of the router and nothing else.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::catalog::{document, ops};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_published_description_is_the_generated_one() {
    let generated = serde_json::to_string_pretty(&document()).unwrap() + "\n";
    let path = repo().join("docs/api/openapi.json");
    if std::env::var("KENTOS_WRITE_OPENAPI").as_deref() == Ok("1") {
        std::fs::write(&path, &generated).unwrap();
    }
    let published = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        published == generated,
        "docs/api/openapi.json eski: `KENTOS_WRITE_OPENAPI=1 cargo test -p kentos-api openapi` ile yeniden yazın."
    );
}

/// `(method, path)` of every `.route("…", get(…).post(…))` in the router's sources, test modules left out.
fn routed() -> BTreeSet<(String, String)> {
    let mut found = BTreeSet::new();
    let mut files = vec![repo().join("apps/api/src/http")];
    while let Some(at) = files.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let p = entry.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            if p.is_dir() {
                if !name.ends_with("tests") && name != "openapi" {
                    files.push(p);
                }
                continue;
            }
            if !name.ends_with(".rs") || name.ends_with("_tests.rs") || name == "tests.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap();
            let text = match text.find("#[cfg(test)]\nmod tests {") {
                Some(at) => &text[..at],
                None => &text,
            };
            let mut rest = text;
            while let Some(at) = rest.find(".route(") {
                rest = &rest[at + 7..];
                let Some(open) = rest.find('"') else { break };
                let Some(len) = rest[open + 1..].find('"') else {
                    break;
                };
                let path = rest[open + 1..open + 1 + len].to_owned();
                // The handlers: up to the parenthesis closing `.route(`.
                let mut depth = 1;
                let mut end = 0;
                for (i, ch) in rest.char_indices() {
                    match ch {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = i;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let handlers = &rest[open + 1 + len..end];
                for verb in ["get", "post", "put", "patch", "delete"] {
                    if handlers.contains(&format!("{verb}(")) {
                        found.insert((verb.to_uppercase(), path.clone()));
                    }
                }
                rest = &rest[end..];
            }
        }
    }
    found
}

#[test]
fn every_route_is_described_and_nothing_else() {
    let routes = routed();
    assert!(routes.len() > 100, "{} routes found", routes.len());
    let described: BTreeSet<(String, String)> = ops()
        .iter()
        .map(|o| (o.method.to_owned(), o.path.to_owned()))
        .collect();
    let missing: Vec<_> = routes.difference(&described).collect();
    let extra: Vec<_> = described.difference(&routes).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "açıklanmayan: {missing:?}\nyönlendiricide olmayan: {extra:?}"
    );
}

#[test]
fn every_operation_says_what_it_does() {
    let mut ids = BTreeSet::new();
    for o in ops() {
        assert!(
            !o.summary.is_empty() && !o.about.is_empty(),
            "{} {}",
            o.method,
            o.path
        );
        assert!(
            ids.insert(super::catalog::operation_id(o.method, o.path)),
            "{} {}",
            o.method,
            o.path
        );
    }
}
