//! `file.export.gis` (docs/adr/0530, 0634 §3): the drawing's layers as a derived dataset. The tables are built in
//! memory (`gis_derive::build`) and written to the format's container: a GeoPackage made in a temporary place and
//! read back, a zip of Shapefiles, GeoJSON; `plan` stops there and `execute` writes the bytes whole or not at all, as
//! `file.export.gpkg` does (docs/adr/0502). A PostGIS database's tables are made in one transaction by the provider.

use std::path::PathBuf;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    CommandWarning, DEFAULT_DERIVATION_TOLERANCE, ExportGis, GisExported, GisFormat, LayerNodeType,
    codes, derivation_tolerance_ok,
};
use kentos_domain::Document;
use kentos_geometry_core::ops::derive::ALGORITHM;
use kentos_native_application::gis_derive::{self, Built, Options};
use kentos_native_application::gis_set;
use kentos_native_application::provider::{ProviderError, TableClash};

use crate::file_commands::{Refused, refuse, target};

/// A derived dataset made: where it goes, its bytes, what it holds and what it says.
pub type GisMade = (PathBuf, Vec<u8>, GisExported, Vec<CommandWarning>);

/// The extensions a format's file takes.
fn extensions(format: GisFormat) -> &'static [&'static str] {
    match format {
        GisFormat::GeoPackage => &["gpkg"],
        GisFormat::Shapefile => &["zip"],
        GisFormat::GeoJson => &["geojson", "json", "zip"],
        GisFormat::PostGis => &[],
    }
}

/// The tolerance, the layers by id and the built dataset, or the refusal.
fn built(doc: &Document, input: &ExportGis) -> Result<(f64, Built), Refused> {
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
    let options = Options {
        tolerance,
        annotations: input.annotations,
    };
    let built = gis_derive::build(doc, &ids, &options);
    if built.tables.is_empty() {
        return Err(refuse(
            codes::NOTHING_TO_EXPORT,
            "Seçilen katmanlarda türetilecek nesne yok: taramalar, kılavuzlar ve benzeri nesnelerin GIS'te geometrisi yok.",
            if input.layers.is_empty() {
                "path"
            } else {
                "layers"
            },
        ));
    }
    Ok((tolerance, built))
}

/// The container's bytes of the built dataset, or why it could not be written.
fn bytes_of(doc: &Document, input: &ExportGis, built: &Built) -> Result<Vec<u8>, Refused> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let unwritable = |why: String| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("GIS türevi yazılamadı: {why}"),
            "path",
        )
    };
    match input.format {
        GisFormat::GeoPackage | GisFormat::PostGis => {
            let tmp = std::env::temp_dir().join(format!(
                "kentos-gis-{}-{}.gpkg",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = std::fs::remove_file(&tmp);
            let written = gis_derive::write_gpkg(built, &tmp);
            let bytes = std::fs::read(&tmp);
            let _ = std::fs::remove_file(&tmp);
            written.map_err(unwritable)?;
            bytes.map_err(|e| unwritable(format!("geri okunamadı: {e}")))
        }
        GisFormat::Shapefile => {
            gis_set::write_shapefile(built, kentos_native_application::shapefile::wkt1_prj(doc))
                .map_err(unwritable)
        }
        GisFormat::GeoJson => {
            let (bytes, zipped) =
                gis_set::write_geojson(built, doc.settings().srid).map_err(unwritable)?;
            let zip_path = input.path.to_ascii_lowercase().ends_with(".zip");
            if zipped != zip_path {
                return Err(refuse(
                    codes::INVALID_FILE_PATH,
                    if zipped {
                        format!(
                            "Türevin {} tablosu var: GeoJSON'da her tablo bir dosyadır, bunlar bir .zip'e yazılır. Yolu .zip ile verin.",
                            built.tables.len()
                        )
                    } else {
                        "Türevin tek tablosu var: bir .geojson dosyasına yazılır. Yolu .geojson ile verin.".into()
                    },
                    "path",
                ));
            }
            Ok(bytes)
        }
    }
}

pub fn make(doc: &Document, input: &ExportGis) -> Result<GisMade, Refused> {
    let (path, _) = target(&input.path, input.overwrite)?;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let allowed = extensions(input.format);
    if !allowed.is_empty() && !allowed.contains(&ext.as_str()) {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "{} biçiminin dosyası .{} uzantılıdır; “{}” değil.",
                format_name(input.format),
                allowed.join(" ya da ."),
                input.path
            ),
            "path",
        ));
    }
    let (tolerance, built) = built(doc, input)?;
    let bytes = bytes_of(doc, input, &built)?;
    let (output, warnings) = answer(
        doc,
        input,
        tolerance,
        built,
        &path.display().to_string(),
        &bytes,
    );
    Ok((path, bytes, output, warnings))
}

fn format_name(f: GisFormat) -> &'static str {
    match f {
        GisFormat::GeoPackage => "GeoPackage",
        GisFormat::Shapefile => "Shapefile",
        GisFormat::GeoJson => "GeoJSON",
        GisFormat::PostGis => "PostGIS",
    }
}

/// The answer and its warnings for a built dataset written as `bytes` to `place`.
fn answer(
    doc: &Document,
    input: &ExportGis,
    tolerance: f64,
    built: Built,
    place: &str,
    bytes: &[u8],
) -> (GisExported, Vec<CommandWarning>) {
    let written = built.written;
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
        path: place.to_owned(),
        bytes: bytes.len() as u64,
        sha256: if bytes.is_empty() {
            String::new()
        } else {
            sha256_hex(bytes)
        },
        algorithm: ALGORITHM.to_owned(),
        tolerance,
        tables: written.tables,
        left: written.left,
        chorded: written.chorded,
        max_deviation: written.max_deviation,
        revision: doc.revision().to_string(),
    };
    let _ = input;
    (output, warnings)
}

/// `file.export.gis` to a PostGIS database (docs/adr/0634 §3): the tables made in `schema` in one transaction, with
/// `write`; without, the names checked against the database's.
pub fn to_database(
    doc: &Document,
    input: &ExportGis,
    write: bool,
) -> Result<(GisExported, Vec<CommandWarning>), Refused> {
    let address = input.path.as_str();
    if !kentos_commands::is_database(address) {
        return Err(refuse(
            codes::INVALID_ADDRESS,
            format!(
                "“{address}” bir PostGIS adresi değil (postgresql://kullanıcı@sunucu:5432/veritabanı)."
            ),
            "path",
        ));
    }
    if let Some(why) = kentos_commands::address_problem(address) {
        return Err(refuse(codes::INVALID_ADDRESS, why, "path"));
    }
    let schema = input
        .schema
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("public")
        .to_owned();
    let (tolerance, built) = built(doc, input)?;
    let Some(db) = kentos_native_application::provider::postgis() else {
        return Err(refuse(
            codes::SOURCE_UNREACHABLE,
            "Bu ortamda PostGIS sağlayıcısı yok; türev veritabanına yazılamaz.",
            "path",
        ));
    };
    let tables = gis_set::postgis_tables(&built, &schema);
    let clash = if input.overwrite {
        TableClash::Replace
    } else {
        TableClash::Refuse
    };
    let failed = |e: ProviderError| match e {
        ProviderError::Exists(name) => refuse(
            codes::TABLE_EXISTS,
            format!(
                "“{name}” tablosu veritabanında var; yenisiyle değiştirmek için overwrite: true verin ya da başka bir şemaya (schema) yazın. Veritabanına dokunulmadı."
            ),
            "path",
        ),
        ProviderError::Unreachable(why) => refuse(
            codes::SOURCE_UNREACHABLE,
            format!("{address} veritabanına ulaşılamadı: {why}. Veritabanına dokunulmadı."),
            "path",
        ),
        other => refuse(
            codes::FILE_UNWRITABLE,
            format!("Veritabanına yazılamadı: {other:?}. Hiçbir tablo yazılmadı."),
            "path",
        ),
    };
    if write {
        let mut go = |_: u64, _: u64| true;
        db.create_tables(address, &tables, clash, 5000, &mut go)
            .map_err(|(_, e)| failed(e))?;
    } else {
        let there: std::collections::HashSet<String> = db
            .tables(address)
            .map_err(failed)?
            .into_iter()
            .map(|t| t.name)
            .collect();
        if clash == TableClash::Refuse
            && let Some(t) = tables.iter().find(|t| there.contains(&t.name))
        {
            return Err(failed(ProviderError::Exists(t.name.clone())));
        }
    }
    let mut built = built;
    for t in &mut built.written.tables {
        t.table = format!("{schema}.{}", t.table);
    }
    Ok(answer(doc, input, tolerance, built, address, &[]))
}

/// The derived dataset written whole, as `file.export.gis` writes it (docs/adr/0530).
pub fn export(
    doc: &Document,
    input: &ExportGis,
) -> Result<(GisExported, Vec<CommandWarning>), Refused> {
    if input.format == GisFormat::PostGis {
        return to_database(doc, input, true);
    }
    let (path, bytes, output, warnings) = make(doc, input)?;
    crate::file_commands::write(&path, &bytes)?;
    Ok((output, warnings))
}
