//! The architectural objects' source model (docs/adr/0339, plan/cad.md §14.1 `CAD-ARCH-01` … `05`, `12`): a wall is
//! its own kind, the path it was drawn along (its base line), its width, where the body lies across the path and the
//! components it is built of; its two faces, ends, component lines and hatches are drawn from that, never stored. The
//! level, phase, construction status and discipline every architectural object carries are typed fields
//! ([`ArchInfo`]); the project's levels, wall styles and how walls are shown at a scale are project settings.
//!
//! Lengths are metres in the drawing's plane (`length`), heights metres (`elevation`), whatever unit the project shows
//! (docs/adr/0165 §2, 0256). A wall's left and right are those of its path's travel.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{EntityBase, Vec2};

/// The most vertices one wall's path may have.
pub const MAX_WALL_POINTS: usize = 10_000;
/// The most components one wall (or wall style) may have.
pub const MAX_WALL_COMPONENTS: usize = 16;
/// The widest wall, metres: wider is a mistake of unit, not a wall.
pub const MAX_WALL_WIDTH: f64 = 100.0;
/// A new wall's width when neither its input nor its style gives one, metres.
pub const DEFAULT_WALL_WIDTH: f64 = 0.2;
/// Up to 1:30 the walls are drawn in detail, up to 1:75 for construction, beyond schematically (`ArchDisplay`).
pub const DETAILED_UP_TO: f64 = 30.0;
pub const STANDARD_UP_TO: f64 = 75.0;

/// Where the wall's body lies across its path (AutoCAD Architecture's justification): `Left`, the path is the wall's
/// left face and the body to its right; `Center`, the path is its middle; `Right`, the path is its right face.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WallJustify {
    Left,
    #[default]
    Center,
    Right,
}

impl WallJustify {
    pub const ALL: [WallJustify; 3] = [WallJustify::Left, WallJustify::Center, WallJustify::Right];

    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            WallJustify::Left => "Sol",
            WallJustify::Center => "Orta",
            WallJustify::Right => "Sağ",
        }
    }

    /// The left face's distance from the justification line, positive to the left, for a wall `width` wide.
    pub fn left_face(self, width: f64) -> f64 {
        match self {
            WallJustify::Left => 0.0,
            WallJustify::Center => width / 2.0,
            WallJustify::Right => width,
        }
    }
}

/// Whether an open wall's ends are closed by a line across it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WallCaps {
    #[default]
    Closed,
    Open,
}

impl WallCaps {
    pub fn label(self) -> &'static str {
        match self {
            WallCaps::Closed => "Kapalı",
            WallCaps::Open => "Açık",
        }
    }
}

/// How much of a wall is drawn (`CAD-ARCH-12`), the least first: `Schematic` its outline and its style's fill,
/// `Standard` the components shown at construction scales, `Detailed` every component.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WallDetail {
    Schematic,
    #[default]
    Standard,
    Detailed,
}

impl WallDetail {
    pub const ALL: [WallDetail; 3] = [
        WallDetail::Schematic,
        WallDetail::Standard,
        WallDetail::Detailed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            WallDetail::Schematic => "Şematik",
            WallDetail::Standard => "Uygulama",
            WallDetail::Detailed => "Ayrıntılı",
        }
    }

    /// The detail a drawing at 1:`scale` shows: up to 1:30 detailed, up to 1:75 standard, beyond schematic.
    pub fn at_scale(scale: f64) -> WallDetail {
        if scale <= DETAILED_UP_TO {
            WallDetail::Detailed
        } else if scale <= STANDARD_UP_TO {
            WallDetail::Standard
        } else {
            WallDetail::Schematic
        }
    }
}

/// The lines along a component's sides: its own colour, line weight and line type; absent fields the wall's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ComponentEdge {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Paper mm, 0 to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<crate::layer::LineType>,
}

/// A component's hatch: a pattern of the library or the project by its name (`SOLID` a solid fill), as large on paper
/// as at 1:1 times `scale` at every drawing scale, turned `angle` degrees along the wall's path, in its own colour.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ComponentHatch {
    pub pattern: String,
    /// Times the pattern's paper size, above 0.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
}

/// One layer of a wall's body, from its left face towards its right (`CAD-ARCH-04`): plaster, brick, insulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallComponent {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    /// Its thickness, metres; absent: what the wall's width leaves of the others (one component at most).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub thickness: Option<f64>,
    /// The lines along its sides; absent: the wall's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub edge: Option<ComponentEdge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hatch: Option<ComponentHatch>,
    /// The least detail it is drawn at as its own (its lines and hatch); below, it is part of the body.
    #[serde(default)]
    pub detail: WallDetail,
    /// Its strength where walls meet (docs/adr/0340 §1), 1 to 9, the smaller the stronger: a joining wall's component
    /// passes the host's weaker ones and stops at the first as strong. Absent: 3 (between a core's 1 and a finish's 5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 9), extend("x-unit" = "1")))]
    pub priority: Option<u8>,
}

/// A component's priority when it has none.
pub const DEFAULT_COMPONENT_PRIORITY: u8 = 3;

/// How one end of a wall joins another (docs/adr/0340 §1, `CAD-ARCH-07`): by the rule; not at all (its end line
/// drawn); at a corner mitred; butting the other (it stops at the other's face); running through it (the other butts).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WallJoin {
    #[default]
    Auto,
    None,
    Miter,
    Butt,
    Through,
}

impl WallJoin {
    pub const ALL: [WallJoin; 5] = [
        WallJoin::Auto,
        WallJoin::None,
        WallJoin::Miter,
        WallJoin::Butt,
        WallJoin::Through,
    ];

    pub fn label(self) -> &'static str {
        match self {
            WallJoin::Auto => "Otomatik",
            WallJoin::None => "Yok",
            WallJoin::Miter => "Gönye",
            WallJoin::Butt => "Alın",
            WallJoin::Through => "Geçen",
        }
    }
}

/// A wall's start's and end's join.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallJoins {
    #[serde(default, skip_serializing_if = "is_auto")]
    #[cfg_attr(feature = "ts", ts(as = "Option<WallJoin>", optional))]
    pub start: WallJoin,
    #[serde(default, skip_serializing_if = "is_auto")]
    #[cfg_attr(feature = "ts", ts(as = "Option<WallJoin>", optional))]
    pub end: WallJoin,
}

impl WallJoins {
    pub fn is_auto(&self) -> bool {
        *self == WallJoins::default()
    }
}

fn is_auto(j: &WallJoin) -> bool {
    *j == WallJoin::Auto
}

/// An architectural object's phase (renovation): what is there, what goes, what is new.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArchPhase {
    Existing,
    Demolish,
    New,
    Temporary,
}

impl ArchPhase {
    pub const ALL: [ArchPhase; 4] = [
        ArchPhase::Existing,
        ArchPhase::Demolish,
        ArchPhase::New,
        ArchPhase::Temporary,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ArchPhase::Existing => "Mevcut",
            ArchPhase::Demolish => "Yıkılacak",
            ArchPhase::New => "Yeni",
            ArchPhase::Temporary => "Geçici",
        }
    }
}

/// Where an architectural object is in its construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArchStatus {
    Proposed,
    Approved,
    UnderConstruction,
    Built,
}

impl ArchStatus {
    pub const ALL: [ArchStatus; 4] = [
        ArchStatus::Proposed,
        ArchStatus::Approved,
        ArchStatus::UnderConstruction,
        ArchStatus::Built,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ArchStatus::Proposed => "Öneri",
            ArchStatus::Approved => "Onaylı",
            ArchStatus::UnderConstruction => "Yapımda",
            ArchStatus::Built => "Yapıldı",
        }
    }
}

/// The discipline an architectural object belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArchDiscipline {
    Architecture,
    Structure,
    Mechanical,
    Electrical,
    Plumbing,
    Landscape,
    Interior,
}

impl ArchDiscipline {
    pub const ALL: [ArchDiscipline; 7] = [
        ArchDiscipline::Architecture,
        ArchDiscipline::Structure,
        ArchDiscipline::Mechanical,
        ArchDiscipline::Electrical,
        ArchDiscipline::Plumbing,
        ArchDiscipline::Landscape,
        ArchDiscipline::Interior,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ArchDiscipline::Architecture => "Mimari",
            ArchDiscipline::Structure => "Statik",
            ArchDiscipline::Mechanical => "Mekanik",
            ArchDiscipline::Electrical => "Elektrik",
            ArchDiscipline::Plumbing => "Sıhhi tesisat",
            ArchDiscipline::Landscape => "Peyzaj",
            ArchDiscipline::Interior => "İç mimari",
        }
    }
}

/// What every architectural object carries beside its geometry (`CAD-ARCH-03`): its level (by the project level's
/// id), phase, construction status and discipline; absent fields are not said.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub phase: Option<ArchPhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub status: Option<ArchStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub discipline: Option<ArchDiscipline>,
}

impl ArchInfo {
    pub fn is_empty(&self) -> bool {
        self == &ArchInfo::default()
    }
}

/// A wall (`CAD-ARCH-01`, `02`): its path, the base line it was drawn along, with arcs as a polyline's bulges (one per
/// vertex, as the document keeps a polyline's: an open wall's last is 0); its width; where its body lies across the
/// justification line (the path moved `offset` to its left); the components it is built of, from its left face; its
/// ends; its height above its level; its architectural information. Its faces, ends, component lines and hatches are
/// drawn from these (`geom::wall` of the shared core), at the detail the drawing's scale asks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub wall: WallFields,
}

/// A wall's own fields: what a command writes and an edit's geometry carries (`EntityGeometry::Wall`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallFields {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    /// Its path's last vertex joins its first: a ring of wall.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub closed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[serde(default)]
    pub justify: WallJustify,
    /// The base line offset: the justification line is the path moved this far to its left (negative: right).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: f64,
    /// The project's wall style it was made with, by its id; its components are the style's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<String>,
    /// Its body's layers from its left face; none: one plain body.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<WallComponent>>", optional))]
    pub components: Vec<WallComponent>,
    #[serde(default, skip_serializing_if = "is_closed_caps")]
    #[cfg_attr(feature = "ts", ts(as = "Option<WallCaps>", optional))]
    pub caps: WallCaps,
    /// The body's solid fill when drawn schematically (`#RRGGBB`); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    /// Its height above its level's floor, metres; absent: the level's height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
    /// How its ends join (docs/adr/0340 §1); absent: both by the rule.
    #[serde(default, skip_serializing_if = "WallJoins::is_auto")]
    #[cfg_attr(feature = "ts", ts(as = "Option<WallJoins>", optional))]
    pub joins: WallJoins,
    /// Its components are cleaned up by their priorities where another wall tees into or crosses it; false: the
    /// walls meet at their faces only.
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub cleanup: bool,
}

fn yes() -> bool {
    true
}

fn is_yes(v: &bool) -> bool {
    *v
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_closed_caps(c: &WallCaps) -> bool {
    *c == WallCaps::Closed
}

/// Each component's thickness for a wall `width` wide: the given ones, and the rest for the one without; none when
/// they do not fit (`wall_problem` says why).
pub fn component_thicknesses(width: f64, components: &[WallComponent]) -> Option<Vec<f64>> {
    let fixed: f64 = components
        .iter()
        .filter_map(|c| c.thickness)
        .fold(0.0, |a, b| a + b);
    let rest = components.iter().filter(|c| c.thickness.is_none()).count();
    match rest {
        0 if (fixed - width).abs() <= 1e-9 * width.max(1.0) => Some(
            components
                .iter()
                .map(|c| c.thickness.unwrap_or(0.0))
                .collect(),
        ),
        1 if fixed < width - 1e-9 * width.max(1.0) => Some(
            components
                .iter()
                .map(|c| c.thickness.unwrap_or(width - fixed))
                .collect(),
        ),
        _ => None,
    }
}

/// The width a wall of these components has when one of them takes the rest: none (any width above the others'), or
/// the components' sum when none does.
pub fn fixed_width(components: &[WallComponent]) -> Option<f64> {
    (!components.is_empty() && components.iter().all(|c| c.thickness.is_some())).then(|| {
        components
            .iter()
            .filter_map(|c| c.thickness)
            .fold(0.0, |a, b| a + b)
    })
}

fn colour_holds(c: &Option<String>) -> bool {
    c.as_deref().is_none_or(crate::hatch::is_hex_colour)
}

/// What is wrong with a wall's components for a width (`width` none: a style's, any width), said so a reader can fix
/// it: the field (relative to the components) and the words.
pub fn components_problem(
    components: &[WallComponent],
    width: Option<f64>,
) -> Option<(String, String)> {
    if components.len() > MAX_WALL_COMPONENTS {
        return Some((
            "components".into(),
            format!(
                "Bir duvarın en çok {MAX_WALL_COMPONENTS} bileşeni olur; {} verildi.",
                components.len()
            ),
        ));
    }
    let mut rest = None;
    for (i, c) in components.iter().enumerate() {
        let at = |f: &str| format!("components[{i}].{f}");
        if c.name.trim().is_empty() {
            return Some((at("name"), format!("{}. bileşenin adı boş.", i + 1)));
        }
        match c.thickness {
            Some(t) if !(t.is_finite() && t > 0.0 && t <= MAX_WALL_WIDTH) => {
                return Some((
                    at("thickness"),
                    format!(
                        "“{}” bileşeninin kalınlığı {t}; sıfırdan büyük, en çok {MAX_WALL_WIDTH} m olmalı.",
                        c.name
                    ),
                ));
            }
            None if rest.is_some() => {
                return Some((
                    at("thickness"),
                    format!(
                        "Kalınlığı verilmeyen ikinci bileşen “{}”: genişliğin kalanını yalnız bir bileşen alır.",
                        c.name
                    ),
                ));
            }
            None => rest = Some(i),
            Some(_) => {}
        }
        if let Some(p) = c.priority
            && !(1..=9).contains(&p)
        {
            return Some((
                at("priority"),
                format!(
                    "“{}” bileşeninin önceliği {p}; 1 ile 9 arasında olmalı (küçük olan güçlü).",
                    c.name
                ),
            ));
        }
        if let Some(e) = &c.edge {
            if !colour_holds(&e.color) {
                return Some((
                    at("edge.color"),
                    format!("“{}” bileşeninin çizgi rengi #RRGGBB olmalı.", c.name),
                ));
            }
            if e.line_weight
                .is_some_and(|w| !(w.is_finite() && (0.0..=crate::MAX_LINE_WEIGHT).contains(&w)))
            {
                return Some((
                    at("edge.lineWeight"),
                    format!(
                        "“{}” bileşeninin çizgi kalınlığı 0 ile 100 mm arasında olmalı.",
                        c.name
                    ),
                ));
            }
        }
        if let Some(h) = &c.hatch {
            if h.pattern.trim().is_empty() {
                return Some((
                    at("hatch.pattern"),
                    format!("“{}” bileşeninin tarama deseni boş.", c.name),
                ));
            }
            if !(h.scale.is_finite() && h.scale > 0.0) {
                return Some((
                    at("hatch.scale"),
                    format!(
                        "“{}” bileşeninin tarama ölçeği sıfırdan büyük olmalı.",
                        c.name
                    ),
                ));
            }
            if !h.angle.is_finite() {
                return Some((
                    at("hatch.angle"),
                    format!("“{}” bileşeninin tarama açısı sonlu olmalı.", c.name),
                ));
            }
            if !colour_holds(&h.color) {
                return Some((
                    at("hatch.color"),
                    format!("“{}” bileşeninin tarama rengi #RRGGBB olmalı.", c.name),
                ));
            }
        }
    }
    if let Some(w) = width
        && !components.is_empty()
        && component_thicknesses(w, components).is_none()
    {
        let fixed: f64 = components
            .iter()
            .filter_map(|c| c.thickness)
            .fold(0.0, |a, b| a + b);
        let words = match rest {
            Some(i) => format!(
                "Duvarın genişliği {w} m, “{}” dışındaki bileşenler {fixed} m; kalan bileşene yer kalmıyor. Duvarı genişletin ya da bileşenleri inceltin.",
                components[i].name
            ),
            None => format!(
                "Duvarın genişliği {w} m, bileşenlerin toplamı {fixed} m: kalınlığı verilmeyen bileşen olmadığından genişlik toplama eşit olmalı."
            ),
        };
        return Some(("width".into(), words));
    }
    None
}

impl WallFields {
    /// What is wrong with the wall, said so a reader can fix it: the field and the words; none: it is a wall.
    pub fn problem(&self) -> Option<(String, String)> {
        let n = self.pts.len();
        if n < 2 {
            return Some((
                "pts".into(),
                format!("Duvarın yolu en az iki noktadır; {n} verildi."),
            ));
        }
        if n > MAX_WALL_POINTS {
            return Some((
                "pts".into(),
                format!("Duvarın yolu en çok {MAX_WALL_POINTS} noktadır; {n} verildi."),
            ));
        }
        if let Some(i) = self
            .pts
            .iter()
            .position(|p| !(p.x.is_finite() && p.y.is_finite()))
        {
            return Some((
                format!("pts[{i}]"),
                format!("{}. noktanın koordinatları sonlu olmalı.", i + 1),
            ));
        }
        if let Some(b) = &self.bulges {
            if b.len() != n {
                return Some((
                    "bulges".into(),
                    format!(
                        "Her noktanın bir bükümü olur: {n} nokta, {} büküm.",
                        b.len()
                    ),
                ));
            }
            if let Some(i) = b.iter().position(|x| !x.is_finite()) {
                return Some((
                    format!("bulges[{i}]"),
                    format!("{}. büküm sonlu olmalı.", i + 1),
                ));
            }
            if !self.closed && b[n - 1] != 0.0 {
                return Some((
                    format!("bulges[{}]", n - 1),
                    "Açık duvarın son noktasından kenar çıkmaz: son büküm 0 olmalı.".into(),
                ));
            }
        }
        let edges = if self.closed { n } else { n - 1 };
        for i in 0..edges {
            let (a, b) = (self.pts[i], self.pts[(i + 1) % n]);
            if a.x == b.x && a.y == b.y {
                return Some((
                    format!("pts[{}]", (i + 1) % n),
                    format!(
                        "{}. ve {}. nokta aynı yerde: duvarın kenarı uzunluksuz olur. Yinelenen noktayı çıkarın.",
                        i + 1,
                        (i + 1) % n + 1
                    ),
                ));
            }
        }
        let curved = self
            .bulges
            .as_ref()
            .is_some_and(|b| b.iter().any(|x| *x != 0.0));
        if self.closed && n < 3 && !curved {
            return Some((
                "pts".into(),
                "Kapalı duvar düz kenarlarla en az üç noktadır.".into(),
            ));
        }
        if !(self.width.is_finite() && self.width > 0.0 && self.width <= MAX_WALL_WIDTH) {
            return Some((
                "width".into(),
                format!(
                    "Duvarın genişliği {}; sıfırdan büyük, en çok {MAX_WALL_WIDTH} m olmalı.",
                    self.width
                ),
            ));
        }
        if !(self.offset.is_finite() && self.offset.abs() <= 1000.0) {
            return Some((
                "offset".into(),
                "Duvarın ötelemesi sonlu olmalı, en çok 1000 m.".into(),
            ));
        }
        if let Some(problem) = components_problem(&self.components, Some(self.width)) {
            return Some(problem);
        }
        if !colour_holds(&self.fill) {
            return Some(("fill".into(), "Duvarın dolgu rengi #RRGGBB olmalı.".into()));
        }
        if let Some(h) = self.height
            && !(h.is_finite() && h > 0.0)
        {
            return Some((
                "height".into(),
                format!("Duvarın yüksekliği {h}; sıfırdan büyük olmalı."),
            ));
        }
        if self
            .arch
            .level
            .as_deref()
            .is_some_and(|l| l.trim().is_empty())
        {
            return Some((
                "arch.level".into(),
                "Duvarın katı boş olamaz; katı kaldırın ya da bir kat seçin.".into(),
            ));
        }
        None
    }

    /// Each component's thickness, the rest given to the one without; none for a wall without components.
    pub fn thicknesses(&self) -> Option<Vec<f64>> {
        if self.components.is_empty() {
            return None;
        }
        component_thicknesses(self.width, &self.components)
    }
}

/// A named wall style (`CAD-ARCH-04`): the components a wall is built of, its ends, its schematic fill and the width a
/// new wall takes when one component takes the rest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallStyleDef {
    /// One of its kind among the project's wall styles; KentOS's own are named (`kentos-…`), the user's UUIDv7.
    pub id: String,
    pub name: String,
    pub components: Vec<WallComponent>,
    #[serde(default, skip_serializing_if = "is_closed_caps")]
    #[cfg_attr(feature = "ts", ts(as = "Option<WallCaps>", optional))]
    pub caps: WallCaps,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    /// A new wall's width, metres, when one component takes the rest; the components' sum when none does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
}

impl WallStyleDef {
    /// The width a new wall of this style takes: its components' sum when none takes the rest, else its own, else the
    /// others' sum and 10 cm for the rest.
    pub fn new_width(&self) -> f64 {
        match fixed_width(&self.components) {
            Some(w) => w,
            None if self.components.is_empty() => self.width.unwrap_or(DEFAULT_WALL_WIDTH),
            None => self.width.unwrap_or_else(|| {
                self.components
                    .iter()
                    .filter_map(|c| c.thickness)
                    .fold(0.0, |a, b| a + b)
                    + 0.1
            }),
        }
    }

    /// What is wrong with it: the field and the words.
    pub fn problem(&self) -> Option<(String, String)> {
        let name = &self.name;
        if let Some((field, words)) = components_problem(&self.components, None) {
            return Some((field, format!("“{name}” duvar stili: {words}")));
        }
        if !colour_holds(&self.fill) {
            return Some((
                "fill".into(),
                format!("“{name}” duvar stilinin dolgu rengi #RRGGBB olmalı."),
            ));
        }
        if let Some(w) = self.width {
            if fixed_width(&self.components).is_some() {
                return Some((
                    "width".into(),
                    format!(
                        "“{name}” duvar stilinin bütün bileşenlerinin kalınlığı belli: genişliği bileşenlerinin toplamıdır, ayrıca verilmez."
                    ),
                ));
            }
            let floor: f64 = self
                .components
                .iter()
                .filter_map(|c| c.thickness)
                .fold(0.0, |a, b| a + b);
            if !(w.is_finite() && w > floor && w <= MAX_WALL_WIDTH) {
                return Some((
                    "width".into(),
                    format!(
                        "“{name}” duvar stilinin genişliği {w} m; bileşenlerinin toplamından ({floor} m) büyük, en çok {MAX_WALL_WIDTH} m olmalı."
                    ),
                ));
            }
        }
        None
    }
}

/// What is wrong with a project's wall styles: a name or id rule, or a style's (as the other tables say theirs,
/// without a closing full stop); none when they hold.
pub fn wall_styles_problem(styles: &[WallStyleDef]) -> Option<String> {
    crate::annotation::names_problem(
        "duvar stili",
        styles.iter().map(|s| (s.id.as_str(), s.name.as_str())),
    )
    .or_else(|| {
        styles.iter().find_map(|s| {
            s.problem()
                .map(|(_, words)| words.trim_end_matches('.').to_owned())
        })
    })
}

/// A level (kat) of the project: its id, name, floor's elevation and storey height.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchLevel {
    pub id: String,
    pub name: String,
    /// The floor's elevation, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: f64,
    /// Floor to floor, metres; absent: not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<f64>,
}

/// The project's architectural settings: its levels, low to high as made, and the detail walls are drawn at (absent:
/// by the drawing's scale, `WallDetail::at_scale`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchSettings {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ArchLevel>>", optional))]
    pub levels: Vec<ArchLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub detail: Option<WallDetail>,
    /// The method new spaces are measured by (docs/adr/0342 §1): the office's profile; absent: net.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub space_method: Option<crate::SpaceMethod>,
}

impl ArchSettings {
    pub fn is_empty(&self) -> bool {
        self == &ArchSettings::default()
    }

    /// The detail walls are drawn at in a drawing at 1:`scale`.
    pub fn detail_at(&self, scale: f64) -> WallDetail {
        self.detail.unwrap_or_else(|| WallDetail::at_scale(scale))
    }

    /// What is wrong with it; none when it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        if let Some(why) = crate::annotation::names_problem(
            "kat",
            self.levels.iter().map(|l| (l.id.as_str(), l.name.as_str())),
        ) {
            return Some((
                "levels".into(),
                format!("Katlar geçersiz: {why}. Her katın kimliği ve adı ayrı olmalı."),
            ));
        }
        for (i, l) in self.levels.iter().enumerate() {
            if !(l.elevation.is_finite() && l.elevation.abs() <= 1.0e5) {
                return Some((
                    format!("levels[{i}].elevation"),
                    format!("“{}” katının kotu sonlu olmalı.", l.name),
                ));
            }
            if l.height.is_some_and(|h| !(h.is_finite() && h > 0.0)) {
                return Some((
                    format!("levels[{i}].height"),
                    format!("“{}” katının yüksekliği sıfırdan büyük olmalı.", l.name),
                ));
            }
        }
        None
    }
}

/// The settings' problems a wall's level reference meets: none when it names one of `levels`.
pub fn level_problem(level: Option<&str>, levels: &[ArchLevel]) -> Option<String> {
    let id = level?;
    (!levels.iter().any(|l| l.id == id)).then(|| {
        format!("Projede “{id}” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da var olan bir katı seçin.")
    })
}

fn component(
    name: &str,
    thickness: Option<f64>,
    detail: WallDetail,
    hatch: Option<(&str, f64, Option<&str>)>,
    priority: u8,
) -> WallComponent {
    WallComponent {
        priority: Some(priority),
        name: name.into(),
        material: Some(name.into()),
        thickness,
        edge: None,
        hatch: hatch.map(|(pattern, scale, color)| ComponentHatch {
            pattern: pattern.into(),
            scale,
            angle: 0.0,
            color: color.map(str::to_owned),
        }),
        detail,
    }
}

/// KentOS's own wall styles (`CAD-ARCH-04`'s office template): a wall made with one takes it into the project, under
/// the same id. Thicknesses metres.
pub fn builtin_wall_styles() -> Vec<WallStyleDef> {
    use WallDetail::{Detailed, Standard};
    // Plaster and boards are finishes (5), insulation between (3), the masonry and concrete the core (1).
    let plaster = |t: f64, name: &str| component(name, Some(t), Detailed, None, 5);
    vec![
        WallStyleDef {
            id: "kentos-dis-duvar-25".into(),
            name: "Dış duvar 25".into(),
            components: vec![
                plaster(0.02, "İç sıva"),
                component("Tuğla", None, Standard, Some(("ANSI31", 1.0, None)), 1),
                component(
                    "Isı yalıtımı",
                    Some(0.05),
                    Standard,
                    Some(("ANSI37", 0.5, None)),
                    3,
                ),
                plaster(0.03, "Dış sıva"),
            ],
            caps: WallCaps::Closed,
            fill: Some("#5A5A5A".into()),
            width: Some(0.25),
        },
        WallStyleDef {
            id: "kentos-ic-duvar-15".into(),
            name: "İç duvar 15".into(),
            components: vec![
                plaster(0.02, "Sıva"),
                component("Tuğla", None, Standard, Some(("ANSI31", 1.0, None)), 1),
                plaster(0.02, "Sıva"),
            ],
            caps: WallCaps::Closed,
            fill: Some("#5A5A5A".into()),
            width: Some(0.15),
        },
        WallStyleDef {
            id: "kentos-gazbeton-20".into(),
            name: "Gazbeton 20".into(),
            components: vec![
                plaster(0.02, "Sıva"),
                component("Gazbeton", None, Standard, Some(("DOTS", 1.0, None)), 1),
                plaster(0.02, "Sıva"),
            ],
            caps: WallCaps::Closed,
            fill: Some("#5A5A5A".into()),
            width: Some(0.20),
        },
        WallStyleDef {
            id: "kentos-betonarme-25".into(),
            name: "Betonarme perde 25".into(),
            components: vec![component(
                "Betonarme",
                None,
                Standard,
                Some(("SOLID", 1.0, Some("#A6A6A6"))),
                1,
            )],
            caps: WallCaps::Closed,
            fill: Some("#3C3C3C".into()),
            width: Some(0.25),
        },
        WallStyleDef {
            id: "kentos-bolme-10".into(),
            name: "Bölme duvar 10".into(),
            components: vec![
                plaster(0.0125, "Alçı levha"),
                component("Yalıtım", None, Detailed, Some(("ANSI37", 0.5, None)), 2),
                plaster(0.0125, "Alçı levha"),
            ],
            caps: WallCaps::Closed,
            fill: None,
            width: Some(0.10),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall(pts: &[(f64, f64)], width: f64) -> WallFields {
        WallFields {
            pts: pts.iter().map(|&(x, y)| Vec2 { x, y }).collect(),
            bulges: None,
            closed: false,
            width,
            justify: WallJustify::Center,
            offset: 0.0,
            style: None,
            components: Vec::new(),
            caps: WallCaps::Closed,
            fill: None,
            height: None,
            arch: ArchInfo::default(),
            joins: WallJoins::default(),
            cleanup: true,
        }
    }

    #[test]
    fn the_builtin_styles_hold_and_their_walls_too() {
        let styles = builtin_wall_styles();
        assert_eq!(wall_styles_problem(&styles), None);
        for s in &styles {
            let mut w = wall(&[(0.0, 0.0), (5.0, 0.0)], s.new_width());
            w.components = s.components.clone();
            assert_eq!(w.problem(), None, "{}", s.name);
            let t = w.thicknesses().unwrap();
            assert!(
                (t.iter().fold(0.0, |a, b| a + b) - w.width).abs() < 1e-12,
                "{}",
                s.name
            );
        }
    }

    #[test]
    fn the_rest_takes_what_the_width_leaves() {
        let s = &builtin_wall_styles()[0];
        assert_eq!(
            component_thicknesses(0.30, &s.components).unwrap()[1],
            0.30 - 0.10
        );
        assert_eq!(component_thicknesses(0.09, &s.components), None);
        let mut w = wall(&[(0.0, 0.0), (5.0, 0.0)], 0.09);
        w.components = s.components.clone();
        assert_eq!(w.problem().unwrap().0, "width");
    }

    #[test]
    fn a_path_needs_two_distinct_points_and_a_closed_one_three() {
        assert_eq!(wall(&[(0.0, 0.0)], 0.2).problem().unwrap().0, "pts");
        assert_eq!(
            wall(&[(0.0, 0.0), (0.0, 0.0)], 0.2).problem().unwrap().0,
            "pts[1]"
        );
        let mut w = wall(&[(0.0, 0.0), (4.0, 0.0)], 0.2);
        w.closed = true;
        w.bulges = Some(vec![0.0, 0.0]);
        assert_eq!(w.problem().unwrap().0, "pts");
        w.bulges = Some(vec![1.0, 1.0]);
        assert_eq!(w.problem(), None, "a round wall of two half circles");
    }

    #[test]
    fn the_detail_follows_the_scale_unless_set() {
        assert_eq!(WallDetail::at_scale(20.0), WallDetail::Detailed);
        assert_eq!(WallDetail::at_scale(50.0), WallDetail::Standard);
        assert_eq!(WallDetail::at_scale(100.0), WallDetail::Schematic);
        let set = ArchSettings {
            levels: Vec::new(),
            detail: Some(WallDetail::Detailed),
            space_method: None,
        };
        assert_eq!(set.detail_at(500.0), WallDetail::Detailed);
    }
}
