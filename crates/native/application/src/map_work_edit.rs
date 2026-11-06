//! `map.work.edit` v1 (plan/map-cad.md MAP-01, docs/adr/1200 §3): a map project's items added, changed, relinked,
//! refreshed and removed, derivations recorded and forgotten; one undo step each, whole or not at all. The shared cases
//! are `fixtures/commands/v1/map.work.edit.json`.

use std::path::Path;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, MapWork, MapWorkEdit, MapWorkEdited, MapWorkOperation as Op, WorkDerivation,
    WorkFile, WorkItem, WorkStatus, next_numbered,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::map_work::{file_sha, item_of_layer, map_project, stale_count, stale_objects_in};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn text_ok(s: &str) -> bool {
    let n = s.chars().count();
    n > 0 && n <= kentos_commands::MOST_WORK_TEXT && s.trim() == s
}

fn name_ok(name: &str, path: &str) -> Result<(), Stop> {
    if text_ok(name) {
        return Ok(());
    }
    Err(fail(
        codes::INVALID_WORK_TEXT,
        "Öğenin adı boşlukla başlamayan ve bitmeyen 1–200 karakter olmalı.".into(),
        path,
    ))
}

fn item_at<'w>(work: &'w MapWork, id: &str, path: &str) -> Result<&'w WorkItem, Stop> {
    work.item(id).ok_or_else(|| {
        fail(codes::UNKNOWN_WORK_ITEM, format!("Harita işinde “{id}” kimlikli öğe yok."), path)
    })
}

/// The layers an item takes: each in the drawing, not given twice and in no other item than `own`.
fn layers_ok(doc: &Document, work: &MapWork, layers: &[String], own: Option<&str>) -> Result<(), Stop> {
    for (i, l) in layers.iter().enumerate() {
        let at = format!("operation.layers[{i}]");
        if doc.layers().get(l).is_none() {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{l}” kimlikli katman çizimde yok."), &at));
        }
        if layers[..i].contains(l) {
            return Err(fail(codes::LAYER_IN_WORK_ITEM, format!("“{l}” katmanı iki kez verilmiş."), &at));
        }
        if let Some(other) = work.items.iter().find(|w| Some(w.id.as_str()) != own && w.layers.contains(l)) {
            return Err(fail(
                codes::LAYER_IN_WORK_ITEM,
                format!("“{l}” katmanı harita işinin “{}” öğesinde; bir katman tek öğenin olur.", other.name),
                &at,
            ));
        }
    }
    Ok(())
}

/// The file at `path` and its SHA-256: an absolute path to a file that reads.
fn read_file(path: &str, at: &str) -> Result<String, Stop> {
    if path.trim().is_empty() || path.trim() != path || !Path::new(path).is_absolute() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!("“{path}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin."),
            at,
        ));
    }
    if !Path::new(path).is_file() {
        return Err(fail(codes::FILE_NOT_FOUND, format!("“{path}” dosyası yok."), at));
    }
    file_sha(path).ok_or_else(|| fail(codes::FILE_NOT_FOUND, format!("“{path}” dosyası okunamadı."), at))
}

fn with_file<'w>(item: &'w WorkItem, path: &str) -> Result<&'w WorkFile, Stop> {
    item.file.as_ref().ok_or_else(|| {
        fail(
            codes::WORK_ITEM_NO_FILE,
            format!("“{}” öğesinin dosyası yok; yalnız dosyası olan bir kaynak yeniden bağlanır, yenilenir ya da türetmenin kaynağı olur.", item.name),
            path,
        )
    })
}

fn nothing() -> Stop {
    fail(codes::NOTHING_TO_SET, "Değişecek bir şey yok.".into(), "operation")
}

fn full() -> Stop {
    fail(codes::WORK_FULL, "Harita işinde en çok 1000 öğe ve 10 000 türetme kaydı olur.".into(), "operation")
}

/// What the operation does, worked out: the work structure after, the rasters and images to point at a new file, the
/// undo step's name, the output's ids and the warnings.
struct Change {
    work: MapWork,
    relink: Vec<(Slot, Entity)>,
    label: &'static str,
    item: Option<String>,
    derivation: Option<String>,
    file_revision: Option<String>,
    released: u32,
    warnings: Vec<CommandWarning>,
}

impl Change {
    fn new(work: MapWork, label: &'static str) -> Self {
        Change {
            work,
            relink: Vec::new(),
            label,
            item: None,
            derivation: None,
            file_revision: None,
            released: 0,
            warnings: Vec::new(),
        }
    }
}

fn check(doc: &Document, input: &MapWorkEdit) -> Result<Change, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    map_project(doc)?;
    let now = doc.map_work().cloned().unwrap_or_default();
    match &input.operation {
        Op::Add { kind, name, layers, file } => {
            name_ok(name, "operation.name")?;
            layers_ok(doc, &now, layers, None)?;
            let file = match file {
                Some(path) => Some(WorkFile { path: path.clone(), revision: read_file(path, "operation.file")? }),
                None => None,
            };
            if now.items.len() >= kentos_commands::MAX_WORK_ITEMS {
                return Err(full());
            }
            let id = next_numbered(now.items.iter().map(|i| i.id.as_str()), "is-");
            let mut work = now.clone();
            let revision = file.as_ref().map(|f| f.revision.clone());
            work.items.push(WorkItem {
                id: id.clone(),
                kind: *kind,
                name: name.clone(),
                status: WorkStatus::Draft,
                layers: layers.clone(),
                file,
            });
            Ok(Change { item: Some(id), file_revision: revision, ..Change::new(work, "Harita işine ekle") })
        }
        Op::Set { item, name, status, layers } => {
            let old = item_at(&now, item, "operation.item")?;
            if let Some(n) = name {
                name_ok(n, "operation.name")?;
            }
            if let Some(l) = layers {
                layers_ok(doc, &now, l, Some(item))?;
            }
            let mut next = old.clone();
            if let Some(n) = name {
                next.name = n.clone();
            }
            if let Some(s) = status {
                next.status = *s;
            }
            if let Some(l) = layers {
                next.layers = l.clone();
            }
            if next == *old {
                return Err(nothing());
            }
            let mut work = now.clone();
            if let Some(slot) = work.items.iter_mut().find(|i| i.id == *item) {
                *slot = next.clone();
            }
            if next.status == WorkStatus::Approved {
                let n = stale_objects_in(doc, &work, &next, false);
                if n > 0 {
                    return Err(fail(
                        codes::APPROVAL_STALE,
                        format!("“{}” onaylanamaz: katmanlarındaki {n} türetilmiş nesnenin kaynağı değişti; önce onları güncelleyin.", next.name),
                        "operation.status",
                    ));
                }
            }
            let revision = next.file.as_ref().map(|f| f.revision.clone());
            Ok(Change { item: Some(item.clone()), file_revision: revision, ..Change::new(work, "Harita işi öğesini değiştir") })
        }
        Op::Remove { item } => {
            item_at(&now, item, "operation.item")?;
            let mut work = now.clone();
            work.items.retain(|i| i.id != *item);
            let before = work.derivations.len();
            work.derivations.retain(|d| d.source != *item);
            let released = u32::try_from(before - work.derivations.len()).unwrap_or(u32::MAX);
            Ok(Change { item: Some(item.clone()), released, ..Change::new(work, "Harita işinden çıkar") })
        }
        Op::Relink { item, file } => {
            let old = item_at(&now, item, "operation.item")?;
            let was = with_file(old, "operation.item")?.clone();
            let sha = read_file(file, "operation.file")?;
            if was.path == *file && was.revision == sha {
                return Err(nothing());
            }
            // The rasters and images on the item's layers that showed the old file follow it.
            let mut relink = Vec::new();
            for (uid, entity) in doc.entities_with_uids() {
                let layer = &entity.base().layer_id;
                if !item_of_layer(doc, &now, layer).is_some_and(|i| i.id == *item) {
                    continue;
                }
                let mut next = entity.clone();
                let shown = match &mut next {
                    Entity::Raster(r) => r.raster.file.as_mut(),
                    Entity::Image(i) => i.image.file.as_mut(),
                    _ => None,
                };
                let Some(shown) = shown.filter(|f| **f == was.path) else { continue };
                if doc.layers().is_locked(layer) {
                    return Err(fail(
                        codes::LAYER_LOCKED,
                        format!("“{}” katmanı kilitli: üstündeki resim yeni dosyaya bağlanamaz. Kilidi açın.", doc.layers().path(layer)),
                        "operation.item",
                    ));
                }
                *shown = file.clone();
                if let Some(slot) = doc.slot_of(uid) {
                    relink.push((slot, next));
                }
            }
            let mut work = now.clone();
            if let Some(f) = work.items.iter_mut().find(|i| i.id == *item).and_then(|i| i.file.as_mut()) {
                f.path = file.clone();
                f.revision = sha.clone();
            }
            let mut warnings = Vec::new();
            if sha != was.revision {
                let n = work.derivations.iter().filter(|d| d.source == *item).count();
                warnings.push(CommandWarning {
                    code: codes::SOURCE_CONTENT_CHANGED.into(),
                    message: format!(
                        "Yeni dosyanın içeriği öncekinden farklı: “{}” kaynağından türetilmiş {n} kayıt güncel değil.",
                        old.name
                    ),
                    path: Some("operation.file".into()),
                });
            }
            Ok(Change {
                relink,
                item: Some(item.clone()),
                file_revision: Some(sha),
                warnings,
                ..Change::new(work, "Kaynağı yeniden bağla")
            })
        }
        Op::Refresh { item } => {
            let old = item_at(&now, item, "operation.item")?;
            let was = with_file(old, "operation.item")?.clone();
            let sha = match file_sha(&was.path) {
                Some(sha) if Path::new(&was.path).is_file() => sha,
                _ => {
                    return Err(fail(
                        codes::FILE_NOT_FOUND,
                        format!("“{}” dosyası yok; Yeniden bağla ile yeni yerini gösterin.", was.path),
                        "operation.item",
                    ));
                }
            };
            if sha == was.revision {
                return Err(nothing());
            }
            let mut work = now.clone();
            if let Some(f) = work.items.iter_mut().find(|i| i.id == *item).and_then(|i| i.file.as_mut()) {
                f.revision = sha.clone();
            }
            Ok(Change { item: Some(item.clone()), file_revision: Some(sha), ..Change::new(work, "Kaynağı yenile") })
        }
        Op::Derive { source, uids, operation } => {
            let src = item_at(&now, source, "operation.source")?;
            let revision = with_file(src, "operation.source")?.revision.clone();
            if !text_ok(operation) {
                return Err(fail(
                    codes::INVALID_WORK_TEXT,
                    "Üretim işlemi boşlukla başlamayan ve bitmeyen 1–200 karakter olmalı.".into(),
                    "operation.operation",
                ));
            }
            if uids.is_empty() {
                return Err(fail(
                    codes::NO_ENTITIES,
                    "Türetilen nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.".into(),
                    "operation.uids",
                ));
            }
            let mut ids = Vec::new();
            for (i, u) in uids.iter().enumerate() {
                let at = format!("operation.uids[{i}]");
                if !checks::is_uid_text(u) {
                    return Err(fail(codes::INVALID_UID, format!("“{u}” geçerli bir nesne kimliği değil."), &at));
                }
                let Some(uuid) = Uuid::parse_str(u).ok().filter(|x| doc.slot_of(*x).is_some()) else {
                    return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{u}” kimlikli nesne çizimde yok."), &at));
                };
                let id = kentos_commands::EntityId(*uuid.as_bytes());
                if let Some(d) = now.derivations.iter().find(|d| d.uids.contains(&id)) {
                    return Err(fail(
                        codes::ALREADY_DERIVED,
                        format!("“{u}” nesnesi “{}” türetme kaydında; önce o kaydı silin.", d.id),
                        &at,
                    ));
                }
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            if now.derivations.len() >= kentos_commands::MAX_DERIVATIONS {
                return Err(full());
            }
            let id = next_numbered(now.derivations.iter().map(|d| d.id.as_str()), "tr-");
            let mut work = now.clone();
            work.derivations.push(WorkDerivation {
                id: id.clone(),
                source: source.clone(),
                revision: revision.clone(),
                operation: operation.clone(),
                uids: ids,
            });
            Ok(Change {
                item: Some(source.clone()),
                derivation: Some(id),
                file_revision: Some(revision),
                ..Change::new(work, "Kaynaktan türet")
            })
        }
        Op::Forget { derivation } => {
            if !now.derivations.iter().any(|d| d.id == *derivation) {
                return Err(fail(
                    codes::UNKNOWN_DERIVATION,
                    format!("Harita işinde “{derivation}” kimlikli türetme kaydı yok."),
                    "operation.derivation",
                ));
            }
            let mut work = now.clone();
            work.derivations.retain(|d| d.id != *derivation);
            Ok(Change { derivation: Some(derivation.clone()), ..Change::new(work, "Türetme kaydını sil") })
        }
    }
}

fn output(c: &Change, revision: String) -> MapWorkEdited {
    MapWorkEdited {
        item: c.item.clone(),
        derivation: c.derivation.clone(),
        file_revision: c.file_revision.clone(),
        relinked: u32::try_from(c.relink.len()).unwrap_or(u32::MAX),
        released: c.released,
        stale: stale_count(&c.work),
        revision,
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: MapWorkEdit) -> CommandResult<MapWorkEdited> {
    let change = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let label = change.label;
    let relink = change.relink.clone();
    let work = change.work.clone();
    let _ = cx.doc.transact(label, |doc| -> Result<(), ()> {
        if !relink.is_empty() {
            doc.update_many(relink, label);
        }
        doc.set_map_work(Some(work), label);
        Ok(())
    });
    let output = output(&change, cx.doc.revision().to_string());
    CommandResult::Completed { output, warnings: change.warnings }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &MapWorkEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &MapWorkEdit) -> CommandResult<MapWorkEdited> {
    match check(cx.doc, input) {
        Ok(c) => {
            let output = output(&c, cx.doc.revision().to_string());
            CommandResult::Completed { output, warnings: c.warnings }
        }
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: MapWorkEdit) -> CommandResult<MapWorkEdited> {
    crate::call::recorded(cx, kentos_commands::MAP_WORK_EDIT, kentos_commands::MAP_WORK_EDIT_VERSION, input, write)
}
