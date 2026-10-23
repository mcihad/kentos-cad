//! A strict drawing-set delivery reuses the project packager and carries real font/plot bytes.
use std::{collections::BTreeSet, fs, path::Path};
use kentos_commands::{
    PackageProject, PublishSheets, SheetDeliveryResource, SheetDeliveryResources,
    SheetPublishIssue, SheetResourceKind, encoding::sha256_hex,
};
use kentos_domain::Document;
use kentos_native_application::jobs::Progress;
use kentos_sheet::SheetBook;
use crate::{
    file_commands::{Refused, refuse},
    package,
};

const LICENSES: [(&str, &[u8]); 7] = [
    (
        "ArchitectsDaughter-OFL.txt",
        include_bytes!("../../../../apps/desktop/assets/fonts/drawing/ArchitectsDaughter-OFL.txt"),
    ),
    (
        "Arimo-OFL.txt",
        include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Arimo-OFL.txt"),
    ),
    (
        "Barlow-OFL.txt",
        include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Barlow-OFL.txt"),
    ),
    (
        "CourierPrime-OFL.txt",
        include_bytes!("../../../../apps/desktop/assets/fonts/drawing/CourierPrime-OFL.txt"),
    ),
    (
        "IBMPlexMono-OFL.txt",
        include_bytes!("../../../../apps/desktop/assets/fonts/drawing/IBMPlexMono-OFL.txt"),
    ),
    (
        "Overpass-OFL.txt",
        include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Overpass-OFL.txt"),
    ),
    (
        "Quicksand-OFL.txt",
        include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Quicksand-OFL.txt"),
    ),
];
fn entry(path: String, kind: SheetResourceKind, bytes: &[u8]) -> SheetDeliveryResource {
    SheetDeliveryResource {
        path,
        kind,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(bytes),
    }
}
fn issue(message: String) -> SheetPublishIssue {
    SheetPublishIssue{sheet:String::new(),code:"package_incomplete".into(),message,fix:"Eksik/taşınamayan kaynağı düzeltip yeniden denetleyin; yalnız PDF isteniyorsa kaynak paketi seçimini kapatın.".into(),error:true}
}
pub(crate) struct Prepared {
    pub output: SheetDeliveryResources,
    pub issues: Vec<SheetPublishIssue>,
    base: package::Planned,
    generated: Vec<(String, Vec<u8>)>,
}
impl Prepared {
    pub fn plan(
        doc: &Document,
        input: &PublishSheets,
        book: &SheetBook,
        sheets: &[String],
    ) -> Result<Self, Refused> {
        let base = package::plan_nested(
            doc,
            &PackageProject {
                path: input.path.clone(),
                name: Some("cizim.kcad".into()),
                from: input
                    .drawing
                    .as_ref()
                    .and_then(|s| Path::new(s).parent())
                    .map(|p| p.display().to_string()),
            },
        )?;
        let mut issues = base
            .output
            .left
            .iter()
            .map(|l| {
                let why = match l.why {
                    kentos_commands::LeftWhy::Missing => "dosya bulunamadı",
                    kentos_commands::LeftWhy::Database => {
                        "veritabanı kaynağını önce yerel dosyaya aktarın"
                    }
                    kentos_commands::LeftWhy::Address => "ağ kaynağını önce yerel dosyaya kaydedin",
                    kentos_commands::LeftWhy::Substituted => {
                        "özgün yazı tipi yerine başka bir yüz kullanılıyor"
                    }
                    kentos_commands::LeftWhy::Unreadable => "dosya okunamadı",
                };
                issue(format!("“{}” kaynak paketine taşınamadı: {why}.", l.path))
            })
            .collect::<Vec<_>>();
        for layer in doc.layers().leaves() {
            if layer.service.is_some() {
                issues.push(issue(format!("“{}” harita servisi çevrimdışı kaynak olarak taşınamadı; önce yerel veri/altlık üretin.",layer.name)));
            }
        }
        // A complete source package includes unused embedded assets too; their external hrefs cannot be left behind.
        if let Some(kept) = doc.sheets() {
            for a in book
                .assets
                .iter()
                .filter(|a| a.kind == kentos_sheet::model::AssetKind::Svg)
            {
                match kept.pictures.iter().find(|p| p.sha256 == a.sha256) {
                    Some(p) => {
                        if let Err(why) = kentos_plot::validate_svg_source(&p.bytes) {
                            issues.push(issue(format!(
                                "“{}” SVG kaynağı taşınamıyor: {why}",
                                a.name
                            )));
                        }
                    }
                    None => issues.push(issue(format!(
                        "“{}” SVG kaynağının gömülü baytları yok.",
                        a.name
                    ))),
                }
            }
        }
        fn style_svg(v: &serde_json::Value, issues: &mut Vec<SheetPublishIssue>) {
            match v {
                serde_json::Value::String(s) => {
                    let text = s.trim_start_matches('\u{feff}').trim_start();
                    let bytes = if text.starts_with("<svg") || text.starts_with("<?xml") {
                        Some(text.as_bytes().to_vec())
                    } else if text.starts_with("data:image/svg+xml") {
                        match kentos_plot::decode_svg_source_url(text) {
                            Some(b) => Some(b),
                            None => {
                                issues.push(issue("Kitaplıktaki SVG veri adresi okunamadı; geçerli base64 kullanın.".into()));
                                None
                            }
                        }
                    } else {
                        None
                    };
                    if let Some(bytes) = bytes
                        && let Err(why) = kentos_plot::validate_svg_source(&bytes)
                    {
                        issues.push(issue(format!(
                            "Kitaplıktaki SVG kaynağı taşınamıyor: {why}"
                        )));
                    }
                }
                serde_json::Value::Array(a) => {
                    for v in a {
                        style_svg(v, issues)
                    }
                }
                serde_json::Value::Object(o) => {
                    for v in o.values() {
                        style_svg(v, issues)
                    }
                }
                _ => {}
            }
        }
        for item in &doc.styles().items {
            style_svg(item, &mut issues);
        }
        let mut files = base
            .output
            .files
            .iter()
            .map(|f| SheetDeliveryResource {
                path: format!("kaynaklar/{}", f.path),
                kind: SheetResourceKind::Dependency,
                bytes: f.bytes,
                sha256: f.sha256.clone(),
            })
            .collect::<Vec<_>>();
        files.push(SheetDeliveryResource {
            path: "kaynaklar/cizim.kcad".into(),
            kind: SheetResourceKind::Drawing,
            bytes: base.output.project_bytes,
            sha256: base.output.project_sha256.clone(),
        });
        let manifest = base.manifest_bytes();
        files.push(entry(
            "kaynaklar/paket.json".into(),
            SheetResourceKind::Manifest,
            manifest,
        ));
        let mut generated = Vec::new();
        let mut add = |path: String, kind: SheetResourceKind, bytes: Vec<u8>| {
            files.push(entry(path.clone(), kind, &bytes));
            generated.push((path, bytes));
        };
        for (data, (font, weight, italic)) in kentos_plot::fonts::FACES
            .iter()
            .zip(kentos_plot::fonts::FACE_KEYS)
        {
            let suffix = if italic { "-italic" } else { "" };
            add(
                format!("kaynaklar/fontlar/{font}-{weight}{suffix}.ttf"),
                SheetResourceKind::Font,
                data.to_vec(),
            );
        }
        for (name, data) in LICENSES {
            add(
                format!("kaynaklar/fontlar/{name}"),
                SheetResourceKind::License,
                data.to_vec(),
            );
        }
        let mut profiles = book.plot_profiles.iter().collect::<Vec<_>>();
        for id in sheets {
            if let Some(p) = kentos_sheet::plot_profile::for_sheet(book, id)
                .map_err(|e| refuse(&e.code, e.message, "resources"))?
            {
                profiles.push(p);
            }
        }
        for setup in &book.page_setups {
            if let Some(p) = &setup.settings.profile_snapshot {
                profiles.push(p);
            }
        }
        let mut used = BTreeSet::new();
        for profile in profiles {
            let bytes = kentos_sheet::plot_profile::write(profile)
                .map_err(|e| refuse(&e.code, e.message, "resources"))?;
            let hash = sha256_hex(&bytes);
            if used.insert(hash.clone()) {
                add(
                    format!("kaynaklar/baski/{hash}.kplot"),
                    SheetResourceKind::PlotProfile,
                    bytes,
                );
            }
        }
        let bytes = serde_json::to_vec_pretty(
            &serde_json::json!({"schema":"kentos.sheet/1","pageSetups":book.page_setups}),
        )
        .map_err(|e| refuse("file_unwritable", e.to_string(), "resources"))?;
        add(
            "kaynaklar/baski/sayfa-duzenleri.json".into(),
            SheetResourceKind::PageSetups,
            bytes,
        );
        for f in &files {
            if !crate::delivery_integrity::portable_relative(&f.path) {
                issues.push(issue(format!("“{}” teslim yolu başka işletim sistemlerine taşınamıyor; kaynağın dosya adını kısaltıp ayırıcı/özel karakterleri kaldırın.",f.path)));
            }
        }
        let output = SheetDeliveryResources {
            project: "kaynaklar/cizim.kcad".into(),
            bytes: files.iter().map(|f| f.bytes).sum(),
            files,
        };
        Ok(Self {
            output,
            issues,
            base,
            generated,
        })
    }
    pub fn write(&self, root: &Path, progress: Option<&Progress>) -> Result<(), Refused> {
        let total = self.output.files.len() as u64;
        let base_total = self.base.output.files.len() as u64 + 2;
        let base_progress = progress.map(|p| p.span(0, base_total, total));
        package::write_into(&self.base, &root.join("kaynaklar"), base_progress.as_ref())?;
        for (n, (rel, bytes)) in self.generated.iter().enumerate() {
            if progress.is_some_and(Progress::stopped) {
                return Err(refuse(
                    "cancelled",
                    "Kaynak paketi iptal edildi; teslim oluşturulmadı.",
                    "path",
                ));
            }
            let to = root.join(rel);
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| refuse("file_unwritable", e.to_string(), "path"))?;
            }
            crate::files::write_bytes(bytes, &to)
                .map_err(|e| refuse("file_unwritable", e.message, "path"))?;
            if let Some(p) = progress {
                p.set(base_total + n as u64 + 1, total);
            }
        }
        for file in &self.output.files {
            crate::delivery_integrity::file(root, &file.path, file.bytes, &file.sha256)
                .map_err(|s| refuse("integrity_failed", s, "resources"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
