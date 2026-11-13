//! Önerilen paketler (P04; the owner, 13 November 2026: "Paketler … aranabilmeli, listeden seçilerek kurulabilmeli"): a
//! short list of packages a municipality's engineer often needs (`paketler.json`), each with a Turkish line and its
//! group, searched offline in Turkish ("sayisal" finds "Sayısal"). It is a starting point, not a gate: any other
//! package of the index can be asked for by its name.

use std::sync::OnceLock;

use serde::Deserialize;

/// A recommended package.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    /// Its name in the index, normalised.
    pub name: String,
    /// What it is for, in a line.
    pub title: String,
    pub group: String,
    /// Words it is also found by.
    #[serde(default)]
    pub words: Vec<String>,
}

/// The list.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalog {
    pub format: String,
    pub schema_version: u32,
    pub note: String,
    /// The groups, in the order shown.
    pub groups: Vec<String>,
    pub packages: Vec<Entry>,
}

/// The list built into this program; empty when it cannot be read (a test reads it).
pub fn builtin() -> &'static Catalog {
    static BUILT_IN: OnceLock<Catalog> = OnceLock::new();
    BUILT_IN.get_or_init(|| {
        serde_json::from_str(include_str!("../paketler.json")).unwrap_or(Catalog {
            format: String::new(),
            schema_version: 0,
            note: String::new(),
            groups: Vec::new(),
            packages: Vec::new(),
        })
    })
}

/// Text as the search compares it: lower case, Turkish letters as their plain ones (`ı`, `İ` → `i`, `ş` → `s` …), so
/// a query typed without them finds them.
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            'ı' | 'i' | 'î' => Some('i'),
            'ş' => Some('s'),
            'ğ' => Some('g'),
            'ü' | 'û' => Some('u'),
            'ö' => Some('o'),
            'ç' => Some('c'),
            'â' => Some('a'),
            // The dot `İ` leaves when lowered.
            '\u{307}' => None,
            c => Some(c),
        })
        .collect()
}

impl Catalog {
    /// The packages a query finds, in the list's order: every word of it in the name, line, group or words. All of
    /// them for an empty query.
    pub fn search(&self, query: &str) -> Vec<&Entry> {
        let words: Vec<String> = fold(query).split_whitespace().map(str::to_owned).collect();
        self.packages
            .iter()
            .filter(|entry| {
                let text = fold(&format!(
                    "{} {} {} {}",
                    entry.name,
                    entry.title,
                    entry.group,
                    entry.words.join(" ")
                ));
                words.iter().all(|word| text.contains(word.as_str()))
            })
            .collect()
    }

    /// The entry of a package, by its normalised name.
    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.packages.iter().find(|entry| entry.name == name)
    }
}
