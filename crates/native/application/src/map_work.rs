//! KentOS Harita's work structure (plan/map-cad.md MAP-01, docs/adr/1200): what an object is in the map's work (the
//! item of its layer or of the nearest group above it that an item holds), how an item's file and a derivation stand,
//! an item's content revision and approval stamp (docs/adr/2080), and `map.work.query`'s answer. Reads only;
//! `map_work_edit.rs` writes. The shared cases are `fixtures/commands/v1/map.work.*.json`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use kentos_commands::{
    CommandResult, WorkDerivationRow, MapWork, MapWorkQuery, MapWorkTree, WorkObjectRole, WorkObjectSource, WorkDerivation,
    WorkFileRow, WorkFileState as F, WorkItem, WorkRow, WorkState as S, WorkStatus, encoding::sha256_hex,
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
pub fn item_of_layer<'w>(doc: &Document, work: &'w MapWork, layer: &str) -> Option<&'w WorkItem> {
    let mut id = Some(layer.to_owned());
    while let Some(at) = id {
        if let Some(item) = work.items.iter().find(|i| i.layers.contains(&at)) {
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

/// How an item's file stands: its state, read deep its content's SHA-256, where it was found and how.
#[derive(Clone, Debug)]
pub(crate) struct FileLook {
    pub state: F,
    pub sha: Option<String>,
    pub at: Option<std::path::PathBuf>,
    pub via: kentos_commands::AssetVia,
}

/// How each item's file stands, by item id: looked for at its path, then (with the open project's file `drawing`) at
/// its relative path from the project's folder and in the project's search paths (docs/adr/1201 §1); read deep, its
/// content's SHA-256 against the item's revision.
pub(crate) fn files_at(doc: &Document, work: &MapWork, deep: bool, drawing: Option<&str>) -> HashMap<String, FileLook> {
    let search = &doc.settings().xref_paths;
    work.items
        .iter()
        .filter_map(|i| {
            let f = i.file.as_ref()?;
            let found = crate::links::find(&f.path, f.relative.as_deref(), drawing, search);
            let look = match (&found.at, deep) {
                (None, _) => FileLook { state: F::Missing, sha: None, at: None, via: found.via },
                (Some(at), false) => FileLook { state: F::Found, sha: None, at: Some(at.clone()), via: found.via },
                (Some(at), true) => match file_sha(&at.display().to_string()) {
                    Some(sha) if sha == f.revision => FileLook { state: F::Current, sha: Some(sha), at: Some(at.clone()), via: found.via },
                    Some(sha) => FileLook { state: F::Changed, sha: Some(sha), at: Some(at.clone()), via: found.via },
                    None => FileLook { state: F::Missing, sha: None, at: None, via: kentos_commands::AssetVia::None },
                },
            };
            Some((i.id.clone(), look))
        })
        .collect()
}

/// [`files_at`] at the items' paths only, by item id: the state and, read deep, the SHA-256.
fn files(doc: &Document, work: &MapWork, deep: bool) -> HashMap<String, (F, Option<String>)> {
    files_at(doc, work, deep, None).into_iter().map(|(id, l)| (id, (l.state, l.sha))).collect()
}

/// The content revision's name and version (docs/adr/2080 §1).
pub const WORK_CONTENT: &str = "kentos.work-content/1";

/// The content revision of `item` ([`WORK_CONTENT`], docs/adr/2080 §1): SHA-256 of the compact JSON list
/// `[[uid, object digest], …]` of the objects its layers hold (the layer profile: its layers' and its groups'), by uid;
/// the object digest is `kentos.object-digest/1` (docs/adr/1320), the object without its slot id. An item that holds no
/// objects has the digest of `[]`. Its layers' look (colour, style) is not its content; an object's own fields are.
pub fn content_revision(doc: &Document, work: &MapWork, item: &str) -> String {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for leaf in doc.layers().leaves() {
        if item_of_layer(doc, work, &leaf.id).is_none_or(|i| i.id != item) {
            continue;
        }
        for (uid, entity) in doc.by_layer_with_uids(&leaf.id) {
            pairs.push((uid.to_string(), crate::sheets_sources::object_digest(entity)));
        }
    }
    pairs.sort_unstable();
    sha256_hex(&serde_json::to_vec(&pairs).unwrap_or_default())
}

/// Each item's revision now, worked out once a read: an item with a file, its kept revision (or read deep, a changed
/// file's content); one without, its content revision, worked out when first asked (docs/adr/2080 §1).
pub(crate) struct Revisions<'a> {
    doc: &'a Document,
    work: &'a MapWork,
    files: HashMap<String, (F, Option<String>)>,
    contents: RefCell<HashMap<String, String>>,
}

impl<'a> Revisions<'a> {
    pub(crate) fn new(doc: &'a Document, work: &'a MapWork, files: HashMap<String, (F, Option<String>)>) -> Self {
        Revisions { doc, work, files, contents: RefCell::new(HashMap::new()) }
    }

    /// The content revision of an item without a file, worked out once.
    pub(crate) fn content(&self, item: &str) -> String {
        if let Some(c) = self.contents.borrow().get(item) {
            return c.clone();
        }
        let c = content_revision(self.doc, self.work, item);
        self.contents.borrow_mut().insert(item.to_owned(), c.clone());
        c
    }

    /// The revision an approval stamps and is judged against: its file's kept revision, or its content revision.
    pub(crate) fn kept(&self, item: &WorkItem) -> String {
        match &item.file {
            Some(f) => f.revision.clone(),
            None => self.content(&item.id),
        }
    }

    /// The revision a derivation from `source` is judged against: the item's, or read deep, its changed file's content.
    fn now(&self, source: &str) -> Option<String> {
        if let Some((F::Changed, Some(sha))) = self.files.get(source) {
            return Some(sha.clone());
        }
        self.work.item(source).map(|i| self.kept(i))
    }

    /// Whether a derivation's source moved on since its objects were made.
    fn stale(&self, d: &WorkDerivation) -> bool {
        self.now(&d.source).as_deref() != Some(d.revision.as_str())
    }
}

/// How many derivations are out of date by the items' revisions (their kept files' and their contents now).
pub(crate) fn stale_count(doc: &Document, work: &MapWork) -> u32 {
    let revisions = Revisions::new(doc, work, HashMap::new());
    count(work.derivations.iter().filter(|d| revisions.stale(d)).count())
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn item_name(work: &MapWork, id: &str) -> String {
    work.item(id).map_or_else(|| id.to_owned(), |i| i.name.clone())
}

/// The objects of out-of-date derivations whose layer `item` holds (an approved item's warning, approval's refusal).
pub(crate) fn stale_objects_in(doc: &Document, work: &MapWork, item: &WorkItem, deep_files: bool) -> u32 {
    let files = if deep_files { files(doc, work, true) } else { HashMap::new() };
    stale_objects_by(doc, work, item, &Revisions::new(doc, work, files))
}

/// [`stale_objects_in`] by the revisions a read already worked out.
fn stale_objects_by(doc: &Document, work: &MapWork, item: &WorkItem, revisions: &Revisions<'_>) -> u32 {
    let mut n = 0usize;
    for d in work.derivations.iter().filter(|d| revisions.stale(d)) {
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
pub(crate) fn tree(doc: &Document, deep: bool, uid: Option<&str>, drawing: Option<&str>) -> Result<MapWorkTree, Stop> {
    map_project(doc)?;
    if let Some(d) = drawing.filter(|d| !Path::new(d).is_absolute()) {
        return Err(Stop::failed(error(
            codes::INVALID_FILE_PATH,
            format!("“{d}” mutlak bir yol değil; açık projenin dosyasının tam yolunu verin."),
            Some("drawing".into()),
        )));
    }
    let empty = MapWork::default();
    let work = doc.map_work().unwrap_or(&empty);
    let object = match uid {
        Some(u) => Some(object_source(doc, work, u, deep, None)?),
        None => None,
    };
    let looks = files_at(doc, work, deep, drawing);
    let files: HashMap<String, (F, Option<String>)> =
        looks.iter().map(|(id, l)| (id.clone(), (l.state, l.sha.clone()))).collect();
    let revisions = Revisions::new(doc, work, files);
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
        let old = revisions.stale(d);
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
            let look = looks.get(&i.id);
            WorkFileRow {
                path: f.path.clone(),
                revision: f.revision.clone(),
                state: look.map_or(F::Missing, |l| l.state),
                via: look.map_or(kentos_commands::AssetVia::None, |l| l.via),
                // Where it is when not at its path: the relative path's file or a search path's.
                at: look
                    .and_then(|l| l.at.as_ref())
                    .map(|p| p.display().to_string())
                    .filter(|p| *p != f.path),
                relative: f.relative.clone(),
            }
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
            let n = stale_objects_by(doc, work, i, &revisions);
            if n > 0 {
                worse(
                    S::Stale,
                    format!("Onaylı, ama katmanlarındaki {n} türetilmiş nesnenin kaynağı onaydan sonra değişti."),
                    &mut why,
                );
            }
        }
        // Without a file: its content revision where it is judged, or read deep (docs/adr/2080 §1).
        let judged = i.approved.is_some() || work.derivations.iter().any(|d| d.source == i.id);
        let content = (i.file.is_none() && (deep || judged)).then(|| revisions.content(&i.id));
        // An approval's stamp against the item's revision now (docs/adr/2080 §3).
        if let Some(stamp) = i.approved.as_ref().filter(|_| i.status == WorkStatus::Approved) {
            let now = revisions.kept(i);
            if now != *stamp {
                let what = if i.file.is_some() { "dosyası" } else { "içeriği" };
                worse(
                    S::Revised,
                    format!(
                        "Onaydan sonra {what} değişti (onaylanan {}, şimdi {}): değişikliği denetleyip yeniden onaylayın.",
                        short(stamp),
                        short(&now)
                    ),
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
            content,
            approved: i.approved.clone(),
            state,
            why,
        });
    }
    Ok(MapWorkTree { items, derivations, object, stale, problems, deep, revision: doc.revision().to_string() })
}

/// A revision's first 12 hex digits, as the panel shows it.
fn short(revision: &str) -> &str {
    revision.get(..12).unwrap_or(revision)
}

/// Where the object `uid` comes from; `shared` lets the objects of one selection share the content revisions worked
/// out.
fn object_source(
    doc: &Document,
    work: &MapWork,
    uid: &str,
    deep: bool,
    shared: Option<&Revisions<'_>>,
) -> Result<WorkObjectSource, Stop> {
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
    let own;
    let revisions = match shared {
        Some(r) => r,
        None => {
            let files = if deep { files(doc, work, true) } else { HashMap::new() };
            own = Revisions::new(doc, work, files);
            &own
        }
    };
    if let Some(d) = work.derivations.iter().find(|d| d.uids.iter().any(|u| u.to_text() == uid)) {
        out.role = WorkObjectRole::Derived;
        out.derivation = Some(d.id.clone());
        out.source = Some(d.source.clone());
        out.revision = Some(d.revision.clone());
        out.operation = Some(d.operation.clone());
        out.current = Some(!revisions.stale(d));
    } else if let Some(i) = item.filter(|i| i.kind.is_source()) {
        out.role = WorkObjectRole::Source;
        out.source = Some(i.id.clone());
        out.revision = Some(revisions.kept(i));
    }
    Ok(out)
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &MapWorkQuery) -> CommandResult<()> {
    match tree(cx.doc, input.deep, input.uid.as_deref(), input.drawing.as_deref()) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &MapWorkQuery) -> CommandResult<MapWorkTree> {
    match tree(cx.doc, input.deep, input.uid.as_deref(), input.drawing.as_deref()) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: MapWorkQuery) -> CommandResult<MapWorkTree> {
    plan(cx, &input)
}

/// The tree for a panel (and, with `uid`, the object's source), or why it does not read.
pub fn preview(doc: &Document, deep: bool, uid: Option<&str>, drawing: Option<&str>) -> Result<MapWorkTree, String> {
    tree(doc, deep, uid, drawing).map_err(|stop| stop.into_error().message)
}

/// Where one object comes from, read quickly (the items' revisions, no file read), for the properties; none outside a
/// map project with a work structure or for an object not in the drawing.
pub fn source_of(doc: &Document, uid: &str) -> Option<WorkObjectSource> {
    sources_of(doc, std::slice::from_ref(&uid.to_owned())).pop()
}

/// [`source_of`] for each of `uids` that is in the drawing, one content revision worked out per item.
pub fn sources_of(doc: &Document, uids: &[String]) -> Vec<WorkObjectSource> {
    if doc.settings().cad_work != Some(kentos_commands::CadWork::Map) {
        return Vec::new();
    }
    let Some(work) = doc.map_work() else { return Vec::new() };
    let revisions = Revisions::new(doc, work, HashMap::new());
    uids.iter().filter_map(|u| object_source(doc, work, u, false, Some(&revisions)).ok()).collect()
}

/// An object's role and the item its layer belongs to, nothing worked out (the ribbon asks every frame); none outside
/// a map project with a work structure or for an object not in the drawing.
pub fn role_of(doc: &Document, uid: &str) -> Option<(WorkObjectRole, Option<String>)> {
    if doc.settings().cad_work != Some(kentos_commands::CadWork::Map) {
        return None;
    }
    let work = doc.map_work()?;
    let uuid = Uuid::parse_str(uid).ok()?;
    let entity = doc.slot_of(uuid).and_then(|s| doc.get(s))?;
    let item = item_of_layer(doc, work, &entity.base().layer_id);
    let id = kentos_commands::EntityId(*uuid.as_bytes());
    let role = if work.derivations.iter().any(|d| d.uids.contains(&id)) {
        WorkObjectRole::Derived
    } else if item.is_some_and(|i| i.kind.is_source()) {
        WorkObjectRole::Source
    } else {
        WorkObjectRole::Free
    };
    Some((role, item.map(|i| i.id.clone())))
}
