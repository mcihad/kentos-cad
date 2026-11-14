//! The built-in icons an extension may name (docs/adr/8057 §21): the web app's icon set, which the desktop draws as
//! the web does, written as a manifest writes every name, lowercase with dashes (`zoom-extents`). The set names its
//! icons in camelCase (`zoomExtents`); the two forms map one to one, measured over all 1346 names (none lost, no two
//! the same). Only 213 of them were lowercase already: before this list, the manifest refused the other 1133.
//!
//! The list is generated from `docs/inventory/web.json` and `catalog/v1/additions.json` (their `icons`, the set the
//! desktop keeps) by this module's tests, which fail when it differs (`KENTOS_WRITE_SDK=1` writes it). A name not in
//! it is a warning, not a refusal: a package made for a later KentOS may name an icon this one does not have, and its
//! button is drawn with the plain glyph.

/// The names, one a line, sorted.
pub const LIST: &str = include_str!("../sdk/icons.txt");

/// The built-in names, sorted.
pub fn names() -> impl Iterator<Item = &'static str> {
    LIST.lines()
}

/// Whether `name` is a built-in icon.
pub fn is_builtin(name: &str) -> bool {
    names().any(|n| n == name)
}

/// The icon set's own name of a built-in icon: `zoom-extents` → `zoomExtents`; a name without dashes is its own.
pub fn web_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper = false;
    for c in name.chars() {
        if c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// The built-in names near `name`, at most `n`, nearest first: a slip of a letter or two (`space-numbr`), then a name
/// holding it or held in it (`number` → `space-number`), then one a third of its letters away; none when nothing is
/// that near, rather than names that only share letters.
pub fn nearest(name: &str, n: usize) -> Vec<&'static str> {
    let len = name.chars().count();
    let mut scored: Vec<(u8, usize, &'static str)> = names()
        .filter_map(|known| {
            let d = crate::verify::distance(name, known);
            let tier = if d <= 2.min(len / 2) {
                0
            } else if known.contains(name) || (known.len() >= 4 && name.contains(known)) {
                1
            } else if d <= len / 3 {
                2
            } else {
                return None;
            };
            Some((tier, d, known))
        })
        .collect();
    scored.sort_unstable();
    scored
        .into_iter()
        .take(n)
        .map(|(_, _, known)| known)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The camelCase name as a manifest writes it: `zoomExtents` → `zoom-extents`.
    fn dashed(name: &str) -> String {
        let mut out = String::with_capacity(name.len() + 4);
        for c in name.chars() {
            if c.is_ascii_uppercase() {
                out.push('-');
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn the_list_is_the_icon_set_s() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut set: Vec<String> = Vec::new();
        for file in ["docs/inventory/web.json", "catalog/v1/additions.json"] {
            let text = std::fs::read_to_string(root.join(file)).expect(file);
            let json: serde_json::Value = serde_json::from_str(&text).expect(file);
            set.extend(
                json["icons"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .map(|(k, _)| k.clone()),
            );
        }
        let mut list: Vec<String> = set.iter().map(|n| dashed(n)).collect();
        list.sort_unstable();
        list.dedup();
        // One to one: every name comes back, none is a manifest's invalid name, no two the same.
        for name in &set {
            assert_eq!(web_name(&dashed(name)), *name);
        }
        assert!(
            list.iter().all(|n| crate::manifest::is_local_id(n)),
            "a name a manifest cannot write"
        );
        assert_eq!(list.len(), {
            let mut s = set.clone();
            s.sort_unstable();
            s.dedup();
            s.len()
        });
        let text = list.iter().map(|n| format!("{n}\n")).collect::<String>();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sdk/icons.txt");
        if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
            std::fs::write(path, &text).expect("the list");
            return;
        }
        assert!(
            LIST == text,
            "the icon set changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- icon_names"
        );
    }

    #[test]
    fn a_name_is_found_and_the_nearest_said() {
        assert_eq!(web_name("zoom-extents"), "zoomExtents");
        assert_eq!(web_name("grid"), "grid");
        assert!(is_builtin("space-number") && is_builtin("grid") && !is_builtin("mezar"));
        assert_eq!(nearest("layer-field", 1), ["layer-fields"]);
        assert_eq!(nearest("space-numbr", 1), ["space-number"]);
        assert!(
            nearest("number", 3).iter().all(|n| n.contains("number")),
            "{:?}",
            nearest("number", 3)
        );
        assert!(nearest("mezar", 3).is_empty(), "{:?}", nearest("mezar", 3));
    }
}
