//! `map.work.edit` v1 (plan/map-cad.md MAP-01, docs/adr/1200 §3): a map project's items added, changed, relinked,
//! refreshed and removed, derivations recorded and forgotten; one undo step each, whole or not at all. An approval
//! stamps the item's revision and an item without a file is a source by its content revision (docs/adr/2080). The
//! shared cases are `fixtures/commands/v1/map.work.edit.json`.

use std::path::Path;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, MapWork, MapWorkEdit, MapWorkEdited, MapWorkOperation as Op, WorkDerivation,
    WorkFile, WorkItem, WorkStatus, next_numbered,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::map_work::{Revisions, file_sha, item_of_layer, map_project, stale_count, stale_objects_in};

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
            format!("“{}” öğesinin dosyası yok; yalnız dosyası olan bir kaynak yeniden bağlanır ya da yenilenir.", item.name),
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
    /// External references whose source follows the relinked file (docs/adr/1201 §2).
    blocks: Vec<kentos_commands::BlockDefinition>,
    freed: u32,
    label: &'static str,
    item: Option<String>,
    derivation: Option<String>,
    file_revision: Option<String>,
    /// Without a file: the content revision a derive recorded or an approval stamped (docs/adr/2080 §1).
    content: Option<String>,
    /// The stamp an approval wrote (docs/adr/2080 §3).
    approved: Option<String>,
    released: u32,
    warnings: Vec<CommandWarning>,
    /// The item's profile, narrowed to its classes, to write before the work (docs/adr/4880): its classes' layers and
    /// fields the drawing lacks; the classes' layers then join the item.
    profile: Option<kentos_commands::DataProfile>,
    /// The layers and fields the profile adds (a plan's are worked out on a reading copy).
    layers: Vec<String>,
    fields: Vec<String>,
}

impl Change {
    fn new(work: MapWork, label: &'static str) -> Self {
        Change {
            work,
            relink: Vec::new(),
            blocks: Vec::new(),
            freed: 0,
            label,
            item: None,
            derivation: None,
            file_revision: None,
            content: None,
            approved: None,
            released: 0,
            warnings: Vec::new(),
            profile: None,
            layers: Vec::new(),
            fields: Vec::new(),
        }
    }
}

/// The classes' layers of `p` in `doc` (those it has).
fn class_layers(doc: &Document, p: &kentos_commands::DataProfile) -> Vec<String> {
    p.classes
        .iter()
        .filter_map(|c| match crate::profiles::find(doc.layers().nodes(), &c.layer) {
            crate::profiles::Place::Layer(id) => Some(id),
            _ => None,
        })
        .collect()
}

/// The work with `item`'s layers joined by the profile's classes' layers in `doc` that it does not hold yet; a class
/// layer another item holds is refused.
fn joined(doc: &Document, work: &MapWork, item: &str, p: &kentos_commands::DataProfile) -> Result<MapWork, Stop> {
    let mut next = work.clone();
    let Some(at) = next.items.iter().position(|i| i.id == item) else { return Ok(next) };
    for layer in class_layers(doc, p) {
        match item_of_layer(doc, work, &layer) {
            Some(i) if i.id == item => {}
            Some(other) => {
                return Err(fail(
                    codes::LAYER_IN_WORK_ITEM,
                    format!(
                        "Profilin “{}” katmanı harita işinin “{}” öğesinde; bir katman tek öğenin olur.",
                        doc.layers().path(&layer),
                        other.name
                    ),
                    "operation.name",
                ));
            }
            None => {
                if next.items[at].layers.len() >= kentos_commands::MAX_ITEM_LAYERS {
                    return Err(full());
                }
                next.items[at].layers.push(layer);
            }
        }
    }
    Ok(next)
}

/// Whether an item's typed layers keep its profile: refused with the first finding when not (docs/adr/4880).
fn profile_kept(doc: &Document, item: &WorkItem, drawing: Option<&str>) -> Result<(), Stop> {
    let Some(p) = crate::map_work_profile::of_item(doc, item, drawing) else { return Ok(()) };
    let p = p?;
    let (_, mine) = crate::map_work_profile::findings(doc, &p);
    if let Some(first) = mine.first() {
        return Err(fail(
            codes::APPROVAL_PROFILE,
            format!(
                "“{}” onaylanamaz: tipli katmanlarında “{}” profiline uymayan {} bulgu var; ilki: {} Profili denetle ile düzeltin.",
                item.name,
                p.title,
                mine.len(),
                first.message
            ),
            "operation.status",
        ));
    }
    Ok(())
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
                Some(path) => Some(WorkFile { path: path.clone(), revision: read_file(path, "operation.file")?, relative: None }),
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
                approved: None,
                approved_objects: Vec::new(),
                approved_parts: Vec::new(),
                approved_values: Vec::new(),
                profile: None,
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
            let mut work = now.clone();
            if let Some(slot) = work.items.iter_mut().find(|i| i.id == *item) {
                *slot = next.clone();
            }
            // An approval stamps the item's revision with its layers after (docs/adr/2080 §3): approving an approved
            // item that moved on since stamps it again; leaving the approved status takes the stamp away.
            let mut stamped = None;
            if next.status != WorkStatus::Approved {
                next.approved = None;
                next.approved_objects.clear();
                next.approved_parts.clear();
                next.approved_values.clear();
            } else if status == &Some(WorkStatus::Approved) {
                let stamp = Revisions::new(doc, &work, std::collections::HashMap::new()).kept(&next);
                next.approved = Some(stamp.clone());
                // Docs/adr/2880: an item without a file keeps its objects' digests, the list its stamp is the digest
                // of, so what changes after can be told object by object; an item of too many objects is stamped whole.
                next.approved_objects = match next.file {
                    None => {
                        let pairs = crate::map_work::content_objects(doc, &work, &next.id);
                        if pairs.len() <= kentos_commands::MAX_APPROVED_OBJECTS { pairs } else { Vec::new() }
                    }
                    Some(_) => Vec::new(),
                };
                // Docs/adr/3280: and each object's parts' digests, so a change can be told part by part.
                next.approved_parts = crate::map_work::parts_of(doc, &next.approved_objects);
                // Docs/adr/3680: and each object's value, so what an attribute was and what a removed object was can be
                // said (none past the byte limit).
                next.approved_values = crate::map_work::values_of(doc, &next.approved_objects);
                stamped = Some(stamp);
            }
            if next == *old {
                return Err(nothing());
            }
            if let Some(slot) = work.items.iter_mut().find(|i| i.id == *item) {
                *slot = next.clone();
            }
            if next.status == WorkStatus::Approved && status == &Some(WorkStatus::Approved) {
                profile_kept(doc, &next, input.drawing.as_deref())?;
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
            let content = stamped.clone().filter(|_| next.file.is_none());
            Ok(Change {
                item: Some(item.clone()),
                file_revision: revision,
                content,
                approved: stamped,
                ..Change::new(work, "Harita işi öğesini değiştir")
            })
        }
        Op::Profile { item, name, classes } => {
            let old = item_at(&now, item, "operation.item")?;
            let Some(name) = name else {
                if old.profile.is_none() || !classes.is_empty() {
                    return Err(nothing());
                }
                let mut work = now.clone();
                if let Some(slot) = work.items.iter_mut().find(|i| i.id == *item) {
                    slot.profile = None;
                }
                return Ok(Change { item: Some(item.clone()), ..Change::new(work, "Harita işi öğesinin profilini kaldır") });
            };
            if !text_ok(name) {
                return Err(fail(
                    codes::INVALID_WORK_TEXT,
                    "Profilin adı boşlukla başlamayan ve bitmeyen 1–200 karakter olmalı.".into(),
                    "operation.name",
                ));
            }
            if classes.len() > kentos_commands::MAX_PROFILE_CLASS_IDS {
                return Err(fail(
                    codes::INVALID_WORK_TEXT,
                    format!("Bir öğenin profilinde en çok {} sınıf olur.", kentos_commands::MAX_PROFILE_CLASS_IDS),
                    "operation.classes",
                ));
            }
            for (i, c) in classes.iter().enumerate() {
                if classes[..i].contains(c) {
                    return Err(fail(
                        codes::INVALID_WORK_TEXT,
                        format!("“{c}” sınıfı iki kez verilmiş."),
                        &format!("operation.classes[{i}]"),
                    ));
                }
            }
            let whole = crate::map_work_profile::load(doc, name, input.drawing.as_deref(), "operation.name")?;
            let p = crate::map_work_profile::narrowed(&whole, classes)?;
            // What the profile writes, on a reading copy: the layers and fields it adds, its warnings.
            let mut copy = doc.reading_copy();
            let (applied, warnings) = crate::profiles::write(&mut copy, &p).map_err(|s| s.at("operation.name"))?;
            let mut work = now.clone();
            if let Some(slot) = work.items.iter_mut().find(|i| i.id == *item) {
                slot.profile = Some(kentos_commands::WorkProfile { name: name.clone(), classes: classes.clone() });
            }
            let work = joined(&copy, &work, item, &p)?;
            if work == now && applied.layers.is_empty() && applied.fields.is_empty() {
                return Err(nothing());
            }
            Ok(Change {
                item: Some(item.clone()),
                warnings,
                profile: Some(p),
                layers: applied.layers,
                fields: applied.fields,
                ..Change::new(work, "Harita işi öğesine profil ver")
            })
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
                if item_of_layer(doc, &now, layer).is_none_or(|i| i.id != *item) {
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
                // The next save writes the new place's relative path (docs/adr/1201 §1).
                f.relative = None;
            }
            let mut warnings = Vec::new();
            // External references on the old file follow it when they loaded the same content (docs/adr/1201 §2): a
            // path, never a reload; one that loaded other content is left to Dış başvurular › Yolu değiştir.
            let mut blocks = Vec::new();
            for def in doc.blocks() {
                let Some(x) = def.xref.as_ref().filter(|x| x.parent.is_none() && same_file(&x.path, &was.path)) else {
                    continue;
                };
                if x.digest == sha {
                    let mut next = (**def).clone();
                    if let Some(nx) = next.xref.as_mut() {
                        nx.path = file.clone();
                        nx.relative = None;
                    }
                    blocks.push(next);
                } else {
                    warnings.push(CommandWarning {
                        code: codes::LINK_NOT_FOLLOWED.into(),
                        message: format!(
                            "“{}” dış başvurusu başka bir içerik yüklemiş; yeni dosyaya Dış başvurular › Yolu değiştir ile geçirin.",
                            def.name
                        ),
                        path: Some("operation.file".into()),
                    });
                }
            }
            // A bound layer's link is the project's setting, outside undo (docs/adr/0506): said, not changed.
            for link in doc.settings().sources.iter().filter(|l| same_file(&l.path, &was.path)) {
                warnings.push(CommandWarning {
                    code: codes::LINK_NOT_FOLLOWED.into(),
                    message: format!(
                        "“{}” katmanı eski dosyaya bağlı; Kaynak › Yeniden bağla ile yeni dosyaya bağlayın.",
                        doc.layers().path(&link.layer)
                    ),
                    path: Some("operation.file".into()),
                });
            }
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
                blocks,
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
                // An item without a file is its objects (docs/adr/2080 §2): they are not made from it.
                let layer = doc.slot_of(uuid).and_then(|s| doc.get(s)).map(|e| e.base().layer_id.clone());
                if src.file.is_none()
                    && layer.is_some_and(|l| item_of_layer(doc, &now, &l).is_some_and(|i| i.id == src.id))
                {
                    return Err(fail(
                        codes::OWN_CONTENT,
                        format!(
                            "“{u}” nesnesi “{}” öğesinin kendi katmanında; dosyası olmayan öğe kendi nesnelerinin kaynağı olamaz.",
                            src.name
                        ),
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
            // Its file's revision, or without a file its content revision now (docs/adr/2080 §2).
            let revision = Revisions::new(doc, &now, std::collections::HashMap::new()).kept(src);
            let content = src.file.is_none().then(|| revision.clone());
            let id = next_numbered(now.derivations.iter().map(|d| d.id.as_str()), "tr-");
            let mut work = now.clone();
            work.derivations.push(WorkDerivation {
                id: id.clone(),
                source: source.clone(),
                revision: revision.clone(),
                operation: operation.clone(),
                uids: ids,
            });
            let file_revision = content.is_none().then_some(revision);
            Ok(Change {
                item: Some(source.clone()),
                derivation: Some(id),
                file_revision,
                content,
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
        Op::Release { uids } => {
            if uids.is_empty() {
                return Err(fail(
                    codes::NO_ENTITIES,
                    "Bağımsız yapılacak nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.".into(),
                    "operation.uids",
                ));
            }
            let mut ids: Vec<kentos_commands::EntityId> = Vec::new();
            for (i, u) in uids.iter().enumerate() {
                let at = format!("operation.uids[{i}]");
                if !checks::is_uid_text(u) {
                    return Err(fail(codes::INVALID_UID, format!("“{u}” geçerli bir nesne kimliği değil."), &at));
                }
                let id = Uuid::parse_str(u).ok().map(|x| kentos_commands::EntityId(*x.as_bytes()));
                // A derived object deleted from the drawing may still be let go of: its record names it.
                let Some(id) = id.filter(|id| now.derivations.iter().any(|d| d.uids.contains(id))) else {
                    return Err(fail(
                        codes::NOT_DERIVED,
                        format!("“{u}” nesnesi hiçbir türetme kaydında değil; zaten bağımsız çizim."),
                        &at,
                    ));
                };
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            let mut work = now.clone();
            let before = work.derivations.len();
            for d in &mut work.derivations {
                d.uids.retain(|u| !ids.contains(u));
            }
            work.derivations.retain(|d| !d.uids.is_empty());
            let released = u32::try_from(before - work.derivations.len()).unwrap_or(u32::MAX);
            let freed = u32::try_from(ids.len()).unwrap_or(u32::MAX);
            Ok(Change { released, freed, ..Change::new(work, "Bağımsız yap") })
        }
    }
}

/// Whether two kept paths name the same file: the same text, else the same file on disk.
fn same_file(a: &str, b: &str) -> bool {
    a == b || std::fs::canonicalize(a).ok().is_some_and(|x| std::fs::canonicalize(b).ok().is_some_and(|y| x == y))
}

fn output(doc: &Document, c: &Change, revision: String) -> MapWorkEdited {
    MapWorkEdited {
        item: c.item.clone(),
        derivation: c.derivation.clone(),
        file_revision: c.file_revision.clone(),
        content: c.content.clone(),
        approved: c.approved.clone(),
        relinked: u32::try_from(c.relink.len()).unwrap_or(u32::MAX),
        released: c.released,
        links: u32::try_from(c.blocks.len()).unwrap_or(u32::MAX),
        freed: c.freed,
        stale: stale_count(doc, &c.work),
        layers: c.layers.clone(),
        fields: c.fields.clone(),
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
    let blocks = change.blocks.clone();
    let mut change = change;
    let work = change.work.clone();
    let profile = change.profile.clone();
    let item = change.item.clone();
    let done = cx.doc.transact(label, |doc| -> Result<(), Option<Stop>> {
        let mut work = work;
        // Docs/adr/4880: the profile's layers and fields first, then its classes' layers join the item.
        if let (Some(p), Some(item)) = (&profile, &item) {
            let (applied, _) = crate::profiles::write(doc, p).map_err(|s| Some(s.at("operation.name")))?;
            work = joined(doc, &work, item, p).map_err(Some)?;
            change.layers = applied.layers;
            change.fields = applied.fields;
        }
        if !relink.is_empty() {
            doc.update_many(relink, label);
        }
        for block in blocks {
            // A source's path only: the definition stays as it was checked.
            if doc.update_block(block).is_err() {
                return Err(None);
            }
        }
        change.work = work.clone();
        doc.set_map_work(Some(work), label);
        Ok(())
    });
    if let Err(Some(stop)) = done {
        return stop.into();
    }
    let output = output(cx.doc, &change, cx.doc.revision().to_string());
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
            let output = output(cx.doc, &c, cx.doc.revision().to_string());
            CommandResult::Completed { output, warnings: c.warnings }
        }
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: MapWorkEdit) -> CommandResult<MapWorkEdited> {
    crate::call::recorded(cx, kentos_commands::MAP_WORK_EDIT, kentos_commands::MAP_WORK_EDIT_VERSION, input, write)
}
