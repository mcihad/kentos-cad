//! A catalog of packages (plan/eklenti.md ADIM 18, D1, D14): `kentos.extension-catalog` v1, a static index file beside
//! the `.kek` files of a folder (the organisation's share, an offline disk) or behind an address. `kentos kek index`
//! writes it from the packages themselves, so what it says is what the inspector read: the manager shows a package's
//! description, change notes, dependencies and reasoned capabilities **before** anything is downloaded, and an update's
//! difference against the installed version.
//!
//! A downloaded package must be the catalog's: its bytes' SHA-256 and its content digest are both in the index and
//! both are checked before installing ([`CatalogEntry::verify`]). The public catalog's signatures are ADIM 22A's; this
//! index is trusted as far as its folder is.

use serde::{Deserialize, Serialize};

use crate::archive::{Limits, inspect};
use crate::integrity::sha256_hex;
use crate::manifest::{
    Compatibility, ExtensionDependency, ExtensionKind, Links, Manifest, Module, RequestedCapability,
};
use crate::problem::Problem;

pub const FORMAT: &str = "kentos.extension-catalog";
pub const SCHEMA_VERSION: u32 = 1;
/// The index's name in a catalog folder.
pub const FILE: &str = "index.json";
/// Characters of change notes kept in the index per package.
const CHANGELOG_CHARS: usize = 16_000;

/// A package as the catalog lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogEntry {
    pub id: String,
    pub version: String,
    pub name: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub kind: ExtensionKind,
    pub modules: Vec<Module>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_id: Option<String>,
    pub license: String,
    #[serde(default, skip_serializing_if = "Links::is_empty")]
    pub links: Links,
    pub compatibility: Compatibility,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<ExtensionDependency>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_capabilities: Vec<RequestedCapability>,
    /// The package's change notes, cut at 16 000 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
    /// The icon's SVG text, when the package has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// The `.kek` file, relative to the index.
    pub file: String,
    /// The file's bytes.
    pub size: u64,
    /// SHA-256 of the file's bytes.
    pub sha256: String,
    /// The package's content digest (`sha256:…`).
    pub digest: String,
}

impl CatalogEntry {
    /// On the preview channel: its version is a pre-release.
    pub fn is_preview(&self) -> bool {
        self.version.contains('-')
    }

    /// Whether downloaded bytes are this entry's package: the file's size and SHA-256, then its content digest.
    pub fn verify(&self, bytes: &[u8], limits: &Limits) -> Result<(), Problem> {
        if bytes.len() as u64 != self.size || sha256_hex(bytes) != self.sha256 {
            return Err(Problem::new(
                "catalog_mismatch",
                format!(
                    "İndirilen {} dosyası katalogdakiyle aynı değil (boyu ya da özeti tutmuyor); kurulmadı. Kataloğu \
                     yenileyip yeniden deneyin.",
                    self.file
                ),
            ));
        }
        let inspection = inspect(bytes, limits);
        if inspection.digest.as_deref() != Some(self.digest.as_str()) {
            return Err(Problem::new(
                "catalog_mismatch",
                format!(
                    "{} paketinin içeriği katalogdakiyle aynı değil; kurulmadı.",
                    self.file
                ),
            ));
        }
        Ok(())
    }
}

/// `index.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalog {
    pub format: String,
    pub schema_version: u32,
    /// What the catalog is ("Örnek Belediyesi eklentileri").
    pub name: String,
    pub packages: Vec<CatalogEntry>,
}

impl Catalog {
    /// Reads `index.json`.
    pub fn read(text: &str) -> Result<Catalog, Problem> {
        let mut catalog: Catalog = serde_json::from_str(text).map_err(|e| {
            Problem::new(
                "catalog_invalid",
                format!(
                    "Katalog dizini okunamadı ({}).",
                    e.to_string().split(" at line ").next().unwrap_or_default()
                ),
            )
            .file(FILE)
        })?;
        if catalog.format != FORMAT || catalog.schema_version > SCHEMA_VERSION {
            return Err(Problem::new(
                "catalog_invalid",
                format!(
                    "Katalog “{}” sürüm {}; bu KentOS “{FORMAT}” sürüm {SCHEMA_VERSION}'i okur.",
                    catalog.format, catalog.schema_version
                ),
            )
            .file(FILE));
        }
        // The index is a file in a share anyone with write access can change: an icon the installer would refuse is
        // not drawn (the window draws the checked subset only).
        for entry in &mut catalog.packages {
            if entry
                .icon
                .as_deref()
                .is_some_and(|svg| crate::icon::problem(svg).is_some())
            {
                entry.icon = None;
            }
        }
        Ok(catalog)
    }

    /// The file as `kentos kek index` writes it: two-space JSON and a newline.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// The newest version of each package, on the stable channel unless `preview`.
    pub fn latest(&self, preview: bool) -> Vec<&CatalogEntry> {
        let mut out: Vec<&CatalogEntry> = Vec::new();
        for entry in self.packages.iter().filter(|e| preview || !e.is_preview()) {
            let version = crate::version::Version::parse(&entry.version);
            match out.iter_mut().find(|o| o.id == entry.id) {
                Some(found) => {
                    if version > crate::version::Version::parse(&found.version) {
                        *found = entry;
                    }
                }
                None => out.push(entry),
            }
        }
        out
    }
}

/// The catalog of these packages (file name, bytes): each inspected; one that does not pass is refused with its
/// problems, so a catalog never lists what would not install. Sorted by id, then version.
pub fn index(
    name: &str,
    packages: &[(String, Vec<u8>)],
    limits: &Limits,
) -> Result<Catalog, Vec<Problem>> {
    let mut entries = Vec::new();
    let mut problems = Vec::new();
    for (file, bytes) in packages {
        let opened = crate::archive::open(bytes, limits);
        let inspection = opened.inspection;
        if !inspection.problems.is_empty() {
            problems.extend(inspection.problems.into_iter().map(|p| {
                let message = format!("{file}: {}", p.message);
                Problem { message, ..p }
            }));
            continue;
        }
        let (Some(manifest), Some(digest)) = (inspection.manifest, inspection.digest) else {
            continue;
        };
        let text = |path: &Option<String>| {
            path.as_ref()
                .and_then(|p| opened.files.get(p))
                .and_then(|b| std::str::from_utf8(b).ok())
                .map(str::to_owned)
        };
        let changelog =
            text(&manifest.changelog_file).map(|t| t.chars().take(CHANGELOG_CHARS).collect());
        let icon = text(&manifest.icon);
        entries.push(entry(manifest, changelog, icon, file, bytes, digest));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    entries.sort_by(|a, b| {
        a.id.cmp(&b.id).then_with(|| {
            crate::version::Version::parse(&a.version)
                .cmp(&crate::version::Version::parse(&b.version))
        })
    });
    if let Some(twice) = entries
        .windows(2)
        .find(|w| w[0].id == w[1].id && w[0].version == w[1].version)
    {
        return Err(vec![Problem::new(
            "catalog_duplicate",
            format!(
                "{} {} iki dosyada var ({} ve {}); birini kaldırın.",
                twice[0].id, twice[0].version, twice[0].file, twice[1].file
            ),
        )]);
    }
    Ok(Catalog {
        format: FORMAT.to_owned(),
        schema_version: SCHEMA_VERSION,
        name: name.to_owned(),
        packages: entries,
    })
}

fn entry(
    m: Manifest,
    changelog: Option<String>,
    icon: Option<String>,
    file: &str,
    bytes: &[u8],
    digest: String,
) -> CatalogEntry {
    CatalogEntry {
        id: m.id,
        version: m.version,
        name: m.name,
        summary: m.summary,
        description: m.description,
        kind: m.kind,
        modules: m.modules,
        keywords: m.keywords,
        authors: m.authors,
        publisher_id: m.publisher_id,
        license: m.license,
        links: m.links,
        compatibility: m.compatibility,
        dependencies: m.dependencies.extensions,
        requested_capabilities: m.requested_capabilities,
        changelog,
        icon,
        file: file.to_owned(),
        size: bytes.len() as u64,
        sha256: sha256_hex(bytes),
        digest,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use super::*;
    use crate::archive::pack;

    fn folder(name: &str) -> BTreeMap<String, Vec<u8>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/extensions/v1")
            .join(name);
        let mut files = BTreeMap::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let relative = path
                        .strip_prefix(&root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/");
                    files.insert(relative, std::fs::read(&path).unwrap());
                }
            }
        }
        files
    }

    fn kek(name: &str) -> Vec<u8> {
        pack(&folder(name), &Limits::default()).unwrap().bytes
    }

    #[test]
    fn the_index_says_what_the_packages_say_and_round_trips() {
        let packages = vec![
            ("tr.ornek.mebis-1.0.0.kek".to_owned(), kek("mebis")),
            ("tr.ornek.altlik-1.0.0.kek".to_owned(), kek("ornek-altlik")),
        ];
        let catalog = index(
            "Örnek Belediyesi eklentileri",
            &packages,
            &Limits::default(),
        )
        .unwrap();
        assert_eq!(
            catalog
                .packages
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["tr.ornek.altlik", "tr.ornek.mebis"]
        );
        let mebis = &catalog.packages[1];
        assert!(
            mebis
                .changelog
                .as_deref()
                .unwrap()
                .starts_with("# Değişiklikler")
        );
        assert!(
            mebis
                .icon
                .as_deref()
                .unwrap()
                .contains("viewBox=\"0 0 20 20\"")
        );
        assert_eq!(mebis.requested_capabilities.len(), 10);
        assert_eq!(
            mebis.digest,
            "sha256:ec9730da3c4841b7ec3b5ff82103e90db52766eb7d3e3c3da47ebec40ac46501"
        );
        assert_eq!(Catalog::read(&catalog.to_json()).unwrap(), catalog);
        // An icon changed in the share after indexing, to one the installer refuses, is not drawn.
        let mut changed = catalog.clone();
        changed.packages[1].icon = Some(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 20 20\"><script>alert(1)</script></svg>"
                .to_owned(),
        );
        assert_eq!(
            Catalog::read(&changed.to_json()).unwrap().packages[1].icon,
            None
        );
        mebis.verify(&packages[0].1, &Limits::default()).unwrap();
        let p = mebis
            .verify(&packages[1].1, &Limits::default())
            .unwrap_err();
        assert_eq!(p.code, "catalog_mismatch");
    }

    #[test]
    fn a_package_that_would_not_install_is_not_listed() {
        let mut broken = folder("ornek-altlik");
        broken.insert("manifest.json".into(), b"{}".to_vec());
        let bytes = kentos_zip::write(
            &broken
                .iter()
                .map(|(p, d)| (p.as_str(), d.as_slice()))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let problems = index(
            "Katalog",
            &[("bozuk.kek".into(), bytes)],
            &Limits::default(),
        )
        .unwrap_err();
        assert!(
            problems
                .iter()
                .all(|p| p.message.starts_with("bozuk.kek: ")),
            "{problems:#?}"
        );
        let twice = vec![
            ("a.kek".to_owned(), kek("ornek-altlik")),
            ("b.kek".to_owned(), kek("ornek-altlik")),
        ];
        assert_eq!(
            index("Katalog", &twice, &Limits::default()).unwrap_err()[0].code,
            "catalog_duplicate"
        );
    }

    #[test]
    fn the_latest_stable_version_unless_a_preview_is_asked_for() {
        let catalog = index(
            "K",
            &[("a.kek".into(), kek("ornek-altlik"))],
            &Limits::default(),
        )
        .unwrap();
        let mut newer = catalog.packages[0].clone();
        newer.version = "1.1.0-beta.1".into();
        let mut older = catalog.packages[0].clone();
        older.version = "0.9.0".into();
        let catalog = Catalog {
            packages: vec![older, catalog.packages[0].clone(), newer],
            ..catalog
        };
        assert_eq!(catalog.latest(false)[0].version, "1.0.0");
        assert_eq!(catalog.latest(true)[0].version, "1.1.0-beta.1");
        let other = r#"{"format": "kentos.extension-catalog", "schemaVersion": 2, "name": "x", "packages": []}"#;
        assert_eq!(Catalog::read(other).unwrap_err().code, "catalog_invalid");
    }
}
