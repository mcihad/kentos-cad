//! Eklentiler's views (plan/eklenti.md ADIM 18, D1): Kurulu, Bulunabilir, Güncellenebilir, Yeni and Sorunlu, each with
//! its count, the lists filtered by a word (name, summary, description, keywords, id, publisher) and by the program.
//! "Yeni" is what came to the catalog since its previous opening: what was seen is kept beside the registry
//! (`katalog-gorulen.json`), and a session's list does not change while it is looked at.

use std::collections::BTreeSet;
use std::path::Path;

use kentos_extension_host::ExtensionView;
use kentos_extensions::catalog::{Catalog, CatalogEntry};
use kentos_extensions::manifest::{Manifest, Module};

use super::{Extensions, is_update};
use crate::exchange::apply::fold_turkish;

/// The catalog entries seen, `id@version`, beside the registry.
const SEEN: &str = "katalog-gorulen.json";

/// What the list shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Installed,
    Available,
    Updates,
    New,
    Troubled,
}

impl Part {
    pub(crate) const ALL: [Part; 5] = [
        Part::Installed,
        Part::Available,
        Part::Updates,
        Part::New,
        Part::Troubled,
    ];

    fn words(self) -> &'static str {
        match self {
            Part::Installed => "Kurulu",
            Part::Available => "Bulunabilir",
            Part::Updates => "Güncellenebilir",
            Part::New => "Yeni",
            Part::Troubled => "Sorunlu",
        }
    }

    /// A catalog's part: its rows are catalog entries, not installed extensions.
    pub(crate) fn is_catalog(self) -> bool {
        matches!(self, Part::Available | Part::Updates | Part::New)
    }
}

impl std::fmt::Display for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.words())
    }
}

/// A part with its count, as the segmented control writes it: "Güncellenebilir (2)"; no count when none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Counted(pub(crate) Part, pub(crate) usize);

impl std::fmt::Display for Counted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.1 {
            0 => write!(f, "{}", self.0),
            n => write!(f, "{} ({n})", self.0),
        }
    }
}

/// The program filter's choices: every program, then each one.
pub(crate) const PROGRAMS: [(Option<Module>, &str); 5] = [
    (None, "Tüm programlar"),
    (Some(Module::Mimari), "KentOS Mimari"),
    (Some(Module::Harita), "KentOS Harita"),
    (Some(Module::Cbs), "KentOS GIS"),
    (Some(Module::Yol), "KentOS Yol"),
];

/// Whether `q` (folded) is in any of the texts.
fn holds(q: &str, texts: &[&str]) -> bool {
    q.is_empty() || texts.iter().any(|t| fold_turkish(t).contains(q))
}

impl Extensions {
    /// The word typed, folded.
    fn folded(&self) -> String {
        fold_turkish(self.query.trim())
    }

    /// An installed extension the word and the program filter let through.
    pub(crate) fn shows_installed(&self, e: &ExtensionView, manifest: Option<&Manifest>) -> bool {
        if let (Some(program), Some(m)) = (self.program, manifest)
            && !m.modules.contains(&program)
        {
            return false;
        }
        let q = self.folded();
        let mut texts: Vec<&str> = vec![e.name.as_str(), e.id.as_str()];
        if let Some(m) = manifest {
            texts.push(m.summary.as_str());
            texts.extend(m.description.as_deref());
            texts.extend(m.keywords.iter().map(String::as_str));
            texts.extend(m.authors.iter().map(String::as_str));
        }
        holds(&q, &texts)
    }

    /// A catalog entry the word and the program filter let through.
    pub(crate) fn shows_entry(&self, entry: &CatalogEntry) -> bool {
        if let Some(program) = self.program
            && !entry.modules.contains(&program)
        {
            return false;
        }
        let q = self.folded();
        let mut texts: Vec<&str> = vec![
            entry.name.as_str(),
            entry.id.as_str(),
            entry.summary.as_str(),
        ];
        texts.extend(entry.description.as_deref());
        texts.extend(entry.keywords.iter().map(String::as_str));
        texts.extend(entry.authors.iter().map(String::as_str));
        holds(&q, &texts)
    }

    /// The installed extensions of a part (Kurulu, Sorunlu), filtered.
    pub(crate) fn installed_of(&self, part: Part) -> Vec<ExtensionView> {
        let Some(registry) = self.registry.as_ref() else {
            return Vec::new();
        };
        registry
            .extensions()
            .into_iter()
            .filter(|e| part != Part::Troubled || self.troubles.iter().any(|t| t.id == e.id))
            .filter(|e| self.shows_installed(e, registry.manifest(&e.id)))
            .collect()
    }

    /// The catalog's entries of a part (Bulunabilir, Güncellenebilir, Yeni), their newest version, filtered.
    pub(crate) fn entries_of(&self, part: Part) -> Vec<&CatalogEntry> {
        let Some(found) = self.catalog.as_ref() else {
            return Vec::new();
        };
        let installed = |id: &str| -> Option<String> {
            self.registry
                .as_ref()?
                .extensions()
                .into_iter()
                .find(|e| e.id == id)
                .map(|e| e.version)
        };
        found
            .catalog
            .latest(true)
            .into_iter()
            .filter(|entry| match part {
                Part::Available => installed(&entry.id).is_none(),
                Part::Updates => is_update(entry, installed(&entry.id).as_deref()),
                Part::New => found.fresh.contains(&seen_key(entry)),
                Part::Installed | Part::Troubled => false,
            })
            .filter(|entry| self.shows_entry(entry))
            .collect()
    }

    /// How many each part holds, filtered as shown.
    pub(crate) fn count(&self, part: Part) -> usize {
        if part.is_catalog() {
            self.entries_of(part).len()
        } else {
            self.installed_of(part).len()
        }
    }
}

/// An entry as the seen file keeps it.
fn seen_key(entry: &CatalogEntry) -> String {
    format!("{}@{}", entry.id, entry.version)
}

/// The entries of `catalog` new since its previous opening, and the seen file written with all of it: a first opening
/// finds everything new. A seen file that cannot be read counts as none seen.
pub(crate) fn fresh_since_last(root: Option<&Path>, catalog: &Catalog) -> BTreeSet<String> {
    let all: BTreeSet<String> = catalog.packages.iter().map(seen_key).collect();
    let Some(root) = root else {
        return BTreeSet::new();
    };
    let file = root.join(SEEN);
    let seen: BTreeSet<String> = std::fs::read_to_string(&file)
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .map(|list| list.into_iter().collect())
        .unwrap_or_default();
    let fresh: BTreeSet<String> = all.difference(&seen).cloned().collect();
    let kept: Vec<String> = seen.union(&all).cloned().collect();
    let _ = std::fs::create_dir_all(root);
    let _ = std::fs::write(&file, serde_json::to_string(&kept).unwrap_or_default());
    fresh
}
