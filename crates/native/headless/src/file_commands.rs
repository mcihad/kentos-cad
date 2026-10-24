//! The commands that write files (docs/adr/0253 §10): `file.export.dxf`
//! and `file.export.pdf`, in every command's shape (`validate`, `plan`,
//! `execute`). They write no document: no undo step, and undo takes no file
//! back. `plan` makes the file's bytes and says what they would be, writing
//! nothing; `execute` writes them whole or not at all (`files::write_bytes`:
//! a temporary file beside the target, made durable, read back, renamed).
//! A write leaves a journal row with the file it wrote (its path, bytes and
//! SHA-256), and a request id is remembered as any command's.

use std::path::{Path, PathBuf};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    BlockFileExported, CallAnswer, CallEnvelope, CallOp, CallRecord, CommandError, CommandResult,
    CommandWarning, DxfExported, ExportBlock, ExportDxf, ExportGpkg, ExportPdf, ExportShp,
    FILE_COMMANDS, FILE_EXPORT_BLOCK, FILE_EXPORT_BLOCK_VERSION, FILE_EXPORT_DXF,
    FILE_EXPORT_DXF_VERSION, FILE_EXPORT_GPKG, FILE_EXPORT_GPKG_VERSION, FILE_EXPORT_PDF,
    FILE_EXPORT_ATLAS, FILE_EXPORT_ATLAS_VERSION, FILE_EXPORT_CSV, FILE_EXPORT_CSV_VERSION, FILE_EXPORT_PDF_VERSION, FILE_EXPORT_SHP, FILE_EXPORT_SHP_VERSION,
    FILE_EXPORT_SLD,
    FILE_EXPORT_SLD_VERSION, FILE_PACKAGE, FILE_PACKAGE_VERSION, FileEffect, GpkgExported,
    GpkgTableExported, LayerNodeType, PackageProject, PdfExported, ReplayOutcome, ReplayStep,
    codes,
};
use kentos_domain::Document;
use kentos_native_application::call::{self, CallState};
use serde_json::Value;

use crate::files;

/// Whether `command` is one of these.
pub fn is_file_command(command: &str) -> bool {
    FILE_COMMANDS.iter().any(|(c, _)| *c == command)
}

/// Why a file command writes nothing: its input or the file refused
/// (`failed`). Boxed: a call is refused at most once.
pub type Refused = Box<CommandError>;

pub(crate) fn refuse(code: &str, message: impl Into<String>, path: &str) -> Refused {
    Box::new(CommandError {
        code: code.to_owned(),
        message: message.into(),
        path: Some(path.to_owned()),
        ..CommandError::default()
    })
}

/// The file's path: absolute, not a folder, in a folder that is there;
/// a file there only with `overwrite`. Whether one is there.
pub(crate) fn target(path: &str, overwrite: bool) -> Result<(PathBuf, bool), Refused> {
    let p = Path::new(path);
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{path}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.dxf)."
            ),
            "path",
        ));
    }
    if p.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("{path} bir klasör; yazılacak dosyanın adını da verin."),
            "path",
        ));
    }
    if !p.parent().is_some_and(Path::is_dir) {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("{path} dosyasının klasörü yok; önce klasörü açın ya da başka bir yol verin."),
            "path",
        ));
    }
    let exists = p.exists();
    if exists && !overwrite {
        return Err(refuse(
            codes::FILE_EXISTS,
            format!("{path} var; üzerine yazmak için overwrite: true verin."),
            "path",
        ));
    }
    Ok((p.to_path_buf(), exists))
}

/// The DXF's bytes and what they hold, or the refusal.
fn dxf(
    doc: &Document,
    input: &ExportDxf,
) -> Result<(PathBuf, bool, Vec<u8>, DxfExported), Refused> {
    dxf_selected(doc, input, None)
}

fn dxf_selected(
    doc: &Document, input: &ExportDxf, uids: Option<&[String]>,
) -> Result<(PathBuf, bool, Vec<u8>, DxfExported), Refused> {
    let (path, exists) = target(&input.path, input.overwrite)?;
    let mut objects: Vec<&kentos_commands::Entity> = match &input.layers {
        None => doc.entities().collect(),
        Some(ids) => {
            let mut chosen = Vec::new();
            for (i, id) in ids.iter().enumerate() {
                let Some(node) = doc.layers().get(id) else {
                    return Err(refuse(
                        codes::LAYER_NOT_FOUND,
                        format!(
                            "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                        ),
                        &format!("layers[{i}]"),
                    ));
                };
                // A group gives its layers.
                let leaves: Vec<String> = match node.kind {
                    LayerNodeType::Layer => vec![id.clone()],
                    LayerNodeType::Group => doc
                        .layers()
                        .leaves_of(id)
                        .iter()
                        .map(|n| n.id.clone())
                        .collect(),
                };
                for leaf in leaves {
                    chosen.extend(doc.by_layer(&leaf));
                }
            }
            chosen
        }
    };
    if let Some(uids) = uids {
        let mut chosen = std::collections::HashSet::new();
        for (i, text) in uids.iter().enumerate() {
            let uid = kentos_domain::Uuid::parse_str(text).map_err(|_| refuse(
                codes::INVALID_UID, "Nesne kimliği geçerli bir UUID değil.", &format!("uids[{i}]")))?;
            let slot = doc.slot_of(uid).ok_or_else(|| refuse(
                codes::ENTITY_NOT_FOUND, "Nesne çizimde yok.", &format!("uids[{i}]")))?;
            chosen.insert(slot.0);
        }
        objects.retain(|e| chosen.contains(&e.base().id));
    }
    let mut seen = std::collections::HashSet::new();
    objects.retain(|e| seen.insert(e.base().id));
    let grouped = crate::export::by_layer(doc, objects);
    let mut dxf_input = crate::export::dxf_input(doc, &grouped);
    let paper_only = input.layers.is_none() && uids.is_none() && dxf_input.layouts.iter().any(|p| !p.entities.is_empty() || !p.viewports.is_empty());
    if grouped.is_empty() && !paper_only {
        return Err(refuse(
            codes::NOTHING_TO_EXPORT,
            "Seçilen katmanlarda nesne yok: DXF'e yazılacak bir şey yok.",
            if input.layers.is_some() {
                "layers"
            } else {
                "path"
            },
        ));
    }
    dxf_input.allow_polyline_loss = input.allow_polyline_loss;
    dxf_input.annotations = input.annotations.unwrap_or_default();
    let layers = u32::try_from(dxf_input.layers.len()).unwrap_or(u32::MAX);
    let (bytes, report) = kentos_formats::dxf::write_checked(&dxf_input)
        .map_err(|e| refuse(e.code, e.message, "allowPolylineLoss"))?;
    let output = DxfExported {
        path: path.display().to_string(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        objects: report.counts.values().sum::<u32>(),
        layers,
        replaced: exists,
        report: serde_json::to_value(&report).unwrap_or(Value::Null),
    };
    Ok((path, exists, bytes, output))
}


/// The same CAD export in a selected DWG binary format, verified before the atomic write.
fn dwg(
    doc: &Document, input: &kentos_commands::ExportDwg,
) -> Result<(PathBuf, bool, Vec<u8>, kentos_commands::DwgExported), Refused> {
    let dxf_input = ExportDxf { allow_polyline_loss: input.allow_polyline_loss, path: input.path.clone(), layers: input.layers.clone(),
        overwrite: input.overwrite, annotations: input.annotations };
    let (path, exists, dxf, mut output) = dxf_selected(doc, &dxf_input, input.uids.as_deref())?;
    let bytes = kentos_dwg::encode(&dxf, input.version)
        .map_err(|e| refuse(e.code, e.message, "path"))?;
    output.bytes = bytes.len() as u64;
    output.sha256 = sha256_hex(&bytes);
    if input.version == kentos_commands::DwgVersion::AC1014
        && let Some(notes) = output.report["notes"].as_array_mut() {
            notes.push(serde_json::json!({"what": "R14 biçimi", "count": 1,
                "reason": "INSUNITS birimi ve nesne çizgi kalınlığı bu sürümde taşınmaz. Koordinatlar metredir; alıcıda metre birimini seçin. Yeni DWG sürümü bu kaybı önler.", "lines": []}));
    }
    output.report["codec"] = serde_json::json!({
        "provider": kentos_dwg::PROVIDER, "revision": kentos_dwg::REVISION,
        "sourceFormat": "DXF", "sourceVersion": "AC1021",
        "targetFormat": "DWG", "targetVersion": input.version.signature(),
        "reopened": true, "independentValidation": false
    });
    Ok((path, exists, bytes, output))
}

/// The desktop and command use the same builder and atomic file writer.
pub fn export_dwg(doc: &Document, input: &kentos_commands::ExportDwg)
    -> Result<kentos_commands::DwgExported, Refused> {
    let (path, _, bytes, output) = dwg(doc, input)?;
    write_guarded(&path, &bytes, input.overwrite)?;
    Ok(output)
}

/// The PDF's bytes and what they hold, or the refusal.
fn pdf(
    doc: &Document,
    input: &ExportPdf,
) -> Result<(PathBuf, bool, Vec<u8>, PdfExported), Refused> {
    let (path, exists) = target(&input.path, input.overwrite)?;
    if doc.sheets().is_none() {
        return Err(refuse(
            codes::NO_SHEETS,
            "Projede pafta yok: önce bir pafta ekleyin (cad.sheets.edit ya da Pafta kipi).",
            "path",
        ));
    }
    let Some(sheets) = kentos_native_application::sheets_edit::sheets_in(doc) else {
        return Err(refuse(
            codes::BOOK_UNREADABLE,
            "Projenin pafta kitabı pafta çekirdeğinin okuyamadığı bir kitap; olduğu gibi kalır. Kitabı masaüstünün Pafta kipinde açıp denetleyin.",
            "path",
        ));
    };
    if sheets.is_empty() {
        return Err(refuse(
            codes::NO_SHEETS,
            "Pafta kitabında pafta yok: önce bir pafta ekleyin.",
            "path",
        ));
    }
    let wanted = input.sheets.clone().unwrap_or_default();
    for (i, id) in wanted.iter().enumerate() {
        if !sheets.iter().any(|s| s.id == *id) {
            return Err(refuse(
                codes::UNKNOWN_SHEET,
                format!("“{id}” paftası kitapta yok."),
                &format!("sheets[{i}]"),
            ));
        }
    }
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let made = kentos_plot::book::book_pdf_at(doc, &wanted, &user, &kentos_plot::book::today(), input.drawing.as_deref())
        .map_err(|e| refuse(codes::PDF_UNWRITABLE, e.message, "path"))?;
    let output = PdfExported {
        path: path.display().to_string(),
        bytes: made.bytes.len() as u64,
        sha256: sha256_hex(&made.bytes),
        sheets: made.sheets,
        replaced: exists,
        findings: serde_json::to_value(&made.findings).unwrap_or(Value::Null),
        notes: made.notes,
    };
    Ok((path, exists, made.bytes, output))
}

/// WBLOCK's drawing (docs/adr/0316 §3): a block's objects, or the objects named, as a drawing of their own with the
/// layers, blocks, styles, library items and settings they use (`exchange::selection`), `base` moved to the
/// origin. A block's objects on its own layer (`""`) go onto a layer “0”. The desktop's Bloğu dosyaya yaz writes
/// the same drawing.
pub fn block_drawing(
    doc: &Document,
    input: &ExportBlock,
) -> Result<(kentos_commands::DocumentSnapshotV2, u32), Refused> {
    use kentos_commands::{EntityId, Vec2};
    let mut whole = doc.to_snapshot_v2();
    let (uids, name, base): (Vec<EntityId>, String, Option<Vec2>) = match (
        &input.block,
        &input.uids,
    ) {
        (Some(id), _) => {
            let Some(def) = doc.block(*id) else {
                return Err(refuse(
                    codes::UNKNOWN_BLOCK,
                    format!(
                        "“{}” kimlikli blok çizimde tanımlı değil. Çizimde tanımlı bir bloğun kimliğini verin.",
                        id.to_text()
                    ),
                    "block",
                ));
            };
            if def.entities.is_empty() {
                return Err(refuse(
                    codes::NOTHING_TO_EXPORT,
                    format!("“{}” bloğunda nesne yok: yazılacak bir şey yok.", def.name),
                    "block",
                ));
            }
            // The block's objects as the drawing's, on their layers or on “0”.
            let known: std::collections::HashSet<String> =
                doc.layers().leaves().iter().map(|n| n.id.clone()).collect();
            let mut zero = false;
            whole.entities = def
                .entities
                .iter()
                .map(|e| {
                    let mut e = e.clone();
                    let b = e.base_mut();
                    if b.layer_id.is_empty() || !known.contains(&b.layer_id) {
                        b.layer_id = "0".into();
                        zero = true;
                    }
                    b.look.by_layer.clear();
                    e
                })
                .collect();
            if zero && !known.contains("0") {
                let layer: kentos_commands::LayerNode = serde_json::from_value(serde_json::json!({
                    "id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true,
                    "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": []
                }))
                .map_err(|e| refuse(codes::FILE_UNWRITABLE, e.to_string(), "block"))?;
                whole.layers.insert(0, layer);
            }
            // Ids of the block and its order, so the same block makes the same file (its plan's bytes).
            whole.uids = (0..whole.entities.len())
                .map(|i| {
                    EntityId(kentos_commands::uuid_v5(
                        &id.0,
                        format!("piece/{i}").as_bytes(),
                    ))
                })
                .collect();
            let uids = whole.uids.clone();
            (uids, def.name.clone(), Some(input.base.unwrap_or(def.base)))
        }
        (None, Some(list)) => {
            if list.is_empty() {
                return Err(refuse(
                    codes::NO_ENTITIES,
                    "Yazılacak nesne verilmedi. Nesnelerin kalıcı kimliklerini verin.",
                    "uids",
                ));
            }
            let mut uids = Vec::with_capacity(list.len());
            for (i, text) in list.iter().enumerate() {
                let Ok(u) = kentos_domain::Uuid::parse_str(text) else {
                    return Err(refuse(
                        codes::INVALID_UID,
                        format!("“{text}” geçerli bir nesne kimliği değil."),
                        &format!("uids[{i}]"),
                    ));
                };
                if doc.slot_of(u).is_none() {
                    return Err(refuse(
                        codes::ENTITY_NOT_FOUND,
                        format!("“{text}” kimlikli nesne çizimde yok."),
                        &format!("uids[{i}]"),
                    ));
                }
                uids.push(EntityId(*u.as_bytes()));
            }
            (uids, doc.name().to_owned(), input.base)
        }
        (None, None) => {
            return Err(refuse(
                codes::NO_BLOCK,
                "Yazılacak blok ya da nesne verilmedi. block ile bir bloğu ya da uids ile nesneleri verin.",
                "block",
            ));
        }
    };
    if let Some(b) = base
        && !(b.x.is_finite() && b.y.is_finite())
    {
        return Err(refuse(
            codes::NOT_FINITE,
            "Taban noktası sonlu bir sayı değil. Koordinatı sonlu bir sayıyla verin.",
            "base",
        ));
    }
    let mut drawing = kentos_domain::exchange::selection(&whole, &uids, &name)
        .map_err(|why| refuse(codes::FILE_UNWRITABLE, why, "path"))?;
    // The base point to the origin.
    if let Some(b) = base.filter(|b| b.x != 0.0 || b.y != 0.0) {
        let m = kentos_geometry_core::geom::affine::translation(-b.x, -b.y);
        for e in &mut drawing.entities {
            let moved = kentos_geometry_core::ops::transform::transform_shape(
                &kentos_native_application::geometry::shape(e),
                &m,
            );
            if let Some(next) = kentos_native_application::geometry::with_shape(e, moved) {
                *e = next;
            }
        }
    }
    drawing.arrays.clear();
    drawing.sheets = None;
    drawing.requests.clear();
    let objects = u32::try_from(drawing.entities.len()).unwrap_or(u32::MAX);
    Ok((drawing, objects))
}

/// WBLOCK's bytes and what they hold, or the refusal (docs/adr/0316 §3).
fn block_file(
    doc: &Document,
    input: &ExportBlock,
) -> Result<(PathBuf, bool, Vec<u8>, BlockFileExported), Refused> {
    let (path, exists) = target(&input.path, input.overwrite)?;
    let (drawing, objects) = block_drawing(doc, input)?;
    let bytes = kentos_kcad::encode_verified(&drawing)
        .map_err(|e| refuse(codes::FILE_UNWRITABLE, e.to_string(), "path"))?;
    let output = BlockFileExported {
        path: path.display().to_string(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        objects,
        blocks: u32::try_from(drawing.blocks.len()).unwrap_or(u32::MAX),
        replaced: exists,
    };
    Ok((path, exists, bytes, output))
}

/// WBLOCK written whole, as `file.export.block` writes it: the desktop's Bloğu dosyaya yaz (docs/adr/0316 §3).
pub fn export_block(doc: &Document, input: &ExportBlock) -> Result<BlockFileExported, Refused> {
    let (path, _, bytes, output) = block_file(doc, input)?;
    write(&path, &bytes)?;
    Ok(output)
}

/// The GeoPackage's bytes, what they hold and what was not written, or the refusal (docs/adr/0502): the layers
/// (groups give their layers) written to a temporary file by the product commands' conversion, read back.
/// A GeoPackage made: where it goes, whether a file is there, its bytes, what it holds and what it says.
type GpkgMade = (PathBuf, bool, Vec<u8>, GpkgExported, Vec<CommandWarning>);

fn gpkg(doc: &Document, input: &ExportGpkg) -> Result<GpkgMade, Refused> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let (path, exists) = target(&input.path, input.overwrite)?;
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
        "kentos-gpkg-{}-{}.gpkg",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&tmp);
    let written = kentos_native_application::geopackage::write(doc, &ids, &tmp);
    let bytes = std::fs::read(&tmp);
    let _ = std::fs::remove_file(&tmp);
    let written = written.map_err(|why| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("GeoPackage yazılamadı: {why}"),
            "path",
        )
    })?;
    let bytes = bytes.map_err(|e| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("GeoPackage geri okunamadı: {e}"),
            "path",
        )
    })?;
    if written.tables.is_empty() {
        return Err(refuse(
            codes::NOTHING_TO_EXPORT,
            "Seçilen katmanlarda GeoPackage'a yazılacak nesne yok: yazılar, ölçüler, bloklar ve taramaların GeoPackage'da geometrisi yok.",
            if input.layers.is_empty() {
                "path"
            } else {
                "layers"
            },
        ));
    }
    let mut warnings = Vec::new();
    if !written.left.is_empty() || written.approximated > 0 || !written.renamed.is_empty() {
        let mut parts: Vec<String> = written
            .left
            .iter()
            .map(|(k, n)| format!("{n} {k}"))
            .collect();
        let mut message = String::new();
        if !parts.is_empty() {
            message.push_str(&format!(
                "GeoPackage'da karşılığı olmayan nesneler yazılmadı: {}.",
                std::mem::take(&mut parts).join(", ")
            ));
        }
        if written.approximated > 0 {
            if !message.is_empty() {
                message.push(' ');
            }
            message.push_str(&format!(
                "{} eğri (elips, eğri, NURBS) 0,1 mm'lik kirişlerle yazıldı.",
                written.approximated
            ));
        }
        for (table, from, to) in &written.renamed {
            if !message.is_empty() {
                message.push(' ');
            }
            message.push_str(&format!(
                "“{table}” tablosunda “{from}” özniteliği “{to}” sütununa yazıldı: aynı adda (büyük küçük harf ayrı) bir sütun vardı."
            ));
        }
        warnings.push(CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message,
            path: Some("layers".into()),
        });
    }
    // What of the relations and looks the file could not carry (docs/adr/0627 §6, §7).
    if !written.profile.left.is_empty() {
        warnings.push(CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: format!(
                "İlişkilerin ya da görünüşlerin bir kısmı yazılmadı: {}.",
                written.profile.left.join("; ")
            ),
            path: Some("layers".into()),
        });
    }
    let output = GpkgExported {
        path: path.display().to_string(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        tables: written
            .tables
            .into_iter()
            .map(|t| GpkgTableExported {
                table: t.table,
                layer: t.layer.or(t.output).unwrap_or_default(),
                rows: t.rows,
                geometry: t.geometry,
            })
            .collect(),
        left: written.left,
        approximated: written.approximated,
        relations: written
            .profile
            .relations
            .into_iter()
            .map(|(mapping, base, related, rows)| kentos_commands::GpkgRelationExported {
                mapping,
                base,
                related,
                rows,
            })
            .collect(),
        styles: written.profile.styles,
    };
    Ok((path, exists, bytes, output, warnings))
}

/// The Shapefiles written whole, as `file.export.shp` writes them (docs/adr/0514).
pub fn export_shp(
    doc: &Document,
    input: &ExportShp,
) -> Result<(kentos_commands::ShpExported, Vec<CommandWarning>), Refused> {
    let made = crate::shp_export::make(doc, input)?;
    crate::shp_export::write(&made)?;
    Ok((made.output, made.warnings))
}

/// The SLD of a layer's look: where it goes, its bytes, what it holds and what it could not say (docs/adr/0516).
fn sld(
    doc: &Document,
    input: &kentos_commands::ExportSld,
) -> Result<
    (
        PathBuf,
        Vec<u8>,
        kentos_commands::SldExported,
        Vec<CommandWarning>,
    ),
    Refused,
> {
    let (path, _) = target(&input.path, input.overwrite)?;
    let (_, written) = kentos_native_application::sld::export(doc, &input.layer)?;
    let bytes = written.xml.into_bytes();
    let warnings = if written.lost.is_empty() {
        Vec::new()
    } else {
        vec![CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: format!("SLD'nin söyleyemediği: {}.", written.lost.join("; ")),
            path: Some("layer".into()),
        }]
    };
    let output = kentos_commands::SldExported {
        path: path.display().to_string(),
        layer: input.layer.clone(),
        rules: written.rules as u32,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        lost: written.lost,
    };
    Ok((path, bytes, output, warnings))
}
/// The QML of a layer's look: where it goes, its bytes, what it holds and what it could not say (docs/adr/0555).
fn qml(
    doc: &Document,
    input: &kentos_commands::ExportQml,
) -> Result<
    (
        PathBuf,
        Vec<u8>,
        kentos_commands::QmlExported,
        Vec<CommandWarning>,
    ),
    Refused,
> {
    let (path, _) = target(&input.path, input.overwrite)?;
    let (_, written) = kentos_native_application::qml::export(doc, &input.layer)?;
    let bytes = written.xml.into_bytes();
    let warnings = if written.lost.is_empty() {
        Vec::new()
    } else {
        vec![CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: format!("QML'in söyleyemediği: {}.", written.lost.join("; ")),
            path: Some("layer".into()),
        }]
    };
    let output = kentos_commands::QmlExported {
        path: path.display().to_string(),
        layer: input.layer.clone(),
        rules: written.rules as u32,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        lost: written.lost,
    };
    Ok((path, bytes, output, warnings))
}

/// The CSV of the layers and its `.csvt` and `.prj`: the files to write (path, bytes), the answer, the warnings
/// (docs/adr/0522).
type CsvMade = (
    Vec<(PathBuf, Vec<u8>)>,
    kentos_commands::CsvExported,
    Vec<CommandWarning>,
);

fn csv(doc: &Document, input: &kentos_commands::ExportCsv) -> Result<CsvMade, Refused> {
    let (path, _) = target(&input.path, input.overwrite)?;
    let made = kentos_native_application::csv_table::export(doc, input)
        .map_err(|(code, message, at)| refuse(&code, message, &at))?;
    let mut files = vec![(path.clone(), made.csv), (path.with_extension("csvt"), made.csvt)];
    if let Some(prj) = made.prj {
        files.push((path.with_extension("prj"), prj));
    }
    for (p, _) in files.iter().skip(1) {
        if p.exists() && !input.overwrite {
            return Err(refuse(
                codes::FILE_EXISTS,
                format!("{} var; üzerine yazmak için overwrite: true verin.", p.display()),
                "path",
            ));
        }
    }
    let mut output = made.output;
    output.path = path.display().to_string();
    output.files = files
        .iter()
        .map(|(p, b)| kentos_commands::CsvFileWritten {
            name: p
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            bytes: b.len() as u64,
            sha256: sha256_hex(b),
        })
        .collect();
    let warnings = if output.lost.is_empty() {
        Vec::new()
    } else {
        vec![CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: format!("CSV'nin tutamadığı: {}.", output.lost.join("; ")),
            path: Some("layers".into()),
        }]
    };
    Ok((files, output, warnings))
}

/// The FlatGeobuf of the layers: the file to write, its bytes, the answer and the warnings (docs/adr/0544).
type FgbMade = (PathBuf, Vec<u8>, kentos_commands::FgbExported, Vec<CommandWarning>);

fn fgb(doc: &Document, input: &kentos_commands::ExportFgb) -> Result<FgbMade, Refused> {
    let (path, _) = target(&input.path, input.overwrite)?;
    let made = kentos_native_application::fgb_table::export(doc, input)
        .map_err(|(code, message, at)| refuse(&code, message, &at))?;
    let mut output = made.output;
    output.path = path.display().to_string();
    let warnings = if output.lost.is_empty() {
        Vec::new()
    } else {
        vec![CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: format!("FlatGeobuf'un tutamadığı: {}.", output.lost.join("; ")),
            path: Some("layers".into()),
        }]
    };
    Ok((path, made.bytes, output, warnings))
}

/// The FlatGeobuf written whole, as `file.export.fgb` writes it (docs/adr/0544).
pub fn export_fgb(
    doc: &Document,
    input: &kentos_commands::ExportFgb,
) -> Result<(kentos_commands::FgbExported, Vec<CommandWarning>), Refused> {
    let (path, bytes, output, warnings) = fgb(doc, input)?;
    write(&path, &bytes)?;
    Ok((output, warnings))
}

/// The GML of the layers: the files to write (the schema first, the GML last), the answer and the warnings
/// (docs/adr/0545).
type GmlMade = (Vec<(PathBuf, Vec<u8>)>, kentos_commands::GmlExported, Vec<CommandWarning>);

fn gml(doc: &Document, input: &kentos_commands::ExportGml) -> Result<GmlMade, Refused> {
    let (path, _) = target(&input.path, input.overwrite)?;
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("xsd"))
    {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "{} .xsd ile bitiyor; GML'in şeması aynı adla .xsd olarak yazılır. GML dosyasına .gml uzantısını verin.",
                input.path
            ),
            "path",
        ));
    }
    let made = kentos_native_application::gml_table::export(doc, input)
        .map_err(|(code, message, at)| refuse(&code, message, &at))?;
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let files: Vec<(PathBuf, Vec<u8>)> = made
        .files
        .into_iter()
        .map(|(name, bytes)| (dir.join(name), bytes))
        .collect();
    for (p, _) in &files {
        if p.exists() && !input.overwrite {
            return Err(refuse(
                codes::FILE_EXISTS,
                format!("{} var; üzerine yazmak için overwrite: true verin.", p.display()),
                "path",
            ));
        }
    }
    let mut output = made.output;
    output.path = path.display().to_string();
    let warnings = if output.lost.is_empty() {
        Vec::new()
    } else {
        vec![CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: format!("GML'in tutamadığı: {}.", output.lost.join("; ")),
            path: Some("layers".into()),
        }]
    };
    Ok((files, output, warnings))
}

/// The GML and its schema written, as `file.export.gml` writes them (docs/adr/0545): the schema first, the GML last,
/// each whole.
pub fn export_gml(
    doc: &Document,
    input: &kentos_commands::ExportGml,
) -> Result<(kentos_commands::GmlExported, Vec<CommandWarning>), Refused> {
    let (files, output, warnings) = gml(doc, input)?;
    for (p, b) in &files {
        write(p, b)?;
    }
    Ok((output, warnings))
}

/// The XYZ file of the layers: the file to write, its bytes, the answer and the warnings (docs/adr/0545).
type XyzMade = (PathBuf, Vec<u8>, kentos_commands::XyzExported, Vec<CommandWarning>);

fn xyz(doc: &Document, input: &kentos_commands::ExportXyz) -> Result<XyzMade, Refused> {
    let (path, _) = target(&input.path, input.overwrite)?;
    let made = kentos_native_application::xyz_table::export(doc, input)
        .map_err(|(code, message, at)| refuse(&code, message, &at))?;
    let mut output = made.output;
    output.path = path.display().to_string();
    let warnings = if output.lost.is_empty() {
        Vec::new()
    } else {
        vec![CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: format!("XYZ'nin tutamadığı: {}.", output.lost.join("; ")),
            path: Some("layers".into()),
        }]
    };
    Ok((path, made.bytes, output, warnings))
}

/// The XYZ file written whole, as `file.export.xyz` writes it (docs/adr/0545).
pub fn export_xyz(
    doc: &Document,
    input: &kentos_commands::ExportXyz,
) -> Result<(kentos_commands::XyzExported, Vec<CommandWarning>), Refused> {
    let (path, bytes, output, warnings) = xyz(doc, input)?;
    write(&path, &bytes)?;
    Ok((output, warnings))
}

/// The CSV and its `.csvt` and `.prj` written, as `file.export.csv` writes them (docs/adr/0522): the sidecars first,
/// the CSV last, each whole.
pub fn export_csv(
    doc: &Document,
    input: &kentos_commands::ExportCsv,
) -> Result<(kentos_commands::CsvExported, Vec<CommandWarning>), Refused> {
    let (files, output, warnings) = csv(doc, input)?;
    for (p, b) in files.iter().rev() {
        write(p, b)?;
    }
    Ok((output, warnings))
}

/// The atlas written as one PDF, as `file.export.atlas` writes it (docs/adr/0534): the pages and the pin of the
/// state they are made from; a bound file that changed meanwhile, or a pin that is not the state, refuses it.
pub fn export_atlas(
    doc: &Document,
    input: &kentos_commands::ExportAtlas,
) -> Result<(kentos_commands::AtlasExported, Vec<CommandWarning>), Refused> {
    let (path, bytes, output, warnings) = atlas(doc, input, true)?;
    write(&path, &bytes)?;
    Ok((output, warnings))
}

/// The layer's look written as an SLD, as `file.export.sld` writes it (docs/adr/0516).
pub fn export_sld(
    doc: &Document,
    input: &kentos_commands::ExportSld,
) -> Result<(kentos_commands::SldExported, Vec<CommandWarning>), Refused> {
    let (path, bytes, output, warnings) = sld(doc, input)?;
    write(&path, &bytes)?;
    Ok((output, warnings))
}
/// The layer's look written as a QML, as `file.export.qml` writes it (docs/adr/0555).
pub fn export_qml(
    doc: &Document,
    input: &kentos_commands::ExportQml,
) -> Result<(kentos_commands::QmlExported, Vec<CommandWarning>), Refused> {
    let (path, bytes, output, warnings) = qml(doc, input)?;
    write(&path, &bytes)?;
    Ok((output, warnings))
}

/// The layers' looks written as a MapLibre style, as `file.export.maplibre` writes it (docs/adr/0560).
pub fn export_maplibre(
    doc: &Document,
    input: &kentos_commands::ExportMaplibre,
) -> Result<(kentos_commands::MaplibreExported, Vec<CommandWarning>), Refused> {
    let made = crate::maplibre_export::make(doc, input)?;
    crate::maplibre_export::write(&made)?;
    Ok((made.output, made.warnings))
}

/// The GeoPackage written whole, as `file.export.gpkg` writes it (docs/adr/0502).
pub fn export_gpkg(
    doc: &Document,
    input: &ExportGpkg,
) -> Result<(GpkgExported, Vec<CommandWarning>), Refused> {
    let (path, _, bytes, output, warnings) = gpkg(doc, input)?;
    write(&path, &bytes)?;
    Ok((output, warnings))
}

/// Writes `bytes` to `path` whole (`files::write_bytes`), or the refusal.
pub(crate) fn write(path: &Path, bytes: &[u8]) -> Result<(), Refused> {
    files::write_bytes(bytes, path)
        .map(|_| ())
        .map_err(|e| refuse(codes::FILE_UNWRITABLE, e.message, "path"))
}

fn write_guarded(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), Refused> {
    files::write_bytes_with_overwrite(bytes, path, overwrite).map(|_| ())
        .map_err(|e| refuse(e.code, e.message, "path"))
}

/// What a file command answers with, written or planned: its output as JSON.
struct Done {
    output: Value,
    effect: Option<FileEffect>,
}

/// The atlas's planned pages, its pin and — unless `bytes` is false — its PDF (docs/adr/0534): the file's path,
/// whether it replaces one, the bytes, the answer and the warnings; the refusal otherwise.
fn atlas(
    doc: &Document,
    input: &kentos_commands::ExportAtlas,
    bytes: bool,
) -> Result<AtlasMade, Refused> {
    let (path, exists) = target(&input.path, input.overwrite)?;
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let date = kentos_plot::book::today();
    let asked = kentos_plot::atlas::AtlasInput {
        sheet: &input.sheet,
        fields: input.fields.as_deref(),
        theme: input.theme.as_deref(),
        time: input.time.as_deref(),
        crs: input.crs,
        pin: input.pin.as_ref(),
        user: &user,
        date: &date,
    };
    let at = |e: kentos_plot::atlas::AtlasError| refuse(&e.code, e.message, &e.path);
    let planned = kentos_plot::atlas::plan(doc, &asked).map_err(at)?;
    let (bytes, findings, notes) = if bytes {
        let made = planned.pdf_at(doc,input.drawing.as_deref()).map_err(at)?;
        (made.bytes, made.findings, made.notes)
    } else {
        (Vec::new(), Vec::new(), planned.notes.clone())
    };
    let output = kentos_commands::AtlasExported {
        path: path.display().to_string(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        replaced: exists,
        sheet: planned.sheet().to_owned(),
        pages: planned.pages.clone(),
        pin: planned.pin.clone(),
        bindings: planned.bindings.clone(),
        fields: planned.fields.clone(),
        withheld: planned.withheld.clone(),
        filtered_out: planned.filtered_out,
        findings: serde_json::to_value(&findings).unwrap_or(Value::Null),
        notes,
    };
    let warnings = planned
        .warnings
        .iter()
        .map(|w| CommandWarning {
            code: w.code.clone(),
            message: w.message.clone(),
            path: Some(w.path.clone()),
        })
        .collect();
    Ok((path, bytes, output, warnings))
}

type AtlasMade = (
    PathBuf,
    Vec<u8>,
    kentos_commands::AtlasExported,
    Vec<CommandWarning>,
);

/// A refusal as the protocol answers it: a conflict for a code of that kind (`pin_mismatch`, `source_changed`).
pub(crate) fn refused_result(error: CommandError) -> CommandResult<Value> {
    let conflict = codes::info(&error.code).is_some_and(|c| c.kind == codes::CodeKind::Conflict);
    if conflict {
        CommandResult::Conflict { error }
    } else {
        CommandResult::Failed { error }
    }
}

/// Runs `op` of the file command `command`; its answer, and the file it
/// wrote (execute).
fn typed_version_loss(command: &str, output: &Value) -> bool {
    command == kentos_commands::FILE_EXPORT_DWG && output["report"]["codec"]["targetVersion"] == "AC1014"
}

fn run_typed(
    doc: &Document,
    command: &str,
    op: CallOp,
    input: Value,
) -> Result<(CommandResult<Value>, Option<FileEffect>), String> {
    run_typed_with_progress(doc,command,op,input,None)
}
fn run_typed_with_progress(doc:&Document,command:&str,op:CallOp,input:Value,progress:Option<&kentos_native_application::jobs::Progress>)->Result<(CommandResult<Value>,Option<FileEffect>),String>{
    if command==kentos_commands::FILE_PREFLIGHT_SHEETS {
        let typed:kentos_commands::PreflightSheets=serde_json::from_value(input).map_err(|e|e.to_string())?;
        return Ok((crate::sheet_preflight::run(doc,&typed,op,progress)?,None));
    }
    if command==kentos_commands::FILE_PUBLISH_SHEETS {
        let typed:kentos_commands::PublishSheets=serde_json::from_value(input).map_err(|e|e.to_string())?;
        return Ok(crate::publish::run(doc,&typed,op,progress));
    }
    macro_rules! go {
        ($input:ty, $make:ident) => {{
            let typed: $input = serde_json::from_value(input).map_err(|e| {
                format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
            })?;
            let made = $make(doc, &typed);
            let done = match made {
                Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
                Ok((path, _, bytes, output)) => match op {
                    CallOp::Validate => Done {
                        output: Value::Null,
                        effect: None,
                    },
                    CallOp::Plan => Done {
                        output: serde_json::to_value(&output).unwrap_or(Value::Null),
                        effect: None,
                    },
                    CallOp::Execute => {
                        if let Err(refused) = write_guarded(&path, &bytes, typed.overwrite) {
                            return Ok((CommandResult::Failed { error: *refused }, None));
                        }
                        Done {
                            effect: Some(FileEffect {
                                path: output.path.clone(),
                                bytes: output.bytes,
                                sha256: output.sha256.clone(),
                            }),
                            output: serde_json::to_value(&output).unwrap_or(Value::Null),
                        }
                    }
                },
            };
            let warnings = if command == kentos_commands::FILE_EXPORT_DWG
                && (done.output["report"]["skipped"].as_array().is_some_and(|a| !a.is_empty())
                    || typed_version_loss(command, &done.output)) {
                vec![CommandWarning { code: codes::EXPORT_LOSSES.into(),
                    message: "DWG aktarımında kayıp var; yazılanlar ve sürüm kayıpları için report alanına bakın.".into(),
                    path: Some("report".into()) }]
            } else { Vec::new() };
            Ok((
                CommandResult::Completed {
                    output: done.output,
                    warnings,
                },
                done.effect,
            ))
        }};
    }
    if command == FILE_PACKAGE {
        let typed: PackageProject = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        // A validate reads no database: a table's copy is made for a plan and an execute (docs/adr/0596 §2).
        let planned = match crate::package::plan_reading(doc, &typed, op != CallOp::Validate) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(p) => p,
        };
        let value = serde_json::to_value(&planned.output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (
                CommandResult::Completed {
                    output: value,
                    warnings: planned.warnings,
                },
                None,
            ),
            CallOp::Execute => {
                if let Err(refused) = crate::package::write(&planned) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                let effect = FileEffect {
                    path: planned.output.project.clone(),
                    bytes: planned.output.project_bytes,
                    sha256: planned.output.project_sha256.clone(),
                };
                (
                    CommandResult::Completed {
                        output: value,
                        warnings: planned.warnings,
                    },
                    Some(effect),
                )
            }
        });
    }
    if command == FILE_EXPORT_ATLAS {
        let typed: kentos_commands::ExportAtlas = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        // A validate plans the pages and the pin; a PDF is made only for a plan and an execute.
        let made = match atlas(doc, &typed, !matches!(op, CallOp::Validate)) {
            Err(refused) => return Ok((refused_result(*refused), None)),
            Ok(made) => made,
        };
        let (path, bytes, output, warnings) = made;
        let value = serde_json::to_value(&output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (CommandResult::Completed { output: value, warnings }, None),
            CallOp::Execute => {
                if let Err(refused) = write(&path, &bytes) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                (
                    CommandResult::Completed { output: value, warnings },
                    Some(FileEffect {
                        path: output.path.clone(),
                        bytes: output.bytes,
                        sha256: output.sha256.clone(),
                    }),
                )
            }
        });
    }
    if command == FILE_EXPORT_CSV {
        let typed: kentos_commands::ExportCsv = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let (files, output, warnings) = match csv(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (CommandResult::Completed { output: value, warnings }, None),
            CallOp::Execute => {
                for (p, b) in files.iter().rev() {
                    if let Err(refused) = write(p, b) {
                        return Ok((CommandResult::Failed { error: *refused }, None));
                    }
                }
                let csv = &output.files[0];
                (
                    CommandResult::Completed { output: value, warnings },
                    Some(FileEffect {
                        path: output.path.clone(),
                        bytes: csv.bytes,
                        sha256: csv.sha256.clone(),
                    }),
                )
            }
        });
    }
    if command == kentos_commands::FILE_EXPORT_GIS {
        let typed: kentos_commands::ExportGis = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let (path, bytes, output, warnings) = match crate::gis_export::make(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (CommandResult::Completed { output: value, warnings }, None),
            CallOp::Execute => {
                if let Err(refused) = write(&path, &bytes) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                (
                    CommandResult::Completed { output: value, warnings },
                    Some(FileEffect {
                        path: output.path.clone(),
                        bytes: output.bytes,
                        sha256: output.sha256.clone(),
                    }),
                )
            }
        });
    }
    if command == kentos_commands::FILE_EXPORT_FGB {
        let typed: kentos_commands::ExportFgb = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let (path, bytes, output, warnings) = match fgb(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (CommandResult::Completed { output: value, warnings }, None),
            CallOp::Execute => {
                if let Err(refused) = write(&path, &bytes) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                (
                    CommandResult::Completed { output: value, warnings },
                    Some(FileEffect {
                        path: output.path.clone(),
                        bytes: output.bytes,
                        sha256: output.sha256.clone(),
                    }),
                )
            }
        });
    }
    if command == kentos_commands::FILE_EXPORT_GML {
        let typed: kentos_commands::ExportGml = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let (files, output, warnings) = match gml(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (CommandResult::Completed { output: value, warnings }, None),
            CallOp::Execute => {
                for (p, b) in &files {
                    if let Err(refused) = write(p, b) {
                        return Ok((CommandResult::Failed { error: *refused }, None));
                    }
                }
                let main = &output.files[0];
                (
                    CommandResult::Completed { output: value, warnings },
                    Some(FileEffect {
                        path: output.path.clone(),
                        bytes: main.bytes,
                        sha256: main.sha256.clone(),
                    }),
                )
            }
        });
    }
    if command == kentos_commands::FILE_EXPORT_XYZ {
        let typed: kentos_commands::ExportXyz = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let (path, bytes, output, warnings) = match xyz(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (CommandResult::Completed { output: value, warnings }, None),
            CallOp::Execute => {
                if let Err(refused) = write(&path, &bytes) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                (
                    CommandResult::Completed { output: value, warnings },
                    Some(FileEffect {
                        path: output.path.clone(),
                        bytes: output.bytes,
                        sha256: output.sha256.clone(),
                    }),
                )
            }
        });
    }
    if command == FILE_EXPORT_SLD {
        let typed: kentos_commands::ExportSld = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let (path, bytes, output, warnings) = match sld(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (
                CommandResult::Completed {
                    output: value,
                    warnings,
                },
                None,
            ),
            CallOp::Execute => {
                if let Err(refused) = write(&path, &bytes) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                (
                    CommandResult::Completed {
                        output: value,
                        warnings,
                    },
                    Some(FileEffect {
                        path: output.path.clone(),
                        bytes: output.bytes,
                        sha256: output.sha256.clone(),
                    }),
                )
            }
        });
    }
    if command == kentos_commands::FILE_EXPORT_MAPLIBRE {
        let typed: kentos_commands::ExportMaplibre = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let made = match crate::maplibre_export::make(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&made.output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (
                CommandResult::Completed {
                    output: value,
                    warnings: made.warnings,
                },
                None,
            ),
            CallOp::Execute => {
                if let Err(refused) = crate::maplibre_export::write(&made) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                (
                    CommandResult::Completed {
                        output: value,
                        warnings: made.warnings,
                    },
                    Some(FileEffect {
                        path: made.output.path.clone(),
                        bytes: made.output.bytes,
                        sha256: made.output.sha256.clone(),
                    }),
                )
            }
        });
    }
    if command == kentos_commands::FILE_EXPORT_QML {
        let typed: kentos_commands::ExportQml = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let (path, bytes, output, warnings) = match qml(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (
                CommandResult::Completed {
                    output: value,
                    warnings,
                },
                None,
            ),
            CallOp::Execute => {
                if let Err(refused) = write(&path, &bytes) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                (
                    CommandResult::Completed {
                        output: value,
                        warnings,
                    },
                    Some(FileEffect {
                        path: output.path.clone(),
                        bytes: output.bytes,
                        sha256: output.sha256.clone(),
                    }),
                )
            }
        });
    }
    if command == FILE_EXPORT_SHP {
        let typed: ExportShp = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let made = match crate::shp_export::make(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&made.output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (
                CommandResult::Completed {
                    output: value,
                    warnings: made.warnings,
                },
                None,
            ),
            CallOp::Execute => {
                if let Err(refused) = crate::shp_export::write(&made) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                let (path, bytes, sha256) = crate::shp_export::effect(&made);
                (
                    CommandResult::Completed {
                        output: value,
                        warnings: made.warnings,
                    },
                    Some(FileEffect {
                        path,
                        bytes,
                        sha256,
                    }),
                )
            }
        });
    }
    if command == FILE_EXPORT_GPKG {
        let typed: ExportGpkg = serde_json::from_value(input).map_err(|e| {
            format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
        })?;
        let (path, _, bytes, output, warnings) = match gpkg(doc, &typed) {
            Err(refused) => return Ok((CommandResult::Failed { error: *refused }, None)),
            Ok(made) => made,
        };
        let value = serde_json::to_value(&output).unwrap_or(Value::Null);
        return Ok(match op {
            CallOp::Validate => (
                CommandResult::Completed {
                    output: Value::Null,
                    warnings: Vec::new(),
                },
                None,
            ),
            CallOp::Plan => (
                CommandResult::Completed {
                    output: value,
                    warnings,
                },
                None,
            ),
            CallOp::Execute => {
                if let Err(refused) = write(&path, &bytes) {
                    return Ok((CommandResult::Failed { error: *refused }, None));
                }
                (
                    CommandResult::Completed {
                        output: value,
                        warnings,
                    },
                    Some(FileEffect {
                        path: output.path.clone(),
                        bytes: output.bytes,
                        sha256: output.sha256.clone(),
                    }),
                )
            }
        });
    }
    match command {
        FILE_EXPORT_DXF => go!(ExportDxf, dxf),
        kentos_commands::FILE_EXPORT_DWG => go!(kentos_commands::ExportDwg, dwg),
        FILE_EXPORT_PDF => go!(ExportPdf, pdf),
        FILE_EXPORT_BLOCK => go!(ExportBlock, block_file),
        other => Err(format!("“{other}” bir dosya komutu değil.")),
    }
}

fn version_of(command: &str) -> u32 {
    match command {
        kentos_commands::FILE_EXPORT_DWG => kentos_commands::FILE_EXPORT_DWG_VERSION,
        kentos_commands::FILE_PUBLISH_SHEETS=>kentos_commands::FILE_PUBLISH_SHEETS_VERSION,
        kentos_commands::FILE_PREFLIGHT_SHEETS=>kentos_commands::FILE_PREFLIGHT_SHEETS_VERSION,
        FILE_EXPORT_PDF => FILE_EXPORT_PDF_VERSION,
        FILE_EXPORT_BLOCK => FILE_EXPORT_BLOCK_VERSION,
        FILE_EXPORT_GPKG => FILE_EXPORT_GPKG_VERSION,
        FILE_EXPORT_SHP => FILE_EXPORT_SHP_VERSION,
        FILE_EXPORT_SLD => FILE_EXPORT_SLD_VERSION,
        kentos_commands::FILE_EXPORT_QML => kentos_commands::FILE_EXPORT_QML_VERSION,
        kentos_commands::FILE_EXPORT_MAPLIBRE => kentos_commands::FILE_EXPORT_MAPLIBRE_VERSION,
        FILE_EXPORT_CSV => FILE_EXPORT_CSV_VERSION,
        kentos_commands::FILE_EXPORT_FGB => kentos_commands::FILE_EXPORT_FGB_VERSION,
        kentos_commands::FILE_EXPORT_GML => kentos_commands::FILE_EXPORT_GML_VERSION,
        kentos_commands::FILE_EXPORT_XYZ => kentos_commands::FILE_EXPORT_XYZ_VERSION,
        FILE_EXPORT_ATLAS => FILE_EXPORT_ATLAS_VERSION,
        kentos_commands::FILE_EXPORT_GIS => kentos_commands::FILE_EXPORT_GIS_VERSION,
        FILE_PACKAGE => FILE_PACKAGE_VERSION,
        _ => FILE_EXPORT_DXF_VERSION,
    }
}

/// The input as its type writes it back, or the refusal.
fn normalize(command: &str, input: &Value) -> Result<Value, String> {
    let text = |e: serde_json::Error| {
        format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın.")
    };
    match command {
        kentos_commands::FILE_EXPORT_DWG => serde_json::from_value::<kentos_commands::ExportDwg>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        FILE_EXPORT_DXF => serde_json::from_value::<ExportDxf>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        FILE_EXPORT_BLOCK => serde_json::from_value::<ExportBlock>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        FILE_EXPORT_GPKG => serde_json::from_value::<ExportGpkg>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        FILE_EXPORT_SHP => serde_json::from_value::<ExportShp>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        FILE_EXPORT_ATLAS => serde_json::from_value::<kentos_commands::ExportAtlas>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        FILE_EXPORT_CSV => serde_json::from_value::<kentos_commands::ExportCsv>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        kentos_commands::FILE_EXPORT_FGB => {
            serde_json::from_value::<kentos_commands::ExportFgb>(input.clone())
                .map_err(text)
                .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string()))
        }
        kentos_commands::FILE_EXPORT_GML => {
            serde_json::from_value::<kentos_commands::ExportGml>(input.clone())
                .map_err(text)
                .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string()))
        }
        kentos_commands::FILE_EXPORT_XYZ => {
            serde_json::from_value::<kentos_commands::ExportXyz>(input.clone())
                .map_err(text)
                .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string()))
        }
        kentos_commands::FILE_EXPORT_GIS => {
            serde_json::from_value::<kentos_commands::ExportGis>(input.clone())
                .map_err(text)
                .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string()))
        }
        FILE_EXPORT_SLD => serde_json::from_value::<kentos_commands::ExportSld>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        kentos_commands::FILE_EXPORT_MAPLIBRE => {
            serde_json::from_value::<kentos_commands::ExportMaplibre>(input.clone())
                .map_err(text)
                .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string()))
        }
        kentos_commands::FILE_EXPORT_QML => serde_json::from_value::<kentos_commands::ExportQml>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        kentos_commands::FILE_PREFLIGHT_SHEETS=>serde_json::from_value::<kentos_commands::PreflightSheets>(input.clone()).map_err(text).and_then(|t|serde_json::to_value(t).map_err(|e|e.to_string())),
        kentos_commands::FILE_PUBLISH_SHEETS=>serde_json::from_value::<kentos_commands::PublishSheets>(input.clone()).map_err(text).and_then(|t|serde_json::to_value(t).map_err(|e|e.to_string())),
        FILE_PACKAGE => serde_json::from_value::<PackageProject>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
        _ => serde_json::from_value::<ExportPdf>(input.clone())
            .map_err(text)
            .and_then(|t| serde_json::to_value(t).map_err(|e| e.to_string())),
    }
}

/// A file command by the protocol (docs/adr/0253 §4, §10): its call's
/// faults, its request, its answer; an execute's write in the journal.
pub fn call(doc: &mut Document, envelope: CallEnvelope) -> CallAnswer {
    let command = envelope.command.clone();
    let version = version_of(&command);
    let failed = |code: &str, message: String, path: &str| {
        CallAnswer::of(CommandResult::Failed {
            error: CommandError {
                code: code.to_owned(),
                message,
                path: Some(path.to_owned()),
                ..CommandError::default()
            },
        })
    };
    let input = match normalize(&command, &envelope.input) {
        Ok(v) => v,
        Err(message) => return failed(codes::INVALID_INPUT, message, "input"),
    };
    let state = match call::preflight(doc, &envelope, version, &input) {
        Ok(state) => state,
        Err(answer) => return crate::delivery_integrity::checked_replay(&command,*answer),
    };
    let mut answer = run_answer(doc, &command, version, envelope.op, &input, &state);
    call::complete(&mut answer, &input);
    answer
}

fn run_answer(
    doc: &mut Document,
    command: &str,
    version: u32,
    op: CallOp,
    input: &Value,
    state: &CallState,
) -> CallAnswer {
    match run_typed(doc, command, op, input.clone()) {
        Ok((result, effect)) => {
            let mut answer = CallAnswer::of(result);
            if let Some(effect) = effect {
                let call = call::record_effect(doc, command, version, input, vec![effect], state);
                answer.call = Some(call);
                if state.request().is_some()
                    && let Ok(text) = serde_json::to_string(&answer)
                {
                    doc.set_request_answer(call, text);
                }
            }
            answer
        }
        Err(message) => CallAnswer::of(CommandResult::Failed {
            error: CommandError {
                code: codes::INVALID_INPUT.into(),
                message,
                path: Some("input".into()),
                ..CommandError::default()
            },
        }),
    }
}

/// A file command run as a job (docs/adr/0253 §9): its file made and
/// written on a reading copy on another thread, its row recorded on the
/// document when the job is looked at.
pub fn job_work(
    envelope: CallEnvelope,
) -> impl FnOnce(
    Document,
    &kentos_native_application::jobs::Progress,
) -> kentos_native_application::jobs::JobOutcome
+ Send
+ 'static {
    move |copy, progress| {
        use kentos_native_application::jobs::{Effect, JobOutcome};
        let command = envelope.command.clone();
        let version = version_of(&command);
        let input = match normalize(&command, &envelope.input) {
            Ok(v) => v,
            Err(message) => {
                return JobOutcome {
                    answer: CallAnswer::of(CommandResult::Failed {
                        error: CommandError {
                            code: codes::INVALID_INPUT.into(),
                            message,
                            path: Some("input".into()),
                            ..CommandError::default()
                        },
                    }),
                    copy: None,
                    effect: None,
                };
            }
        };
        let state = match call::preflight(&copy, &envelope, version, &input) {
            Ok(state) => state,
            Err(answer) => {
                return JobOutcome {
                    answer: crate::delivery_integrity::checked_replay(&command,*answer),
                    copy: None,
                    effect: None,
                };
            }
        };
        match run_typed_with_progress(&copy, &command, envelope.op, input.clone(),Some(progress)) {
            Ok((result, effect)) => {
                let mut answer = CallAnswer::of(result);
                call::complete(&mut answer, &input);
                // The copy is not the document: the row is recorded there when the job is staged.
                JobOutcome {
                    answer,
                    copy: None,
                    effect: effect.map(|e| Effect {
                        command,
                        version,
                        input,
                        effects: vec![e],
                        state,
                    }),
                }
            }
            Err(message) => JobOutcome {
                answer: CallAnswer::of(CommandResult::Failed {
                    error: CommandError {
                        code: codes::INVALID_INPUT.into(),
                        message,
                        path: Some("input".into()),
                        ..CommandError::default()
                    },
                }),
                copy: None,
                effect: None,
            },
        }
    }
}

/// A file command's plain answer (`Session::run`, the `run` request): the
/// `CommandResult` as the catalog writes it; an execute's write in the journal.
pub fn run(doc: &mut Document, command: &str, op: CallOp, input: Value) -> Result<Value, String> {
    let normalized = normalize(command, &input)?;
    let state = CallState::default();
    let answer = run_answer(doc, command, version_of(command), op, &normalized, &state);
    serde_json::to_value(&answer.result).map_err(|e| e.to_string())
}

/// A journal row that wrote a file, replayed (docs/adr/0253 §10): the file
/// planned again on `doc` as the replay has it and not written; `same` when
/// its bytes' SHA-256 is the one written then, `differs` when not (a PDF
/// carries the day it is made), `skipped` when it cannot be planned here
/// (its folder is not on this machine, …), with the refusal.
pub fn replay_row(doc: &mut Document, row: &CallRecord) -> Option<ReplayStep> {
    let command = row.command.as_deref()?;
    let mut input = row.input.clone()?;
    if command==kentos_commands::FILE_PUBLISH_SHEETS {
        return Some(crate::delivery_integrity::replay_row(row));
    }
    // A plan writes nothing: the file written then may be there.
    input["overwrite"] = Value::Bool(true);
    let (answer, _) = run_typed(doc, command, CallOp::Plan, input).ok()?;
    let mut step = ReplayStep {
        seq: row.seq,
        command: Some(command.to_owned()),
        outcome: ReplayOutcome::Same,
        note: None,
        error: None,
        created: Vec::new(),
    };
    match answer {
        CommandResult::Completed { output, .. } => {
            let now = output["sha256"].as_str().unwrap_or_default();
            let then = row.effects.first().map_or("", |e| e.sha256.as_str());
            if now != then {
                step.outcome = ReplayOutcome::Differs;
                step.note = Some(format!(
                    "Dosya yazılmadı; yazılsaydı başka olurdu (SHA-256 {now}, kayıtta {then})."
                ));
            } else {
                step.note = Some("Dosya yazılmadı; yazılsaydı aynısı olurdu.".to_owned());
            }
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            step.outcome = ReplayOutcome::Skipped;
            step.note = Some(format!(
                "Dosya burada planlanamadı ({}): karşılaştırılmadı.",
                error.code
            ));
            step.error = Some(error);
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => return None,
    }
    Some(step)
}
