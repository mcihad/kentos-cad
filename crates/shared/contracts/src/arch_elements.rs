//! Grids, columns, stairs and ramps (docs/adr/0343 §1): their own fields. Their drawing is derived from these, not kept:
//! a grid's line and bubbles, a column's outline, a stair's flights, landings, treads and arrow, a ramp's flights and
//! levels.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ArchInfo;
use crate::entity::{EntityBase, Vec2};

/// The most vertices a stair's walking line or a ramp's centre line holds.
pub const MAX_FLIGHT_POINTS: usize = 4;
/// The most risers a stair has.
pub const MAX_RISERS: u32 = 400;

/// Which sides of a stair or a ramp carry a railing, left and right as one walks up it from its first point
/// (docs/adr/0705 §2). Absent on the object: none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Railings {
    Left,
    Right,
    Both,
}

impl Railings {
    pub const ALL: [Railings; 3] = [Railings::Left, Railings::Right, Railings::Both];

    /// Its name on the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            Railings::Left => "left",
            Railings::Right => "right",
            Railings::Both => "both",
        }
    }

    /// Its words in the interface.
    pub fn words(self) -> &'static str {
        match self {
            Railings::Left => "Sol",
            Railings::Right => "Sağ",
            Railings::Both => "İki yan",
        }
    }

    pub fn left(self) -> bool {
        matches!(self, Railings::Left | Railings::Both)
    }

    pub fn right(self) -> bool {
        matches!(self, Railings::Right | Railings::Both)
    }
}

/// Which ends of a grid carry a bubble.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GridBubbles {
    #[default]
    Both,
    Start,
    End,
    None,
}

impl GridBubbles {
    pub const ALL: [GridBubbles; 4] = [
        GridBubbles::Both,
        GridBubbles::Start,
        GridBubbles::End,
        GridBubbles::None,
    ];

    pub fn label(self) -> &'static str {
        match self {
            GridBubbles::Both => "İki uçta",
            GridBubbles::Start => "Başta",
            GridBubbles::End => "Sonda",
            GridBubbles::None => "Yok",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            GridBubbles::Both => "both",
            GridBubbles::Start => "start",
            GridBubbles::End => "end",
            GridBubbles::None => "none",
        }
    }
}

fn is_both(b: &GridBubbles) -> bool {
    *b == GridBubbles::Both
}

/// A grid's own fields (docs/adr/0343 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GridFields {
    /// Its start and end.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// One bulge per point (the first the arc's; absent: straight).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    /// Its label (A, B … 1, 2 …), on the wire `name`: an object's own `label` is another field.
    #[serde(rename = "name")]
    pub label: String,
    #[serde(default, skip_serializing_if = "is_both")]
    #[cfg_attr(feature = "ts", ts(as = "Option<GridBubbles>", optional))]
    pub bubbles: GridBubbles,
    /// The bubble's diameter on paper, millimetres; absent: 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub bubble_size: Option<f64>,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
}

/// A grid (docs/adr/0343).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GridEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub grid: GridFields,
}

/// A column's outline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ColumnShape {
    #[default]
    Rect,
    Round,
}

impl ColumnShape {
    pub const ALL: [ColumnShape; 2] = [ColumnShape::Rect, ColumnShape::Round];

    pub fn label(self) -> &'static str {
        match self {
            ColumnShape::Rect => "Dikdörtgen",
            ColumnShape::Round => "Daire",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ColumnShape::Rect => "rect",
            ColumnShape::Round => "round",
        }
    }
}

fn is_rect(s: &ColumnShape) -> bool {
    *s == ColumnShape::Rect
}

/// Why a column is not where its grids say.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ColumnBreak {
    /// One of its grids is gone, or they no longer cross.
    GridGone,
}

impl ColumnBreak {
    pub fn words(self) -> &'static str {
        "Kolonun akslarından biri yok ya da akslar kesişmiyor; kolon son yerinde kaldı. Aksı geri getirin ya da kolonu başka akslara bağlayın."
    }
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// A column's own fields (docs/adr/0343 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ColumnFields {
    /// Its centre.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    #[serde(default, skip_serializing_if = "is_rect")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ColumnShape>", optional))]
    pub shape: ColumnShape,
    /// Across, metres (a round column's diameter).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// Along, metres; absent: its width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
    /// Its turn, radians counter-clockwise.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: f64,
    /// Its type's name.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub fill: bool,
    /// The two grids it stands where they cross, by their persistent ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grids: Option<Vec<String>>,
    /// Its place by its grids' labels (“B-3”), which the document writes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub place: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<ColumnBreak>,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
}

impl ColumnFields {
    /// Its depth: its own, else its width.
    pub fn depth(&self) -> f64 {
        self.depth.unwrap_or(self.width)
    }
}

/// A column (docs/adr/0343).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ColumnEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub column: ColumnFields,
}

/// A stair's own fields (docs/adr/0343 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StairFields {
    /// Its walking line, up from its first point: two points straight, three an L, four a U.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// A curved (spiral) stair's walking line (docs/adr/0709, schema 117): two bulges, the first its arc's (the tangent
    /// of a quarter of its turn, counter-clockwise positive), the second 0; only on a two-point line. Absent: straight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub risers: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub riser_height: f64,
    /// A tread's depth along the walking line.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tread: f64,
    /// A landing's length along the walking line at a turn; absent: the width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub landing: Option<f64>,
    /// The riser its cut line stands at; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub cut: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub numbering: bool,
    /// Its railings' sides; absent: none (docs/adr/0705 §2, schema 115).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub railings: Option<Railings>,
    /// Its whole rise is its level's floor-to-floor height (docs/adr/0724 §1, schema 125): its riser height is that
    /// height over its risers, written again whenever the level's height changes; needs its level.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub height_from_level: bool,
    /// A curved stair's centre post, its diameter (docs/adr/0724 §3, schema 125); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub post: Option<f64>,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
}

/// A stair (docs/adr/0343).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StairEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub stair: StairFields,
}

/// A ramp's own fields (docs/adr/0343 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RampFields {
    /// Its centre line, up from its first point.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// How much higher its end is than its start.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub rise: f64,
    /// Its slope, per cent; absent: its rise over its sloped length.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub slope: Option<f64>,
    /// Its start's elevation; absent: 0. Bound to its level (`base_offset`), the document writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub base: Option<f64>,
    /// Its start's elevation over its level's (docs/adr/0723 §2, schema 124): `base` is then its level's elevation
    /// plus this, written again whenever the level's elevation changes; absent: `base` is its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub base_offset: Option<f64>,
    /// A landing's length at a turn; absent: the width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub landing: Option<f64>,
    /// Its railings' sides; absent: none (docs/adr/0705 §2, schema 115).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub railings: Option<Railings>,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
}

/// A ramp (docs/adr/0343).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RampEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub ramp: RampFields,
}

type Problem = Option<(String, String)>;

fn point_problem(p: &Vec2, at: &str, whose: &str) -> Problem {
    (!(p.x.is_finite() && p.y.is_finite())).then(|| {
        (
            at.into(),
            format!("{whose} koordinatları sonlu olmalı."),
        )
    })
}

fn positive(v: f64, field: &str, whose: &str, max: f64) -> Problem {
    (!(v.is_finite() && v > 0.0 && v <= max)).then(|| {
        (
            field.into(),
            format!("{whose} {v}; sıfırdan büyük, en çok {max} olmalı."),
        )
    })
}

fn blank(v: &Option<String>, field: &str) -> Problem {
    v.as_deref().is_some_and(|t| t.trim().is_empty()).then(|| {
        (
            field.into(),
            "Boş metin yazılmaz; alanı kaldırın ya da bir değer yazın.".into(),
        )
    })
}

/// A path of 2 to `MAX_FLIGHT_POINTS` points, none two the same in a row.
fn flight_problem(pts: &[Vec2], what: &str) -> Problem {
    let n = pts.len();
    if !(2..=MAX_FLIGHT_POINTS).contains(&n) {
        return Some((
            "pts".into(),
            format!("{what} yolu 2, 3 ya da 4 noktadır (düz, L, U); {n} verildi."),
        ));
    }
    for (i, p) in pts.iter().enumerate() {
        if let Some(found) = point_problem(p, &format!("pts[{i}]"), &format!("{}. noktanın", i + 1)) {
            return Some(found);
        }
    }
    if let Some(i) = pts.windows(2).position(|w| w[0].x == w[1].x && w[0].y == w[1].y) {
        return Some((
            format!("pts[{}]", i + 1),
            format!("{what} yolunun {}. ve {}. noktası aynı.", i + 1, i + 2),
        ));
    }
    None
}

impl GridFields {
    /// What is wrong with the grid: the field and the words; none: it holds.
    pub fn problem(&self) -> Problem {
        if self.pts.len() != 2 {
            return Some((
                "pts".into(),
                format!("Aks iki noktadır (başı ve sonu); {} verildi.", self.pts.len()),
            ));
        }
        for (i, p) in self.pts.iter().enumerate() {
            if let Some(found) = point_problem(p, &format!("pts[{i}]"), &format!("{}. noktanın", i + 1)) {
                return Some(found);
            }
        }
        if self.pts[0].x == self.pts[1].x && self.pts[0].y == self.pts[1].y {
            return Some((
                "pts".into(),
                "Aksın başı ve sonu aynı; aksın bir boyu olmalı.".into(),
            ));
        }
        if let Some(b) = &self.bulges
            && (b.len() != 2 || b.iter().any(|x| !x.is_finite()))
        {
            return Some((
                "bulges".into(),
                "Aksın iki noktası ve iki sonlu bükümü olur.".into(),
            ));
        }
        if self.label.trim().is_empty() {
            return Some(("name".into(), "Aksın etiketi boş olamaz.".into()));
        }
        if let Some(s) = self.bubble_size {
            return positive(s, "bubbleSize", "Balonun çapı", 100.0);
        }
        None
    }
}

impl ColumnFields {
    /// What is wrong with the column: the field and the words; none: it holds.
    pub fn problem(&self) -> Problem {
        if let Some(found) = point_problem(&self.at, "at", "Kolonun merkezinin") {
            return Some(found);
        }
        if let Some(found) = positive(self.width, "width", "Kolonun eni", 100.0) {
            return Some(found);
        }
        if let Some(d) = self.depth
            && let Some(found) = positive(d, "depth", "Kolonun boyu", 100.0)
        {
            return Some(found);
        }
        if !self.rotation.is_finite() {
            return Some(("rotation".into(), "Kolonun dönüşü sonlu olmalı.".into()));
        }
        if let Some(g) = &self.grids
            && (g.len() != 2 || g[0] == g[1])
        {
            return Some((
                "grids".into(),
                "Kolon iki ayrı aksın kesişimine bağlanır.".into(),
            ));
        }
        blank(&self.kind, "type").or_else(|| blank(&self.tag, "tag"))
    }
}

impl StairFields {
    /// What is wrong with the stair's fields themselves (not how they fit: `geom::arch` says that); none: it holds.
    pub fn problem(&self) -> Problem {
        if let Some(found) = flight_problem(&self.pts, "Merdivenin") {
            return Some(found);
        }
        if let Some(found) = self.arc_problem() {
            return Some(found);
        }
        for (field, v, whose) in [
            ("width", self.width, "Merdivenin genişliği"),
            ("riserHeight", self.riser_height, "Rıht yüksekliği"),
            ("tread", self.tread, "Basamak derinliği"),
        ] {
            if let Some(found) = positive(v, field, whose, 100.0) {
                return Some(found);
            }
        }
        if !(1..=MAX_RISERS).contains(&self.risers) {
            return Some((
                "risers".into(),
                format!("Rıht sayısı {}; 1 ile {MAX_RISERS} arasında olmalı.", self.risers),
            ));
        }
        if let Some(l) = self.landing
            && let Some(found) = positive(l, "landing", "Sahanlığın boyu", 100.0)
        {
            return Some(found);
        }
        if let Some(c) = self.cut
            && !(1..=self.risers).contains(&c)
        {
            return Some((
                "cut".into(),
                format!("Kesme çizgisi {c}. rıhtta; 1 ile {} arasında olmalı.", self.risers),
            ));
        }
        if let Some(found) = self.post_problem() {
            return Some(found);
        }
        if self.height_from_level && self.arch.level.is_none() {
            return Some((
                "heightFromLevel".into(),
                "Merdivenin yüksekliği kata bağlı ama merdivenin katı yok. Merdivene kat verin ya da bağı kaldırın."
                    .into(),
            ));
        }
        None
    }

    /// Its landing's length: its own, else its width.
    pub fn landing(&self) -> f64 {
        self.landing.unwrap_or(self.width)
    }

    /// Its riser height read from its level among `levels` (docs/adr/0724 §1): the level's height over its risers;
    /// none when its rise is not its level's, the project lacks the level or the level has no height.
    pub fn level_riser(&self, levels: &[crate::ArchLevel]) -> Option<f64> {
        if !self.height_from_level || self.risers == 0 {
            return None;
        }
        let id = self.arch.level.as_deref()?;
        let h = levels.iter().find(|l| l.id == id)?.height?;
        (h.is_finite() && h > 0.0).then(|| h / f64::from(self.risers))
    }

    /// A curved stair's walking line's radius; none for a straight one.
    fn radius(&self) -> Option<f64> {
        let b = self.bulge();
        if self.pts.len() != 2 || b == 0.0 || !b.is_finite() {
            return None;
        }
        let (dx, dy) = (self.pts[1].x - self.pts[0].x, self.pts[1].y - self.pts[0].y);
        Some((dx * dx + dy * dy).sqrt() * (1.0 + b * b) / (4.0 * b.abs()))
    }

    /// What is wrong with a centre post (docs/adr/0724 §3): only on a curved stair, its diameter above 0 and its radius
    /// less than the stair's inner radius.
    fn post_problem(&self) -> Problem {
        let d = self.post?;
        let bad = |words: String| Some(("post".to_owned(), words));
        let Some(r) = self.radius() else {
            return bad("Merkez direği yalnız döner merdivende olur.".into());
        };
        if !(d.is_finite() && d > 0.0) {
            return bad(format!("Merkez direğinin çapı {d}; sıfırdan büyük, sonlu olmalı."));
        }
        let inner = r - self.width / 2.0;
        if d / 2.0 >= inner - 1e-9 {
            return bad(format!(
                "Merkez direğinin yarıçapı {:.3} m; merdivenin iç yarıçapından ({inner:.3} m) küçük olmalı.",
                d / 2.0
            ));
        }
        None
    }

    /// Its arc's bulge (docs/adr/0709): the first of `bulges`; 0 for a straight stair.
    pub fn bulge(&self) -> f64 {
        self.bulges.as_ref().and_then(|b| b.first().copied()).unwrap_or(0.0)
    }

    /// What is wrong with a curved stair's arc: two bulges on a two-point line, the first finite and not 0, the second
    /// 0; its inner side's radius above 0 (the walking line's radius more than half the width).
    fn arc_problem(&self) -> Problem {
        let Some(b) = &self.bulges else {
            return None;
        };
        let bad = |words: &str| Some(("bulges".to_owned(), words.to_owned()));
        if self.pts.len() != 2 {
            return bad("Döner merdivenin yürüme çizgisi iki noktadır (başı ve sonu) ve bir yaydır.");
        }
        if b.len() != 2 || !b.iter().all(|x| x.is_finite()) || b[1] != 0.0 || b[0] == 0.0 {
            return bad("Döner merdivenin iki bükümü olur: ilki yayınki (sıfır değil, sonlu), ikincisi 0.");
        }
        let (dx, dy) = (self.pts[1].x - self.pts[0].x, self.pts[1].y - self.pts[0].y);
        let chord = (dx * dx + dy * dy).sqrt();
        let r = chord * (1.0 + b[0] * b[0]) / (4.0 * b[0].abs());
        if self.width.is_finite() && r - self.width / 2.0 <= 1e-9 {
            return bad(&format!(
                "Yürüme çizgisinin yarıçapı {r:.3} m; merdivenin genişliğinin yarısından ({:.3} m) büyük olmalı.",
                self.width / 2.0
            ));
        }
        None
    }
}

impl RampFields {
    /// What is wrong with the ramp's fields themselves; none: it holds.
    pub fn problem(&self) -> Problem {
        if let Some(found) = flight_problem(&self.pts, "Rampanın") {
            return Some(found);
        }
        if let Some(found) = positive(self.width, "width", "Rampanın genişliği", 100.0) {
            return Some(found);
        }
        if !(self.rise.is_finite() && self.rise.abs() <= 1000.0) {
            return Some((
                "rise".into(),
                format!("Rampanın yüksekliği {}; sonlu ve en çok 1000 m olmalı.", self.rise),
            ));
        }
        if let Some(s) = self.slope
            && let Some(found) = positive(s, "slope", "Rampanın eğimi (%)", 1000.0)
        {
            return Some(found);
        }
        if let Some(b) = self.base
            && !b.is_finite()
        {
            return Some(("base".into(), "Rampanın başlangıç kotu sonlu olmalı.".into()));
        }
        if let Some(o) = self.base_offset {
            if !(o.is_finite() && o.abs() <= crate::MAX_LEVEL_OFFSET) {
                return Some((
                    "baseOffset".into(),
                    format!(
                        "Rampanın kata göre başlangıç kotu {o}; sonlu ve en çok {} m olmalı.",
                        crate::MAX_LEVEL_OFFSET
                    ),
                ));
            }
            if self.arch.level.is_none() {
                return Some((
                    "baseOffset".into(),
                    "Rampanın başlangıç kotu kata göre verildi ama rampanın katı yok. Rampaya kat verin ya da \
                     başlangıç kotunu kendisi olarak yazın."
                        .into(),
                ));
            }
        }
        if let Some(l) = self.landing
            && let Some(found) = positive(l, "landing", "Sahanlığın boyu", 100.0)
        {
            return Some(found);
        }
        None
    }

    /// Its landing's length: its own, else its width.
    pub fn landing(&self) -> f64 {
        self.landing.unwrap_or(self.width)
    }

    /// Its start's elevation read from its level among `levels` (docs/adr/0723 §2): the level's elevation plus its
    /// offset; none when it is not bound to its level or the project lacks the level.
    pub fn bound_base(&self, levels: &[crate::ArchLevel]) -> Option<f64> {
        let offset = self.base_offset?;
        let id = self.arch.level.as_deref()?;
        levels.iter().find(|l| l.id == id).map(|l| l.elevation + offset)
    }
}

/// A grid label's next (docs/adr/0343 §4): letters A … Z skipping I and O, then AA, AB …; numbers one more; anything
/// else ending in digits one more there (A1 → A2); else the same.
pub fn next_label(label: &str) -> String {
    let t = label.trim();
    if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
        return t
            .parse::<u64>()
            .map_or_else(|_| t.to_owned(), |n| (n + 1).to_string());
    }
    if !t.is_empty() && t.chars().all(|c| c.is_ascii_uppercase()) {
        return letters_after(t);
    }
    let digits = t.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits > 0 {
        let (head, tail) = t.split_at(t.len() - digits);
        if let Ok(n) = tail.parse::<u64>() {
            return format!("{head}{:0width$}", n + 1, width = digits);
        }
    }
    t.to_owned()
}

/// The letters used for grids (I and O are skipped: they read as 1 and 0).
const GRID_LETTERS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";

fn letters_after(t: &str) -> String {
    // The label as a number in base 24 over GRID_LETTERS, one more; a letter not among them counts as the one before it.
    let mut digits: Vec<usize> = t
        .bytes()
        .map(|b| {
            GRID_LETTERS
                .iter()
                .position(|&l| l == b)
                .unwrap_or_else(|| GRID_LETTERS.iter().filter(|&&l| l < b).count().saturating_sub(1))
        })
        .collect();
    let mut i = digits.len();
    loop {
        if i == 0 {
            digits.insert(0, 0);
            break;
        }
        i -= 1;
        if digits[i] + 1 < GRID_LETTERS.len() {
            digits[i] += 1;
            break;
        }
        digits[i] = 0;
    }
    digits.iter().map(|&d| GRID_LETTERS[d] as char).collect()
}

#[cfg(test)]
mod tests {
    use super::next_label;

    #[test]
    fn a_grid_keeps_its_label_through_json() {
        let text = r#"{"kind":"grid","id":3,"layerId":"aks","attrs":{},"pts":[{"x":0,"y":0},{"x":5,"y":0}],"name":"B"}"#;
        let e: crate::Entity = serde_json::from_str(text).expect("reads");
        let crate::Entity::Grid(g) = &e else { panic!("a grid") };
        assert_eq!(g.grid.label, "B");
        assert_eq!(g.base.label, None, "the grid's label is its own field");
        let back = serde_json::to_value(&e).expect("writes");
        assert_eq!(back["name"], "B");
    }

    #[test]
    fn grid_labels_follow_on() {
        for (a, b) in [
            ("A", "B"),
            ("H", "J"),
            ("N", "P"),
            ("Z", "AA"),
            ("AZ", "BA"),
            ("1", "2"),
            ("9", "10"),
            ("A1", "A2"),
            ("S09", "S10"),
            ("x", "x"),
        ] {
            assert_eq!(next_label(a), b, "{a}");
        }
    }
}
