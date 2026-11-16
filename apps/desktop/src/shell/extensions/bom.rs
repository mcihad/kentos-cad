//! An installed extension's software bill of materials, saved where the user says (docs/adr/8057 §91, PY-UV-29): its
//! installed package's lock read as CycloneDX 1.6 JSON (`kentos_extensions::sbom`), the same list `kentos kek sbom` and
//! the catalog's `…/sbom` give; with each package's licence when its environment is prepared (its distributions'
//! `METADATA`). What an organisation's IT keeps for every extension it installs.

use std::collections::BTreeMap;
use std::path::Path;

use iced::Task;

use super::{Event, msg};
use crate::app::{App, Message};

/// The files of the package installed at `folder`, by their paths in it (`/` between folders).
fn files_of(folder: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![folder.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let (Some(rel), Ok(bytes)) = (
                path.strip_prefix(folder).ok().and_then(Path::to_str),
                std::fs::read(&path),
            ) {
                files.insert(rel.replace('\\', "/"), bytes);
            }
        }
    }
    files
}

/// The licences the distributions installed in `venv` declare, by normalised name: `lib/python*/site-packages` (and
/// Windows's `Lib/site-packages`), each `*.dist-info/METADATA`.
pub(crate) fn licences_in(venv: &Path) -> BTreeMap<String, serde_json::Value> {
    let mut sites = vec![venv.join("Lib").join("site-packages")];
    if let Ok(libs) = std::fs::read_dir(venv.join("lib")) {
        sites.extend(libs.flatten().map(|e| e.path().join("site-packages")));
    }
    let mut out = BTreeMap::new();
    for site in sites {
        let Ok(entries) = std::fs::read_dir(&site) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.extension().is_some_and(|e| e == "dist-info") {
                continue;
            }
            if let Some((name, licence)) = std::fs::read_to_string(path.join("METADATA"))
                .ok()
                .and_then(|m| kentos_extensions::sbom::license_of(&m))
            {
                out.insert(name, licence);
            }
        }
    }
    out
}

impl App {
    /// The installed extension `id`'s bill of materials as JSON text and a file name for it; why not otherwise.
    pub(crate) fn extension_bom(&self, id: &str) -> Result<(String, String), String> {
        let registry = self
            .extensions
            .registry
            .as_ref()
            .ok_or_else(|| "Eklentiler açık değil.".to_owned())?;
        let manifest = registry
            .manifest(id)
            .ok_or_else(|| format!("“{id}” kurulu değil."))?;
        let folder = registry.package_dir(id).ok_or_else(|| {
            format!(
                "“{}” eklentisinin kurulu dosyaları bulunamadı.",
                manifest.name
            )
        })?;
        let now = kentos_extensions::sbom::utc_text(std::time::SystemTime::now());
        let mut bom = kentos_extensions::sbom::cyclonedx(
            manifest,
            &files_of(&folder),
            &now,
            env!("CARGO_PKG_VERSION"),
        )
        .ok_or_else(|| {
            format!(
                "“{}” Python paketi kurmuyor: malzeme listesinde listelenecek paket yok.",
                manifest.name
            )
        })?;
        // The licences, when its environment is prepared: the lock does not say them.
        if let Some(super::environment::EnvStatus::Ready(ready)) =
            self.extensions.envs.get(id).and_then(|s| s.status.as_ref())
            && let Some(venv) = ready.python.parent().and_then(Path::parent)
        {
            kentos_extensions::sbom::add_licenses(&mut bom, &licences_in(venv));
        }
        let text = serde_json::to_string_pretty(&bom).unwrap_or_default() + "\n";
        Ok((text, format!("{id}-{}.cdx.json", manifest.version)))
    }

    /// Malzeme listesini kaydet…: where to, then written; the outcome said in the dialog's line.
    pub(crate) fn save_extension_bom(&mut self, id: &str) -> Task<Message> {
        let (text, file) = match self.extension_bom(id) {
            Ok(made) => made,
            Err(why) => {
                self.extensions.note = Some((true, why));
                return Task::none();
            }
        };
        Task::perform(
            async move {
                let to = rfd::AsyncFileDialog::new()
                    .set_title("Malzeme listesini kaydet (SBOM)")
                    .set_file_name(&file)
                    .add_filter("CycloneDX JSON", &["json"])
                    .save_file()
                    .await?;
                let path = to.path().to_path_buf();
                Some(
                    std::fs::write(&path, text)
                        .map(|()| path.display().to_string())
                        .map_err(|e| format!("{} yazılamadı: {e}", path.display())),
                )
            },
            |saved| msg(Event::BomSaved(saved)),
        )
    }
}
