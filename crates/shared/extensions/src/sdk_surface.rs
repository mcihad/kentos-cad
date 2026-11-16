//! What leaves the SDK's stable surface (docs/adr/8061): a name the SDK deprecates, with what replaces it and since
//! which KentOS, and a name still experimental. A package's Python is read for them as text where its other rules are
//! (python_rules.rs, gate 1: packing, installing, `verify`): `sdk_deprecated` and `sdk_experimental` are warnings;
//! the catalog's package gate refuses experimental use (ADR 8061 §5). `kentos kek upgrade` writes a deprecated name's
//! replacement when it is a plain rename.
//!
//! Today nothing is deprecated or experimental: both lists are empty, and the tests give their own. A deprecation is
//! a line here, a `DeprecationWarning` in `python/kentos/ext.py` and a line in ADR 8061 §10, in one commit.

use std::collections::BTreeMap;

use crate::problem::Problem;

/// A name the SDK deprecates: kept at least one KentOS release, removed only when the SDK protocol goes up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deprecated {
    /// The name as code writes it after a dot (`ctx.<name>(`, `ext.<name>`).
    pub name: &'static str,
    /// Whose it is (`CommandContext`, `kentos.ext`), for the words.
    pub on: &'static str,
    /// The name that replaces it, when the change is a plain rename `kek upgrade` writes; none when the note says what
    /// to do.
    pub replacement: Option<&'static str>,
    /// The KentOS version that deprecated it.
    pub since: &'static str,
    /// What to do instead, in Turkish.
    pub note: &'static str,
}

/// A name still experimental: usable at home and in the workshop, not published (ADR 8061 §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Experimental {
    pub name: &'static str,
    pub on: &'static str,
    pub note: &'static str,
}

/// The SDK's deprecated names (ADR 8061 §10).
pub const DEPRECATED: &[Deprecated] = &[];

/// The SDK's experimental names (ADR 8061 §10).
pub const EXPERIMENTAL: &[Experimental] = &[];

/// Where `line` (code, a docstring and a comment cut) uses `name` after a dot or imports it: its byte offsets.
fn uses(line: &str, name: &str) -> Vec<usize> {
    let imported = line.strip_prefix("from kentos").is_some_and(|r| {
        r.contains(" import ") && r.split([' ', ',', '(', ')']).any(|w| w == name)
    });
    let mut at: Vec<usize> = line
        .match_indices(name)
        .filter(|(i, _)| {
            let before = line[..*i].chars().last();
            let after = line[i + name.len()..].chars().next();
            before == Some('.') && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
        .map(|(i, _)| i)
        .collect();
    if imported && at.is_empty() {
        at.push(line.find(name).unwrap_or_default());
    }
    at
}

/// The surface's warnings on Python file `path`, against the lists given (the SDK's, or a test's).
pub(crate) fn scan_with(
    path: &str,
    text: &str,
    deprecated: &[Deprecated],
    experimental: &[Experimental],
    warnings: &mut Vec<Problem>,
) {
    if deprecated.is_empty() && experimental.is_empty() {
        return;
    }
    for (n, line) in crate::python_rules::code_lines(text) {
        for d in deprecated
            .iter()
            .filter(|d| !uses(&line, d.name).is_empty())
        {
            let instead = d
                .replacement
                .map(|r| format!("yerine “{r}” yazın (kentos kek upgrade yazar)"))
                .unwrap_or_else(|| d.note.to_owned());
            warnings.push(
                Problem::new(
                    "sdk_deprecated",
                    format!(
                        "“{}” ({}) KentOS {} ile eskidi; SDK'nın protokolü artınca kalkar: {instead}.",
                        d.name, d.on, d.since
                    ),
                )
                .file(path)
                .at(format!("satır {n}")),
            );
        }
        for e in experimental
            .iter()
            .filter(|e| !uses(&line, e.name).is_empty())
        {
            warnings.push(
                Problem::new(
                    "sdk_experimental",
                    format!(
                        "“{}” ({}) deneysel: atölyede ve yerel kurulumda çalışır, katalogda yayımlanamaz. {}",
                        e.name, e.on, e.note
                    ),
                )
                .file(path)
                .at(format!("satır {n}")),
            );
        }
    }
}

/// The surface's warnings on Python file `path` against the SDK's lists.
pub(crate) fn scan(path: &str, text: &str, warnings: &mut Vec<Problem>) {
    scan_with(path, text, DEPRECATED, EXPERIMENTAL, warnings);
}

/// One rewrite `kek upgrade` made: the file, the line, the name and what it became.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rewrite {
    pub file: String,
    pub line: usize,
    pub from: &'static str,
    pub to: &'static str,
}

/// `kentos kek upgrade` on a working folder (path → bytes): every deprecated name with a plain rename rewritten in the
/// code of its Python files (a docstring and a comment left as written); the files that changed, with their new text,
/// and each rewrite.
pub fn upgrade(folder: &BTreeMap<String, Vec<u8>>) -> (BTreeMap<String, String>, Vec<Rewrite>) {
    upgrade_with(folder, DEPRECATED)
}

/// [`upgrade`] against the list given.
pub fn upgrade_with(
    folder: &BTreeMap<String, Vec<u8>>,
    deprecated: &[Deprecated],
) -> (BTreeMap<String, String>, Vec<Rewrite>) {
    let renames: Vec<(&'static str, &'static str)> = deprecated
        .iter()
        .filter_map(|d| Some((d.name, d.replacement?)))
        .collect();
    let mut files = BTreeMap::new();
    let mut rewrites = Vec::new();
    if renames.is_empty() {
        return (files, rewrites);
    }
    for (path, bytes) in folder.iter().filter(|(p, _)| p.ends_with(".py")) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            continue;
        };
        let code: BTreeMap<usize, String> =
            crate::python_rules::code_lines(text).into_iter().collect();
        let mut out = String::with_capacity(text.len());
        let mut changed = false;
        for (i, raw) in text.split_inclusive('\n').enumerate() {
            let Some(line) = code.get(&(i + 1)) else {
                out.push_str(raw);
                continue;
            };
            // The code part of the raw line: up to a comment, as the scan cut it.
            let end = raw.find('#').unwrap_or(raw.len());
            let (head, tail) = raw.split_at(end);
            let mut head = head.to_owned();
            for (from, to) in &renames {
                if uses(line, from).is_empty() {
                    continue;
                }
                let mut at: Vec<usize> = uses(&head, from);
                if at.is_empty() {
                    continue;
                }
                at.reverse();
                for i in at {
                    head.replace_range(i..i + from.len(), to);
                }
                changed = true;
                rewrites.push(Rewrite {
                    file: path.clone(),
                    line: i + 1,
                    from,
                    to,
                });
            }
            out.push_str(&head);
            out.push_str(tail);
        }
        if changed {
            files.insert(path.clone(), out);
        }
    }
    (files, rewrites)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: Deprecated = Deprecated {
        name: "set_rows",
        on: "PanelSession",
        replacement: Some("rows"),
        since: "0.2.0",
        note: "",
    };
    const GONE: Deprecated = Deprecated {
        name: "run_model",
        on: "CommandContext",
        replacement: None,
        since: "0.2.0",
        note: "modeli ctx.run_tool(\"model:…\") ile çalıştırın",
    };
    const TRY: Experimental = Experimental {
        name: "stream_rows",
        on: "PanelSession",
        note: "Tablonun satırlarını parça parça gönderir.",
    };

    const CODE: &str = "def ara(ctx):\n    \"\"\"Eski: ctx.panel.set_rows(...)\"\"\"\n    \
                        ctx.panel.set_rows(\"t\", rows)  # set_rows eskidi\n    ctx.run_model(\"m\")\n    \
                        ctx.panel.stream_rows(\"t\")\n    offset_rows = 1\n";

    /// A deprecated name is warned on its line with its replacement or what to do; an experimental one says it is not
    /// published; a docstring, a comment and a name ending in another are not uses.
    #[test]
    fn deprecated_and_experimental_names_are_warned_on_their_lines() {
        let mut warnings = Vec::new();
        scan_with("src/a.py", CODE, &[OLD, GONE], &[TRY], &mut warnings);
        let said: Vec<(&str, String)> = warnings
            .iter()
            .map(|p| (p.code, p.at.clone().unwrap_or_default()))
            .collect();
        assert_eq!(
            said,
            [
                ("sdk_deprecated", "satır 3".to_owned()),
                ("sdk_deprecated", "satır 4".to_owned()),
                ("sdk_experimental", "satır 5".to_owned())
            ]
        );
        assert!(
            warnings[0].message.contains("“rows”"),
            "{}",
            warnings[0].message
        );
        assert!(
            warnings[1].message.contains("ctx.run_tool"),
            "{}",
            warnings[1].message
        );
        let mut none = Vec::new();
        scan("src/a.py", CODE, &mut none);
        assert!(none.is_empty(), "the SDK deprecates nothing today");
    }

    /// `kek upgrade` writes a plain rename in the code and leaves the docstring, the comment and a name without a
    /// replacement as they were.
    #[test]
    fn upgrade_rewrites_plain_renames_in_the_code_only() {
        let folder = BTreeMap::from([("src/a.py".to_owned(), CODE.as_bytes().to_vec())]);
        let (files, rewrites) = upgrade_with(&folder, &[OLD, GONE]);
        assert_eq!(
            rewrites,
            [Rewrite {
                file: "src/a.py".into(),
                line: 3,
                from: "set_rows",
                to: "rows"
            }]
        );
        let text = &files["src/a.py"];
        assert!(
            text.contains("ctx.panel.rows(\"t\", rows)  # set_rows eskidi\n"),
            "{text}"
        );
        assert!(
            text.contains("\"\"\"Eski: ctx.panel.set_rows(...)\"\"\""),
            "{text}"
        );
        assert!(text.contains("ctx.run_model(\"m\")"), "{text}");
        let (files, rewrites) = upgrade(&folder);
        assert!(
            files.is_empty() && rewrites.is_empty(),
            "nothing to upgrade today"
        );
    }
}
