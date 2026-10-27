//! Persistent architectural dimension recipes. Members stay ordinary dimensions;
//! the recipe owns their source selection and the user's annotation placement.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{DimensionLook, DimensionSetId, EntityId, SubRef, Vec2};

pub const DIMENSION_SET_MEMBERS_MAX: usize = 10_000;
pub const DIMENSION_SETS_MAX: usize = 10_000;

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

/// The adjacent wall/opening intervals, or the source wall's whole length.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionSetRow {
    Openings,
    Overall,
}
impl DimensionSetRow {
    pub fn name(self) -> &'static str {
        match self {
            Self::Openings => "openings",
            Self::Overall => "overall",
        }
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
}

impl DimensionSetBreak {
    pub fn name(self) -> &'static str {
        match self {
            Self::HostGone => "hostGone",
            Self::UnsupportedWall => "unsupportedWall",
            Self::OpeningUnplaced => "openingUnplaced",
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
        if set.rows.is_empty()
            || set.rows.len() > 2
            || set.rows.iter().collect::<BTreeSet<_>>().len() != set.rows.len()
        {
            return fail("Ölçü dizisinin sıraları boş veya yineleniyor.");
        }
        if let Some((_, why)) = set.look.problem() {
            return Some((i, why));
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
            let source_place = |r: &SubRef| {
                r.path.is_empty()
                    && matches!(
                        r.place,
                        crate::SubPlace::Vertex { .. } | crate::SubPlace::Center
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
