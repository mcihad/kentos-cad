//! Veri karşılaştır (docs/adr/0179; the web's `app/dataCompare.ts` and
//! `ui/data/DataCompareDialog.ts`): two sides, each a layer, a group or a
//! whole drawing (the old one may come from another drawing file), paired and
//! compared by the core (`kentos_geometry_core::ops::compare`, the web's
//! through WASM): the rows' words, the report, and Farkları çizime yaz, which
//! writes the differences' copies into a “Karşılaştırma” group in one undo
//! step “Veri karşılaştır”.

use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

use iced::widget::{Column, container, row, text, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{CommandResult, EntitiesCreate, Entity, LayerNode, LayerNodeType};
use kentos_domain::{NewLayer, Slot};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::compare::{self, Attrs, Matching, Member, Row, Settings, Status};
use kentos_interaction::layer_move::new_object;
use kentos_interaction::{Level, ViewChange};
use kentos_native_application::{ExecutionContext, create};
use kentos_ui::theme::Tokens;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{Dialog as Frame, Segmented, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const COMPARE_TITLE: &str = "Veri karşılaştır";
const SEARCH: &str = "Arama uzaklığı (m)";
const TOLERANCE: &str = "Tolerans (m)";
const ONLY: &str = "Yalnız farklar";
const COMPARE: &str = "Karşılaştır";
const FROM_FILE: &str = "Dosyadan…";
const COPY: &str = "Panoya kopyala";
const SAVE: &str = "CSV olarak kaydet…";
const WRITE: &str = "Farkları çizime yaz";
const CLOSE: &str = "Kapat";
const OLD_DATE: &str = "Eski tarih";
const NEW_DATE: &str = "Yeni tarih";

/// A finding's words.
pub fn status_words(s: Status) -> &'static str {
    match s {
        Status::Same => "Aynı",
        Status::Geometry => "Geometrisi değişti",
        Status::Attributes => "Öznitelikleri değişti",
        Status::Both => "Geometrisi ve öznitelikleri değişti",
        Status::Added => "Eklenen",
        Status::Removed => "Silinen",
        Status::Key => "Anahtar sorunu",
    }
}

/// How the window pairs, as the segmented control says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pairing {
    Location,
    Key,
}

impl std::fmt::Display for Pairing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Pairing::Location => "Konumla",
            Pairing::Key => "Anahtar alanla",
        })
    }
}

/// A side's drawing: another read from a file (the open one is the app's).
#[derive(Debug, Clone)]
pub struct Other {
    name: String,
    tree: Vec<LayerNode>,
    entities: Vec<Entity>,
    system: String,
}

/// A side's objects and the tree their layers are in.
struct Side<'a> {
    tree: &'a [LayerNode],
    entities: Vec<&'a Entity>,
    system: String,
    here: bool,
    name: String,
    /// A layer's or a group's id; none, the whole drawing.
    node: Option<&'a str>,
    /// The moment its temporal layers are seen at (docs/adr/0210 §8).
    at: Option<f64>,
}

/// The comparison's rows with the two sides' objects they index.
#[derive(Debug, Clone)]
pub struct Outcome {
    rows: Vec<Row>,
    old: Vec<Entity>,
    new: Vec<Entity>,
    old_tree: Vec<LayerNode>,
    new_tree: Vec<LayerNode>,
    old_here: bool,
    key: Option<String>,
}

/// The window.
#[derive(Debug, Clone)]
pub struct Window {
    other: Option<Other>,
    old_here: bool,
    old_node: String,
    new_node: String,
    pairing: Pairing,
    key: String,
    search: String,
    tolerance: String,
    ignored: BTreeSet<String>,
    only_diffs: bool,
    outcome: Option<Outcome>,
    /// The sides' dates as written (docs/adr/0210 §8): empty, every object.
    old_date: String,
    new_date: String,
}

/// What a window opened for a purpose starts with (Zamanı karşılaştır,
/// Senaryoyu karşılaştır, a revision; docs/adr/0210 §8).
#[derive(Debug, Clone, Default)]
pub struct Preset {
    /// Eski veri from another drawing (a revision's).
    pub other: Option<Other>,
    pub old_node: Option<String>,
    pub new_node: Option<String>,
    pub old_date: String,
    pub new_date: String,
    /// Anahtar alanla, by this field.
    pub key: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Eski veri: this drawing (true) or the file read.
    Source(bool),
    FromFile,
    FilePicked(Option<PathBuf>),
    OldNode(String),
    NewNode(String),
    Pairing(Pairing),
    Key(String),
    Search(String),
    Tolerance(String),
    OldDate(String),
    NewDate(String),
    /// A field compared (true) or left out.
    Field(String, bool),
    OnlyDiffs(bool),
    Compare,
    /// A row of the list (its index among those shown): its object shown.
    Show(usize),
    Copy,
    Save,
    Saved(Option<PathBuf>),
    Write,
    Close,
}

fn msg(event: Event) -> Message {
    Message::DataCompare(event)
}

fn walk<'a>(nodes: &'a [LayerNode], above: &[String], out: &mut Vec<(&'a LayerNode, String)>) {
    for n in nodes {
        let mut path = above.to_vec();
        path.push(n.name.clone());
        out.push((n, path.join(" / ")));
        walk(&n.children, &path, out);
    }
}

/// A side's choices: the whole drawing, then every group and layer by its path.
fn node_choices(tree: &[LayerNode]) -> Vec<(String, String)> {
    let mut all = Vec::new();
    walk(tree, &[], &mut all);
    std::iter::once((String::new(), "Bütün çizim".to_owned()))
        .chain(all.into_iter().map(|(n, path)| (n.id.clone(), path)))
        .collect()
}

/// The layers under a node (itself when a layer); every layer for the whole drawing.
fn layers_of(tree: &[LayerNode], node: Option<&str>) -> HashSet<String> {
    fn take(n: &LayerNode, out: &mut HashSet<String>) {
        if n.kind == LayerNodeType::Layer {
            out.insert(n.id.clone());
        }
        for c in &n.children {
            take(c, out);
        }
    }
    let mut out = HashSet::new();
    let mut all = Vec::new();
    walk(tree, &[], &mut all);
    match node {
        None => tree.iter().for_each(|n| take(n, &mut out)),
        Some(id) => {
            if let Some((n, _)) = all.iter().find(|(n, _)| n.id == id) {
                take(n, &mut out);
            }
        }
    }
    out
}

/// The objects on the layers under a node (none: the whole drawing), in their
/// order; at a moment, its temporal layers' objects shown then (docs/adr/0210 §8).
fn objects_under<'e>(
    tree: &[LayerNode],
    all: Vec<&'e Entity>,
    node: &str,
    at: Option<f64>,
) -> Vec<&'e Entity> {
    let layers = layers_of(tree, (!node.is_empty()).then_some(node));
    let list: Vec<&Entity> = all
        .into_iter()
        .filter(|e| layers.contains(&e.base().layer_id))
        .collect();
    let Some(at) = at else {
        return list;
    };
    let mut nodes = Vec::new();
    walk(tree, &[], &mut nodes);
    let rules: std::collections::HashMap<&str, &kentos_contracts::LayerTime> = nodes
        .iter()
        .filter(|(n, _)| n.kind == LayerNodeType::Layer && layers.contains(&n.id))
        .filter_map(|(n, _)| n.time.as_ref().map(|t| (n.id.as_str(), t)))
        .collect();
    let window = kentos_geometry_core::time::Window::Instant(at);
    list.into_iter()
        .filter(|e| {
            let Some(rule) = rules.get(e.base().layer_id.as_str()) else {
                return true;
            };
            let a = &e.base().attrs;
            let core = kentos_geometry_core::time::Rule {
                ranged: rule.end.is_some(),
                cumulative: rule.cumulative,
            };
            let read = |k: Option<&String>| {
                k.and_then(|k| a.get(k))
                    .map_or(kentos_geometry_core::time::Read::Empty, |v| {
                        kentos_geometry_core::time::read(v)
                    })
            };
            let end = if core.ranged {
                read(rule.end.as_ref())
            } else {
                kentos_geometry_core::time::Read::Empty
            };
            kentos_geometry_core::time::object_time(core, read(Some(&rule.start)), end)
                .is_none_or(|t| kentos_geometry_core::time::shows(&t, &window))
        })
        .collect()
}

/// A date field's moment: none when empty, an error when it does not read.
fn date_of(words: &str, field: &str) -> Result<Option<f64>, String> {
    match kentos_geometry_core::time::read(words.trim()) {
        kentos_geometry_core::time::Read::Empty => Ok(None),
        kentos_geometry_core::time::Read::Moment(t) => Ok(Some(t)),
        kentos_geometry_core::time::Read::Unreadable => Err(format!(
            "{field} okunamadı: “{}”. Tarihi 05.03.2024 ya da 2024-03-05 gibi yazın ya da boş bırakın.",
            words.trim()
        )),
    }
}

/// A layer's path in a tree.
fn path_of(tree: &[LayerNode], id: &str) -> String {
    let mut all = Vec::new();
    walk(tree, &[], &mut all);
    all.into_iter()
        .find(|(n, _)| n.id == id)
        .map_or_else(|| id.to_owned(), |(_, p)| p)
}

/// The attribute fields of the sides' objects, each once, in the order of
/// their characters' codes (as the core's rows).
fn fields_of(sides: &[&Side<'_>]) -> Vec<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    for s in sides {
        for e in &s.entities {
            out.extend(e.base().attrs.keys().cloned());
        }
    }
    out.into_iter().collect()
}

/// A project's coordinate system as the comparison checks it: the EPSG code,
/// or its own definition.
fn system_of(settings: &kentos_contracts::ProjectSettings) -> String {
    match &settings.custom_crs {
        Some(def) => serde_json::to_string(def).unwrap_or_default(),
        None => settings.srid.to_string(),
    }
}

/// “1 eklenen, 1 silinen, 4 değişen, 2 aynı, 1 anahtar sorunu”: the counts that are not naught.
pub fn counts_text(rows: &[Row]) -> String {
    let count = |f: &dyn Fn(Status) -> bool| rows.iter().filter(|r| f(r.status)).count();
    let words = [
        (count(&|s| s == Status::Added), "eklenen"),
        (count(&|s| s == Status::Removed), "silinen"),
        (
            count(&|s| matches!(s, Status::Geometry | Status::Attributes | Status::Both)),
            "değişen",
        ),
        (count(&|s| s == Status::Same), "aynı"),
        (count(&|s| s == Status::Key), "anahtar sorunu"),
    ];
    let said: Vec<String> = words
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, w)| format!("{n} {w}"))
        .collect();
    if said.is_empty() {
        "karşılaştırılacak nesne yok".to_owned()
    } else {
        said.join(", ")
    }
}

/// The report's header (the window's columns too).
pub const REPORT_HEADER: [&str; 7] = [
    "Durum",
    "Tür",
    "Anahtar",
    "Eski katman",
    "Yeni katman",
    "Konum farkı (m)",
    "Değişen alanlar",
];

/// A row's words: Durum, Tür, Anahtar, Eski katman, Yeni katman, Konum farkı, Değişen alanlar.
fn row_words(o: &Outcome, row: &Row, metres: &dyn Fn(f64) -> String) -> Vec<String> {
    let a = row.old.map(|i| &o.old[i]);
    let b = row.new.map(|i| &o.new[i]);
    let either = b.or(a);
    vec![
        status_words(row.status).to_owned(),
        either.map_or_else(String::new, |e| {
            crate::selecting::kind_title(e.kind()).to_owned()
        }),
        match (&o.key, either) {
            (Some(k), Some(e)) => e.base().attrs.get(k).cloned().unwrap_or_default(),
            _ => String::new(),
        },
        a.map_or_else(String::new, |e| path_of(&o.old_tree, &e.base().layer_id)),
        b.map_or_else(String::new, |e| path_of(&o.new_tree, &e.base().layer_id)),
        row.distance.map_or_else(String::new, metres),
        row.fields.join(", "),
    ]
}

/// The report's rows: the header, then every row (the decimal comma, as the CSV file takes it).
fn report_rows(o: &Outcome, only_diffs: bool) -> Vec<Vec<String>> {
    let mut out = vec![REPORT_HEADER.iter().map(|h| (*h).to_owned()).collect()];
    for row in o
        .rows
        .iter()
        .filter(|r| !only_diffs || r.status != Status::Same)
    {
        out.push(row_words(o, row, &|m| fixed(m, 3).replace('.', ",")));
    }
    out
}

impl App {
    /// Veri karşılaştır (`data.compare`): the window, Eski the first layer
    /// that is not the active one, Yeni the active layer.
    pub(crate) fn open_data_compare(&mut self) {
        self.open_data_compare_with(Preset::default());
    }

    /// Revizyonla karşılaştır (docs/adr/0210 §8): a revision's or a
    /// checkpoint's drawing, downloaded to `path`, as Eski veri; Yeni the open
    /// drawing; both sides the whole drawing. The file is read, then removed.
    pub(crate) fn compare_with_file(&mut self, name: &str, path: &std::path::Path) {
        let read = crate::document::Document::read(path);
        let _ = std::fs::remove_file(path);
        match read {
            Ok(read) => {
                let model = &read.model;
                let other = Other {
                    name: name.to_owned(),
                    tree: model.layers().nodes().to_vec(),
                    entities: model.entities().cloned().collect(),
                    system: system_of(model.settings()),
                };
                self.open_data_compare_with(Preset {
                    other: Some(other),
                    old_node: Some(String::new()),
                    new_node: Some(String::new()),
                    ..Preset::default()
                });
            }
            Err(why) => self.warn(why),
        }
    }

    /// The window as a purpose opens it (Zamanı karşılaştır, Senaryoyu
    /// karşılaştır, a revision; docs/adr/0210 §8): what the preset gives, the rest as Veri karşılaştır's.
    pub(crate) fn open_data_compare_with(&mut self, preset: Preset) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let layers = doc.model.layers();
        let active = layers.active().to_owned();
        let mut all = Vec::new();
        walk(layers.nodes(), &[], &mut all);
        let first = all
            .iter()
            .find(|(n, _)| n.kind == LayerNodeType::Layer && n.id != active)
            .map(|(n, _)| n.id.clone())
            .unwrap_or_default();
        let old_here = preset.other.is_none();
        self.data_compare = Some(Window {
            other: preset.other,
            old_here,
            old_node: preset.old_node.unwrap_or(first),
            new_node: preset.new_node.unwrap_or(active),
            pairing: if preset.key.is_some() {
                Pairing::Key
            } else {
                Pairing::Location
            },
            key: preset.key.unwrap_or_default(),
            search: "1".to_owned(),
            tolerance: "0.001".to_owned(),
            ignored: BTreeSet::new(),
            only_diffs: true,
            outcome: None,
            old_date: preset.old_date,
            new_date: preset.new_date,
        });
        self.dialog = Some(Dialog::DataCompare);
    }

    /// The two sides as the window sets them.
    fn compare_sides(&self) -> Option<(Side<'_>, Side<'_>)> {
        let (Some(w), Some(doc)) = (&self.data_compare, &self.document) else {
            return None;
        };
        let model = &doc.model;
        let here_tree = model.layers().nodes();
        let here_system = system_of(model.settings());
        let (old_tree, old_all, old_system, old_name): (
            &[LayerNode],
            Vec<&Entity>,
            String,
            String,
        ) = match (&w.other, w.old_here) {
            (Some(o), false) => (
                &o.tree,
                o.entities.iter().collect(),
                o.system.clone(),
                o.name.clone(),
            ),
            _ => (
                here_tree,
                model.entities().collect(),
                here_system.clone(),
                "Bu çizim".to_owned(),
            ),
        };
        let old_at = date_of(&w.old_date, OLD_DATE).ok().flatten();
        let new_at = date_of(&w.new_date, NEW_DATE).ok().flatten();
        let old = Side {
            tree: old_tree,
            entities: objects_under(old_tree, old_all, &w.old_node, old_at),
            system: old_system,
            here: w.old_here || w.other.is_none(),
            name: old_name,
            node: (!w.old_node.is_empty()).then_some(w.old_node.as_str()),
            at: old_at,
        };
        let new = Side {
            tree: here_tree,
            entities: objects_under(here_tree, model.entities().collect(), &w.new_node, new_at),
            system: here_system,
            here: true,
            name: "Bu çizim".to_owned(),
            node: (!w.new_node.is_empty()).then_some(w.new_node.as_str()),
            at: new_at,
        };
        Some((old, new))
    }

    /// Why the sides cannot be compared, or none.
    fn compare_refused(old: &Side<'_>, new: &Side<'_>) -> Option<String> {
        // The same layers at two different dates are two states of them (docs/adr/0210 §8).
        let dated = matches!((old.at, new.at), (Some(a), Some(b)) if a != b);
        if old.here && new.here && !dated {
            let a = layers_of(old.tree, old.node);
            let b = layers_of(new.tree, new.node);
            if a.iter().any(|id| b.contains(id)) {
                return Some(
                    "Eski ve Yeni aynı katmanları içeriyor; ayrı katmanlar ya da gruplar seçin ya da iki tarafa farklı tarihler yazın."
                        .to_owned(),
                );
            }
        }
        (old.system != new.system).then(|| {
            format!(
                "“{}” başka bir koordinat sisteminde; dönüştürme yapılmaz. Bu çizimle aynı koordinat sistemindeki bir çizim seçin.",
                old.name
            )
        })
    }

    /// Compares the sides as the window sets them; the outcome, or why not.
    fn run_compare(&self) -> Result<Outcome, String> {
        let Some(w) = &self.data_compare else {
            return Err("Açık çizim yok.".to_owned());
        };
        date_of(&w.old_date, OLD_DATE)?;
        date_of(&w.new_date, NEW_DATE)?;
        let Some((old, new)) = self.compare_sides() else {
            return Err("Açık çizim yok.".to_owned());
        };
        if let Some(why) = Self::compare_refused(&old, &new) {
            return Err(why);
        }
        if w.pairing == Pairing::Key && w.key.is_empty() {
            return Err(
                "Anahtar alanı seçin: eşleşen nesnelerde aynı değeri taşıyan öznitelik.".to_owned(),
            );
        }
        let number = |t: &str| t.trim().replace(',', ".").parse::<f64>().ok();
        let (Some(search), Some(tolerance)) = (number(&w.search), number(&w.tolerance)) else {
            return Err(
                "Arama uzaklığı ve tolerans sıfır ya da artı birer sayı olmalı (metre).".to_owned(),
            );
        };
        if !(search >= 0.0 && tolerance >= 0.0) {
            return Err(
                "Arama uzaklığı ve tolerans sıfır ya da artı birer sayı olmalı (metre).".to_owned(),
            );
        }
        let member = |e: &Entity| Member {
            shape: kentos_native_application::geometry::shape(e),
            attrs: Attrs(
                e.base()
                    .attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
        };
        let a: Vec<Member> = old.entities.iter().map(|e| member(e)).collect();
        let b: Vec<Member> = new.entities.iter().map(|e| member(e)).collect();
        let settings = Settings {
            matching: match w.pairing {
                Pairing::Location => Matching::Location,
                Pairing::Key => Matching::Key,
            },
            key: (w.pairing == Pairing::Key).then(|| w.key.clone()),
            search,
            tolerance,
            ignore: Some(w.ignored.iter().cloned().collect()),
        };
        let rows = compare::compare(&a, &b, &settings);
        Ok(Outcome {
            rows,
            old: old.entities.iter().map(|e| (*e).clone()).collect(),
            new: new.entities.iter().map(|e| (*e).clone()).collect(),
            old_tree: old.tree.to_vec(),
            new_tree: new.tree.to_vec(),
            old_here: old.here,
            key: settings.key,
        })
    }

    /// Farkları çizime yaz: a “Karşılaştırma” group (a name of its own) with a
    /// layer for each kind of difference there is — Eklenen (green), Silinen
    /// (red), Değişen (yellow) — holding the copies of the new side's added and
    /// changed objects and the old side's removed ones, with their own
    /// properties; one undo step “Veri karşılaştır”. An insert of a block this
    /// drawing lacks is left out and said. Whether it wrote.
    fn write_differences(&mut self) -> bool {
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return false;
        }
        let Some(o) = self.data_compare.as_ref().and_then(|w| w.outcome.clone()) else {
            return false;
        };
        let Some(doc) = self.document.as_ref() else {
            return false;
        };
        let model = &doc.model;
        let pick = |pred: &dyn Fn(Status) -> bool, old: bool| -> Vec<Entity> {
            o.rows
                .iter()
                .filter(|r| pred(r.status))
                .filter_map(|r| {
                    if old {
                        r.old.map(|i| o.old[i].clone())
                    } else {
                        r.new.map(|i| o.new[i].clone())
                    }
                })
                .collect()
        };
        let sets = [
            ("Eklenen", "#5FBF77", pick(&|s| s == Status::Added, false)),
            ("Silinen", "#E5484D", pick(&|s| s == Status::Removed, true)),
            (
                "Değişen",
                "#F2C94C",
                pick(
                    &|s| matches!(s, Status::Geometry | Status::Attributes | Status::Both),
                    false,
                ),
            ),
        ];
        let kept: Vec<(&str, &str, Vec<Entity>)> = sets
            .into_iter()
            .map(|(name, color, list)| {
                let fit: Vec<Entity> = list
                    .into_iter()
                    .filter(|e| match e {
                        Entity::Insert(i) => model.block(i.block).is_some(),
                        _ => true,
                    })
                    .collect();
                (name, color, fit)
            })
            .collect();
        let total: usize = kept.iter().map(|(_, _, l)| l.len()).sum();
        let all: usize = o
            .rows
            .iter()
            .filter(|r| r.status != Status::Same && r.status != Status::Key)
            .count();
        let skipped = all - total;
        if total == 0 {
            self.say(Level::Info, "Yazılacak fark yok: bütün nesneler aynı.");
            return false;
        }
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let model = &mut doc.model;
        let group_name = model.layers().unique_name("Karşılaştırma");
        let step = model.begin_group("Veri karşılaştır");
        let group = match model.add_layer(NewLayer::group(group_name.clone()), None, false) {
            Ok(id) => id,
            Err(refused) => {
                model.cancel_group(step);
                self.warn(refused.to_string());
                return false;
            }
        };
        let mut warnings = Vec::new();
        let mut said = Vec::new();
        for (name, color, list) in &kept {
            if list.is_empty() {
                continue;
            }
            let mut new = NewLayer::layer(*name);
            new.style.color = (*color).to_owned();
            let layer = match model.add_layer(new, Some(&group), false) {
                Ok(id) => id,
                Err(refused) => {
                    model.cancel_group(step);
                    self.warn(refused.to_string());
                    return false;
                }
            };
            let input = EntitiesCreate {
                layer_id: layer,
                objects: list.iter().filter_map(new_object).collect(),
                operation: None,
                expected_revision: None,
            };
            match create::execute(&mut ExecutionContext::new(model), input) {
                CommandResult::Completed { warnings: w, .. } => {
                    warnings.extend(w.into_iter().map(|w| w.message));
                }
                other => {
                    model.cancel_group(step);
                    let why = match other {
                        CommandResult::Failed { error }
                        | CommandResult::Conflict { error }
                        | CommandResult::NeedsInput { error } => error.message,
                        _ => "Farklar yazılamadı.".to_owned(),
                    };
                    self.warn(why);
                    return false;
                }
            }
            said.push(format!("{} {}", list.len(), name.to_lowercase()));
        }
        model.end_group(step);
        for text in warnings {
            self.warn(text);
        }
        self.say(
            Level::Success,
            format!(
                "Farklar çizime yazıldı: “{group_name}” grubunda {} nesne.",
                said.join(", ")
            ),
        );
        if skipped > 0 {
            self.warn(format!(
                "{skipped} blok yerleştirmesi yazılmadı: bloğu bu çizimde yok."
            ));
        }
        true
    }

    /// A row's object on the drawing: the new one, else the old one of this
    /// drawing, selected; another drawing's only shown.
    fn show_compare_row(&mut self, index: usize) -> Task<Message> {
        let Some(o) = self.data_compare.as_ref().and_then(|w| w.outcome.as_ref()) else {
            return Task::none();
        };
        let only = self.data_compare.as_ref().is_some_and(|w| w.only_diffs);
        let Some(row) = o
            .rows
            .iter()
            .filter(|r| !only || r.status != Status::Same)
            .nth(index)
            .cloned()
        else {
            return Task::none();
        };
        let mine = row
            .new
            .map(|i| o.new[i].base().id)
            .or_else(|| row.old.filter(|_| o.old_here).map(|i| o.old[i].base().id));
        if let Some(id) = mine {
            self.selection.set([Slot(id)]);
            return self.run("view.zoomSelection");
        }
        // Another drawing's object: the view on the box around its coordinates.
        if let Some(e) = row.old.map(|i| &o.old[i]) {
            let mut b = Bounds {
                min_x: f64::INFINITY,
                min_y: f64::INFINITY,
                max_x: f64::NEG_INFINITY,
                max_y: f64::NEG_INFINITY,
            };
            fn take(v: &serde_json::Value, b: &mut Bounds) {
                match v {
                    serde_json::Value::Array(list) => list.iter().for_each(|x| take(x, b)),
                    serde_json::Value::Object(map) => match (
                        map.get("x").and_then(|x| x.as_f64()),
                        map.get("y").and_then(|y| y.as_f64()),
                    ) {
                        (Some(x), Some(y)) => {
                            b.min_x = b.min_x.min(x);
                            b.min_y = b.min_y.min(y);
                            b.max_x = b.max_x.max(x);
                            b.max_y = b.max_y.max(y);
                        }
                        _ => map.values().for_each(|x| take(x, b)),
                    },
                    _ => {}
                }
            }
            if let Ok(json) = serde_json::to_value(e) {
                take(&json, &mut b);
            }
            if b.min_x <= b.max_x {
                self.viewport.change(ViewChange::Fit {
                    bounds: b,
                    padding: 96.0,
                });
            }
        }
        Task::none()
    }

    pub(crate) fn data_compare_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Source(here) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.old_here = here;
                    w.old_node.clear();
                }
            }
            Event::FromFile => {
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Karşılaştırılacak çizim")
                            .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                            .pick_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::FilePicked(path)),
                );
            }
            Event::FilePicked(None) => {}
            Event::FilePicked(Some(path)) => match crate::document::Document::read(&path) {
                Ok(read) => {
                    let name = path
                        .file_name()
                        .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
                    let model = &read.model;
                    let other = Other {
                        name: name.clone(),
                        tree: model.layers().nodes().to_vec(),
                        entities: model.entities().cloned().collect(),
                        system: system_of(model.settings()),
                    };
                    let count = other.entities.len();
                    if let Some(w) = self.data_compare.as_mut() {
                        w.other = Some(other);
                        w.old_here = false;
                        w.old_node.clear();
                    }
                    self.say(
                        Level::Info,
                        format!("“{name}” okundu: {count} nesne; Eski veri olarak seçildi."),
                    );
                }
                Err(why) => self.warn(why),
            },
            Event::OldNode(id) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.old_node = id;
                }
            }
            Event::NewNode(id) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.new_node = id;
                }
            }
            Event::Pairing(p) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.pairing = p;
                }
            }
            Event::Key(k) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.key = k;
                }
            }
            Event::Search(t) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.search = t;
                }
            }
            Event::Tolerance(t) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.tolerance = t;
                }
            }
            Event::OldDate(t) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.old_date = t;
                }
            }
            Event::NewDate(t) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.new_date = t;
                }
            }
            Event::Field(name, on) => {
                if let Some(w) = self.data_compare.as_mut() {
                    if on {
                        w.ignored.remove(&name);
                    } else {
                        w.ignored.insert(name);
                    }
                }
            }
            Event::OnlyDiffs(on) => {
                if let Some(w) = self.data_compare.as_mut() {
                    w.only_diffs = on;
                }
            }
            Event::Compare => match self.run_compare() {
                Ok(outcome) => {
                    let said = counts_text(&outcome.rows);
                    if let Some(w) = self.data_compare.as_mut() {
                        w.outcome = Some(outcome);
                    }
                    self.say(Level::Success, format!("Karşılaştırıldı: {said}."));
                }
                Err(why) => self.warn(why),
            },
            Event::Show(i) => return self.show_compare_row(i),
            Event::Copy => {
                if let Some(w) = &self.data_compare
                    && let Some(o) = &w.outcome
                {
                    let lines = report_rows(o, w.only_diffs);
                    let n = lines.len() - 1;
                    let text = crate::layer_list::tsv(&lines);
                    self.say(
                        Level::Success,
                        format!("Karşılaştırma raporu panoya kopyalandı ({n} satır; elektronik tabloya yapıştırılabilir)."),
                    );
                    return iced::clipboard::write(text);
                }
            }
            Event::Save => {
                let name = self
                    .document
                    .as_ref()
                    .map_or_else(|| "cizim".to_owned(), |d| d.model.name().to_owned());
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Karşılaştırma raporunu kaydet")
                            .add_filter("CSV (.csv)", &["csv"])
                            .set_file_name(format!("{name}-karsilastirma.csv"))
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::Saved(path)),
                );
            }
            Event::Saved(None) => {}
            Event::Saved(Some(path)) => {
                if let Some(w) = &self.data_compare
                    && let Some(o) = &w.outcome
                {
                    let lines = report_rows(o, w.only_diffs);
                    match std::fs::write(&path, crate::layer_list::csv(&lines)) {
                        Ok(()) => {
                            let file = path
                                .file_name()
                                .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
                            self.say(
                                Level::Success,
                                format!(
                                    "Karşılaştırma raporu CSV olarak kaydedildi: {file} ({} satır).",
                                    lines.len() - 1
                                ),
                            );
                        }
                        Err(e) => self.warn(format!(
                            "Rapor kaydedilemedi ({e}); başka bir klasör seçip yeniden deneyin."
                        )),
                    }
                }
            }
            Event::Write => {
                self.write_differences();
            }
            Event::Close => {
                self.data_compare = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn data_compare_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.data_compare, &self.document) else {
            return text("").into();
        };
        let model = &doc.model;
        let here_tree = model.layers().nodes();
        let old_tree: &[LayerNode] = match (&w.other, w.old_here) {
            (Some(o), false) => &o.tree,
            _ => here_tree,
        };
        // Eski veri: this drawing or the file read.
        let mut sources = vec![Choice::new("Bu çizim")];
        if let Some(o) = &w.other {
            sources.push(Choice::new(o.name.clone()));
        }
        let source = Select::new(sources, Some(if w.old_here { 0 } else { 1 }), |i| {
            msg(Event::Source(i == 0))
        });
        let pick = |tree: &[LayerNode], chosen: &str, on: fn(String) -> Event| {
            let choices = node_choices(tree);
            let at = choices.iter().position(|(id, _)| id == chosen);
            let ids: Vec<String> = choices.iter().map(|(id, _)| id.clone()).collect();
            Select::new(
                choices
                    .into_iter()
                    .map(|(_, p)| Choice::new(p))
                    .collect::<Vec<_>>(),
                at,
                move |i| msg(on(ids.get(i).cloned().unwrap_or_default())),
            )
        };
        let first = row![
            container(words::field(
                "Eski veri",
                row![
                    source,
                    words::secondary(FROM_FILE, Some(msg(Event::FromFile)))
                ]
                .spacing(8)
                .align_y(Center),
                None
            ))
            .width(iced::Length::FillPortion(2)),
            container(words::field(
                "Eski katman",
                pick(old_tree, &w.old_node, Event::OldNode),
                None
            ))
            .width(iced::Length::FillPortion(2)),
            container(words::field(
                "Yeni katman",
                pick(here_tree, &w.new_node, Event::NewNode),
                None
            ))
            .width(iced::Length::FillPortion(2)),
        ]
        .spacing(18);
        let fields = self
            .compare_sides()
            .map(|(o, n)| fields_of(&[&o, &n]))
            .unwrap_or_default();
        let key_choices: Vec<Choice> = std::iter::once(Choice::new("Seçin"))
            .chain(fields.iter().map(|f| Choice::new(f.clone())))
            .collect();
        let key_at = if w.key.is_empty() {
            Some(0)
        } else {
            fields.iter().position(|f| *f == w.key).map(|i| i + 1)
        };
        let key_fields = fields.clone();
        let key_choices: Vec<Choice> = if w.pairing == Pairing::Key {
            key_choices
        } else {
            // Paired by location there is no key: the list is off.
            vec![Choice::new("Konumla eşlemede yok").disabled()]
        };
        let key_at = if w.pairing == Pairing::Key {
            key_at
        } else {
            Some(0)
        };
        let key = Select::new(key_choices, key_at, move |i| {
            msg(Event::Key(if i == 0 {
                String::new()
            } else {
                key_fields.get(i - 1).cloned().unwrap_or_default()
            }))
        });
        let number = |value: &str, on: fn(String) -> Event| {
            focus_ring(
                text_input("", value)
                    .on_input(move |t| msg(on(t)))
                    .padding([5, 8])
                    .width(110)
                    .style(style::field::input),
            )
        };
        let second = row![
            words::field(
                "Eşleme",
                Segmented::new([Pairing::Location, Pairing::Key], w.pairing, |p| msg(
                    Event::Pairing(p)
                )),
                None
            ),
            words::field("Anahtar alan", key, None),
            words::field(SEARCH, number(&w.search, Event::Search), None),
            words::field(TOLERANCE, number(&w.tolerance, Event::Tolerance), None),
            column_bottom(words::primary(COMPARE, Some(msg(Event::Compare)))),
        ]
        .spacing(18)
        .align_y(iced::Alignment::End);
        let mut checks = row![].spacing(16);
        if fields.is_empty() {
            checks = checks.push(label::caption("İki tarafın nesnelerinde öznitelik yok."));
        }
        for f in &fields {
            let on = !w.ignored.contains(f);
            checks = checks.push(words::check(
                on,
                f.clone(),
                Some(msg(Event::Field(f.clone(), !on))),
            ));
        }
        let third = row![
            container(words::field(
                "Karşılaştırılan öznitelikler",
                checks.wrap(),
                None
            ))
            .width(Fill),
            words::field(
                "Liste",
                words::check(
                    w.only_diffs,
                    ONLY,
                    Some(msg(Event::OnlyDiffs(!w.only_diffs)))
                ),
                None
            ),
        ]
        .spacing(18);
        let summary = match &w.outcome {
            Some(o) => words::summary(vec![words::text_line(
                if o.rows.iter().any(|r| r.status != Status::Same) {
                    Kind::Info
                } else {
                    Kind::Ok
                },
                format!("{}.", counts_text(&o.rows)),
            )]),
            None => words::summary(vec![words::text_line(
                Kind::Info,
                "Eski ve Yeni veriyi seçip Karşılaştır’a basın. Konumla eşlemede arama uzaklığı içindeki aynı türden nesneler eşlenir; tolerans içinde kalan geometri aynı sayılır.",
            )]),
        };
        let format = self.format();
        let shown: Vec<Vec<String>> = w.outcome.as_ref().map_or_else(Vec::new, |o| {
            o.rows
                .iter()
                .filter(|r| !w.only_diffs || r.status != Status::Same)
                .map(|r| row_words(o, r, &|m| format.length(m)))
                .collect()
        });
        let statuses: Vec<Status> = w.outcome.as_ref().map_or_else(Vec::new, |o| {
            o.rows
                .iter()
                .filter(|r| !w.only_diffs || r.status != Status::Same)
                .map(|r| r.status)
                .collect()
        });
        let widths: [f32; 7] = [200.0, 90.0, 70.0, 110.0, 110.0, 80.0, 120.0];
        let heads = [
            "Durum",
            "Tür",
            "Anahtar",
            "Eski katman",
            "Yeni katman",
            "Konum farkı",
            "Değişen alanlar",
        ];
        let columns = heads
            .iter()
            .zip(widths)
            .map(|(h, wd)| table::Column::new(*h).width(wd));
        let body = Table::new(columns)
            .virtualized(shown.len(), move |i| {
                let words = &shown[i];
                let status = statuses[i];
                let cells = words.iter().enumerate().map(move |(c, t)| {
                    let cell = label::caption(t.clone());
                    if c == 0 {
                        cell.style(move |theme: &iced::Theme| {
                            let tk = Tokens::of(theme);
                            iced::widget::text::Style {
                                color: Some(match status {
                                    Status::Added => tk.success,
                                    Status::Removed => tk.danger,
                                    Status::Geometry | Status::Attributes | Status::Both => {
                                        tk.warning
                                    }
                                    _ => tk.muted,
                                }),
                            }
                        })
                        .into()
                    } else {
                        cell.into()
                    }
                });
                table::Row::new(cells).on_press(msg(Event::Show(i)))
            })
            .horizontal()
            .height(240.0);
        let list = container(body)
            .style(style::container::field_box)
            .width(Fill);
        let any = w.outcome.is_some();
        // Each side's date (docs/adr/0210 §8): empty, every object; a date, the temporal layers' objects shown then.
        let date = |value: &str, on: fn(String) -> Event| {
            focus_ring(
                text_input("GG.AA.YYYY", value)
                    .on_input(move |t| msg(on(t)))
                    .padding([5, 8])
                    .width(150)
                    .style(style::field::input),
            )
        };
        let dates = row![
            words::field(OLD_DATE, date(&w.old_date, Event::OldDate), None),
            words::field(NEW_DATE, date(&w.new_date, Event::NewDate), None),
            column_bottom(
                label::caption("Tarih yazılan tarafta zamansal katmanların o anda görünen nesneleri karşılaştırılır.")
                    .style(kentos_ui::style::text::muted)
                    .into()
            ),
        ]
        .spacing(18)
        .align_y(iced::Alignment::End);
        let content = Column::new()
            .spacing(12)
            .push(first)
            .push(dates)
            .push(second)
            .push(third)
            .push(summary)
            .push(list);
        overlay::modal(
            Frame::new(COMPARE_TITLE)
                .push(content)
                .action(words::secondary(COPY, any.then(|| msg(Event::Copy))))
                .action(words::secondary(SAVE, any.then(|| msg(Event::Save))))
                .action(words::secondary(WRITE, any.then(|| msg(Event::Write))))
                .action(words::primary(CLOSE, Some(msg(Event::Close))))
                .width(920.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn data_compare_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.data_compare else {
            return Err(format!("{COMPARE_TITLE} penceresi açık değil"));
        };
        let any = w.outcome.is_some();
        Ok(match control {
            Control::Fill(SEARCH, t) => Some(msg(Event::Search(t.to_owned()))),
            Control::Fill(TOLERANCE, t) => Some(msg(Event::Tolerance(t.to_owned()))),
            Control::Fill(OLD_DATE, t) => Some(msg(Event::OldDate(t.to_owned()))),
            Control::Fill(NEW_DATE, t) => Some(msg(Event::NewDate(t.to_owned()))),
            Control::Check(ONLY, on) => (w.only_diffs != on).then(|| msg(Event::OnlyDiffs(on))),
            Control::Press(COMPARE) => Some(msg(Event::Compare)),
            Control::Press(COPY) => any.then(|| msg(Event::Copy)),
            Control::Press(WRITE) => any.then(|| msg(Event::Write)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(words) if words == Pairing::Location.to_string() => {
                Some(msg(Event::Pairing(Pairing::Location)))
            }
            Control::Press(words) if words == Pairing::Key.to_string() => {
                Some(msg(Event::Pairing(Pairing::Key)))
            }
            Control::Check(field, on) => {
                let now = !w.ignored.contains(field);
                (now != on).then(|| msg(Event::Field(field.to_owned(), on)))
            }
            other => return Err(format!("“{COMPARE_TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A button set at the bottom of its row, beside fields with labels above them.
fn column_bottom<'a>(e: Element<'a, Message>) -> Element<'a, Message> {
    Column::new().push(e).into()
}
