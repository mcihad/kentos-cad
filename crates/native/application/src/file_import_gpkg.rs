//! `file.import.gpkg` v1 (docs/adr/0502, plan/gis.md CBS-03): a GeoPackage's feature and attribute tables taken into
//! the drawing as one undo step “GeoPackage: ad.gpkg”, each row an object or a record with its lasting id
//! (`geopackage::read`). A table goes onto the layer `layers` names, else onto the drawing's layer of the same name,
//! else onto a new layer, with the table's columns as its fields, in a group named after the file. A file in another
//! coordinate system is refused unless `reproject` asks for its coordinates to be moved into the project's (the
//! project's datum choices taken; arcs and vertices the move does not reach are left out, said). The shared cases are
//! `fixtures/commands/v1/file.import.gpkg.json` (`scripts/fixtures/gpkg_import_cases.py`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, GpkgImported, GpkgTableImported, ImportGpkg,
    ImportResult, ImportedLayer, LayerNodeType,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geopackage::{self, Read};
use crate::import_apply::{ImportPlan, LayerTarget, apply_import_with_uids, layer_named};

/// The undo step's name: “GeoPackage: ad.gpkg”.
pub fn step_name(file: &str) -> String {
    format!("GeoPackage: {file}")
}

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

/// Checks `input`, reading the file.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportGpkg) -> CommandResult<()> {
    let mut copy = cx.doc.clone();
    match run(&mut copy, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What the import would take, writing nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportGpkg) -> CommandResult<GpkgImported> {
    let mut copy = cx.doc.clone();
    match run(&mut copy, input) {
        Ok((mut output, warnings)) => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        Err(stop) => stop.into(),
    }
}

/// Takes the file in.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportGpkg) -> CommandResult<GpkgImported> {
    match run(cx.doc, &input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

fn read_file(input: &ImportGpkg) -> Result<Read, Stop> {
    let path = Path::new(&input.path);
    if input.path.trim().is_empty() || !path.is_absolute() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” mutlak bir dosya yolu değil; okunacak dosyanın tam yolunu verin (/klasör/ad.gpkg).",
                input.path
            ),
            "path",
        ));
    }
    if path.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "{} bir klasör; okunacak GeoPackage dosyasının adını da verin.",
                input.path
            ),
            "path",
        ));
    }
    if !path.exists() {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("{} yok; dosyanın yolunu denetleyin.", input.path),
            "path",
        ));
    }
    let wanted = (!input.tables.is_empty()).then_some(&input.tables[..]);
    geopackage::read(path, wanted).map_err(|why| {
        if let Some(i) = input
            .tables
            .iter()
            .position(|t| why == format!("“{t}” tablosu dosyada yok"))
        {
            refuse(
                codes::TABLE_NOT_FOUND,
                format!("{why}; dosyanın tablolarından birinin adını verin."),
                &format!("tables[{i}]"),
            )
        } else {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("{} GeoPackage olarak okunamadı: {why}", input.path),
                "path",
            )
        }
    })
}

/// The drawing's layer a table goes onto.
fn target(
    doc: &Document,
    input: &ImportGpkg,
    t: &geopackage::TableRead,
) -> Result<LayerTarget, Stop> {
    if let Some(id) = input.layers.get(&t.name) {
        let at = format!("layers.{}", t.name);
        return match doc.layers().get(id) {
            None => Err(refuse(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman çizimde yok; “{}” tablosunun kayıtlarını var olan bir katmanın kimliğine yönlendirin.",
                    t.name
                ),
                &at,
            )),
            Some(node) if node.kind != LayerNodeType::Layer => Err(refuse(
                codes::NOT_A_LAYER,
                format!(
                    "“{}” bir grup; kayıtlar yalnız katmana konur. Grubun altındaki bir katmanı verin.",
                    node.name
                ),
                &at,
            )),
            Some(node) if doc.layers().is_locked(id) => Err(refuse(
                codes::LAYER_LOCKED,
                format!(
                    "“{}” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir katman verin.",
                    node.name
                ),
                &at,
            )),
            Some(_) => Ok(LayerTarget::Existing(id.clone())),
        };
    }
    let name = t.title.clone().unwrap_or_else(|| t.name.clone());
    if let Some(node) = layer_named(doc, &name) {
        if doc.layers().is_locked(&node.id) {
            return Err(refuse(
                codes::LAYER_LOCKED,
                format!(
                    "Dosyanın “{}” tablosu çizimdeki aynı adlı “{}” katmanına gider ama o katman kilitli. Kilidi açın ya da layers ile başka bir katmana yönlendirin.",
                    t.name, node.name
                ),
                "layers",
            ));
        }
        return Ok(LayerTarget::Existing(node.id.clone()));
    }
    Ok(LayerTarget::New {
        name,
        style: Box::new(kentos_domain::default_style()),
        visible: true,
        locked: false,
        fields: t.fields.clone(),
        plot: None,
        frozen: None,
    })
}

/// The objects of the tables moved into the project's system, one by one so each keeps its id; those the move
/// leaves out are counted by why.
fn moved(
    doc: &Document,
    from: u32,
    read: &mut Read,
    losses: &mut BTreeSet<String>,
) -> Result<(), Stop> {
    let settings = doc.settings();
    let Some(to) = kentos_project::systems::own(settings).and_then(|n| n.system) else {
        return Err(refuse(
            codes::CRS_MISMATCH,
            format!(
                "Dosya EPSG:{from} sisteminde; projenin koordinat sistemi yok (yerel proje). Projeyi bir koordinat sisteminde açın."
            ),
            "reproject",
        ));
    };
    let Some(src) = kentos_project::crs::system(from).and_then(|s| s.transform_system()) else {
        return Err(refuse(
            codes::CRS_MISMATCH,
            format!(
                "Dosyanın sistemi EPSG:{from} KentOS'un kayıtlı sistemlerinde yok; dosyayı projenin sistemine çevirip verin."
            ),
            "reproject",
        ));
    };
    let choices = kentos_project::systems::choices(settings);
    for t in &mut read.tables {
        let objects = std::mem::take(&mut t.objects);
        for (uid, e) in objects {
            if matches!(e, Entity::Record(_)) {
                t.objects.push((uid, e));
                continue;
            }
            let mut one = ImportResult {
                entities: vec![e],
                layers: Vec::new(),
                report: Default::default(),
                bounds: None,
                declared_crs: None,
                view: None,
                blocks: Vec::new(),
                text_styles: Vec::new(),
                dimension_styles: Vec::new(),
                unit: None,
                line_types: Vec::new(),
                line_type_scale: None,
            };
            let m = kentos_formats::reproject::reproject(&mut one, &src, &to, &choices);
            match one.entities.pop() {
                Some(e) => t.objects.push((uid, e)),
                None => {
                    for (why, _) in &m.left {
                        losses.insert(format!(
                            "çevrilemeyen nesne alınmadı: {}",
                            kentos_formats::reproject::why(*why)
                        ));
                    }
                    if m.kinds_left > 0 {
                        losses.insert("yaylı nesne çevrilmedi, alınmadı: yay sistemden sisteme kesin taşınmaz".into());
                    }
                }
            }
        }
    }
    Ok(())
}

fn run(
    doc: &mut Document,
    input: &ImportGpkg,
) -> Result<(GpkgImported, Vec<CommandWarning>), Stop> {
    let mut read = read_file(input)?;
    let mut warnings = Vec::new();
    let mut losses: BTreeSet<String> = read
        .tables
        .iter()
        .flat_map(|t| t.losses.iter().cloned())
        .collect();
    let declared: BTreeSet<u32> = read
        .systems
        .values()
        .flatten()
        .filter(|(org, _)| org == "EPSG")
        .filter_map(|(_, id)| u32::try_from(*id).ok())
        .collect();
    let srid = declared.iter().next().copied();
    let project = doc.settings().srid;
    let mut was_moved = false;
    match srid {
        None => warnings.push(CommandWarning {
            code: codes::CRS_UNDECLARED.into(),
            message: format!(
                "{} koordinat sistemi bildirmiyor; koordinatlar projenin sisteminde sayılıp olduğu gibi alındı.",
                read.file
            ),
            path: Some("path".into()),
        }),
        Some(s) if s == project => {}
        Some(s) => {
            if !input.reproject {
                return Err(refuse(
                    codes::CRS_MISMATCH,
                    format!(
                        "{} EPSG:{s} sisteminde, proje EPSG:{project}. Koordinatların projenin sistemine çevrilmesi için reproject verin ya da projeyi EPSG:{s}'de açın.",
                        read.file
                    ),
                    "reproject",
                ));
            }
            moved(doc, s, &mut read, &mut losses)?;
            was_moved = true;
        }
    }
    if declared.len() > 1 {
        losses.insert(format!(
            "tablolar birden çok sistem bildiriyor ({}); ilki esas alındı",
            declared
                .iter()
                .map(|s| format!("EPSG:{s}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let mut layers = Vec::new();
    let mut objects: Vec<(Uuid, Entity)> = Vec::new();
    for t in &read.tables {
        if t.objects.is_empty() {
            continue;
        }
        layers.push((t.name.clone(), target(doc, input, t)?));
        objects.extend(t.objects.iter().cloned());
    }
    if objects.is_empty() {
        return Err(refuse(
            codes::NOTHING_TO_IMPORT,
            format!(
                "{} dosyasının seçilen tablolarında alınacak kayıt yok.",
                read.file
            ),
            if input.tables.is_empty() {
                "path"
            } else {
                "tables"
            },
        ));
    }
    if let Some((uid, _)) = objects.iter().find(|(u, _)| doc.slot_of(*u).is_some()) {
        return Err(refuse(
            codes::ENTITY_EXISTS,
            format!(
                "Dosyadaki bir kaydın kalıcı kimliği ({uid}) çizimde zaten bir nesnenin: {} daha önce alınmış. Önceki alımı geri alın ya da başka bir çizimde açın.",
                read.file
            ),
            "path",
        ));
    }
    let plan = ImportPlan {
        label: step_name(&read.file),
        layers,
        group: Some(read.file.trim_end_matches(".gpkg").to_owned()),
    };
    let applied = apply_import_with_uids(doc, objects, &plan)
        .map_err(|why| refuse(codes::IMPORT_UNREADABLE, why, "path"))?;
    let targets: BTreeMap<&str, &str> = applied
        .targets
        .iter()
        .map(|(s, t)| (s.as_str(), t.as_str()))
        .collect();
    let skipped: u32 = read.tables.iter().map(|t| t.skipped.len() as u32).sum();
    if skipped > 0 {
        let first: Vec<String> = read
            .tables
            .iter()
            .flat_map(|t| {
                t.skipped
                    .iter()
                    .map(move |(fid, why)| format!("{} {fid}: {why}", t.name))
            })
            .take(3)
            .collect();
        losses.insert(format!("{skipped} satır alınmadı ({})", first.join("; ")));
    }
    if !losses.is_empty() {
        warnings.push(CommandWarning {
            code: codes::IMPORT_LOSSES.into(),
            message: format!(
                "{} dosyasının bazı değerleri ya da satırları olduğu gibi alınamadı: {}.",
                read.file,
                losses.iter().cloned().collect::<Vec<_>>().join("; ")
            ),
            path: Some("path".into()),
        });
    }
    let tables = read
        .tables
        .iter()
        .filter(|t| !t.objects.is_empty())
        .map(|t| GpkgTableImported {
            table: t.name.clone(),
            layer: targets
                .get(t.name.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default(),
            read: t.read,
            objects: t.objects.len() as u32,
            records: t
                .objects
                .iter()
                .filter(|(_, e)| matches!(e, Entity::Record(_)))
                .count() as u32,
            skipped: t.skipped.len() as u32,
        })
        .collect();
    Ok((
        GpkgImported {
            file: read.file.clone(),
            bytes: read.bytes,
            sha256: read.sha256.clone(),
            srid,
            moved: was_moved,
            objects: applied.slots.len() as u32,
            tables,
            layers: applied
                .made
                .iter()
                .map(|(id, name)| ImportedLayer {
                    id: id.clone(),
                    name: name.clone(),
                })
                .collect(),
            group: applied.group.clone(),
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}
