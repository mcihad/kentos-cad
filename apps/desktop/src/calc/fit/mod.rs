//! Vektör oturtma (the web's `ui/calc/FitDialog.ts`, docs/adr/0156 §7): a
//! drawing or a layer fitted to another system by control points. Each row
//! is a pair: where the point is in the drawing (source) and where it is to
//! go (target), typed, pasted from a spreadsheet or shown on the drawing;
//! Kullan leaves a pair out of the solution (its residual is still worked
//! out). The solution is the geometry core's (`ops::fit`): Helmert, affine
//! or projective by least squares, every pair's residual and m0, worked out
//! again at every change. Adla eşle fills the table with the points of the
//! same name on two layers. Uygula writes the transform through
//! `cad.entities.transform` (one undo step, Oturt) to the selected objects,
//! a layer or the whole drawing, or their copies. What is typed stays while
//! the app runs.
//!
//! The window's parts: the form and its solution here, the words it says
//! ([`words`]: the summary, the parameters, the report), its view
//! ([`view`]) and what it does to the app ([`apply`]: Adla eşle, Uygula,
//! the picks).

mod apply;
#[cfg(test)]
mod tests;
mod view;
mod words;

use std::collections::HashMap;

use kentos_contracts::{Entity, Transform};
use kentos_domain::{Document as Model, Slot};
use kentos_geometry_core::ops::fit::{self as solver, FitError, FitKind, FitPair};
use kentos_interaction::{Vec2, fixed, js_trim};
use kentos_processing::text::js_number;

use super::grid::{Col, Mark, Table};
use super::read::read_number;
use super::{Event as Calc, event};
use crate::app::Message;

pub const TITLE: &str = "Vektör oturtma";

/// The table's columns (the web's keys use, name, sy, sx, ty, tx, vy, vx, v).
pub const USE: usize = 0;
pub const NAME: usize = 1;
pub const SOURCE_Y: usize = 2;
pub const SOURCE_X: usize = 3;
pub const TARGET_Y: usize = 4;
pub const TARGET_X: usize = 5;
/// The residuals, worked out (millimetres, one decimal).
pub const RES_Y: usize = 6;
pub const RES_X: usize = 7;
pub const RES: usize = 8;

const COLUMNS: [Col; 9] = [
    col("Kullan", None, false),
    col("Ad", None, false),
    col("Kaynak Y", None, true),
    col("Kaynak X", None, true),
    col("Hedef Y", None, true),
    col("Hedef X", None, true),
    col("vY", Some("mm"), true),
    col("vX", Some("mm"), true),
    col("v", Some("mm"), true),
];

const fn col(label: &'static str, unit: Option<&'static str>, numeric: bool) -> Col {
    Col {
        label,
        unit,
        numeric,
    }
}

/// The objects Uygula takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Selection,
    Layer,
    All,
}

/// Which end of a pair a pick is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Source,
    Target,
}

/// What the window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    Kind(FitKind),
    /// Adla eşle's layers (empty: none) and its button.
    Source(String),
    Target(String),
    Match,
    /// The objects Uygula takes, the layer when a layer's, and whether copies.
    Scope(Scope),
    Layer(String),
    Copy(bool),
    Apply,
    /// A row's source or target shown on the drawing.
    Pick(usize, Side),
}

fn fit_event(e: Event) -> Message {
    event(Calc::Fit(e))
}

/// What is typed, kept while the app runs; four empty rows at first.
#[derive(Clone, Debug)]
pub struct Form {
    pub kind: FitKind,
    pub rows: Vec<[String; 9]>,
    /// Adla eşle's layers: the source points' and the target points'.
    pub source: Option<String>,
    pub target: Option<String>,
    pub scope: Scope,
    pub layer: Option<String>,
    pub copy: bool,
    /// Why Uygula wrote nothing (the web's status), until the window opens again.
    pub status: Option<String>,
    /// The row and end a pick is for, while the window is closed.
    pub(super) picking: Option<(usize, Side)>,
    /// The layers holding named points, in the tree's order: id, path, how
    /// many (read when the window opens; the drawing cannot change under it).
    named: Vec<(String, String, usize)>,
    /// The solution now, the rows its pairs came from, the pairs used.
    solution: Option<Result<solver::Fit, FitError>>,
    pair_rows: Vec<usize>,
    used: usize,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            kind: FitKind::Helmert,
            rows: vec![Default::default(); 4],
            source: None,
            target: None,
            scope: Scope::Selection,
            layer: None,
            copy: false,
            status: None,
            picking: None,
            named: Vec::new(),
            solution: None,
            pair_rows: Vec::new(),
            used: 0,
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

    fn set(&mut self, row: usize, col: usize, text: String) {
        if let Some(r) = self.rows.get_mut(row) {
            r[col] = text;
        }
    }

    fn readonly(&self, _row: usize, col: usize) -> bool {
        col >= RES_Y
    }

    fn check(&self, col: usize) -> bool {
        col == USE
    }

    fn mark(&self, row: usize) -> Option<Mark> {
        if self.rows.get(row).is_some_and(|r| r[USE] == "0") {
            return Some(Mark::Off);
        }
        self.fit()?.m0?;
        let worst = self.worst()?;
        (self.pair_rows.get(worst) == Some(&row)).then_some(Mark::Worst)
    }

    fn insert_after(&mut self, row: usize) {
        let at = (row + 1).min(self.rows.len());
        self.rows.insert(at, Default::default());
    }

    fn can_remove(&self, _row: usize) -> bool {
        self.rows.len() > 1
    }

    fn remove(&mut self, row: usize) {
        if self.rows.len() > 1 && row < self.rows.len() {
            self.rows.remove(row);
        }
    }
}

/// A row's pair, when its four coordinates are numbers.
fn pair_of(row: &[String; 9]) -> Option<FitPair> {
    let n = |c: usize| read_number(&row[c]).filter(|v| !v.is_nan());
    Some(FitPair {
        source: Vec2::new(n(SOURCE_Y)?, n(SOURCE_X)?),
        target: Vec2::new(n(TARGET_Y)?, n(TARGET_X)?),
        used: row[USE] != "0",
    })
}

impl Form {
    /// The solution, when there is one.
    pub fn fit(&self) -> Option<&solver::Fit> {
        self.solution.as_ref()?.as_ref().ok()
    }

    /// What the drawing offers, read when the window opens (the web's
    /// `renderControls`): the layers holding named points, the chosen ones
    /// kept while they still hold some; the layer Uygula takes; Seçili with
    /// nothing selected becomes Tümü.
    pub fn sync(&mut self, model: &Model, selected: usize) {
        self.named = named_layers(model);
        let held = |id: &Option<String>, named: &[(String, String, usize)]| {
            id.as_ref()
                .is_some_and(|id| named.iter().any(|(l, ..)| l == id))
        };
        if !held(&self.source, &self.named) {
            self.source = self.named.first().map(|n| n.0.clone());
        }
        if !held(&self.target, &self.named) {
            self.target = self
                .named
                .get(1)
                .or(self.named.first())
                .map(|n| n.0.clone());
        }
        let layers = model.layers();
        if self
            .layer
            .as_deref()
            .is_none_or(|id| layers.get(id).is_none())
        {
            self.layer = Some(layers.active().to_owned());
        }
        if self.scope == Scope::Selection && selected == 0 {
            self.scope = Scope::All;
        }
        self.status = None;
        self.solve();
    }

    /// The table's pairs solved again, the residuals into their cells (the web's `solve`).
    pub fn solve(&mut self) {
        let mut pairs = Vec::new();
        self.pair_rows.clear();
        for (r, row) in self.rows.iter_mut().enumerate() {
            for c in [RES_Y, RES_X, RES] {
                row[c].clear();
            }
            if let Some(pair) = pair_of(row) {
                pairs.push(pair);
                self.pair_rows.push(r);
            }
        }
        self.used = pairs.iter().filter(|p| p.used).count();
        self.solution = (!pairs.is_empty()).then(|| solver::fit(&pairs, self.kind));
        if let Some(Ok(fit)) = &self.solution {
            for (&r, [vx, vy, v]) in self.pair_rows.iter().zip(&fit.residuals) {
                let row = &mut self.rows[r];
                row[RES_Y] = fixed(vx * 1000.0, 1);
                row[RES_X] = fixed(vy * 1000.0, 1);
                row[RES] = fixed(v * 1000.0, 1);
            }
        }
    }

    /// The used pair with the largest residual (its index among the pairs).
    fn worst(&self) -> Option<usize> {
        let fit = self.fit()?;
        let mut best: Option<usize> = None;
        for (i, &r) in self.pair_rows.iter().enumerate() {
            if self.rows.get(r).is_none_or(|row| row[USE] == "0") {
                continue;
            }
            if best.is_none_or(|b| fit.residuals[i][2] > fit.residuals[b][2]) {
                best = Some(i);
            }
        }
        best
    }

    /// Whether Uygula has objects to take (without listing them: Tümü of a large drawing).
    fn has_targets(&self, model: &Model, selected: usize) -> bool {
        match self.scope {
            Scope::Selection => selected > 0,
            Scope::Layer => self.layer.as_deref().is_some_and(|l| model.count(l) > 0),
            Scope::All => !model.is_empty(),
        }
    }

    /// The objects Uygula takes by their persistent ids (the web's `targets`):
    /// the selection's, a layer's or every object, in document order.
    fn targets(&self, model: &Model, selection: &[Slot]) -> Vec<String> {
        let uid = |slot: Slot| model.uid(slot).map(|u| u.to_string());
        match (self.scope, self.layer.as_deref()) {
            (Scope::Selection, _) => selection.iter().filter_map(|&s| uid(s)).collect(),
            (Scope::Layer, Some(layer)) => model
                .by_layer(layer)
                .filter_map(|e| uid(Slot(e.base().id)))
                .collect(),
            (Scope::Layer, None) => Vec::new(),
            (Scope::All, _) => model
                .entities()
                .filter_map(|e| uid(Slot(e.base().id)))
                .collect(),
        }
    }

    /// Adla eşle (the web's `matchByName`): the points of the same name on
    /// the two layers become the table's pairs; a name on a layer more than
    /// once is left out. How many pairs, and how many names were left out;
    /// with no pair the table stays.
    pub fn match_by_name(&mut self, model: &Model) -> (usize, usize) {
        let (Some(source), Some(target)) = (&self.source, &self.target) else {
            return (0, 0);
        };
        let from = named_points(model, source);
        let to: HashMap<String, Vec<Vec2>> = named_points(model, target).into_iter().collect();
        let mut rows = Vec::new();
        let mut twice = 0;
        for (name, src) in from {
            let Some(dst) = to.get(&name) else {
                continue;
            };
            if src.len() > 1 || dst.len() > 1 {
                twice += 1;
                continue;
            }
            rows.push([
                "1".to_owned(),
                name,
                js_number(src[0].x),
                js_number(src[0].y),
                js_number(dst[0].x),
                js_number(dst[0].y),
                String::new(),
                String::new(),
                String::new(),
            ]);
        }
        let n = rows.len();
        if n > 0 {
            self.rows = rows;
            self.solve();
        }
        (n, twice)
    }

    /// A row's source or target shown on the drawing: its coordinates, and
    /// the name of the point it snapped to when the row has none (the web's `pick`).
    pub fn picked(&mut self, row: usize, side: Side, p: Vec2, name: Option<String>) {
        let Some(r) = self.rows.get_mut(row) else {
            return;
        };
        let (y, x) = match side {
            Side::Source => (SOURCE_Y, SOURCE_X),
            Side::Target => (TARGET_Y, TARGET_X),
        };
        r[y] = js_number(p.x);
        r[x] = js_number(p.y);
        if let Some(name) = name
            && js_trim(&r[NAME]).is_empty()
        {
            r[NAME] = name;
        }
        self.solve();
    }
}

/// The layers holding named points, in the tree's order: id, path, how
/// many (the web's `choices`).
fn named_layers(model: &Model) -> Vec<(String, String, usize)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for e in model.entities() {
        if let Entity::Point(pt) = e
            && pt
                .base
                .label
                .as_deref()
                .is_some_and(|l| !js_trim(l).is_empty())
        {
            *counts.entry(pt.base.layer_id.as_str()).or_default() += 1;
        }
    }
    let layers = model.layers();
    layers
        .leaves()
        .into_iter()
        .filter_map(|n| {
            let count = *counts.get(n.id.as_str())?;
            Some((n.id.clone(), layers.path(&n.id), count))
        })
        .collect()
}

/// A layer's named points by name, in the order their names first come.
fn named_points(model: &Model, layer: &str) -> Vec<(String, Vec<Vec2>)> {
    let mut out: Vec<(String, Vec<Vec2>)> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    for e in model.by_layer(layer) {
        let Entity::Point(pt) = e else {
            continue;
        };
        let Some(name) = pt.base.label.as_deref().map(js_trim) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let p = Vec2::new(pt.p.x, pt.p.y);
        match at.get(name) {
            Some(&i) => out[i].1.push(p),
            None => {
                at.insert(name.to_owned(), out.len());
                out.push((name.to_owned(), vec![p]));
            }
        }
    }
    out
}

/// The solution as `cad.entities.transform`'s transform (its centred form).
fn transform_of(fit: &solver::Fit) -> Option<Transform> {
    let from = kentos_contracts::Vec2 {
        x: fit.from.x,
        y: fit.from.y,
    };
    let to = kentos_contracts::Vec2 {
        x: fit.to.x,
        y: fit.to.y,
    };
    let p = fit.params.as_slice();
    Some(match fit.kind {
        FitKind::Helmert => {
            let [a, b] = p.try_into().ok()?;
            Transform::Similarity { from, to, a, b }
        }
        FitKind::Affine => Transform::Affine {
            from,
            to,
            m: p.try_into().ok()?,
        },
        FitKind::Projective => Transform::Projective {
            from,
            to,
            h: p.try_into().ok()?,
        },
    })
}
