//! Mekânsal istatistik (docs/adr/0238): where objects lie and how their
//! values sit among their neighbours. Centres and spreads (`centers`), the
//! nearest neighbour index (`nearest`), Moran's I and Getis-Ord Gi\*
//! (`autocorrelation`, over `weights`), DBSCAN and k-means (`clusters`), the
//! neighbours from a k-d tree (`kdtree`).
//!
//! An object's place is its `shape_centroid` (the expressions' `$merkez_y`,
//! `$merkez_x`); places are worked as differences from the first one. Values
//! and weights are read by `kentos.statistics/1` (ADR 0200 §4). Every run
//! gives what the tools write in full: the summary, the warnings, the table,
//! the new objects with their attributes or the attributes and colour of each
//! object's copy, the numbers formatted here so both platforms write the same
//! text. The independent reference is scripts/fixtures/spatial_stats_cases.py.

pub mod autocorrelation;
mod calls;
pub mod centers;
pub mod clusters;
pub mod kdtree;
pub mod nearest;
pub mod weights;

pub use calls::OPS;

use crate::entity::Shape;
use crate::geom::centroid::shape_centroid;
use crate::ops::statistics::{fixed, read_number};
use crate::vec2::Vec2;

/// A new object a run writes: its shape and attributes, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct NewObject {
    pub shape: Shape,
    pub attrs: Vec<[String; 2]>,
}

/// An object's copy a run writes: its place in the input, the attributes
/// added to its own, and its colour.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCopy {
    pub index: usize,
    pub attrs: Vec<[String; 2]>,
    pub color: &'static str,
}

/// A table: its columns and rows, as texts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// A named number output (the run's `ratio`, `z`, `p`, `moransI` …).
#[derive(Clone, Debug, PartialEq)]
pub struct Number {
    pub name: &'static str,
    pub value: f64,
}

/// What a run gives the tool: everything it writes or says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatsRun {
    pub summary: String,
    pub infos: Vec<String>,
    pub warnings: Vec<String>,
    pub table: Option<Table>,
    pub numbers: Vec<Number>,
    pub objects: Vec<NewObject>,
    pub copies: Vec<ObjectCopy>,
}

impl StatsRun {
    fn warn_if(&mut self, n: usize, text: impl FnOnce(usize) -> String) {
        if n > 0 {
            self.warnings.push(text(n));
        }
    }
}

/// The most neighbour pairs a run builds (§7).
pub const MOST_PAIRS: usize = 50_000_000;

/// An object's place: its centroid, when finite.
pub fn place(s: &Shape) -> Option<Vec2> {
    shape_centroid(s).filter(|p| p.x.is_finite() && p.y.is_finite())
}

/// The places of the objects that have one: the first as the origin, each
/// as its difference from it, with its place in the input.
#[derive(Clone, Debug, Default)]
pub struct Placed {
    pub origin: Vec2,
    pub pts: Vec<Vec2>,
    pub index: Vec<usize>,
    pub unplaced: usize,
}

impl Placed {
    /// The shapes' places; those kept by `keep` (an object's place in the input).
    pub fn of(shapes: &[Shape], mut keep: impl FnMut(usize) -> bool) -> Placed {
        let mut out = Placed::default();
        let mut first = None;
        for (i, s) in shapes.iter().enumerate() {
            let Some(p) = place(s) else {
                out.unplaced += 1;
                continue;
            };
            if !keep(i) {
                continue;
            }
            let o = *first.get_or_insert(p);
            out.pts.push(Vec2::new(p.x - o.x, p.y - o.y));
            out.index.push(i);
        }
        out.origin = first.unwrap_or_default();
        out
    }

    /// A worked place back on the map.
    pub fn absolute(&self, p: Vec2) -> Vec2 {
        Vec2::new(self.origin.x + p.x, self.origin.y + p.y)
    }
}

/// A value read as a number (`kentos.statistics/1`), nearest double.
pub fn number_of(text: Option<&str>) -> Option<f64> {
    read_number(text?)
        .map(|d| d.to_f64())
        .filter(|v| v.is_finite())
}

/// Two-sided p of a standard normal z: erfc(|z| / √2).
pub fn p_value(z: f64) -> f64 {
    libm::erfc(z.abs() / std::f64::consts::SQRT_2)
}

/// What a test's z says (§2): clustered, dispersed or random at p < 0.05;
/// `clustered_below`: a negative z is the clustered side (the nearest
/// neighbour index), else a positive one (Moran's I).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    Clustered,
    Dispersed,
    Random,
}

impl Pattern {
    pub fn of(z: f64, p: f64, clustered_below: bool) -> Pattern {
        if p.is_nan() || p >= 0.05 {
            return Pattern::Random;
        }
        match (z < 0.0) == clustered_below {
            true => Pattern::Clustered,
            false => Pattern::Dispersed,
        }
    }

    /// The table's word.
    pub fn title(self) -> &'static str {
        match self {
            Pattern::Clustered => "Kümelenmiş",
            Pattern::Dispersed => "Dağınık",
            Pattern::Random => "Rastgele",
        }
    }

    /// The summary's word.
    pub fn word(self) -> &'static str {
        match self {
            Pattern::Clustered => "kümelenmiş",
            Pattern::Dispersed => "dağınık",
            Pattern::Random => "rastgele",
        }
    }
}

/// A double at `d` fraction digits (Rust's formatting, no minus on zero).
pub fn text(x: f64, d: u32) -> String {
    fixed(x, d)
}

/// A pair of an attribute's name and value.
fn pair(name: &str, value: impl Into<String>) -> [String; 2] {
    [name.to_owned(), value.into()]
}

/// The warning about objects without a place.
fn unplaced_note(n: usize) -> String {
    format!("{n} nesnenin yeri bulunamadı; alınmadı.")
}

/// The warning about objects whose number a field does not give.
fn unread_note(n: usize, what: &str, field: &str) -> String {
    format!("{n} nesnenin {what} “{field}” alanından sayı olarak okunamadı; alınmadı.")
}
