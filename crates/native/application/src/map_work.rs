//! KentOS Harita's work structure (plan/map-cad.md MAP-01, docs/adr/1200): what an object is in the map's work (the
//! item of its layer or of the nearest group above it that an item holds), how an item's file and a derivation stand,
//! and `map.work.query`'s answer. Reads only; `map_work_edit.rs` writes. The shared cases are
//! `fixtures/commands/v1/map.work.*.json`.

use std::collections::HashMap;
use std::path::Path;

use kentos_commands::{
    CommandResult, WorkDerivationRow, MapWork, MapWorkQuery, MapWorkTree, WorkObjectRole, WorkObjectSource, WorkDerivation,
    WorkFileRow, WorkFileState as F, WorkItem, WorkRow, WorkState as S, WorkStatus,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

/// Refuses a drawing that is not a map project's.
pub(crate) fn map_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work != Some(kentos_commands::CadWork::Map) {
        return Err(Stop::failed(error(
            codes::NOT_A_MAP_PROJECT,
            "Harita işi yalnız harita projesinde (KentOS Harita) olur; bu çizim harita projesi değil.".into(),
            None,
        )));
    }
    Ok(())
}

/// The item that holds `layer`: the item that names it, else the one that names the nearest group above it.
pub(crate) fn item_of_layer<'w>(doc: &Document, work: &'w MapWork, layer: &str) -> Option<&'w WorkItem> {
    let mut id = Some(layer.to_owned());
    while let Some(at) = id {
        if let Some(item) = work.items.iter().find(|i| i.layers.iter().any(|l| *l == at)) {
            return Some(item);
        }
        id = doc.layers().parent(&at).map(|p| p.id.clone());
    }
    None
}

/// Every layer of the drawing with the item that holds it.
fn layer_items<'w>(doc: &Document, work: &'w MapWork) -> HashMap<String, &'w WorkItem> {
    doc.layers()
        .leaves()
        .into_iter()
        .filter_map(|l| item_of_layer(doc, work, &l.id).map(|i| (l.id.clone(), i)))
        .collect()
}

/// The file's SHA-256, or none when it cannot be read.
pub(crate) fn file_sha(path: &str) -> Option<String> {
    crate::file_digest::sha256(Path::new(path)).ok().map(|(_, sha)| sha)
}

/// How each item's file stands, by item id: its state and, read deep, its content's SHA-256.
fn files(work: &MapWork, deep: bool) -> HashMap<String, (F, Option<String>)> {
    work.items
        .iter()
        .filter_map(|i| {
            let f = i.file.as_ref()?;
            let state = if !deep {
                (if Path::new(&f.path).is_file() { F::Found } else { F::Missing }, None)
            } else {
                match file_sha(&f.path) {
                    Some(sha) if sha == f.revision => (F::Current, Some(sha)),
                    Some(sha) => (F::Changed, Some(sha)),
                    None => (F::Missing, None),
                }
            };
            Some((i.id.clone(), state))
        })
        .collect()
}

/// The source's revision a derivation is judged against: the item's, or read deep, its file's content now.
fn source_now<'a>(work: &'a MapWork, d: &WorkDerivation, files: &'a HashMap<String, (F, Option<String>)>) -> Option<&'a str> {
    if let Some((F::Changed, Some(sha))) = files.get(&d.source) {
        return Some(sha.as_str());
    }
    work.item(&d.source).and_then(|i| i.file.as_ref()).map(|f| f.revision.as_str())
}

/// Whether a derivation's source moved on since its objects were made (quick: the item's revision; deep: its file).
fn is_stale(work: &MapWork, d: &WorkDerivation, files: &HashMap<String, (F, Option<String>)>) -> bool {
    source_now(work, d, files) != Some(d.revision.as_str())
}

/// How many derivations are out of date by the items' revisions.
pub(crate) fn stale_count(work: &MapWork) -> u32 {
    let none = HashMap::new();
    count(work.derivations.iter().filter(|d| is_stale(work, d, &none)).count())
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn item_name(work: &MapWork, id: &str) -> String {
    work.item(id).map_or_else(|| id.to_owned(), |i| i.name.clone())
}

/// The objects of out-of-date derivations whose layer `item` holds (an approved item's warning, approval's refusal).
pub(crate) fn stale_objects_in(doc: &Document, work: &MapWork, item: &WorkItem, deep_files: bool) -> u32 {
    let files = if deep_files { files(work, true) } else { HashMap::new() };
    let mut n = 0usize;
    for d in work.derivations.iter().filter(|d| is_stale(work, d, &files)) {
        for u in &d.uids {
            let Some(entity) = doc.slot_of(Uuid::from_bytes(u.0)).and_then(|s| doc.get(s))
            else {
                continue;
            };
            if item_of_layer(doc, work, &entity.base().layer_id).is_some_and(|i| i.id == item.id) {
                n += 1;
            }
        }
    }
    count(n)
}

/// The rows of the work structure as they stand now.
pub(crate) fn tree(doc: &Document, deep: bool, uid: Option<&str>) -> Result<MapWorkTree, Stop> {
    map_project(doc)?;
    let empty = MapWork::default();
    let work = doc.map_work().unwrap_or(&empty);
    let object = match uid {
        Some(u) => Some(object_source(doc, work, u, deep)?),
        None => None,
    };
    let files = files(work, deep);
    let held = layer_items(doc, work);
    let mut objects: HashMap<&str, usize> = HashMap::new();
    for (layer, item) in &held {
        *objects.entry(item.id.as_str()).or_default() += doc.by_layer(layer).count();
    }
    let mut derivations = Vec::new();
    let mut stale = 0u32;
    let mut problems = 0u32;
    for d in &work.derivations {
        let present = d
            .uids
            .iter()
            .filter(|u| doc.slot_of(Uuid::from_bytes(u.0)).is_some())
            .count();
        let mut why = Vec::new();
        let old = is_stale(work, d, &files);
        if old {
            why.push(format!(
                "Kaynağı “{}” bu nesneler üretildikten sonra değişti.",
                item_name(work, &d.source)
            ));
        }
        let gone = d.uids.len() - present;
        if gone > 0 {
            why.push(format!("{gone} nesnesi çizimde yok."));
            problems += 1;
        }
        let state = if old {
            stale += 1;
            S::Stale
        } else if gone > 0 {
            S::Missing
        } else {
            S::Current
        };
        derivations.push(WorkDerivationRow {
            id: d.id.clone(),
            source: d.source.clone(),
            operation: d.operation.clone(),
            revision: d.revision.clone(),
            objects: count(present),
            recorded: count(d.uids.len()),
            state,
            why,
        });
    }
    let mut items = Vec::new();
    for i in &work.items {
        let mut why = Vec::new();
        let mut state = S::Current;
        let mut worse = |s: S, words: String, why: &mut Vec<String>| {
            why.push(words);
            if state == S::Current {
                state = s;
            }
        };
        let file = i.file.as_ref().map(|f| {
            let fs = files.get(&i.id).map_or(F::Missing, |(s, _)| *s);
            WorkFileRow { path: f.path.clone(), revision: f.revision.clone(), state: fs }
        });
        match file.as_ref().map(|f| f.state) {
            Some(F::Missing) => {
                let path = file.as_ref().map_or("", |f| f.path.as_str());
                worse(S::Missing, format!("Dosyası yok: {path}. Yeniden bağla ile yeni yerini gösterin."), &mut why);
            }
            Some(F::Changed) => worse(
                S::Changed,
                "Dosyası projenin kabul ettiği içerikten farklı: Yenile ile kabul edin ya da Yeniden bağla ile başka dosyayı gösterin."
                    .into(),
                &mut why,
            ),
            _ => {}
        }
        for l in &i.layers {
            if doc.layers().get(l).is_none() {
                worse(S::Incomplete, format!("“{l}” katmanı çizimde yok."), &mut why);
            }
        }
        if i.status == WorkStatus::Approved {
            let n = stale_objects_in(doc, work, i, deep);
            if n > 0 {
                worse(
                    S::Stale,
                    format!("Onaylı, ama katmanlarındaki {n} türetilmiş nesnenin kaynağı onaydan sonra değişti."),
                    &mut why,
                );
            }
        }
        if state != S::Current {
            problems += 1;
        }
        items.push(WorkRow {
            id: i.id.clone(),
            kind: i.kind,
            name: i.name.clone(),
            status: i.status,
            layers: i.layers.clone(),
            objects: count(objects.get(i.id.as_str()).copied().unwrap_or(0)),
            file,
            state,
            why,
        });
    }
    Ok(MapWorkTree { items, derivations, object, stale, problems, deep, revision: doc.revision().to_string() })
}

/// Where the object `uid` comes from.
fn object_source(doc: &Document, work: &MapWork, uid: &str, deep: bool) -> Result<WorkObjectSource, Stop> {
    if !crate::checks::is_uid_text(uid) {
        return Err(Stop::failed(error(
            codes::INVALID_UID,
            format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
            Some("uid".into()),
        )));
    }
    let Some(entity) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
        return Err(Stop::failed(error(
            codes::ENTITY_NOT_FOUND,
            format!("“{uid}” kimlikli nesne çizimde yok."),
            Some("uid".into()),
        )));
    };
    let item = item_of_layer(doc, work, &entity.base().layer_id);
    let mut out = WorkObjectSource {
        uid: uid.to_owned(),
        role: WorkObjectRole::Free,
        item: item.map(|i| i.id.clone()),
        derivation: None,
        source: None,
        revision: None,
        operation: None,
        current: None,
    };
    if let Some(d) = work.derivations.iter().find(|d| d.uids.iter().any(|u| u.to_text() == uid)) {
        let files = if deep { files(work, true) } else { HashMap::new() };
        out.role = WorkObjectRole::Derived;
        out.derivation = Some(d.id.clone());
        out.source = Some(d.source.clone());
        out.revision = Some(d.revision.clone());
        out.operation = Some(d.operation.clone());
        out.current = Some(!is_stale(work, d, &files));
    } else if let Some(i) = item.filter(|i| i.kind.is_source()) {
        out.role = WorkObjectRole::Source;
        out.source = Some(i.id.clone());
        out.revision = i.file.as_ref().map(|f| f.revision.clone());
    }
    Ok(out)
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &MapWorkQuery) -> CommandResult<()> {
    match tree(cx.doc, input.deep, input.uid.as_deref()) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &MapWorkQuery) -> CommandResult<MapWorkTree> {
    match tree(cx.doc, input.deep, input.uid.as_deref()) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: MapWorkQuery) -> CommandResult<MapWorkTree> {
    plan(cx, &input)
}

/// The tree for a panel (and, with `uid`, the object's source), or why it does not read.
pub fn preview(doc: &Document, deep: bool, uid: Option<&str>) -> Result<MapWorkTree, String> {
    tree(doc, deep, uid).map_err(|stop| stop.into_error().message)
}

/// Where one object comes from, read quickly (the items' revisions, no file read), for the properties; none outside a
/// map project with a work structure or for an object not in the drawing.
pub fn source_of(doc: &Document, uid: &str) -> Option<WorkObjectSource> {
    if doc.settings().cad_work != Some(kentos_commands::CadWork::Map) {
        return None;
    }
    object_source(doc, doc.map_work()?, uid, false).ok()
}
