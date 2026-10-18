//! Openings in walls (docs/adr/0341 §1, plan/cad.md `CAD-ARCH-13`, `14`): a door, a window or a bare opening, a
//! drawing object of its own tied to its wall by the wall's persistent id and a distance along the wall's base line.
//! Its jambs' four points (`place`) are its own geometry, written from its wall by the document (docs/adr/0341 §3);
//! what it draws and the gap it leaves in its wall are drawn from them and from its fields. KentOS's opening types
//! (`builtin_opening_types`) carry a door's or window's values; an opening keeps its own copy of them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{EntityBase, Vec2};

/// The widest opening, metres.
pub const MAX_OPENING_WIDTH: f64 = 100.0;
/// A swing's default angle, degrees.
pub const DEFAULT_SWING: f64 = 90.0;

/// What an opening is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningForm {
    /// Kapı.
    #[default]
    Door,
    /// Pencere.
    Window,
    /// Boşluk: a gap in the wall and nothing in it.
    Opening,
}

impl OpeningForm {
    pub const ALL: [OpeningForm; 3] = [OpeningForm::Door, OpeningForm::Window, OpeningForm::Opening];

    pub fn name(self) -> &'static str {
        match self {
            OpeningForm::Door => "door",
            OpeningForm::Window => "window",
            OpeningForm::Opening => "opening",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OpeningForm::Door => "Kapı",
            OpeningForm::Window => "Pencere",
            OpeningForm::Opening => "Boşluk",
        }
    }
}

/// How its leaves move.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningOperation {
    /// Kanatlı: about a hinge.
    #[default]
    Swing,
    /// Sürme: along the wall.
    Slide,
    /// Sabit: it does not open.
    Fixed,
}

impl OpeningOperation {
    pub const ALL: [OpeningOperation; 3] = [
        OpeningOperation::Swing,
        OpeningOperation::Slide,
        OpeningOperation::Fixed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            OpeningOperation::Swing => "swing",
            OpeningOperation::Slide => "slide",
            OpeningOperation::Fixed => "fixed",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OpeningOperation::Swing => "Kanatlı",
            OpeningOperation::Slide => "Sürme",
            OpeningOperation::Fixed => "Sabit",
        }
    }
}

/// Which point of the opening `at` names, along its wall's base line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningAnchor {
    /// Its jamb nearer the wall's start.
    Start,
    #[default]
    Center,
    /// Its jamb nearer the wall's end.
    End,
}

/// An end of the opening along its wall: where its hinge is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningHand {
    /// At its jamb nearer the wall's start.
    #[default]
    Start,
    End,
}

impl OpeningHand {
    pub fn other(self) -> OpeningHand {
        match self {
            OpeningHand::Start => OpeningHand::End,
            OpeningHand::End => OpeningHand::Start,
        }
    }
}

/// A side of its wall's base line: where its leaves open to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningSide {
    #[default]
    Left,
    Right,
}

impl OpeningSide {
    pub fn other(self) -> OpeningSide {
        match self {
            OpeningSide::Left => OpeningSide::Right,
            OpeningSide::Right => OpeningSide::Left,
        }
    }
}

/// Why an opening is not where its wall says (docs/adr/0341 §3): it stays, drawn from its jambs, until it is fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningBreak {
    /// Its wall is gone, or is no wall.
    HostGone,
    /// It runs past its wall's end or round one of its corners.
    OffWall,
    /// It runs into another opening of its wall.
    Overlap,
}

impl OpeningBreak {
    pub fn name(self) -> &'static str {
        match self {
            OpeningBreak::HostGone => "hostGone",
            OpeningBreak::OffWall => "offWall",
            OpeningBreak::Overlap => "overlap",
        }
    }

    /// What is wrong and how to fix it.
    pub fn words(self) -> &'static str {
        match self {
            OpeningBreak::HostGone => {
                "Açıklığın duvarı yok. Açıklığı bir duvara taşıyın (Açıklığı taşı) ya da silin."
            }
            OpeningBreak::OffWall => {
                "Açıklık duvarının ucundan ya da köşesinden taşıyor. Açıklığı duvar boyunca kaydırın ya da daraltın."
            }
            OpeningBreak::Overlap => {
                "Açıklık aynı duvardaki başka bir açıklığa biniyor. Birini kaydırın ya da daraltın."
            }
        }
    }
}

/// An opening's own fields: what a command writes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningFields {
    /// Its wall's persistent id.
    pub host: String,
    /// The distance along its wall's base line, from its start, to its `anchor`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: f64,
    #[serde(default, skip_serializing_if = "is_center")]
    #[cfg_attr(feature = "ts", ts(as = "Option<OpeningAnchor>", optional))]
    pub anchor: OpeningAnchor,
    /// The rough opening in the wall, jamb to jamb, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[serde(default)]
    pub form: OpeningForm,
    /// Its leaves, 1 or 2. Absent: 1.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u8>", optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 2)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub leaves: u8,
    #[serde(default, skip_serializing_if = "is_swing")]
    #[cfg_attr(feature = "ts", ts(as = "Option<OpeningOperation>", optional))]
    pub operation: OpeningOperation,
    /// Its hinge's end. Absent: its start.
    #[serde(default, skip_serializing_if = "is_start")]
    #[cfg_attr(feature = "ts", ts(as = "Option<OpeningHand>", optional))]
    pub hand: OpeningHand,
    /// The side it opens to. Absent: the path's left.
    #[serde(default, skip_serializing_if = "is_left")]
    #[cfg_attr(feature = "ts", ts(as = "Option<OpeningSide>", optional))]
    pub side: OpeningSide,
    /// How far a leaf swings open, degrees, above 0, at most 180. Absent: 90.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub swing: Option<f64>,
    /// The swing's arc is drawn. Absent: it is.
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub show_swing: bool,
    /// Its frame's width at each jamb, along the wall, metres. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub frame: Option<f64>,
    /// Its sill's height above the floor, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub sill: Option<f64>,
    /// Its head's height above the floor, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub head: Option<f64>,
    /// The opening type it was made with, by its id (`builtin_opening_types`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<String>,
    /// Its mark (K1, P3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    /// Its jambs: the start jamb's points on its wall's left and right face, then the end jamb's; written from its
    /// wall by the document.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub place: Vec<Vec2>,
    /// Why it is not where its wall says; absent: it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<OpeningBreak>,
}

/// An opening in a wall (docs/adr/0341).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub opening: OpeningFields,
}

fn one() -> u8 {
    1
}

fn is_one(v: &u8) -> bool {
    *v == 1
}

fn yes() -> bool {
    true
}

fn is_yes(v: &bool) -> bool {
    *v
}

fn is_center(a: &OpeningAnchor) -> bool {
    *a == OpeningAnchor::Center
}

fn is_swing(o: &OpeningOperation) -> bool {
    *o == OpeningOperation::Swing
}

fn is_start(h: &OpeningHand) -> bool {
    *h == OpeningHand::Start
}

fn is_left(s: &OpeningSide) -> bool {
    *s == OpeningSide::Left
}

impl OpeningFields {
    /// Its interval along its wall's base line, from the anchor and the width.
    pub fn span(&self) -> (f64, f64) {
        let w = self.width;
        match self.anchor {
            OpeningAnchor::Start => (self.at, self.at + w),
            OpeningAnchor::Center => (self.at - w / 2.0, self.at + w / 2.0),
            OpeningAnchor::End => (self.at - w, self.at),
        }
    }

    /// The swing's angle, degrees.
    pub fn swing_angle(&self) -> f64 {
        self.swing.unwrap_or(DEFAULT_SWING)
    }

    /// What is wrong with the opening, said so a reader can fix it: the field and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        if self.host.trim().is_empty() {
            return Some(("host".into(), "Açıklığın duvarı belirtilmeli.".into()));
        }
        if !self.at.is_finite() {
            return Some(("at".into(), "Açıklığın konumu sonlu olmalı.".into()));
        }
        if !(self.width.is_finite() && self.width > 0.0 && self.width <= MAX_OPENING_WIDTH) {
            return Some((
                "width".into(),
                format!(
                    "Açıklığın genişliği {}; sıfırdan büyük, en çok {MAX_OPENING_WIDTH} m olmalı.",
                    self.width
                ),
            ));
        }
        if !(1..=2).contains(&self.leaves) {
            return Some((
                "leaves".into(),
                format!("Açıklığın kanat sayısı {}; 1 ya da 2 olmalı.", self.leaves),
            ));
        }
        if let Some(s) = self.swing
            && !(s.is_finite() && s > 0.0 && s <= 180.0)
        {
            return Some((
                "swing".into(),
                format!("Kanadın açılma açısı {s}°; sıfırdan büyük, en çok 180° olmalı."),
            ));
        }
        if let Some(f) = self.frame
            && !(f.is_finite() && f >= 0.0 && 2.0 * f < self.width)
        {
            return Some((
                "frame".into(),
                format!(
                    "Kasa genişliği {f} m; sıfır ya da daha büyük, açıklığın yarısından ({} m) küçük olmalı.",
                    self.width / 2.0
                ),
            ));
        }
        for (name, v) in [("sill", self.sill), ("head", self.head)] {
            if let Some(h) = v
                && !(h.is_finite() && h.abs() <= 1000.0)
            {
                return Some((
                    name.into(),
                    format!("Açıklığın {} yüksekliği sonlu olmalı.", if name == "sill" { "denizlik" } else { "lento" }),
                ));
            }
        }
        if let (Some(s), Some(h)) = (self.sill, self.head)
            && h <= s
        {
            return Some((
                "head".into(),
                format!("Lento ({h} m) denizlikten ({s} m) yüksek olmalı."),
            ));
        }
        if self.number.as_deref().is_some_and(|n| n.trim().is_empty()) {
            return Some((
                "number".into(),
                "Açıklığın numarası boş olamaz; numarayı kaldırın ya da bir numara yazın.".into(),
            ));
        }
        if self.place.len() != 4 {
            return Some((
                "place".into(),
                format!("Açıklığın pervazları dört noktadır; {} verildi.", self.place.len()),
            ));
        }
        if let Some(i) = self
            .place
            .iter()
            .position(|p| !(p.x.is_finite() && p.y.is_finite()))
        {
            return Some((
                format!("place[{i}]"),
                format!("Pervazın {}. noktasının koordinatları sonlu olmalı.", i + 1),
            ));
        }
        None
    }
}

/// A door's, window's or bare opening's values (docs/adr/0341 §1): KentOS's own types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningTypeDef {
    pub id: String,
    pub name: String,
    pub form: OpeningForm,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub leaves: u8,
    pub operation: OpeningOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub swing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub frame: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub sill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub head: Option<f64>,
}

/// KentOS's own opening types, metres: three doors, a sliding door, two windows and a bare opening.
pub fn builtin_opening_types() -> Vec<OpeningTypeDef> {
    use OpeningForm::{Door, Opening, Window};
    use OpeningOperation::{Fixed, Slide, Swing};
    let t = |id: &str,
             name: &str,
             form: OpeningForm,
             width: f64,
             leaves: u8,
             operation: OpeningOperation,
             frame: Option<f64>,
             sill: Option<f64>,
             head: Option<f64>| OpeningTypeDef {
        id: id.into(),
        name: name.into(),
        form,
        width,
        leaves,
        operation,
        swing: None,
        frame,
        sill,
        head,
    };
    vec![
        t("kentos-kapi-90", "Kapı 90", Door, 0.9, 1, Swing, Some(0.05), None, Some(2.1)),
        t("kentos-kapi-80", "Kapı 80", Door, 0.8, 1, Swing, Some(0.05), None, Some(2.1)),
        t(
            "kentos-iki-kanatli-kapi-140",
            "İki kanatlı kapı 140",
            Door,
            1.4,
            2,
            Swing,
            Some(0.05),
            None,
            Some(2.1),
        ),
        t(
            "kentos-surme-kapi-160",
            "Sürme kapı 160",
            Door,
            1.6,
            2,
            Slide,
            Some(0.05),
            None,
            Some(2.1),
        ),
        t(
            "kentos-pencere-120",
            "Pencere 120",
            Window,
            1.2,
            2,
            Swing,
            Some(0.06),
            Some(0.9),
            Some(2.2),
        ),
        t(
            "kentos-sabit-pencere-100",
            "Sabit pencere 100",
            Window,
            1.0,
            1,
            Fixed,
            Some(0.06),
            Some(0.9),
            Some(2.2),
        ),
        t("kentos-bosluk-100", "Boşluk 100", Opening, 1.0, 1, Fixed, None, None, Some(2.1)),
    ]
}

/// KentOS's opening type of this id or name.
pub fn opening_type(name_or_id: &str) -> Option<OpeningTypeDef> {
    let key = name_or_id.trim();
    builtin_opening_types()
        .into_iter()
        .find(|t| t.id == key || t.name == key)
}

impl OpeningTypeDef {
    /// The fields this type gives an opening, over `f`: its form, width, leaves, operation, swing, frame, sill, head
    /// and its id as the opening's style.
    pub fn apply(&self, f: &mut OpeningFields) {
        f.form = self.form;
        f.width = self.width;
        f.leaves = self.leaves;
        f.operation = self.operation;
        f.swing = self.swing;
        f.frame = self.frame;
        f.sill = self.sill;
        f.head = self.head;
        f.style = Some(self.id.clone());
    }
}
