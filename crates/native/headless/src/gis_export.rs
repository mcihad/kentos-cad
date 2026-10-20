//! `file.export.gis` (docs/adr/0530): the drawing's layers as a GeoPackage of derived features. The file is made in a
//! temporary place by `kentos_native_application::gis_derive::write`, its bytes read back; `plan` stops there and
//! `execute` writes them whole or not at all, as `file.export.gpkg` does (docs/adr/0502).

use std::path::PathBuf;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    CommandWarning, DEFAULT_DERIVATION_TOLERANCE, ExportGis, GisExported, LayerNodeType, codes,
    derivation_tolerance_ok,
};
use kentos_domain::Document;
use kentos_geometry_core::ops::derive::ALGORITHM;
use kentos_native_application::gis_derive::{self, Options};

use crate::file_commands::{Refused, refuse, target};

/// A derived dataset made: where it goes, its bytes, what it holds and what it says.
pub type GisMade = (PathBuf, Vec<u8>, GisExported, Vec<CommandWarning>);

pub fn make(doc: &Document, input: &ExportGis) -> Result<GisMade, Refused> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let (path, _) = target(&input.path, input.overwrite)?;
    let tolerance = input.tolerance.unwrap_or(DEFAULT_DERIVATION_TOLERANCE);
    if !derivation_tolerance_ok(tolerance) {
        return Err(refuse(
            codes::INVALID_TOLERANCE,
            format!(
                "Tolerans {tolerance} geçersiz: 0,000001 ile 1 metre arasında olmalı (varsayılan 0,0001)."
            ),
            "tolerance",
        ));
    }
    let mut ids = Vec::new();
    for (i, id) in input.layers.iter().enumerate() {
        let Some(node) = doc.layers().get(id) else {
            return Err(refuse(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                &format!("layers[{i}]"),
            ));
        };
        match node.kind {
            LayerNodeType::Layer => ids.push(id.clone()),
            LayerNodeType::Group => {
                ids.extend(doc.layers().leaves_of(id).iter().map(|n| n.id.clone()))
            }
        }
    }
    let tmp = std::env::temp_dir().join(format!(
        "kentos-gis-{}-{}.gpkg",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&tmp);
    let options = Options {
        tolerance,
        annotations: input.annotations,
    };
    let written = gis_derive::write(doc, &ids, &options, &tmp);
    let bytes = std::fs::read(&tmp);
    let _ = std::fs::remove_file(&tmp);
    let written = written.map_err(|why| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("GIS türevi yazılamadı: {why}"),
            "path",
        )
    })?;
    let bytes = bytes.map_err(|e| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("GIS türevi geri okunamadı: {e}"),
            "path",
        )
    })?;
    if written.tables.is_empty() {
        return Err(refuse(
            codes::NOTHING_TO_EXPORT,
            "Seçilen katmanlarda türetilecek nesne yok: ölçüler, taramalar, kılavuzlar ve benzeri nesnelerin GIS'te geometrisi yok.",
            if input.layers.is_empty() {
                "path"
            } else {
                "layers"
            },
        ));
    }
    let mut said: Vec<String> = Vec::new();
    if !written.left.is_empty() {
        let parts: Vec<String> = written
            .left
            .iter()
            .map(|l| format!("{} {} ({})", l.count, l.kind, l.reason))
            .collect();
        said.push(format!("Türeve yazılmayan nesneler: {}.", parts.join("; ")));
    }
    for (table, from, to) in &written.renamed {
        said.push(format!(
            "“{table}” tablosunda “{from}” sütunu “{to}” adıyla yazıldı: aynı adda (büyük küçük harf ayrı) bir sütun vardı."
        ));
    }
    if written.max_deviation > tolerance * (1.0 + 1e-9) {
        said.push(format!(
            "Bazı yaylar en çok kiriş sayısıyla kesildi: en büyük sapma {} m, istenen tolerans {tolerance} m'nin üstünde.",
            written.max_deviation
        ));
    }
    let mut warnings = Vec::new();
    if !said.is_empty() {
        warnings.push(CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: said.join(" "),
            path: Some("layers".into()),
        });
    }
    let output = GisExported {
        path: path.display().to_string(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        algorithm: ALGORITHM.to_owned(),
        tolerance,
        tables: written.tables,
        left: written.left,
        chorded: written.chorded,
        max_deviation: written.max_deviation,
        revision: doc.revision().to_string(),
    };
    Ok((path, bytes, output, warnings))
}

/// The derived dataset written whole, as `file.export.gis` writes it (docs/adr/0530).
pub fn export(
    doc: &Document,
    input: &ExportGis,
) -> Result<(GisExported, Vec<CommandWarning>), Refused> {
    let (path, bytes, output, warnings) = make(doc, input)?;
    crate::file_commands::write(&path, &bytes)?;
    Ok((output, warnings))
}
