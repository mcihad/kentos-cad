//! A space (mahal, docs/adr/0342 §1): its boundary as parts of an outer ring and holes, drawn or found from the walls
//! round a seed, the measurement method its rings stand for, its number, name, usage, architectural information,
//! height and finishes. Its area, perimeter and volume are measured from its rings; its label is drawn from them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ArchInfo;
use crate::entity::{EntityBase, Vec2};

/// The most vertices a space's rings hold together.
pub const MAX_SPACE_POINTS: usize = 10_000;

/// What a space's rings stand for (docs/adr/0342 §1): the walls' inner faces, their centre lines or their outer faces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SpaceMethod {
    #[default]
    Net,
    Centerline,
    Gross,
}

impl SpaceMethod {
    pub const ALL: [SpaceMethod; 3] = [
        SpaceMethod::Net,
        SpaceMethod::Centerline,
        SpaceMethod::Gross,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SpaceMethod::Net => "Net (iç yüzler)",
            SpaceMethod::Centerline => "Duvar ekseni",
            SpaceMethod::Gross => "Brüt (dış yüzler)",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SpaceMethod::Net => "net",
            SpaceMethod::Centerline => "centerline",
            SpaceMethod::Gross => "gross",
        }
    }
}

fn is_net(m: &SpaceMethod) -> bool {
    *m == SpaceMethod::Net
}

/// Persistent physical column deduction. Absence preserves legacy wall-only areas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SpaceColumns {
    #[default]
    Ignore,
    Net,
    All,
}

impl SpaceColumns {
    pub const ALL: [Self; 3] = [Self::Ignore, Self::Net, Self::All];
    pub fn is_ignore(&self) -> bool { *self == Self::Ignore }
    pub fn applies(self, method: SpaceMethod) -> bool {
        self == Self::All || (self == Self::Net && method == SpaceMethod::Net)
    }
    pub fn name(self) -> &'static str {
        match self { Self::Ignore => "ignore", Self::Net => "net", Self::All => "all" }
    }
    pub fn label(self) -> &'static str {
        match self { Self::Ignore => "Düşme", Self::Net => "Yalnız net", Self::All => "Her yöntemde" }
    }
}

/// Why a space's boundary is not what its walls say.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SpaceBreak {
    /// The walls no longer close round its seed.
    LostEnclosure,
    /// The selected columns cover the complete wall-enclosed area.
    NoFreeArea,
    /// A physical boundary source cannot be read safely.
    InvalidBoundarySource,
}

impl SpaceBreak {
    pub fn words(self) -> &'static str {
        match self {
            Self::LostEnclosure => "Mahali duvarlar artık kapatmıyor; sınırı son hâliyle kaldı. Duvarları kapatın ya da mahali yeniden bulun (Mahali düzenle › Duvarlardan bul).",
            Self::NoFreeArea => "Kolonlar mahalın alanını tamamen kaplıyor; sınırı son hâliyle kaldı. Kolonları veya kolon alanı tercihini düzeltin.",
            Self::InvalidBoundarySource => "Mahalın kolon kaynağı okunamıyor; sınırı son hâliyle kaldı. Kaynak kolonun geometrisini düzeltin.",
        }
    }
}

/// A closed ring: its vertices and, when an edge is an arc, one bulge per edge (absent: every edge straight).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpaceRing {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
}

/// A part of a space: its outer ring and its holes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacePart {
    pub outer: SpaceRing,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SpaceRing>>", optional))]
    pub holes: Vec<SpaceRing>,
}

/// A space's finishes: its floor's, walls' and ceiling's.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpaceFinishes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub floor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wall: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ceiling: Option<String>,
}

impl SpaceFinishes {
    pub fn is_empty(&self) -> bool {
        self == &SpaceFinishes::default()
    }
}

/// A space's own fields: what a command writes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpaceFields {
    /// Its boundary: one part or more, each an outer ring and its holes.
    pub parts: Vec<SpacePart>,
    /// The point inside it its boundary is found round from the walls; absent: its boundary is drawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub seed: Option<Vec2>,
    /// Physical columns to deduct, within the same semantic level (no level: all).
    #[serde(default, skip_serializing_if = "SpaceColumns::is_ignore")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SpaceColumns>", optional))]
    pub columns: SpaceColumns,
    /// What its rings stand for. Absent: the walls' inner faces.
    #[serde(default, skip_serializing_if = "is_net")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SpaceMethod>", optional))]
    pub method: SpaceMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub usage: Option<String>,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
    /// Floor to ceiling, metres; absent: its level's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "SpaceFinishes::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SpaceFinishes>", optional))]
    pub finishes: SpaceFinishes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<SpaceBreak>,
}

/// A space (docs/adr/0342).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpaceEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub space: SpaceFields,
}

fn ring_problem(r: &SpaceRing, at: &str) -> Option<(String, String)> {
    let n = r.pts.len();
    let curved = r
        .bulges
        .as_ref()
        .is_some_and(|b| b.iter().any(|x| *x != 0.0));
    if n < 3 && !(n == 2 && curved) {
        return Some((
            at.into(),
            format!("Mahalin halkası en az üç köşedir; {n} verildi."),
        ));
    }
    if let Some(i) = r
        .pts
        .iter()
        .position(|p| !(p.x.is_finite() && p.y.is_finite()))
    {
        return Some((
            format!("{at}.pts[{i}]"),
            format!("Halkanın {}. köşesinin koordinatları sonlu olmalı.", i + 1),
        ));
    }
    if let Some(b) = &r.bulges {
        if b.len() != n {
            return Some((
                format!("{at}.bulges"),
                format!("Halkanın {n} köşesi, {} bükümü var; eşit olmalı.", b.len()),
            ));
        }
        if b.iter().any(|x| !x.is_finite()) {
            return Some((
                format!("{at}.bulges"),
                "Halkanın bükümleri sonlu olmalı.".into(),
            ));
        }
    }
    None
}

impl SpaceFields {
    /// What is wrong with the space, said so a reader can fix it: the field and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        if self.seed.is_none() && !self.columns.is_ignore() {
            return Some(("columns".into(), "Kolon alanı yalnız duvarlardan bulunan ilişkili mahalda düşülür; çizilmiş sınırda Düşme seçin.".into()));
        }
        if self.parts.is_empty() {
            return Some((
                "parts".into(),
                "Mahalin sınırı yok: en az bir halka verin.".into(),
            ));
        }
        let mut total = 0;
        for (i, p) in self.parts.iter().enumerate() {
            if let Some(found) = ring_problem(&p.outer, &format!("parts[{i}].outer")) {
                return Some(found);
            }
            total += p.outer.pts.len();
            for (k, h) in p.holes.iter().enumerate() {
                if let Some(found) = ring_problem(h, &format!("parts[{i}].holes[{k}]")) {
                    return Some(found);
                }
                total += h.pts.len();
            }
        }
        if total > MAX_SPACE_POINTS {
            return Some((
                "parts".into(),
                format!("Mahalin halkalarında {total} köşe var; en çok {MAX_SPACE_POINTS}."),
            ));
        }
        if let Some(s) = self.seed
            && !(s.x.is_finite() && s.y.is_finite())
        {
            return Some(("seed".into(), "Mahalin iç noktası sonlu olmalı.".into()));
        }
        if let Some(h) = self.height
            && !(h.is_finite() && h > 0.0 && h <= 1000.0)
        {
            return Some((
                "height".into(),
                format!("Mahalin yüksekliği {h}; sıfırdan büyük, en çok 1000 m olmalı."),
            ));
        }
        for (name, v) in [
            ("number", &self.number),
            ("name", &self.name),
            ("usage", &self.usage),
        ] {
            if v.as_deref().is_some_and(|t| t.trim().is_empty()) {
                return Some((
                    name.into(),
                    "Boş metin yazılmaz; alanı kaldırın ya da bir değer yazın.".into(),
                ));
            }
        }
        None
    }
}

impl SpaceEntity {
    /// The space as an area object (its parts and holes): what the formats and the server write where a space has no
    /// kind of its own (docs/adr/0342 §1).
    pub fn as_polygon(&self) -> crate::PathEntity {
        let ring = |r: &SpaceRing| crate::RingGeometry {
            pts: r.pts.clone(),
            bulges: r.bulges.clone(),
            zs: None,
        };
        let holes =
            |p: &SpacePart| (!p.holes.is_empty()).then(|| p.holes.iter().map(ring).collect());
        let mut parts = self.space.parts.iter();
        let first = parts.next().cloned().unwrap_or_default();
        let rest: Vec<crate::AreaPart> = parts
            .map(|p| crate::AreaPart {
                pts: p.outer.pts.clone(),
                bulges: p.outer.bulges.clone(),
                holes: holes(p),
                zs: None,
                ms: None,
            })
            .collect();
        crate::PathEntity {
            base: self.base.clone(),
            pts: first.outer.pts.clone(),
            bulges: first.outer.bulges.clone(),
            holes: holes(&first),
            zs: None,
            ms: None,
            parts: (!rest.is_empty()).then_some(rest),
            closed: false,
            widths: None,
        }
    }
}
