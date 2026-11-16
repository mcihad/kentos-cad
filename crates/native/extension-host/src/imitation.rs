//! What a package being installed reads as (plan/eklenti.md ADIM 22A, D13; docs/adr/8059 §14): its name, its id or its
//! icon read as an installed extension's, or as an entry's of the catalog the desktop read. A `.kek` sent by mail
//! carries no publisher anyone checked, so every extension but the one it updates is compared and the person
//! installing it reads what was found before Kur. It never refuses on its own: a catalog's publisher gate keeps
//! imitations out of a catalog; this says them where no gate ran.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kentos_extensions::catalog::CatalogEntry;
use kentos_extensions::integrity::sha256_hex;
use kentos_extensions::manifest::Manifest;
use kentos_extensions::resemblance::{Known, Resemblance, resemblances};

/// An installed extension as the comparison reads it: its icon's bytes from its package folder.
pub(crate) fn installed(manifest: &Manifest, package: Option<&Path>) -> Known {
    Known {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        icon: manifest
            .icon
            .as_ref()
            .zip(package)
            .and_then(|(icon, dir)| std::fs::read(dir.join(icon)).ok())
            .map(|bytes| sha256_hex(&bytes)),
        publisher: None,
    }
}

/// A catalog's entry as the comparison reads it: the icon's text is its file's.
pub fn of_entry(entry: &CatalogEntry) -> Known {
    Known {
        id: entry.id.clone(),
        name: entry.name.clone(),
        icon: entry.icon.as_ref().map(|svg| sha256_hex(svg.as_bytes())),
        publisher: None,
    }
}

/// What the package (`manifest`, its `files`) reads as among `known`, an id once: the first given kept.
pub(crate) fn of_package(
    manifest: &Manifest,
    files: &BTreeMap<String, Vec<u8>>,
    known: Vec<Known>,
) -> Vec<Resemblance> {
    let candidate = Known {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        icon: manifest
            .icon
            .as_ref()
            .and_then(|icon| files.get(icon))
            .map(|bytes| sha256_hex(bytes)),
        publisher: None,
    };
    let mut seen = BTreeSet::new();
    let known: Vec<Known> = known
        .into_iter()
        .filter(|k| seen.insert(k.id.clone()))
        .collect();
    resemblances(&candidate, &known)
}
