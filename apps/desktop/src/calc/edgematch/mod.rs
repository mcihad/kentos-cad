//! Kenar eşleme (the web's `ui/calc/EdgematchDialog.ts`, docs/adr/0159 §9):
//! the line ends of two sheets put together across their common edge.
//! Kaynak is the selection or a layer, Komşu a layer, Sınır an object shown
//! on the drawing. At every change the core (`ops::edgematch`) finds the
//! links, the best continuation within the search distance and the angle
//! tolerance; the table lists them with Kullan and Göster, and the core
//! works out what Uygula would write, so the summary can say which links
//! cannot be written. Uygula writes through `cad.entities.edit` (one undo
//! step, Kenar eşle) and selects what it put right. What is typed stays
//! while the app runs.
//!
//! The window's parts: the form and its links here, its words ([`words`]:
//! the choices, the summary, the report), its view ([`view`]) and what it
//! does to the app ([`apply`]: Sınır's pick, Göster, Uygula).

mod apply;
#[cfg(test)]
mod tests;
mod view;
mod words;

use std::collections::BTreeSet;

use kentos_contracts::{Entity, EntityEdit, EntityGeometry};
use kentos_domain::{Document as Model, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::edgematch::{
    self as core, Elevations, Found, Link, Meet, Member, Method, Settings,
};
use kentos_interaction::fixed;
use kentos_native_application::elevation::paths as elevated_paths;
use kentos_native_application::geometry::{edit_geometry, shape};

use super::grid::{Col, Mark, Table};
use super::read::read_number;
use super::{Event as Calc, event};
use crate::app::Message;

pub use words::TITLE;

/// The table's columns (the web's keys use, src, adj, gap, angle); a row
/// keeps its link's ends by persistent id after them.
pub const USE: usize = 0;
pub const SOURCE: usize = 1;
pub const ADJACENT: usize = 2;
pub const GAP: usize = 3;
pub const ANGLE: usize = 4;
const KEY: usize = 5;

const COLUMNS: [Col; 5] = [
    col("Kullan", None, false),
    col("Kaynak", None, false),
    col("Komşu", None, false),
    col("Aralık", Some("mm"), true),
    col("Açı farkı", Some("°"), true),
];

const fn col(label: &'static str, unit: Option<&'static str>, numeric: bool) -> Col {
    Col {
        label,
        unit,
        numeric,
    }
}

/// The criterion that compares the layers' names (an attribute's name cannot hold this character).
pub const LAYER_KEY: &str = "\u{0}katman";

/// Which objects Kaynak takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Selection,
    Layer,
}

/// What the window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    Scope(Scope),
    Source(String),
    Adjacent(String),
    Distance(String),
    Angle(String),
    /// The criterion: empty for none, [`LAYER_KEY`], or an attribute's name.
    Key(String),
    Meet(Meet),
    Method(Method),
    /// Sınır shown on the drawing, or taken away.
    PickBorder,
    ClearBorder,
    /// A link looked at on the drawing.
    Show(usize),
    Apply,
}

fn edge_event(e: Event) -> Message {
    event(Calc::Edgematch(e))
}

/// What the window stepped aside for, with the selection to put back.
#[derive(Clone, Debug)]
pub(crate) enum Aside {
    /// Sınır's pick (`PickObjects::one`).
    Border(Vec<Slot>),
    /// Göster's look (`Look`).
    Look(Vec<Slot>),
}

/// What is typed, kept while the app runs, and the links found from it.
#[derive(Clone, Debug)]
pub struct Form {
    pub scope: Scope,
    pub source: Option<String>,
    pub adjacent: Option<String>,
    /// The border object's persistent id.
    pub border: Option<String>,
    pub distance: String,
    pub angle: String,
    pub key: String,
    pub meet: Meet,
    pub method: Method,
    /// Links left out, by their ends (`link_key`).
    off: BTreeSet<String>,
    /// Why Uygula wrote nothing (the web's status), until the window opens again.
    pub status: Option<String>,
    pub(crate) aside: Option<Aside>,
    /// The rows: a link each, its ends' key last.
    rows: Vec<[String; 6]>,
    /// The sets as the core took them (their slots), and what it found.
    source_slots: Vec<Slot>,
    adjacent_slots: Vec<Slot>,
    members: (Vec<Member>, Vec<Member>),
    found: Option<Found>,
    others: usize,
    locked: usize,
    problem: Option<String>,
    /// What Uygula writes now, how many links, and the rows (1-based) of the links that cannot be written.
    changes: Vec<EntityEdit>,
    written: usize,
    refused: Vec<usize>,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            scope: Scope::Layer,
            source: None,
            adjacent: None,
            border: None,
            distance: "0.5".to_owned(),
            angle: "30".to_owned(),
            key: String::new(),
            meet: Meet::Adjacent,
            method: Method::Move,
            off: BTreeSet::new(),
            status: None,
            aside: None,
            rows: Vec::new(),
            source_slots: Vec::new(),
            adjacent_slots: Vec::new(),
            members: (Vec::new(), Vec::new()),
            found: None,
            others: 0,
            locked: 0,
            problem: None,
            changes: Vec::new(),
            written: 0,
            refused: Vec::new(),
        }
    }
}

impl Table for Form {
    fn columns(&self) -> usize {
        COLUMNS.len()
    }

    fn rows(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, row: usize, col: usize) -> &str {
        self.rows.get(row).map_or("", |r| r[col].as_str())
    }

    /// Only Kullan is written: a link left out is remembered by its ends.
    fn set(&mut self, row: usize, col: usize, text: String) {
        if col != USE {
            return;
        }
        if let Some(r) = self.rows.get_mut(row) {
            if text == "0" {
                self.off.insert(r[KEY].clone());
            } else {
                self.off.remove(&r[KEY]);
            }
            r[USE] = text;
        }
    }

    fn readonly(&self, _row: usize, col: usize) -> bool {
        col != USE
    }

    fn check(&self, col: usize) -> bool {
        col == USE
    }

    /// A link left out reads faded; the used link with the largest gap in the warning colour.
    fn mark(&self, row: usize) -> Option<Mark> {
        if self.rows.get(row)?[USE] == "0" {
            return Some(Mark::Off);
        }
        (self.worst()? == row).then_some(Mark::Worst)
    }

    fn can_insert_after(&self, _row: usize) -> bool {
        false
    }

    fn insert_after(&mut self, _row: usize) {}

    fn can_remove(&self, _row: usize) -> bool {
        false
    }

    fn remove(&mut self, _row: usize) {}
}

/// The kinds a sheet's line work has: lines and polylines take part, areas only as junctions.
fn takes(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    )
}

impl Form {
    /// What the drawing offers when the window opens (the web's
    /// `defaults`): Kaynak and Komşu kept while the drawing has them, else
    /// the first two layers holding line work; Seçili with nothing selected
    /// becomes Katman; a border the drawing lost is forgotten.
    pub fn sync(&mut self, model: &Model, selected: usize) {
        let choices = line_layers(model);
        let has = |id: &Option<String>| {
            id.as_ref()
                .is_some_and(|id| choices.iter().any(|(l, _)| l == id))
        };
        if !has(&self.source) {
            self.source = choices.first().map(|c| c.0.clone());
        }
        if !has(&self.adjacent) || self.adjacent == self.source {
            self.adjacent = choices
                .iter()
                .find(|c| Some(&c.0) != self.source.as_ref())
                .map(|c| c.0.clone());
        }
        if self.scope == Scope::Selection && selected == 0 {
            self.scope = Scope::Layer;
        }
        if self
            .border
            .as_deref()
            .is_some_and(|uid| border_slot(model, uid).is_none())
        {
            self.border = None;
        }
        if self.meet == Meet::Border && self.border.is_none() {
            self.meet = Meet::Adjacent;
        }
        self.status = None;
    }

    /// The sets, the links and the table again, then what Uygula would write.
    pub fn solve(&mut self, model: &Model, selection: &[Slot]) {
        self.find(model, selection);
        self.preview(model);
    }

    /// Why nothing can be looked for: the layers, the numbers (the web's `check`).
    fn check(&self) -> Option<String> {
        let text = |s: &str| Some(s.to_owned());
        if self.scope == Scope::Layer && self.source.is_none() {
            return text("Kaynak katmanı seçin: düzeltilecek çizgilerin katmanı.");
        }
        if self.adjacent.is_none() {
            return text("Komşu katmanı seçin: komşu paftanın çizgilerinin katmanı.");
        }
        if self.scope == Scope::Layer && self.source == self.adjacent {
            return text("Kaynak ve komşu aynı katman. Komşu paftanın katmanını seçin.");
        }
        if !read_number(&self.distance).is_some_and(|d| d.is_finite() && d > 0.0) {
            return text("Arama uzaklığı sıfırdan büyük bir uzunluk olmalı (metre).");
        }
        if !read_number(&self.angle).is_some_and(|a| a.is_finite() && a > 0.0 && a <= 180.0) {
            return text("Açı toleransı 0 ile 180 derece arasında olmalı.");
        }
        None
    }

    fn find(&mut self, model: &Model, selection: &[Slot]) {
        self.problem = self.check();
        self.found = None;
        self.rows.clear();
        self.source_slots.clear();
        self.adjacent_slots.clear();
        self.members = (Vec::new(), Vec::new());
        self.others = 0;
        self.locked = 0;
        if self.problem.is_some() {
            return;
        }
        let layers = model.layers();
        let visible = |e: &Entity| layers.is_visible(&e.base().layer_id);
        let locked = |e: &Entity| layers.is_locked(&e.base().layer_id);
        let slot = |e: &Entity| Slot(e.base().id);
        let picked: Vec<Slot> = match (self.scope, self.source.as_deref()) {
            (Scope::Selection, _) => selection.to_vec(),
            (Scope::Layer, Some(layer)) => model.by_layer(layer).map(slot).collect(),
            (Scope::Layer, None) => Vec::new(),
        };
        let sources: Vec<(Slot, &Entity)> = picked
            .iter()
            .filter_map(|&s| model.get(s).map(|e| (s, e)))
            .filter(|(_, e)| visible(e))
            .collect();
        let own: BTreeSet<Slot> = sources.iter().map(|(s, _)| *s).collect();
        let neighbours: Vec<(Slot, &Entity)> = self
            .adjacent_layer()
            .map(|layer| {
                model
                    .by_layer(layer)
                    .map(|e| (slot(e), e))
                    .filter(|(s, e)| !own.contains(s) && visible(e))
                    .collect()
            })
            .unwrap_or_default();
        // A locked object is not put right: a source never, a neighbour when the ends meet halfway or on the border.
        let moves = self.meet != Meet::Adjacent;
        self.locked = sources.iter().filter(|(_, e)| locked(e)).count()
            + if moves {
                neighbours.iter().filter(|(_, e)| locked(e)).count()
            } else {
                0
            };
        let skipped = sources
            .iter()
            .filter(|(_, e)| !takes(e) && !locked(e))
            .count();
        let key = self.key.clone();
        let key_of = |e: &Entity| -> Option<String> {
            match key.as_str() {
                "" => None,
                LAYER_KEY => layers.get(&e.base().layer_id).map(|n| n.name.clone()),
                attr => e.base().attrs.get(attr).cloned(),
            }
        };
        let member = |e: &Entity| Member {
            shape: shape(e),
            zs: elevated_paths(e).into_iter().map(|p| p.zs).collect(),
            key: key_of(e),
        };
        let (mut src_members, mut adj_members) = (Vec::new(), Vec::new());
        for (s, e) in &sources {
            if takes(e) && !locked(e) {
                self.source_slots.push(*s);
                src_members.push(member(e));
            }
        }
        for (s, e) in &neighbours {
            if takes(e) && !(moves && locked(e)) {
                self.adjacent_slots.push(*s);
                adj_members.push(member(e));
            }
        }
        let settings = Settings {
            distance: read_number(&self.distance).unwrap_or(f64::NAN),
            angle: read_number(&self.angle).unwrap_or(f64::NAN),
            border: self.border_shape(model),
            keyed: !self.key.is_empty(),
        };
        let found = core::links(&src_members, &adj_members, &settings);
        self.others = found.others + skipped;
        let name = |s: Slot| {
            model
                .get(s)
                .map(|e| words::describe(e, &layers.path(&e.base().layer_id)))
                .unwrap_or_default()
        };
        for l in &found.links {
            let key = self.link_key(model, l);
            self.rows.push([
                if self.off.contains(&key) { "0" } else { "1" }.to_owned(),
                name(self.source_slots[l.source]),
                name(self.adjacent_slots[l.adjacent]),
                fixed(l.gap * 1000.0, 1),
                fixed(l.angle, 1),
                key,
            ]);
        }
        self.members = (src_members, adj_members);
        self.found = Some(found);
    }

    fn adjacent_layer(&self) -> Option<&str> {
        self.adjacent.as_deref()
    }

    /// The border object's shape, while the drawing has it.
    fn border_shape(&self, model: &Model) -> Option<Shape> {
        let slot = border_slot(model, self.border.as_deref()?)?;
        model.get(slot).map(shape)
    }

    /// A link's ends by persistent id: what Kullan remembers while the links are found again.
    fn link_key(&self, model: &Model, l: &Link) -> String {
        let uid = |s: Slot| {
            model
                .uid(s)
                .map_or_else(|| s.0.to_string(), |u| u.to_string())
        };
        format!(
            "{}:{}>{}:{}",
            uid(self.source_slots[l.source]),
            end_word(l.source_end),
            uid(self.adjacent_slots[l.adjacent]),
            end_word(l.adjacent_end)
        )
    }

    /// The used link with the largest gap (its row).
    fn worst(&self) -> Option<usize> {
        let found = self.found.as_ref()?;
        let mut best: Option<usize> = None;
        for (i, row) in self.rows.iter().enumerate() {
            if row[USE] == "0" {
                continue;
            }
            if best.is_none_or(|b| found.links[i].gap > found.links[b].gap) {
                best = Some(i);
            }
        }
        best
    }

    /// What Uygula would write: the used links put together by the core.
    fn preview(&mut self, model: &Model) {
        self.changes.clear();
        self.written = 0;
        self.refused.clear();
        let Some(found) = &self.found else {
            return;
        };
        let used: Vec<usize> = (0..self.rows.len())
            .filter(|&i| self.rows[i][USE] != "0")
            .collect();
        if self.problem.is_some() || used.is_empty() {
            return;
        }
        let links: Vec<Link> = used.iter().map(|&i| found.links[i].clone()).collect();
        let border = self.border_shape(model);
        let applied = match core::apply(
            &self.members.0,
            &self.members.1,
            &links,
            self.meet,
            self.method,
            border.as_ref(),
        ) {
            Ok(a) => a,
            Err(_) => {
                self.problem = Some("Sınırda buluşma için bir sınır seçin.".to_owned());
                return;
            }
        };
        self.refused = applied.refused.iter().map(|&k| used[k] + 1).collect();
        self.written = links.len() - applied.refused.len();
        for (slots, results) in [
            (&self.source_slots, &applied.sources),
            (&self.adjacent_slots, &applied.adjacent),
        ] {
            for (&slot, result) in slots.iter().zip(results) {
                let (Some((shape, zs)), Some(uid)) = (result, model.uid(slot)) else {
                    continue;
                };
                if let Some(geometry) = geometry_of(shape.clone(), zs) {
                    self.changes.push(EntityEdit::Update {
                        uid: uid.to_string(),
                        geometry,
                    });
                }
            }
        }
    }
}

/// A line or a polyline put right as `cad.entities.edit` takes it, its path's elevations written.
fn geometry_of(shape: Shape, zs: &Elevations) -> Option<EntityGeometry> {
    let mut geometry = edit_geometry(shape)?;
    let written = zs.first().cloned();
    match &mut geometry {
        EntityGeometry::Line { zs, .. } | EntityGeometry::Polyline { zs, .. } => *zs = written,
        _ => {}
    }
    Some(geometry)
}

fn end_word(end: core::End) -> &'static str {
    match end {
        core::End::First => "first",
        core::End::Last => "last",
    }
}

/// The border's slot by its persistent id, while the drawing has it.
fn border_slot(model: &Model, uid: &str) -> Option<Slot> {
    model.slot_of(uid.parse().ok()?)
}

/// The layers holding line work, in the tree's order: id and the select's
/// words, the path and how many lines and polylines (the web's `lineLayers`).
fn line_layers(model: &Model) -> Vec<(String, String)> {
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in model.entities() {
        if matches!(e, Entity::Line(_) | Entity::Polyline(_)) {
            *counts.entry(e.base().layer_id.as_str()).or_default() += 1;
        }
    }
    let layers = model.layers();
    layers
        .leaves()
        .into_iter()
        .filter_map(|n| {
            let count = *counts.get(n.id.as_str())?;
            Some((n.id.clone(), format!("{} ({count})", layers.path(&n.id))))
        })
        .collect()
}
