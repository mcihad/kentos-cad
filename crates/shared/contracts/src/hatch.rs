//! Hatches' patterns and their tie to their boundary (docs/adr/0186): solid,
//! user-defined lines and crossed lines, a pattern of line families (the
//! library's or a file's, its definition carried with it), a gradient; and,
//! for a hatch made inside a closed object, the objects its region follows.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{EntityId, Vec2};

/// The most line families a pattern may have, and dashes a family.
pub const MAX_PATTERN_LINES: usize = 64;
pub const MAX_PATTERN_DASHES: usize = 16;
/// The longest a pattern's name may be.
pub const MAX_PATTERN_NAME: usize = 64;
/// The largest number a pattern's definition may hold, and its scale.
pub const MAX_PATTERN_SIZE: f64 = 1.0e6;
/// The most objects a hatch's tie may name.
pub const MAX_ASSOC_OBJECTS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HatchPatternType {
    Solid,
    /// User-defined lines at `angle`, `spacing` apart.
    Lines,
    /// User-defined lines and the same turned 90°.
    Cross,
    /// A pattern of line families (`name`, `scale`, `lines`).
    Pattern,
    /// A gradient from the hatch's colour to `gradient.color2`.
    Gradient,
}

/// One family of a pattern's lines (docs/adr/0186 §1), in the pattern's
/// units, before the pattern turns: lines at `angle` (degrees) through
/// `origin`, each the next `offset` on (`[along, across]` the line), drawn
/// as `dashes` say (plus drawn, minus a gap, 0 a dot; none: whole).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PatternLine {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "pattern")))]
    pub origin: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "pattern")))]
    pub offset: [f64; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "pattern")))]
    pub dashes: Vec<f64>,
}

/// How a gradient runs over its hatch (docs/adr/0186 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GradientShape {
    /// From one side of the hatch to the other along `angle`.
    Linear,
    /// The second colour along the middle, the first at both sides.
    Cylinder,
    /// The second colour in the middle, the first at the farthest corner.
    Spherical,
}

/// A gradient: its shape, whether it runs the other way, its second colour.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchGradient {
    pub shape: GradientShape,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub inverted: bool,
    /// `#RRGGBB`.
    pub color2: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchPattern {
    #[serde(rename = "type")]
    pub kind: HatchPatternType,
    /// Degrees, counter-clockwise from east: the lines' (lines, cross), the
    /// pattern's turn (pattern), the gradient's direction (gradient).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: f64,
    /// Metres between the lines (lines, cross); 1 and unread for the others.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub spacing: f64,
    /// A pattern's name (`ANSI31`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// A pattern's metres per unit of its definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub scale: Option<f64>,
    /// A pattern's line families.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lines: Option<Vec<PatternLine>>,
    /// A gradient's shape and second colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub gradient: Option<HatchGradient>,
}

/// The objects a hatch follows (docs/adr/0186 §6, 0328 §3): a hatch made inside a closed object follows the
/// object (`outer`), its islands, the texts and inserts left open; a hatch made in the face line work closes
/// (`bounds`: the objects whose line work bounds it, its islands' too) follows that line work; the point clicked
/// inside picks the region's part or face. Exactly one of `outer` and `bounds` is given.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchAssoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub outer: Option<EntityId>,
    /// A face's tie: the gaps its line work was closed across, metres (docs/adr/0329 §3); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub gap: Option<f64>,
    /// The objects whose line work closes its face (docs/adr/0328 §3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<EntityId>>", optional))]
    pub bounds: Vec<EntityId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<EntityId>>", optional))]
    pub islands: Vec<EntityId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<EntityId>>", optional))]
    pub cutouts: Vec<EntityId>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub seed: Vec2,
}

/// The most loops a hatch's boundary has, edges a loop and control points a NURBS edge.
pub const MAX_BOUNDARY_LOOPS: usize = 100_000;
pub const MAX_LOOP_EDGES: usize = 1_000_000;
pub const MAX_CURVE_POINTS: usize = 100_000;
/// The highest degree a NURBS edge may have.
pub const MAX_CURVE_DEGREE: u32 = 10;

/// A boundary edge that is no line or circular arc (docs/adr/0328 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HatchCurve {
    /// An ellipse's arc: centre `c`, major semi-axis `major` (from `c`), minor to major `ratio`, from parameter `t0`
    /// to `t1` (radians): counter-clockwise when `t1` is past `t0`, clockwise when before it; a whole ellipse
    /// `t1 = t0 ± 2π`.
    Ellipse {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        c: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        major: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        ratio: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        t0: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        t1: f64,
    },
    /// A NURBS curve run whole, from its first knot to its last.
    Nurbs {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        degree: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        ctrl: Vec<Vec2>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        knots: Vec<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        weights: Option<Vec<f64>>,
    },
}

/// One closed loop of a hatch's boundary as its source holds it (docs/adr/0328 §1): edge `i` runs from `pts[i]` to
/// the next point (the last to the first); it is straight, a circular arc of `bulges[i]` (tan(θ/4), counter-clockwise
/// positive, as a polygon's), or the curve `curves[i]` (an ellipse's arc or a NURBS curve), whose ends are those
/// points. A loop of one point is one closed curve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchLoop {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub curves: Option<Vec<Option<HatchCurve>>>,
}

impl HatchCurve {
    /// What is wrong with its numbers: finite, a ratio within (0, 1], a sweep within one turn; a NURBS curve's
    /// degree, knots and positive weights. Whether its ends meet its loop's points is the geometry core's.
    pub fn problem(&self) -> Option<String> {
        match self {
            HatchCurve::Ellipse {
                c,
                major,
                ratio,
                t0,
                t1,
            } => {
                if ![c.x, c.y, major.x, major.y, *ratio, *t0, *t1]
                    .iter()
                    .all(|x| x.is_finite())
                {
                    return Some("Elips kenarında sonlu olmayan bir sayı var.".into());
                }
                if !(*ratio > 0.0 && *ratio <= 1.0) || (major.x == 0.0 && major.y == 0.0) {
                    return Some(
                        "Elips kenarının büyük ekseni sıfır olamaz, oranı 0 ile 1 arasında olmalı."
                            .into(),
                    );
                }
                let sweep = (t1 - t0).abs();
                if sweep == 0.0 || sweep > std::f64::consts::TAU * (1.0 + 1e-12) {
                    return Some(
                        "Elips kenarının açıklığı sıfır olamaz ve bir turu aşamaz.".into(),
                    );
                }
                None
            }
            HatchCurve::Nurbs {
                degree,
                ctrl,
                knots,
                weights,
            } => {
                let n = ctrl.len();
                if *degree < 1 || *degree > MAX_CURVE_DEGREE {
                    return Some(format!(
                        "NURBS kenarının derecesi {degree}; 1 ile {MAX_CURVE_DEGREE} arasında olmalı."
                    ));
                }
                if n <= *degree as usize || n > MAX_CURVE_POINTS {
                    return Some(format!(
                        "NURBS kenarının {n} denetim noktası var; dereceden çok, en çok {MAX_CURVE_POINTS} olmalı."
                    ));
                }
                if knots.len() != n + *degree as usize + 1 {
                    return Some(format!(
                        "NURBS kenarının {} düğümü var; denetim noktası + derece + 1 = {} olmalı.",
                        knots.len(),
                        n + *degree as usize + 1
                    ));
                }
                if !ctrl.iter().all(|p| p.x.is_finite() && p.y.is_finite())
                    || !knots.iter().all(|k| k.is_finite())
                    || knots.windows(2).any(|w| w[1] < w[0])
                    || knots[*degree as usize] >= knots[n]
                {
                    return Some("NURBS kenarının noktaları sonlu, düğümleri artan olmalı.".into());
                }
                if let Some(w) = weights
                    && (w.len() != n || !w.iter().all(|x| x.is_finite() && *x > 0.0))
                {
                    return Some(
                        "NURBS kenarının ağırlıkları denetim noktaları kadar ve sıfırdan büyük olmalı."
                            .into(),
                    );
                }
                None
            }
        }
    }
}

impl HatchLoop {
    /// A loop of lines and arcs (a polygon's ring).
    pub fn of_ring(pts: Vec<Vec2>, bulges: Option<Vec<f64>>) -> Self {
        Self {
            pts,
            bulges,
            curves: None,
        }
    }

    /// Whether an edge of it is an ellipse's arc or a NURBS curve.
    pub fn has_curves(&self) -> bool {
        self.curves.iter().flatten().any(Option::is_some)
    }

    /// What is wrong with its form: points finite and as many bulges and curves as points, a loop of one point a
    /// closed curve, of two at least one curved edge, each curve's numbers.
    pub fn problem(&self) -> Option<String> {
        let n = self.pts.len();
        if n == 0 || n > MAX_LOOP_EDGES {
            return Some(format!(
                "Tarama sınırının bir halkasında {n} nokta var; en az 1, en çok {MAX_LOOP_EDGES} olmalı."
            ));
        }
        if !self.pts.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
            return Some("Tarama sınırında sonlu olmayan bir nokta var.".into());
        }
        if let Some(b) = &self.bulges
            && (b.len() != n || !b.iter().all(|x| x.is_finite()))
        {
            return Some("Tarama sınırının bükümleri noktaları kadar ve sonlu olmalı.".into());
        }
        if let Some(c) = &self.curves {
            if c.len() != n {
                return Some("Tarama sınırının eğrileri noktaları kadar olmalı.".into());
            }
            for (i, curve) in c.iter().enumerate() {
                if let Some(curve) = curve {
                    if let Some(words) = curve.problem() {
                        return Some(words);
                    }
                    if self.bulges.as_ref().is_some_and(|b| b[i] != 0.0) {
                        return Some("Tarama sınırının eğri kenarının bükümü 0 olmalı.".into());
                    }
                }
            }
        }
        let curved = |i: usize| {
            self.curves.as_ref().is_some_and(|c| c[i].is_some())
                || self.bulges.as_ref().is_some_and(|b| b[i] != 0.0)
        };
        match n {
            1 if self.curves.as_ref().is_none_or(|c| c[0].is_none()) => {
                Some("Tek noktalı halka kapalı bir eğri olmalı.".into())
            }
            2 if !(curved(0) || curved(1)) => {
                Some("İki noktalı halkanın en az bir kenarı eğri olmalı.".into())
            }
            _ => None,
        }
    }
}

/// What is wrong with a hatch's boundary: its loops within their bound, each loop's form.
pub fn boundary_problem(loops: &[HatchLoop]) -> Option<String> {
    if loops.is_empty() || loops.len() > MAX_BOUNDARY_LOOPS {
        return Some(format!(
            "Tarama sınırında {} halka var; en az 1, en çok {MAX_BOUNDARY_LOOPS} olmalı.",
            loops.len()
        ));
    }
    loops.iter().find_map(HatchLoop::problem)
}

fn finite_all(xs: &[f64]) -> bool {
    xs.iter()
        .all(|x| x.is_finite() && x.abs() <= MAX_PATTERN_SIZE)
}

/// Whether `color` is `#RRGGBB`.
pub fn is_hex_colour(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl HatchPattern {
    /// A user-defined pattern: solid, lines or crossed lines.
    pub fn user(kind: HatchPatternType, angle: f64, spacing: f64) -> Self {
        Self {
            kind,
            angle,
            spacing,
            name: None,
            scale: None,
            lines: None,
            gradient: None,
        }
    }

    /// Whether it is more than the first three kinds wrote: a pattern, a
    /// gradient or one of their fields (`.kcad` schema 23).
    pub fn has_definition(&self) -> bool {
        matches!(
            self.kind,
            HatchPatternType::Pattern | HatchPatternType::Gradient
        ) || self.name.is_some()
            || self.scale.is_some()
            || self.lines.is_some()
            || self.gradient.is_some()
    }

    /// What is wrong with it (docs/adr/0186 §1): the field and the refusal's
    /// words; none when it may be written. Each kind its own fields only: a
    /// pattern's name, scale and families within their bounds, each family's
    /// lines apart; a gradient's second colour.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        let kind = self.kind;
        let pattern = kind == HatchPatternType::Pattern;
        if !pattern && (self.name.is_some() || self.scale.is_some() || self.lines.is_some()) {
            return Some((
                "pattern",
                "Yalnız desen türündeki taramanın adı, ölçeği ve çizgi aileleri olur.".to_owned(),
            ));
        }
        if kind != HatchPatternType::Gradient && self.gradient.is_some() {
            return Some((
                "pattern.gradient",
                "Yalnız degrade taramanın ikinci rengi ve biçimi olur.".to_owned(),
            ));
        }
        if matches!(kind, HatchPatternType::Lines | HatchPatternType::Cross)
            && !(self.spacing > 0.0 && self.spacing <= MAX_PATTERN_SIZE)
        {
            return Some((
                "pattern.spacing",
                format!(
                    "Taramanın çizgi aralığı {}; sıfırdan büyük ve sonlu olmalı.",
                    self.spacing
                ),
            ));
        }
        if pattern {
            let name = self.name.as_deref().unwrap_or("");
            if name.trim().is_empty() || name.chars().count() > MAX_PATTERN_NAME {
                return Some((
                    "pattern.name",
                    format!("Desenin adı boş olamaz ve en çok {MAX_PATTERN_NAME} harf olabilir."),
                ));
            }
            let scale = self.scale.unwrap_or(f64::NAN);
            if !(scale > 0.0 && scale <= MAX_PATTERN_SIZE) {
                return Some((
                    "pattern.scale",
                    format!("Desenin ölçeği {scale}; sıfırdan büyük ve sonlu olmalı."),
                ));
            }
            let lines = self.lines.as_deref().unwrap_or_default();
            if lines.is_empty() || lines.len() > MAX_PATTERN_LINES {
                return Some((
                    "pattern.lines",
                    format!(
                        "Desenin {} çizgi ailesi var; en az 1, en çok {MAX_PATTERN_LINES} olmalı.",
                        lines.len()
                    ),
                ));
            }
            for (k, l) in lines.iter().enumerate() {
                let n = k + 1;
                if !finite_all(&[l.angle, l.origin[0], l.origin[1], l.offset[0], l.offset[1]])
                    || !finite_all(&l.dashes)
                {
                    return Some((
                        "pattern.lines",
                        format!(
                            "Desenin {n}. çizgi ailesinde sonlu olmayan ya da çok büyük bir sayı var."
                        ),
                    ));
                }
                if l.offset[1] == 0.0 {
                    return Some((
                        "pattern.lines",
                        format!(
                            "Desenin {n}. çizgi ailesinin çizgileri arası 0; aileler sıfırdan büyük aralıklı olmalı."
                        ),
                    ));
                }
                if l.dashes.len() > MAX_PATTERN_DASHES {
                    return Some((
                        "pattern.lines",
                        format!(
                            "Desenin {n}. çizgi ailesinde {} kesik var; en çok {MAX_PATTERN_DASHES} olmalı.",
                            l.dashes.len()
                        ),
                    ));
                }
                if !l.dashes.is_empty() && l.dashes.iter().all(|d| *d == 0.0) {
                    return Some((
                        "pattern.lines",
                        format!(
                            "Desenin {n}. çizgi ailesinin kesiklerinin hepsi 0; en az birinin uzunluğu olmalı."
                        ),
                    ));
                }
            }
        }
        if kind == HatchPatternType::Gradient {
            let Some(g) = &self.gradient else {
                return Some((
                    "pattern.gradient",
                    "Degrade taramanın ikinci rengi ve biçimi verilmeli.".to_owned(),
                ));
            };
            if !is_hex_colour(&g.color2) {
                return Some((
                    "pattern.gradient.color2",
                    format!(
                        "Degradenin ikinci rengi “{}”; #RRGGBB biçiminde olmalı.",
                        g.color2
                    ),
                ));
            }
        }
        None
    }
}

impl HatchAssoc {
    /// Every object it names: the closed object or the bounds, the islands, the cutouts.
    pub fn objects(&self) -> impl Iterator<Item = &EntityId> + '_ {
        self.outer
            .iter()
            .chain(&self.bounds)
            .chain(&self.islands)
            .chain(&self.cutouts)
    }

    /// What is wrong with it: its seed finite, its lists within their bounds,
    /// no object named twice.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        if !(self.seed.x.is_finite() && self.seed.y.is_finite()) {
            return Some((
                "assoc.seed",
                "Taramanın tohum noktası sonlu olmalı.".to_owned(),
            ));
        }
        if self.outer.is_some() != self.bounds.is_empty() {
            return Some((
                "assoc",
                "Taramanın ilişkisi ya kapalı nesneyi (outer) ya da sınır nesnelerini (bounds) gösterir; ikisinden biri verilmeli."
                    .to_owned(),
            ));
        }
        if let Some(g) = self.gap
            && (self.outer.is_some() || !(g > 0.0 && g <= 1000.0))
        {
            return Some((
                "assoc.gap",
                "Taramanın boşluk toleransı yalnız yüzün ilişkisinde, 0'dan büyük ve en çok 1000 m olur.".to_owned(),
            ));
        }
        let n = self.islands.len() + self.cutouts.len() + self.bounds.len();
        if n > MAX_ASSOC_OBJECTS {
            return Some((
                "assoc",
                format!(
                    "Taramanın ilişkisi {n} nesne gösteriyor; en çok {MAX_ASSOC_OBJECTS} olmalı."
                ),
            ));
        }
        let mut seen = std::collections::HashSet::with_capacity(n + 1);
        for id in self
            .outer
            .iter()
            .chain(&self.bounds)
            .chain(&self.islands)
            .chain(&self.cutouts)
        {
            if !seen.insert(*id) {
                return Some((
                    "assoc",
                    "Taramanın ilişkisi bir nesneyi iki kez gösteriyor; her nesne bir kez gösterilmeli."
                        .to_owned(),
                ));
            }
        }
        None
    }
}
