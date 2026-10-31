//! Openings in walls (docs/adr/0341 §1, plan/cad.md `CAD-ARCH-13`, `14`): a door, a window or a bare opening, a
//! drawing object of its own tied to its wall by the wall's persistent id and a distance along the wall's base line.
//! A niche and a flue (docs/adr/0719) are openings that do not go through: a niche is cut from one face `depth` into
//! the wall, a flue is a cavity inside it, `inset` from one face (absent: centred) and `depth` across.
//! Its jambs' four points (`place`) are its own geometry, written from its wall by the document (docs/adr/0341 §3);
//! what it draws and the gap it leaves in its wall are drawn from them and from its fields. KentOS's opening types
//! (`builtin_opening_types`) carry a door's or window's values; an opening keeps its own copy of them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{EntityBase, Vec2};

/// The widest opening, metres.
pub const MAX_OPENING_WIDTH: f64 = 100.0;
/// The deepest niche or flue, metres: no wall is thicker.
pub const MAX_RECESS_DEPTH: f64 = 10.0;
/// The plan's cut height above a level's floor, metres (docs/adr/0719 §2): KentOS's default, the stair tool's cut too.
/// A bare opening, a niche or a flue crossing it is drawn cut; one wholly above or below it shows dashed.
pub const PLAN_CUT_HEIGHT: f64 = 1.1;
/// A swing's default angle, degrees.
pub const DEFAULT_SWING: f64 = 90.0;
/// How far plan draws a folding door's panels open when it gives no angle, degrees (docs/adr/0777 §3).
pub const FOLD_DRAWN: f64 = 30.0;

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
    /// Niş: a recess cut from one face of the wall, `depth` into it (docs/adr/0719).
    Niche,
    /// Baca: a cavity inside the wall, clear of both faces (docs/adr/0719).
    Flue,
}

impl OpeningForm {
    pub const ALL: [OpeningForm; 5] = [
        OpeningForm::Door,
        OpeningForm::Window,
        OpeningForm::Opening,
        OpeningForm::Niche,
        OpeningForm::Flue,
    ];

    pub fn name(self) -> &'static str {
        match self {
            OpeningForm::Door => "door",
            OpeningForm::Window => "window",
            OpeningForm::Opening => "opening",
            OpeningForm::Niche => "niche",
            OpeningForm::Flue => "flue",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OpeningForm::Door => "Kapı",
            OpeningForm::Window => "Pencere",
            OpeningForm::Opening => "Boşluk",
            OpeningForm::Niche => "Niş",
            OpeningForm::Flue => "Baca",
        }
    }

    /// A niche or a flue: it has a depth and does not go through its wall (docs/adr/0719).
    pub fn is_recess(self) -> bool {
        matches!(self, OpeningForm::Niche | OpeningForm::Flue)
    }

    /// Drawn cut in plan only where it crosses the plan's cut height (docs/adr/0719 §2): a bare opening, a niche, a
    /// flue. A door and a window are drawn cut whatever their heights.
    pub fn cut_by_height(self) -> bool {
        matches!(self, OpeningForm::Opening | OpeningForm::Niche | OpeningForm::Flue)
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
    /// Döner (docs/adr/0777 §1): 3 or 4 wings turning about the middle of a drum (`build.drum`).
    Revolving,
    /// Cebe giren (docs/adr/0777 §2): its leaves slide into a pocket inside the wall, beside the jamb at its `hand`
    /// (both jambs for two leaves).
    Pocket,
    /// Katlanır (docs/adr/0777 §3): 2 to 6 panels folding to the jamb at its `hand` (both with `build.bothSides`).
    Folding,
}

impl OpeningOperation {
    pub const ALL: [OpeningOperation; 6] = [
        OpeningOperation::Swing,
        OpeningOperation::Slide,
        OpeningOperation::Fixed,
        OpeningOperation::Revolving,
        OpeningOperation::Pocket,
        OpeningOperation::Folding,
    ];

    pub fn name(self) -> &'static str {
        match self {
            OpeningOperation::Swing => "swing",
            OpeningOperation::Slide => "slide",
            OpeningOperation::Fixed => "fixed",
            OpeningOperation::Revolving => "revolving",
            OpeningOperation::Pocket => "pocket",
            OpeningOperation::Folding => "folding",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OpeningOperation::Swing => "Kanatlı",
            OpeningOperation::Slide => "Sürme",
            OpeningOperation::Fixed => "Sabit",
            OpeningOperation::Revolving => "Döner",
            OpeningOperation::Pocket => "Cebe giren",
            OpeningOperation::Folding => "Katlanır",
        }
    }

    /// One docs/adr/0777 added (`.kcad` schema 144): a door only.
    pub fn is_special(self) -> bool {
        matches!(
            self,
            OpeningOperation::Revolving | OpeningOperation::Pocket | OpeningOperation::Folding
        )
    }

    /// How many leaves (a revolving door's wings, a folding door's panels) it may have, least and most.
    pub fn leaf_range(self) -> (u8, u8) {
        match self {
            OpeningOperation::Revolving => (3, 4),
            OpeningOperation::Folding => (2, 6),
            // Three and four leaves on two or three tracks (docs/adr/0792 §1).
            OpeningOperation::Slide => (1, 4),
            _ => (1, 2),
        }
    }

    /// What its leaves are called: kanat, a revolving door's wing (kanat), a folding door's panel.
    pub fn leaf_word(self) -> &'static str {
        match self {
            OpeningOperation::Folding => "panel",
            _ => "kanat",
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
    /// A niche or flue deeper than its wall is thick (docs/adr/0719 §3).
    TooDeep,
}

impl OpeningBreak {
    pub fn name(self) -> &'static str {
        match self {
            OpeningBreak::HostGone => "hostGone",
            OpeningBreak::OffWall => "offWall",
            OpeningBreak::Overlap => "overlap",
            OpeningBreak::TooDeep => "tooDeep",
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
            OpeningBreak::TooDeep => {
                "Niş ya da baca duvarın kalınlığına sığmıyor. Derinliğini azaltın ya da duvarı kalınlaştırın."
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
    /// Its leaves, 1 or 2; a revolving door's wings 3 or 4, a folding door's panels 2 to 6 (docs/adr/0777). Absent: 1.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u8>", optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 6)))]
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
    /// A niche's depth from its face, a flue's size across its wall, metres (docs/adr/0719); none for one that goes
    /// through. Its face is `side`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
    /// A flue's distance from its face (`side`) to its cavity, metres; absent: centred in its wall (docs/adr/0719).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub inset: Option<f64>,
    /// The opening type it was made with, by its id (`builtin_opening_types`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<String>,
    /// Its mark (K1, P3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    /// The version of the project type `style` names it last took (docs/adr/0772 §1): it follows that type. Absent: it
    /// follows none (a KentOS type's values copied once, or none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 1), extend("x-unit" = "version")))]
    pub type_version: Option<u32>,
    /// The fields of its type it keeps its own value of (örnek istisnası), in the order the fields are listed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::OpeningTypeField>>", optional))]
    pub overrides: Vec<crate::OpeningTypeField>,
    /// Its construction (docs/adr/0772 §2, §3): sizes, leaves, frame, threshold, casing, sill board, panes, use zone.
    /// Absent: drawn and measured as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub build: Option<crate::OpeningBuild>,
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

/// Whether `e` says what only schema 137 can (docs/adr/0772): an opening following a project type, keeping
/// exceptions or with a construction, or a schedule reading the new widths.
pub fn needs_opening_types(e: &crate::Entity) -> bool {
    match e {
        crate::Entity::Opening(o) => {
            let f = &o.opening;
            f.type_version.is_some() || !f.overrides.is_empty() || f.build.is_some()
        }
        crate::Entity::Table(t) => matches!(&t.source,
            Some(crate::TableSource::Schedule { columns, .. })
                if columns.iter().any(|c| c.field.is_sized())),
        _ => false,
    }
}

/// Whether `e` says what only schema 144 can (docs/adr/0777): a revolving, pocket or folding door, a construction's
/// drum, overlap, track, both-sides stack or lip, or a schedule reading an opening's operation, leaves, drum or pocket.
pub fn needs_special_doors(e: &crate::Entity) -> bool {
    match e {
        crate::Entity::Opening(o) => {
            let f = &o.opening;
            f.operation.is_special()
                || f.build.as_ref().is_some_and(crate::OpeningBuild::is_special)
                || f.overrides.iter().any(|o| o.is_special())
        }
        crate::Entity::Table(t) => matches!(&t.source,
            Some(crate::TableSource::Schedule { columns, .. })
                if columns.iter().any(|c| c.field.is_special())),
        _ => false,
    }
}

/// Whether an opening's fields are a three- or four-leaf sliding door or window, or carry its tracks (docs/adr/0792):
/// what only schema 157 says.
pub fn is_multi_slide(f: &OpeningFields) -> bool {
    (f.operation == OpeningOperation::Slide && f.leaves > 2)
        || f.build.as_ref().is_some_and(crate::OpeningBuild::is_multi_slide)
        || f.overrides.iter().any(|o| o.is_multi_slide())
}

/// Whether `e` says what only schema 157 can (docs/adr/0792): a three- or four-leaf sliding door or window, its tracks,
/// or a schedule reading an opening's tracks or leaf layout.
pub fn needs_multi_slide(e: &crate::Entity) -> bool {
    match e {
        crate::Entity::Opening(o) => is_multi_slide(&o.opening),
        crate::Entity::Table(t) => matches!(&t.source,
            Some(crate::TableSource::Schedule { columns, .. })
                if columns.iter().any(|c| c.field.is_multi_slide())),
        _ => false,
    }
}

/// Whether `e` says what only schema 119 can (docs/adr/0719): a niche or flue, a depth or inset, the break
/// `tooDeep`, or a schedule whose filter names a niche or flue.
pub fn needs_recesses(e: &crate::Entity) -> bool {
    match e {
        crate::Entity::Opening(o) => {
            let f = &o.opening;
            f.form.is_recess()
                || f.depth.is_some()
                || f.inset.is_some()
                || f.broken == Some(OpeningBreak::TooDeep)
        }
        crate::Entity::Table(t) => matches!(&t.source,
            Some(crate::TableSource::Schedule { filter: Some(f), .. })
                if f.forms.as_ref().is_some_and(|fs| fs.iter().any(|x| x.is_recess()))),
        _ => false,
    }
}

impl OpeningFields {
    /// What is wrong with its depth and inset (docs/adr/0719 §1): a niche and a flue need a depth, a flue's inset is
    /// above 0, a niche has none, one that goes through has neither.
    fn recess_problem(&self) -> Option<(String, String)> {
        let label = self.form.label();
        if !self.form.is_recess() {
            if self.depth.is_some() {
                return Some((
                    "depth".into(),
                    format!("Derinlik yalnız niş ve bacanındır; {} duvarı geçer. Derinliği kaldırın ya da biçimi Niş yapın.", label.to_lowercase()),
                ));
            }
            if self.inset.is_some() {
                return Some((
                    "inset".into(),
                    "Yüzden uzaklık yalnız bacanındır. Kaldırın ya da biçimi Baca yapın.".into(),
                ));
            }
            return None;
        }
        let Some(d) = self.depth else {
            return Some((
                "depth".into(),
                format!("{label} derinliği belirtilmeli: duvarın içine ne kadar girdiği (ör. 0,10 m)."),
            ));
        };
        if !(d.is_finite() && d > 0.0 && d <= MAX_RECESS_DEPTH) {
            return Some((
                "depth".into(),
                format!("{label} derinliği {d}; sıfırdan büyük, en çok {MAX_RECESS_DEPTH} m olmalı."),
            ));
        }
        match (self.form, self.inset) {
            (OpeningForm::Niche, Some(_)) => Some((
                "inset".into(),
                "Niş yüzünden başlar; yüzden uzaklığı yoktur. Duvarın içindeki boşluk için Baca kullanın.".into(),
            )),
            (OpeningForm::Flue, Some(i)) if !(i.is_finite() && i > 0.0 && i <= MAX_RECESS_DEPTH) => Some((
                "inset".into(),
                format!("Bacanın yüzden uzaklığı {i}; sıfırdan büyük olmalı. Yüze açık oyuk için Niş kullanın."),
            )),
            _ => None,
        }
    }

    /// Where a niche or flue lies across a wall `thickness` thick, from its face (`side`): its near and far distance
    /// from that face (a flue without an inset centred); none for one that goes through. It may run past the other
    /// face: [`OpeningFields::fits`] says.
    pub fn recess_in(&self, thickness: f64) -> Option<(f64, f64)> {
        if !self.form.is_recess() {
            return None;
        }
        let d = self.depth?;
        let near = match self.form {
            OpeningForm::Flue => self.inset.unwrap_or((thickness - d) / 2.0),
            _ => 0.0,
        };
        Some((near, near + d))
    }

    /// Whether a niche or flue stays inside a wall `thickness` thick (a niche short of its other face, a flue clear of
    /// both); one that goes through always does.
    pub fn fits(&self, thickness: f64) -> bool {
        let tol = 1e-9 * thickness.abs().max(1.0);
        match self.recess_in(thickness) {
            None => true,
            Some((near, far)) => near > -tol && far < thickness - tol && (self.form != OpeningForm::Flue || near > tol),
        }
    }

    /// Whether plan draws it cut (docs/adr/0719 §2): a door or window always; a bare opening, a niche or a flue only
    /// when it crosses the plan's cut height `cut` above its floor (its sill absent: the floor; its head absent: full
    /// height). One not cut leaves its wall whole and shows dashed.
    pub fn cut_in_plan(&self, cut: f64) -> bool {
        !self.form.cut_by_height()
            || (self.sill.unwrap_or(0.0) < cut && self.head.is_none_or(|h| h > cut))
    }

    /// Its heights above the floor: its sill (none: the floor) to its head (none: full height).
    pub fn heights(&self) -> (f64, f64) {
        (self.sill.unwrap_or(0.0), self.head.unwrap_or(f64::INFINITY))
    }

    /// Whether `self` and `other`, on intervals of one wall that run into each other, clash (docs/adr/0719 §3): their
    /// heights meet, or plan (cut at `cut`) would cut both. A radiator niche under a window, a service hole over a
    /// door do not.
    pub fn clashes_with(&self, other: &OpeningFields, cut: f64) -> bool {
        let ((a0, a1), (b0, b1)) = (self.heights(), other.heights());
        let tol = 1e-9;
        (a0 < b1 - tol && b0 < a1 - tol) || (self.cut_in_plan(cut) && other.cut_in_plan(cut))
    }

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

    /// The angle plan draws its leaves open at, degrees: its swing; a folding door's panels [`FOLD_DRAWN`] when it
    /// gives none (docs/adr/0777 §3).
    pub fn drawn_swing(&self) -> f64 {
        match (self.operation, self.swing) {
            (OpeningOperation::Folding, None) => FOLD_DRAWN,
            _ => self.swing_angle(),
        }
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
        if self.operation.is_special() && self.form != OpeningForm::Door {
            return Some((
                "operation".into(),
                format!(
                    "{} açılış yalnız kapınındır; {} için Kanatlı, Sürme ya da Sabit seçin.",
                    self.operation.label(),
                    self.form.label().to_lowercase()
                ),
            ));
        }
        let (least, most) = self.operation.leaf_range();
        if !(least..=most).contains(&self.leaves) {
            let word = self.operation.leaf_word();
            return Some((
                "leaves".into(),
                if self.operation == OpeningOperation::Slide {
                    format!("Sürme açıklığın kanat sayısı {}; 1 ile 4 arasında olmalı.", self.leaves)
                } else if !self.operation.is_special() {
                    format!("Açıklığın kanat sayısı {}; 1 ya da 2 olmalı.", self.leaves)
                } else if least + 1 == most {
                    format!(
                        "{} kapının {word} sayısı {}; {least} ya da {most} olmalı.",
                        self.operation.label(),
                        self.leaves
                    )
                } else {
                    format!(
                        "{} kapının {word} sayısı {}; {least} ile {most} arasında olmalı.",
                        self.operation.label(),
                        self.leaves
                    )
                },
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
        if let Some(why) = self.recess_problem() {
            return Some(why);
        }
        if self.build.as_ref().is_some_and(crate::OpeningBuild::is_empty) {
            return Some(("build".into(), "Boş yapı yazılmaz; yapının en az bir alanı olur.".into()));
        }
        if let Some(why) = self
            .build
            .as_ref()
            .and_then(|b| b.problem(self.width, self.frame.unwrap_or(0.0)))
        {
            return Some(why);
        }
        if let Some(why) = crate::special_door_problem(self) {
            return Some(why);
        }
        if self.type_version == Some(0) || (self.type_version.is_some() && self.style.is_none()) {
            return Some((
                "typeVersion".into(),
                "Tipin sürümü 1 ya da daha büyük olmalı ve tipin kimliği (style) verilmeli.".into(),
            ));
        }
        if !self.overrides.is_empty() && self.type_version.is_none() {
            return Some((
                "overrides".into(),
                "Örnek istisnası yalnız bir tipi izleyen açıklığındır.".into(),
            ));
        }
        if self.overrides.windows(2).any(|w| w[0] >= w[1]) {
            return Some((
                "overrides".into(),
                "Örnek istisnaları alanların sırasıyla ve birer kez yazılır.".into(),
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
    /// A niche's or flue's depth (docs/adr/0719).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
}

/// KentOS's own opening types, metres: three doors, a sliding door, two windows, a bare opening and a service hole,
/// two niches and a flue.
pub fn builtin_opening_types() -> Vec<OpeningTypeDef> {
    use OpeningForm::{Door, Flue, Niche, Opening, Window};
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
        depth: None,
    };
    let recess = |id: &str, name: &str, form: OpeningForm, width: f64, depth: f64, sill: Option<f64>, head: Option<f64>| {
        OpeningTypeDef {
            depth: Some(depth),
            ..t(id, name, form, width, 1, Fixed, None, sill, head)
        }
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
        // A service hole high in the wall: above the plan's cut, dashed (docs/adr/0719 §2).
        t("kentos-tesisat-20", "Tesisat boşluğu 20", Opening, 0.2, 1, Fixed, None, Some(2.3), Some(2.5)),
        recess("kentos-nis-60", "Niş 60", Niche, 0.6, 0.1, Some(0.9), Some(1.5)),
        // Under a window: below the plan's cut, dashed.
        recess("kentos-kalorifer-nisi-100", "Kalorifer nişi 100", Niche, 1.0, 0.1, Some(0.1), Some(0.8)),
        // Full height: cut, centred in its wall.
        recess("kentos-baca-20", "Baca 20", Flue, 0.2, 0.2, None, None),
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
    /// The fields this type gives an opening, over `f`: its form, width, leaves, operation, swing, frame, sill, head,
    /// depth and its id as the opening's style. An inset stays only on a flue.
    pub fn apply(&self, f: &mut OpeningFields) {
        f.form = self.form;
        f.width = self.width;
        f.leaves = self.leaves;
        f.operation = self.operation;
        f.swing = self.swing;
        f.frame = self.frame;
        f.sill = self.sill;
        f.head = self.head;
        f.depth = self.depth;
        if self.form != OpeningForm::Flue {
            f.inset = None;
        }
        f.style = Some(self.id.clone());
    }
}
