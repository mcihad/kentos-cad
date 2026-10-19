//! `cad.xrefs.edit` v1 and `cad.xrefs.list` v1 (docs/adr/0348; plan/cad.md `CAD-REF-05`…`14`): external references —
//! another drawing file as a block definition that keeps its source, attached (or overlaid), reloaded, unloaded,
//! repathed, detached and bound, each one undo step; and the references listed with their state, read-only. The
//! content rule is `kentos_domain::xref`'s; the shared cases are `fixtures/commands/v1/cad.xrefs.{edit,list}.json`.
//!
//! A source is found at its path, else at its relative path from the open drawing's folder (`drawing`); nothing is
//! taken in without a command: a changed or missing file is said by the list.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use kentos_contracts::encoding::sha256_hex;
use kentos_contracts::{
    AssetVia, BlockDefinition, BlockId, CommandResult, CommandWarning, DocumentSnapshotV2, Entity, EntityBase, InsertEntity,
    Vec2, XrefBindMode, XrefInfo, XrefKind, XrefSource, XrefState, XrefsEdit, XrefsEdited, XrefsList,
    XrefsListed, XrefsOperation,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::exchange_write::{Counts, put_blocks, put_nodes};
use crate::source_places::relative_path;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// A path as the file system names it (links followed), else lexically: two names of one file are equal.
fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| {
        let mut out = PathBuf::new();
        for c in p.components() {
            match c {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    out.pop();
                }
                other => out.push(other),
            }
        }
        out
    })
}

/// The reference named `name` (case folded), with its source.
fn find<'d>(doc: &'d Document, name: &str) -> Result<(&'d BlockDefinition, XrefSource), Stop> {
    let key = kentos_contracts::blocks::name_key(name);
    doc.blocks()
        .iter()
        .find(|b| b.xref.is_some() && kentos_contracts::blocks::name_key(&b.name) == key)
        .and_then(|b| Some((&**b, b.xref.clone()?)))
        .ok_or_else(|| {
            fail(
                codes::XREF_NOT_FOUND,
                format!("“{name}” adlı dış başvuru yok. Dış başvurular penceresindeki adı verin."),
                "operation.name",
            )
        })
}

/// A reference brought by another may not be changed alone.
fn top(def: &BlockDefinition, x: &XrefSource, doc: &Document) -> Result<(), Stop> {
    match x.parent {
        Some(p) => Err(fail(
            codes::NESTED_XREF,
            format!(
                "“{}” dış başvurusunu “{}” getirdi; kendi başına değiştirilemez. Onu getireni yeniden yükleyin, boşaltın ya da kaldırın.",
                def.name,
                doc.block(p).map_or_else(|| "başka bir dış başvuru".to_owned(), |b| b.name.clone())
            ),
            "operation.name",
        )),
        None => Ok(()),
    }
}

/// The source file as read: its path, the SHA-256 of its bytes and its drawing.
struct Source {
    path: PathBuf,
    digest: String,
    drawing: DocumentSnapshotV2,
}

fn read(path: &str, at: &str) -> Result<Source, Stop> {
    let p = Path::new(path);
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!("“{path}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.kcad)."),
            at,
        ));
    }
    let bytes = std::fs::read(p)
        .map_err(|_| fail(codes::FILE_NOT_FOUND, format!("{path} okunamadı: dosya yok ya da açılamıyor."), at))?;
    let drawing = crate::file_import_block::read_drawing(&bytes).map_err(|why| {
        fail(
            codes::IMPORT_UNREADABLE,
            format!("{} bir KentOS çizimi olarak okunamadı: {why}", file_name(path)),
            at,
        )
    })?;
    Ok(Source {
        path: p.to_path_buf(),
        digest: sha256_hex(&bytes),
        drawing,
    })
}

/// A reference to the open drawing itself, or through one of the source's attached references to it or to the
/// source, is a cycle.
fn cycle(drawing: Option<&str>, source: &Source, at: &str) -> Result<(), Stop> {
    let own = drawing.map(|d| canonical(Path::new(d)));
    let src = canonical(&source.path);
    let refused = |what: String| fail(codes::XREF_CYCLE, what, at);
    if own.as_ref() == Some(&src) {
        return Err(refused(format!(
            "{} açık çizimin kendisi; çizim kendine dış başvuru olamaz.",
            file_name(&source.path.to_string_lossy())
        )));
    }
    for b in &source.drawing.blocks {
        let Some(x) = &b.xref else { continue };
        if x.kind == XrefKind::Overlay {
            continue;
        }
        let p = canonical(Path::new(&x.path));
        if p == src || own.as_ref() == Some(&p) {
            return Err(refused(format!(
                "{} içindeki “{}” dış başvurusu {} dosyasına geri bağlanıyor: döngü. O dış başvuruyu kaynağında Bindir yapın ya da kaldırın.",
                file_name(&source.path.to_string_lossy()),
                b.name,
                file_name(&x.path)
            )));
        }
    }
    Ok(())
}

/// The source's file as looked for (docs/adr/0349 §1): its path, its relative path from the drawing's folder, the
/// project's search paths.
fn found(doc: &Document, x: &XrefSource, drawing: Option<&str>) -> crate::links::Found {
    crate::links::find(&x.path, x.relative.as_deref(), drawing, &doc.settings().xref_paths)
}

fn relative_of(drawing: Option<&str>, file: &Path) -> Option<String> {
    let folder = Path::new(drawing?).parent()?;
    relative_path(folder, file)
}

/// The inserts of the drawing that place `block`.
fn inserts_of(doc: &Document, block: BlockId) -> Vec<Slot> {
    doc.entities()
        .filter(|e| matches!(e, Entity::Insert(i) if i.block == block))
        .map(|e| Slot(e.base().id))
        .collect()
}

/// The definitions `block` brought (its nested references and the blocks its content places), deepest last.
fn brought(doc: &Document, block: BlockId) -> Vec<BlockId> {
    let mut out: Vec<BlockId> = Vec::new();
    let mut todo = vec![block];
    while let Some(b) = todo.pop() {
        let Some(def) = doc.block(b) else { continue };
        for e in &def.entities {
            if let Entity::Insert(i) = e
                && !out.contains(&i.block)
                && i.block != block
            {
                out.push(i.block);
                todo.push(i.block);
            }
        }
    }
    for b in doc.blocks() {
        if b.xref.as_ref().and_then(|x| x.parent) == Some(block) && !out.contains(&b.id) {
            out.push(b.id);
        }
    }
    out
}

/// Whether nothing is drawn on the layers under `group`: no object of the drawing and no piece of a definition. Only
/// then does the group go with a reference (removing a layer takes its objects with it).
fn group_unused(doc: &Document, group: &str) -> bool {
    let leaves: std::collections::HashSet<&str> = doc.layers().leaves_of(group).into_iter().map(|n| n.id.as_str()).collect();
    !doc.entities().any(|e| leaves.contains(e.base().layer_id.as_str()))
        && !doc.blocks().iter().any(|b| b.entities.iter().any(|e| leaves.contains(e.base().layer_id.as_str())))
}

/// What a command will write, checked.
enum Plan {
    Load {
        name: String,
        id: BlockId,
        source: XrefSource,
        drawing: Box<DocumentSnapshotV2>,
        insert: Option<Box<InsertEntity>>,
        label: &'static str,
    },
    Unload { id: BlockId, name: String },
    Detach { id: BlockId, name: String },
    Bind { id: BlockId, name: String, mode: XrefBindMode },
    Paths(Vec<String>),
}

fn numbers(at: Option<Vec2>, scale: Option<f64>, rotation: Option<f64>) -> Result<(), Stop> {
    if let Some(p) = at
        && !(p.x.is_finite() && p.y.is_finite())
    {
        return Err(fail(codes::NOT_FINITE, "Dış başvurunun noktası sayı değil ya da sonsuz.".into(), "operation.at"));
    }
    if let Some(r) = rotation
        && !r.is_finite()
    {
        return Err(fail(codes::NOT_FINITE, "Dış başvurunun dönüşü sayı değil ya da sonsuz.".into(), "operation.rotation"));
    }
    if let Some(s) = scale
        && !(s.is_finite() && s > 0.0 && s <= 1_000_000.0)
    {
        return Err(fail(
            codes::INVALID_SCALE,
            "Dış başvurunun ölçeği sıfırdan büyük ve en çok 1 000 000 olmalı.".into(),
            "operation.scale",
        ));
    }
    Ok(())
}

fn check(doc: &Document, input: &XrefsEdit) -> Result<(Plan, Vec<CommandWarning>), Stop> {
    let drawing = input.drawing.as_deref();
    let mut warnings = Vec::new();
    let plan = match &input.operation {
        XrefsOperation::Attach { path, mode, name, at, scale, rotation, layer_id } => {
            numbers(*at, *scale, *rotation)?;
            warnings = checks::layer_at(doc, layer_id, "operation.layerId")?;
            let source = read(path, "operation.path")?;
            cycle(drawing, &source, "operation.path")?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            let wanted = name
                .clone()
                .filter(|n| !checks::is_blank(n))
                .unwrap_or_else(|| {
                    Path::new(path).file_stem().map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned())
                });
            let taken: Vec<String> = doc.blocks().iter().map(|b| b.name.clone()).collect();
            let name = kentos_domain::exchange::unique_names(&taken, std::slice::from_ref(&wanted))
                .pop()
                .unwrap_or(wanted);
            let id = BlockId(*kentos_domain::Uuid::now_v7().as_bytes());
            let insert = InsertEntity {
                parameters: Default::default(),
                attribute_places: Default::default(),
                base: EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: layer_id.clone(),
                    color: None,
                    attrs: Default::default(),
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
                block: id,
                p: at.unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
                scale: scale.unwrap_or(1.0),
                y_scale: None,
                rotation: rotation.unwrap_or(0.0).to_radians(),
                mirror: false,
            };
            let x = XrefSource {
                path: source.path.to_string_lossy().into_owned(),
                relative: relative_of(drawing, &source.path),
                kind: *mode,
                digest: source.digest.clone(),
                unloaded: false,
                group: None,
                parent: None,
                objects: Vec::new(),
            };
            Plan::Load { name, id, source: x, drawing: Box::new(source.drawing), insert: Some(Box::new(insert)), label: "Dış başvuru ekle" }
        }
        XrefsOperation::Reload { name } | XrefsOperation::Repath { name, .. } => {
            let (def, x) = find(doc, name)?;
            top(def, &x, doc)?;
            let (path, label) = match &input.operation {
                XrefsOperation::Repath { path, .. } => (path.clone(), "Dış başvurunun yolunu değiştir"),
                _ => {
                    let at = found(doc, &x, drawing).at.unwrap_or_else(|| PathBuf::from(&x.path));
                    (at.to_string_lossy().into_owned(), "Dış başvuruyu yeniden yükle")
                }
            };
            let at = if matches!(input.operation, XrefsOperation::Repath { .. }) { "operation.path" } else { "operation.name" };
            let source = read(&path, at)?;
            cycle(drawing, &source, at)?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            let next = XrefSource {
                path: source.path.to_string_lossy().into_owned(),
                relative: relative_of(drawing, &source.path).or(x.relative.clone()),
                digest: source.digest.clone(),
                unloaded: false,
                objects: Vec::new(),
                ..x.clone()
            };
            Plan::Load { name: def.name.clone(), id: def.id, source: next, drawing: Box::new(source.drawing), insert: None, label }
        }
        XrefsOperation::Unload { name } => {
            let (def, x) = find(doc, name)?;
            top(def, &x, doc)?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            Plan::Unload { id: def.id, name: def.name.clone() }
        }
        XrefsOperation::Detach { name } => {
            let (def, x) = find(doc, name)?;
            top(def, &x, doc)?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            Plan::Detach { id: def.id, name: def.name.clone() }
        }
        XrefsOperation::Paths { paths } => {
            checks::revision(doc, input.expected_revision.as_deref())?;
            Plan::Paths(kentos_contracts::sanitized_xref_paths(paths.clone()))
        }
        XrefsOperation::Bind { name, mode } => {
            let (def, x) = find(doc, name)?;
            top(def, &x, doc)?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            Plan::Bind { id: def.id, name: def.name.clone(), mode: *mode }
        }
    };
    Ok((plan, warnings))
}

fn write(doc: &mut Document, input: &XrefsEdit) -> Result<(XrefsEdited, Vec<CommandWarning>), Stop> {
    let (plan, mut warnings) = check(doc, input)?;
    let refused = |why: String| fail(codes::BLOCK_REFUSED, format!("{why} Hiçbir şey değişmedi."), "operation");
    match plan {
        Plan::Load { name, id, source, drawing: theirs, insert, label } => {
            let ours = doc.to_snapshot_v2();
            let loaded = kentos_domain::xref::load(&ours, &theirs, &name, id, source)
                .map_err(|why| fail(codes::XREF_EMPTY, why, "operation.path"))?;
            let before_blocks: Vec<BlockId> = doc.blocks().iter().map(|b| b.id).collect();
            let group = doc.begin_group(&format!("{label}: {name}"));
            let mut counts = Counts::default();
            let done = put_nodes(doc, &loaded.drawing.layers, None, &mut counts, label)
                .and_then(|()| put_blocks(doc, &loaded.drawing.blocks, &mut counts));
            if let Err(why) = done {
                doc.cancel_group(group);
                return Err(refused(why));
            }
            // What the reference no longer brings goes.
            let keep: Vec<BlockId> = loaded.drawing.blocks.iter().map(|b| b.id).collect();
            for b in before_blocks.iter().filter(|b| !keep.contains(b)) {
                let _ = doc.remove_block(*b);
            }
            let mut insert_uid = None;
            if let Some(i) = insert {
                match checks::add(doc, Entity::Insert(*i)) {
                    Ok((_, uid, _)) => insert_uid = Some(uid),
                    Err(stop) => {
                        doc.cancel_group(group);
                        return Err(stop);
                    }
                }
            }
            doc.end_group(group);
            if loaded.left > 0 {
                warnings.push(CommandWarning {
                    code: codes::OBJECTS_LEFT_OUT.into(),
                    message: format!("{} resim, raster ya da tablo bloğa konamadığı için alınmadı.", loaded.left),
                    path: Some("operation".into()),
                });
            }
            Ok((
                XrefsEdited {
                    name,
                    block: Some(id),
                    insert: insert_uid,
                    objects: u32::try_from(loaded.objects).unwrap_or(u32::MAX),
                    layers: loaded.layers,
                    nested: loaded.nested.into_iter().map(|(_, n)| n).collect(),
                    removed: 0,
                    merged: Vec::new(),
                    paths: Vec::new(),
                    revision: doc.revision().to_string(),
                },
                warnings,
            ))
        }
        Plan::Unload { id, name } => {
            let Some(mut def) = doc.block(id).cloned() else {
                return Err(refused("Dış başvuru bulunamadı.".into()));
            };
            if let Some(x) = def.xref.as_mut() {
                x.unloaded = true;
                x.objects.clear();
            }
            def.entities.clear();
            let group = doc.begin_group(&format!("Dış başvuruyu boşalt: {name}"));
            let gone = brought(doc, id);
            if let Err(r) = doc.update_block(def) {
                doc.cancel_group(group);
                return Err(refused(r.0));
            }
            for b in gone {
                let _ = doc.remove_block(b);
            }
            doc.end_group(group);
            Ok((XrefsEdited { name, block: Some(id), revision: doc.revision().to_string(), ..Default::default() }, warnings))
        }
        Plan::Detach { id, name } => {
            let slots = inserts_of(doc, id);
            let removed = u32::try_from(slots.len()).unwrap_or(u32::MAX);
            let gone = brought(doc, id);
            let layer_group = doc.block(id).and_then(|b| b.xref.as_ref().and_then(|x| x.group.clone()));
            let group = doc.begin_group(&format!("Dış başvuruyu kaldır: {name}"));
            doc.remove(&slots);
            let mut refusal = doc.remove_block(id).err();
            for b in gone {
                if refusal.is_none() {
                    refusal = doc.remove_block(b).err();
                }
            }
            if let Some(r) = refusal {
                doc.cancel_group(group);
                return Err(refused(r.0));
            }
            // Its layer group goes with what is under it when nothing is drawn on it.
            if let Some(g) = layer_group
                && doc.layers().get(&g).is_some()
                && group_unused(doc, &g)
            {
                let _ = doc.remove_layer(&g);
            }
            doc.end_group(group);
            Ok((XrefsEdited { name, removed, revision: doc.revision().to_string(), ..Default::default() }, warnings))
        }
        Plan::Paths(paths) => {
            let mut settings = doc.settings().clone();
            settings.xref_paths = paths.clone();
            doc.set_settings(settings);
            Ok((XrefsEdited { paths, revision: doc.revision().to_string(), ..Default::default() }, warnings))
        }
        Plan::Bind { id, name, mode } => {
            let gone = brought(doc, id);
            let mut defs: Vec<BlockDefinition> = std::iter::once(id)
                .chain(gone)
                .filter_map(|b| doc.block(b).cloned())
                .collect();
            let layer_group = defs.first().and_then(|d| d.xref.as_ref().and_then(|x| x.group.clone()));
            let group = doc.begin_group(&format!("Dış başvuruyu bağla: {name}"));
            let mut merged = Vec::new();
            if mode == XrefBindMode::Insert
                && let Some(g) = &layer_group
            {
                match crate::xrefs_bind::merge_layers(doc, g, &mut defs) {
                    Ok(names) => merged = names,
                    Err(why) => {
                        doc.cancel_group(group);
                        return Err(refused(why));
                    }
                }
            }
            for mut d in defs {
                d.xref = None;
                if let Err(r) = doc.update_block(d) {
                    doc.cancel_group(group);
                    return Err(refused(r.0));
                }
            }
            if mode == XrefBindMode::Insert
                && let Some(g) = &layer_group
                && doc.layers().get(g).is_some()
                && group_unused(doc, g)
            {
                let _ = doc.remove_layer(g);
            }
            doc.end_group(group);
            Ok((XrefsEdited { name, merged, revision: doc.revision().to_string(), ..Default::default() }, warnings))
        }
    }
}

pub mod edit {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &XrefsEdit) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
            Err(stop) => stop.into(),
        }
    }

    /// What it would do, on a copy; the drawing stays.
    pub fn plan(cx: &ExecutionContext<'_>, input: &XrefsEdit) -> CommandResult<XrefsEdited> {
        let mut copy = cx.doc.clone();
        match write(&mut copy, input) {
            Ok((mut output, warnings)) => {
                output.revision = cx.doc.revision().to_string();
                // A plan names no insert that does not exist.
                output.insert = None;
                CommandResult::Completed { output, warnings }
            }
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: XrefsEdit) -> CommandResult<XrefsEdited> {
        crate::call::recorded(cx, kentos_contracts::CAD_XREFS_EDIT, kentos_contracts::CAD_XREFS_EDIT_VERSION, input, |cx, input| {
            match write(cx.doc, &input) {
                Ok((output, warnings)) => CommandResult::Completed { output, warnings },
                Err(stop) => stop.into(),
            }
        })
    }
}

/// The inserts that show `block`'s content: its own, or for a reference another brought, the top one's (by lasting id).
fn showing(doc: &Document, block: BlockId) -> Vec<String> {
    let mut top = block;
    for _ in 0..64 {
        match doc.block(top).and_then(|b| b.xref.as_ref()).and_then(|x| x.parent) {
            Some(p) => top = p,
            None => break,
        }
    }
    inserts_of(doc, top).into_iter().filter_map(|s| doc.uid(s).map(|u| u.to_string())).collect()
}

/// The sheets (by name, docs/adr/0347's `shows`) and the named views (by name, their window meeting the box) the
/// objects `uids` are seen on.
fn affected_by(doc: &Document, uids: &[String]) -> (Vec<String>, Vec<String>) {
    if uids.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut sheets: Vec<String> = Vec::new();
    let input = kentos_contracts::SheetsSet { uids: Some(uids.to_vec()) };
    if let Ok(set) = crate::sheets_set::set_of(doc, &input) {
        for a in set.affected.unwrap_or_default() {
            if !a.reasons.contains(&kentos_contracts::AffectReason::Shows) {
                continue;
            }
            if let Some(s) = set.sheets.iter().find(|s| s.id == a.sheet)
                && !sheets.contains(&s.name)
            {
                sheets.push(s.name.clone());
            }
        }
    }
    let blocks = crate::blocks::core_blocks(doc.blocks());
    let font = crate::geometry::drawing_font(doc.settings().drawing_font);
    let boxes: Vec<kentos_geometry_core::geometry::Bounds> = uids
        .iter()
        .filter_map(|u| {
            let slot = doc.slot_of(kentos_domain::Uuid::parse_str(u).ok()?)?;
            let e = doc.get(slot)?;
            Some(kentos_geometry_core::ops::arrange::object_bounds(&crate::geometry::shape(e), &blocks, font))
        })
        .collect();
    let views = doc
        .settings()
        .named_views
        .iter()
        .filter(|v| {
            let (c, s) = (v.twist.cos(), v.twist.sin());
            let (hw, hh) = (v.width / 2.0, v.height / 2.0);
            let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)].map(|(x, y)| {
                (v.center.x + x * c - y * s, v.center.y + x * s + y * c)
            });
            let min_x = corners.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let max_x = corners.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let min_y = corners.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let max_y = corners.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            boxes
                .iter()
                .any(|b| b.min_x <= max_x && min_x <= b.max_x && b.min_y <= max_y && min_y <= b.max_y)
        })
        .map(|v| v.name.clone())
        .collect();
    (sheets, views)
}

/// The drawing's references with their state.
pub fn list_of(doc: &Document, input: &XrefsList) -> XrefsListed {
    let drawing = input.drawing.as_deref();
    let names: HashMap<BlockId, String> = doc.blocks().iter().map(|b| (b.id, b.name.clone())).collect();
    let xrefs = doc
        .blocks()
        .iter()
        .filter_map(|b| {
            let x = b.xref.as_ref()?;
            // An unloaded reference's file is not looked at.
            let look = if x.unloaded {
                crate::links::Found { at: None, via: AssetVia::None, tried: Vec::new() }
            } else {
                found(doc, x, drawing)
            };
            let state = if x.unloaded {
                XrefState::Unloaded
            } else {
                match &look.at {
                    None => XrefState::Missing,
                    Some(p) => match std::fs::read(p) {
                        Ok(bytes) if sha256_hex(&bytes) == x.digest => XrefState::Loaded,
                        Ok(_) => XrefState::Changed,
                        Err(_) => XrefState::Missing,
                    },
                }
            };
            let (sheets, views) = if input.affected { affected_by(doc, &showing(doc, b.id)) } else { Default::default() };
            Some(XrefInfo {
                name: b.name.clone(),
                block: b.id,
                kind: x.kind,
                path: look.at.as_ref().map_or_else(|| x.path.clone(), |p| p.to_string_lossy().into_owned()),
                relative: x.relative.clone(),
                state,
                parent: x.parent.and_then(|p| names.get(&p).cloned()),
                inserts: u32::try_from(inserts_of(doc, b.id).len()).unwrap_or(u32::MAX),
                objects: u32::try_from(b.entities.len()).unwrap_or(u32::MAX),
                group: x.group.as_ref().and_then(|g| doc.layers().get(g).map(|n| n.name.clone())),
                via: look.via,
                tried: look.tried,
                sheets,
                views,
            })
        })
        .collect();
    XrefsListed { xrefs, revision: doc.revision().to_string() }
}

pub mod list {
    use super::*;

    pub fn validate(_cx: &ExecutionContext<'_>, _input: &XrefsList) -> CommandResult<()> {
        CommandResult::Completed { output: (), warnings: Vec::new() }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &XrefsList) -> CommandResult<XrefsListed> {
        CommandResult::Completed { output: list_of(cx.doc, input), warnings: Vec::new() }
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(cx: &mut ExecutionContext<'_>, input: XrefsList) -> CommandResult<XrefsListed> {
        plan(cx, &input)
    }
}

/// Each reference's path from the drawing's folder, written when the drawing is saved at `project_file` (the
/// snapshot written; the drawing unchanged), as a bound layer's (docs/adr/0506).
pub fn with_relative(blocks: &mut [BlockDefinition], project_file: &Path) {
    let Some(folder) = project_file.parent() else { return };
    for b in blocks {
        if let Some(x) = b.xref.as_mut()
            && let Some(r) = relative_path(folder, Path::new(&x.path))
        {
            x.relative = Some(r);
        }
    }
}
