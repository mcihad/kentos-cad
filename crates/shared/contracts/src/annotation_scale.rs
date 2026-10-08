//! The project's annotation heights and how annotations follow the scale
//! (docs/adr/0205). A project keeps seven heights on paper
//! (`ProjectSettings::annotation`); a new annotation's height is its kind's
//! at the plot scale (`paper_height`). When the plot scale or a kind's height
//! changes, the annotations still at the old height take the new one
//! (`follow_annotation_scale`), as a style's do when it changes (ADR 0183 §1);
//! what was changed by hand stays. The TypeScript twin is
//! `apps/web/src/model/annotationScale.ts`; `scripts/fixtures/annotation_scale_cases.py`
//! is the independent reference (fixtures/text/v1/scale.json).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{DimensionStyleDef, Entity, TextStyleDef};

/// The tallest an annotation's height may be on paper, mm.
pub const MAX_ANNOTATION_MM: f64 = 100.0;

/// The texts written by İşlemler's Kenar uzunluklarını yaz and Köşe
/// noktalarını numarala (their `Tür`): they follow the Kenar ve köşe
/// yazıları height, any other text the Yazı height (docs/adr/0205 §3).
pub const MEASURE_TEXT_KINDS: [&str; 2] = ["Kenar ölçüsü", "Köşe noktası"];

/// A kind of annotation with a height of its own (docs/adr/0205 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationKind {
    /// Yazı: texts, multi-line texts, texts along a curve, a text file's
    /// lines, a block attribute's first row.
    Text,
    /// Kılavuz: a leader's note and arrowhead.
    Leader,
    /// Ölçü: every dimension, Standart's value.
    Dimension,
    /// Tablo: a table's cells.
    Table,
    /// Koordinat yazısı: Koordinat yaz and its schedule.
    Coordinate,
    /// Km yazısı: Km yaz's texts.
    Station,
    /// Kenar ve köşe yazıları: İşlemler's edge lengths and corner numbers.
    Measure,
}

impl AnnotationKind {
    /// Every kind, in the settings' order.
    pub const ALL: [AnnotationKind; 7] = [
        AnnotationKind::Text,
        AnnotationKind::Leader,
        AnnotationKind::Dimension,
        AnnotationKind::Table,
        AnnotationKind::Coordinate,
        AnnotationKind::Station,
        AnnotationKind::Measure,
    ];

    /// Its key in the settings (`text` …).
    pub fn key(self) -> &'static str {
        match self {
            AnnotationKind::Text => "text",
            AnnotationKind::Leader => "leader",
            AnnotationKind::Dimension => "dimension",
            AnnotationKind::Table => "table",
            AnnotationKind::Coordinate => "coordinate",
            AnnotationKind::Station => "station",
            AnnotationKind::Measure => "measure",
        }
    }

    /// The kind of this key; none for any other.
    pub fn from_key(key: &str) -> Option<AnnotationKind> {
        AnnotationKind::ALL.into_iter().find(|k| k.key() == key)
    }

    /// Its name as the interface says it.
    pub fn label(self) -> &'static str {
        match self {
            AnnotationKind::Text => "Yazı",
            AnnotationKind::Leader => "Kılavuz",
            AnnotationKind::Dimension => "Ölçü",
            AnnotationKind::Table => "Tablo",
            AnnotationKind::Coordinate => "Koordinat yazısı",
            AnnotationKind::Station => "Km yazısı",
            AnnotationKind::Measure => "Kenar ve köşe yazıları",
        }
    }

    /// Its height when a project names none: the tools' own before the
    /// project had heights, so no drawing changes (mm).
    pub fn default_mm(self) -> f64 {
        match self {
            AnnotationKind::Text
            | AnnotationKind::Leader
            | AnnotationKind::Dimension
            | AnnotationKind::Table => 2.5,
            AnnotationKind::Coordinate | AnnotationKind::Station | AnnotationKind::Measure => 2.0,
        }
    }
}

/// An annotation `mm` high on paper at `1:scale`, metres: the expression
/// every tool writes with, so heights compare exactly.
pub fn paper_height(mm: f64, scale: f64) -> f64 {
    mm / 1000.0 * scale
}

/// Whether `mm` may be an annotation's height: finite, over 0, at most
/// `MAX_ANNOTATION_MM`.
pub fn annotation_mm_holds(mm: f64) -> bool {
    mm.is_finite() && mm > 0.0 && mm <= MAX_ANNOTATION_MM
}

/// The project's annotation heights on paper, mm (docs/adr/0205 §1): a
/// height not given is its kind's default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AnnotationHeights {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub leader: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dimension: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub coordinate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub station: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub measure: Option<f64>,
}

impl AnnotationHeights {
    /// The height a kind is given, if any.
    pub fn given(&self, kind: AnnotationKind) -> Option<f64> {
        match kind {
            AnnotationKind::Text => self.text,
            AnnotationKind::Leader => self.leader,
            AnnotationKind::Dimension => self.dimension,
            AnnotationKind::Table => self.table,
            AnnotationKind::Coordinate => self.coordinate,
            AnnotationKind::Station => self.station,
            AnnotationKind::Measure => self.measure,
        }
    }

    fn slot(&mut self, kind: AnnotationKind) -> &mut Option<f64> {
        match kind {
            AnnotationKind::Text => &mut self.text,
            AnnotationKind::Leader => &mut self.leader,
            AnnotationKind::Dimension => &mut self.dimension,
            AnnotationKind::Table => &mut self.table,
            AnnotationKind::Coordinate => &mut self.coordinate,
            AnnotationKind::Station => &mut self.station,
            AnnotationKind::Measure => &mut self.measure,
        }
    }

    /// The same heights with a kind's given (`None`: its default).
    pub fn with(mut self, kind: AnnotationKind, mm: Option<f64>) -> Self {
        *self.slot(kind) = mm;
        self
    }

    /// A kind's height, mm: the one given, else its default.
    pub fn mm(&self, kind: AnnotationKind) -> f64 {
        self.given(kind).unwrap_or_else(|| kind.default_mm())
    }

    /// What is wrong with them: the key and the refusal's words; none when
    /// they hold.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        AnnotationKind::ALL.into_iter().find_map(|kind| {
            let mm = self.given(kind)?;
            (!annotation_mm_holds(mm)).then(|| {
                (
                    kind.key(),
                    format!(
                        "{} yüksekliği kâğıtta sıfırdan büyük, en çok {MAX_ANNOTATION_MM} mm olmalı; {mm} verildi.",
                        kind.label()
                    ),
                )
            })
        })
    }

    /// As a project keeps them: heights that do not hold and heights equal
    /// to their kind's default left out; none when nothing is left (a
    /// project without heights of its own writes no field).
    pub fn sanitized(mut self) -> Option<Self> {
        for kind in AnnotationKind::ALL {
            let slot = self.slot(kind);
            if slot.is_some_and(|mm| !annotation_mm_holds(mm) || mm == kind.default_mm()) {
                *slot = None;
            }
        }
        (self != Self::default()).then_some(self)
    }
}

/// A change of the plot scale or of the heights (docs/adr/0205 §3): what
/// they were and what they are.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleChange {
    pub from_scale: f64,
    pub to_scale: f64,
    pub from: AnnotationHeights,
    pub to: AnnotationHeights,
}

impl ScaleChange {
    /// Whether nothing an annotation's height depends on changed.
    pub fn is_none(&self) -> bool {
        self.from_scale == self.to_scale
            && AnnotationKind::ALL
                .into_iter()
                .all(|k| self.from.mm(k) == self.to.mm(k))
    }

    /// A kind's height before and after, metres.
    fn kind(&self, kind: AnnotationKind) -> (f64, f64) {
        (
            paper_height(self.from.mm(kind), self.from_scale),
            paper_height(self.to.mm(kind), self.to_scale),
        )
    }

    /// A style's fixed height `mm` before and after, metres.
    fn style(&self, mm: f64) -> (f64, f64) {
        (
            paper_height(mm, self.from_scale),
            paper_height(mm, self.to_scale),
        )
    }
}

/// The new height of one at `height` that was at `was` and follows to `now`;
/// none when it stays.
fn moved(height: f64, (was, now): (f64, f64)) -> Option<f64> {
    (height == was && now != height).then_some(now)
}

/// `e` after the scale or the heights changed (docs/adr/0205 §3); none when
/// it stays. A text follows its style's fixed height when its style has one,
/// else the Yazı height (İşlemler's edge lengths and corner numbers, by their
/// `Tür`, the Kenar ve köşe yazıları height); a multi-line text's box grows
/// with it. A leader follows Kılavuz, a dimension its style's height
/// (Standart: Ölçü), a table its text style's fixed height or Tablo, its
/// rows, columns and frame growing with it about its corner. A linked text
/// (docs/adr/0175) and any other kind stay.
pub fn follow_annotation_scale(
    e: &Entity,
    change: &ScaleChange,
    text_styles: &[TextStyleDef],
    dimension_styles: &[DimensionStyleDef],
) -> Option<Entity> {
    let fixed = |id: &Option<String>| {
        id.as_ref()
            .and_then(|id| text_styles.iter().find(|s| s.id == *id))
            .and_then(|s| s.height)
    };
    match e {
        Entity::Text(t) => {
            if t.label_of.is_some() {
                return None;
            }
            let span = match fixed(&t.face.text_style) {
                Some(mm) => change.style(mm),
                None => {
                    let measure = t
                        .base
                        .attrs
                        .get("Tür")
                        .is_some_and(|k| MEASURE_TEXT_KINDS.contains(&k.as_str()));
                    change.kind(if measure {
                        AnnotationKind::Measure
                    } else {
                        AnnotationKind::Text
                    })
                }
            };
            let now = moved(t.height, span)?;
            let factor = now / t.height;
            let mut out = t.clone();
            out.height = now;
            out.paragraph.box_width = t.paragraph.box_width.map(|w| w * factor);
            Some(Entity::Text(out))
        }
        Entity::Leader(l) => {
            let now = moved(l.height, change.kind(AnnotationKind::Leader))?;
            let mut out = l.clone();
            out.height = now;
            Some(Entity::Leader(out))
        }
        Entity::Dimension(d) => {
            let style = d
                .look
                .dim_style
                .as_ref()
                .and_then(|id| dimension_styles.iter().find(|s| s.id == *id));
            let span = match style {
                Some(s) => change.style(s.height),
                None => change.kind(AnnotationKind::Dimension),
            };
            let now = moved(d.height, span)?;
            let mut out = d.clone();
            out.height = now;
            Some(Entity::Dimension(out))
        }
        Entity::Table(t) => {
            let span = match fixed(&t.face.text_style) {
                Some(mm) => change.style(mm),
                None => change.kind(AnnotationKind::Table),
            };
            let now = moved(t.height, span)?;
            let factor = now / t.height;
            let mut out = t.clone();
            out.height = now;
            out.rows = t.rows.iter().map(|r| r * factor).collect();
            out.columns = t.columns.iter().map(|c| c * factor).collect();
            out.frame = t.frame.map(|f| f * factor);
            Some(Entity::Table(out))
        }
        _ => None,
    }
}
