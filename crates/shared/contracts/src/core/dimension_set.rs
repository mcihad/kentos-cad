//! Persistent architectural dimension recipes. Members stay ordinary dimensions;
//! the recipe owns their source selection and the user's annotation placement.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{DimensionLook, DimensionSetId, EntityId, SubRef, TextShift, Vec2};

pub const DIMENSION_SET_MEMBERS_MAX: usize = 10_000;
pub const DIMENSION_SETS_MAX: usize = 10_000;
/// The most edges a measured wall path may have (docs/adr/0700).
pub const DIMENSION_SET_EDGES_MAX: usize = 100_000;

/// A real wall face, relative to the direction of its source path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionSetFace {
    #[default]
    Left,
    Right,
}

/// The adjacent wall/opening intervals, or the source wall's whole length; a façade's or a space's walls and grids
/// (docs/adr/0729).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionSetRow {
    Openings,
    Overall,
    /// The walls' thicknesses, the walls meeting a façade, a space's holes: a `span` recipe's only.
    Walls,
    /// The grids across a façade or a space: a `span` recipe's only.
    Grids,
}
impl DimensionSetRow {
    pub fn name(self) -> &'static str {
        match self {
            Self::Openings => "openings",
            Self::Overall => "overall",
            Self::Walls => "walls",
            Self::Grids => "grids",
        }
    }
}

/// Which of a wall's lines a façade or space recipe measures (docs/adr/0729 §1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionSetReference {
    /// Its two faces.
    #[default]
    Faces,
    /// The two boundaries of its strongest components' band (its faces without components).
    Core,
}

impl DimensionSetReference {
    pub fn is_faces(&self) -> bool {
        *self == Self::Faces
    }
}

/// What a recipe does with a value that does not fit between its extension lines (docs/adr/0730 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionSetTight {
    /// In its place, drawn in the danger tone (docs/adr/0729 §6).
    #[default]
    Keep,
    /// Beside its interval, else above or below its line, where no other value or row's line is.
    Shift,
}

impl DimensionSetTight {
    pub fn is_keep(&self) -> bool {
        *self == Self::Keep
    }
}

/// A recipe over a group of walls (a façade) or a space's outline, along one direction (docs/adr/0729 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetSpan {
    /// A façade's further walls, after the recipe's `wall`, in the user's order; none for a space.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<EntityId>>", optional))]
    pub walls: Vec<EntityId>,
    /// The recipe's `wall` is a space: its outline is measured.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub space: bool,
    /// The unit direction measured along, in the world plane.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub direction: Vec2,
    #[serde(default, skip_serializing_if = "DimensionSetReference::is_faces")]
    #[cfg_attr(feature = "ts", ts(as = "Option<DimensionSetReference>", optional))]
    pub reference: DimensionSetReference,
    /// The sources' architectural level; none when they have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
}

impl DimensionSet {
    /// Every source the recipe names: its `wall` (a space for a space recipe), then a façade's further walls.
    pub fn sources(&self) -> Vec<EntityId> {
        let mut out = vec![self.wall];
        if let Some(span) = &self.span {
            out.extend(span.walls.iter().copied());
        }
        out
    }
}

/// Opening references are rough jamb corners, or their actual centre.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionSetOpenings {
    #[default]
    Ends,
    Centers,
}

/// Why a recipe cannot currently generate a trustworthy complete chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionSetBreak {
    HostGone,
    UnsupportedWall,
    OpeningUnplaced,
    /// A façade's walls on different architectural levels now (docs/adr/0729 §3).
    MixedSources,
}

impl DimensionSetBreak {
    pub fn name(self) -> &'static str {
        match self {
            Self::HostGone => "hostGone",
            Self::UnsupportedWall => "unsupportedWall",
            Self::OpeningUnplaced => "openingUnplaced",
            Self::MixedSources => "mixedSources",
        }
    }
}

/// A row manually placed in world coordinates; source edits keep this override.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetRowPlace {
    pub row: DimensionSetRow,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
}

/// A semantic segment, independent of its current order and coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetSegment {
    pub row: DimensionSetRow,
    pub a: SubRef,
    pub b: SubRef,
}

/// An active segment's ordinary dimension. Unchanged active pairs keep this id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSetMember {
    pub uid: EntityId,
    pub row: DimensionSetRow,
    pub a: SubRef,
    pub b: SubRef,
    /// Where the recipe last placed the value at the drawing's own scale (docs/adr/0730 §2); none: in its place. A value
    /// whose place differs from it was moved by the user and keeps its place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shift: Option<TextShift>,
}

impl DimensionSetMember {
    pub fn segment(&self) -> DimensionSetSegment {
        DimensionSetSegment {
            row: self.row,
            a: self.a.clone(),
            b: self.b.clone(),
        }
    }
}

/// A live recipe with stable member identities and separate layout overrides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionSet {
    pub id: DimensionSetId,
    pub wall: EntityId,
    /// The straight edge of the wall's path measured: vertex `edge` to `edge + 1` (a closed wall's last edge back to
    /// its first vertex; docs/adr/0700). None: a two-point wall's only edge, as before (KCAD schema 113).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edge: Option<u32>,
    pub face: DimensionSetFace,
    pub rows: Vec<DimensionSetRow>,
    pub openings: DimensionSetOpenings,
    pub layer_id: String,
    /// First row placement in the world plane; never inferred from render lines.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// Paper millimetres; model distance is gapMm * plotScale / 1000.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub gap_mm: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    /// The new members' starting template; existing members retain manual looks.
    pub look: DimensionLook,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DimensionSetRowPlace>>", optional))]
    pub row_places: Vec<DimensionSetRowPlace>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DimensionSetMember>>", optional))]
    pub members: Vec<DimensionSetMember>,
    /// Explicitly removed segments stay removed until the user restores them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DimensionSetSegment>>", optional))]
    pub suppressed: Vec<DimensionSetSegment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<DimensionSetBreak>,
    /// A façade or a space measured along a direction (docs/adr/0729); none: one wall's edge (docs/adr/0695/0700).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub span: Option<DimensionSetSpan>,
    /// The sheets' scales (1:n) the members are drawn at too, as large on paper as at the drawing's own, ascending
    /// (docs/adr/0730 §1); none: the recipe leaves the members' annotative representations alone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scales: Vec<f64>,
    /// A value that does not fit between its extension lines: kept in its place or shifted (docs/adr/0730 §2).
    #[serde(default, skip_serializing_if = "DimensionSetTight::is_keep")]
    #[cfg_attr(feature = "ts", ts(as = "Option<DimensionSetTight>", optional))]
    pub tight: DimensionSetTight,
}

/// Why a recipe's sheet scales are not usable (docs/adr/0730 §1): each finite, from 1 to 1:1 000 000, ascending, no two the
/// same scale, at most [`crate::MAX_ANNOTATIVE_SCALES`].
pub fn scales_problem(scales: &[f64]) -> Option<&'static str> {
    if scales.len() > crate::MAX_ANNOTATIVE_SCALES {
        return Some("Ölçü dizisinin pafta ölçeği çok fazla.");
    }
    if scales
        .iter()
        .any(|s| !s.is_finite() || *s < 1.0 || *s > crate::MAX_ANNOTATIVE_SCALE)
    {
        return Some("Ölçü dizisinin pafta ölçeği 1 ile 1000000 arasında bir sayı olmalı (1:n'nin n'si).");
    }
    if scales
        .windows(2)
        .any(|w| w[1] <= w[0] || crate::same_scale(w[0], w[1]))
    {
        return Some("Ölçü dizisinin pafta ölçekleri artan sırada ve birbirinden farklı olmalı.");
    }
    None
}

/// Shape validation independent of a drawing's objects and permissions.
pub fn dimension_sets_problem(sets: &[DimensionSet]) -> Option<(usize, String)> {
    use std::collections::BTreeSet;
    if sets.len() > DIMENSION_SETS_MAX {
        return Some((0, "Çok fazla mimari ölçü dizisi var.".into()));
    }
    let mut ids = BTreeSet::new();
    let mut members = BTreeSet::new();
    for (i, set) in sets.iter().enumerate() {
        let fail = |message: &str| Some((i, message.to_owned()));
        if set.id.is_nil() || !ids.insert(set.id) {
            return fail("Ölçü dizisinin kimliği boş veya yineleniyor.");
        }
        if set.wall.is_nil() {
            return fail("Ölçü dizisinin kaynak duvar kimliği boş.");
        }
        if set.edge.is_some_and(|edge| edge as usize >= DIMENSION_SET_EDGES_MAX) {
            return fail("Ölçü dizisinin kenar numarası duvarın kenar sınırını aşıyor.");
        }
        if set.layer_id.is_empty() {
            return fail("Ölçü dizisinin hedef katmanı boş.");
        }
        if ![set.at.x, set.at.y, set.gap_mm, set.height]
            .into_iter()
            .all(f64::is_finite)
            || set.gap_mm <= 0.0
            || set.height <= 0.0
        {
            return fail("Ölçü dizisinin yeri sonlu, aralığı ve yazı yüksekliği pozitif olmalı.");
        }
        let most = if set.span.is_some() { 4 } else { 2 };
        if set.rows.is_empty()
            || set.rows.len() > most
            || set.rows.iter().collect::<BTreeSet<_>>().len() != set.rows.len()
        {
            return fail("Ölçü dizisinin sıraları boş veya yineleniyor.");
        }
        if set.span.is_none()
            && set
                .rows
                .iter()
                .any(|r| matches!(r, DimensionSetRow::Walls | DimensionSetRow::Grids))
        {
            return fail("Duvarlar ve akslar sırası yalnız cephe ya da mahal ölçü dizisinde olur.");
        }
        if let Some(span) = &set.span {
            let d = span.direction;
            if !d.x.is_finite() || !d.y.is_finite() || ((d.x * d.x + d.y * d.y).sqrt() - 1.0).abs() > 1e-9 {
                return fail("Ölçü dizisinin doğrultusu birim vektör olmalı.");
            }
            if set.edge.is_some() {
                return fail("Cephe ya da mahal ölçü dizisinin kenar numarası olmaz.");
            }
            if span.space && !span.walls.is_empty() {
                return fail("Mahal ölçü dizisinin başka kaynak duvarı olmaz.");
            }
            let mut seen = BTreeSet::new();
            seen.insert(set.wall);
            if span.walls.iter().any(|w| w.is_nil() || !seen.insert(*w)) {
                return fail("Ölçü dizisinin kaynak duvarları boş ya da yineleniyor.");
            }
            if span.level.as_ref().is_some_and(String::is_empty) {
                return fail("Ölçü dizisinin katı boş olamaz.");
            }
        }
        if let Some((_, why)) = set.look.problem() {
            return Some((i, why));
        }
        if let Some(why) = scales_problem(&set.scales) {
            return fail(why);
        }
        if set.members.iter().any(|m| m.shift.is_some_and(|t| !t.along.is_finite() || !t.across.is_finite())) {
            return fail("Ölçü dizisi üyesinin yazı yeri sonlu olmalı.");
        }
        let mut placed = BTreeSet::new();
        for place in &set.row_places {
            if !placed.insert(place.row) || !place.at.x.is_finite() || !place.at.y.is_finite() {
                return fail("Elle sıra yeri geçersiz veya yineleniyor.");
            }
        }
        if set.members.len() + set.suppressed.len() > DIMENSION_SET_MEMBERS_MAX {
            return fail("Ölçü dizisinde çok fazla segment var.");
        }
        let segments: Vec<_> = set
            .members
            .iter()
            .map(DimensionSetMember::segment)
            .chain(set.suppressed.iter().cloned())
            .collect();
        let mut keys = std::collections::HashSet::new();
        for (k, segment) in segments.iter().enumerate() {
            // A wall's, an opening's or a space's vertex, an opening's centre, a grid's end (docs/adr/0729).
            let source_place = |r: &SubRef| {
                r.path.is_empty()
                    && matches!(
                        r.place,
                        crate::SubPlace::Vertex { .. }
                            | crate::SubPlace::Center
                            | crate::SubPlace::Start
                            | crate::SubPlace::End
                    )
            };
            if (k < set.members.len() && !set.rows.contains(&segment.row))
                || segment.a == segment.b
                || !source_place(&segment.a)
                || !source_place(&segment.b)
                || segment.a.problem().is_some()
                || segment.b.problem().is_some()
                || segment.a.broken.is_some()
                || segment.b.broken.is_some()
            {
                return fail("Ölçü dizisinin kaynak segmenti geçersiz.");
            }
            if !keys.insert(serde_json::to_string(segment).expect("finite integer source places")) {
                return fail("Ölçü dizisinin segmenti yineleniyor.");
            }
        }
        for member in &set.members {
            if member.uid.is_nil() || !members.insert(member.uid) {
                return fail("Bir ölçü birden fazla dizi öğesinde veya kimliği boş.");
            }
        }
    }
    None
}
