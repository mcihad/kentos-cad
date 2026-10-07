//! Topoloji kuralları (docs/adr/0202): a project's topology rules, the
//! tolerance they are checked with and the findings marked as exceptions,
//! as the project keeps them (`ProjectSettings::topology`). What each rule
//! finds is the shared core's (`ops::topology_rules`); here are the rules'
//! kinds, what each takes, and what a project file may hold.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{EntityId, Vec2};

/// The tolerance when a project names none (metres).
pub const TOPOLOGY_TOLERANCE: f64 = 0.001;

/// The least and the greatest tolerance a project may name (metres).
pub const TOPOLOGY_TOLERANCES: (f64, f64) = (0.000_001, 1.0);

/// The project's topology rules, tolerance and exceptions (docs/adr/0202 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologySettings {
    /// Metres, within [`TOPOLOGY_TOLERANCES`]; absent: [`TOPOLOGY_TOLERANCE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tolerance: Option<f64>,
    /// The rules, in the order they are checked and listed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TopologyRule>>", optional))]
    pub rules: Vec<TopologyRule>,
    /// Findings marked as left on purpose (docs/adr/0202 §3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TopologyException>>", optional))]
    pub exceptions: Vec<TopologyException>,
}

/// A rule: its kind on a layer and, between layers, another layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyRule {
    /// One of its kind among the project's rules; exceptions name it.
    pub id: String,
    pub kind: TopologyRuleKind,
    /// The layer's id (a layer, not a group). A rule whose layer the project
    /// no longer has stays and is not checked.
    pub layer: String,
    /// The other layer's id, for a rule between layers only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub other: Option<String>,
    /// The value of a kind that takes one: a length in metres, an angle in
    /// radians; absent: the kind's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<f64>,
}

/// The thirteen kinds (docs/adr/0202 §1–§2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TopologyRuleKind {
    /// Çakışmamalı: the layer's areas do not overlap.
    MustNotOverlap,
    /// Boşluk olmamalı: no gap is closed in among the layer's areas.
    MustNotHaveGaps,
    /// İnce alan olmamalı: no part of an area is thinner than the value.
    MustNotHaveSlivers,
    /// Yinelenmemeli: no edge of a line lies on another line, no point on another point.
    MustNotHaveDuplicates,
    /// Sarkan uç olmamalı: every line end meets the layer's line work.
    MustNotHaveDangles,
    /// Kısa kenar olmamalı: no edge is shorter than the value.
    MustNotHaveShortEdges,
    /// Küçük açı olmamalı: no corner is sharper than the value.
    MustNotHaveSmallAngles,
    /// Geçerli olmalı: ADR 0201 §6's problems.
    MustBeValid,
    /// Ortak sınırda köşe eksik olmamalı: an area's vertex on a neighbour's edge is the neighbour's too.
    MustNotHaveMissingVertices,
    /// … ile çakışmamalı: the layer's areas do not overlap the other's.
    MustNotOverlapWith,
    /// … içinde kalmalı: the layer's objects stay within the other's areas.
    MustBeCoveredBy,
    /// Sınırı … sınırlarında olmalı: the layer's areas' boundaries lie on the other's line work.
    BoundaryMustBeCoveredBy,
    /// … çizgilerinin ucunda olmalı: the layer's points are at the other's line ends.
    MustBeOnEndOf,
}

/// What a kind's value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopologyValue {
    /// A length in metres.
    Length,
    /// An angle in radians, below a right angle.
    Angle,
}

impl TopologyRuleKind {
    /// Every kind, in the order the rules window lists them.
    pub const ALL: [TopologyRuleKind; 13] = [
        Self::MustNotOverlap,
        Self::MustNotHaveGaps,
        Self::MustNotHaveSlivers,
        Self::MustNotHaveDuplicates,
        Self::MustNotHaveDangles,
        Self::MustNotHaveShortEdges,
        Self::MustNotHaveSmallAngles,
        Self::MustBeValid,
        Self::MustNotHaveMissingVertices,
        Self::MustNotOverlapWith,
        Self::MustBeCoveredBy,
        Self::BoundaryMustBeCoveredBy,
        Self::MustBeOnEndOf,
    ];

    /// Its name in files and on the wire.
    pub fn key(self) -> &'static str {
        match self {
            Self::MustNotOverlap => "mustNotOverlap",
            Self::MustNotHaveGaps => "mustNotHaveGaps",
            Self::MustNotHaveSlivers => "mustNotHaveSlivers",
            Self::MustNotHaveDuplicates => "mustNotHaveDuplicates",
            Self::MustNotHaveDangles => "mustNotHaveDangles",
            Self::MustNotHaveShortEdges => "mustNotHaveShortEdges",
            Self::MustNotHaveSmallAngles => "mustNotHaveSmallAngles",
            Self::MustBeValid => "mustBeValid",
            Self::MustNotHaveMissingVertices => "mustNotHaveMissingVertices",
            Self::MustNotOverlapWith => "mustNotOverlapWith",
            Self::MustBeCoveredBy => "mustBeCoveredBy",
            Self::BoundaryMustBeCoveredBy => "boundaryMustBeCoveredBy",
            Self::MustBeOnEndOf => "mustBeOnEndOf",
        }
    }

    /// The kind a name means.
    pub fn of_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == key)
    }

    /// Whether the rule is between its layer and another.
    pub fn between(self) -> bool {
        matches!(
            self,
            Self::MustNotOverlapWith
                | Self::MustBeCoveredBy
                | Self::BoundaryMustBeCoveredBy
                | Self::MustBeOnEndOf
        )
    }

    /// The value it takes, if any.
    pub fn value(self) -> Option<TopologyValue> {
        match self {
            Self::MustNotHaveSlivers | Self::MustNotHaveShortEdges => Some(TopologyValue::Length),
            Self::MustNotHaveSmallAngles => Some(TopologyValue::Angle),
            _ => None,
        }
    }

    /// Its value when the rule names none: 0.1 m, 0.05 m, 5°.
    pub fn default_value(self) -> Option<f64> {
        match self {
            Self::MustNotHaveSlivers => Some(0.1),
            Self::MustNotHaveShortEdges => Some(0.05),
            Self::MustNotHaveSmallAngles => Some(5.0_f64.to_radians()),
            _ => None,
        }
    }
}

/// A finding marked as left on purpose: its rule, its objects' persistent
/// ids in the finding's order, and its place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyException {
    /// The rule's id.
    pub rule: String,
    /// At least one.
    pub objects: Vec<EntityId>,
    pub at: Vec2,
}

impl TopologySettings {
    /// The tolerance it is checked with.
    pub fn tolerance(&self) -> f64 {
        self.tolerance.unwrap_or(TOPOLOGY_TOLERANCE)
    }

    /// Whether `t` is a tolerance a project may name.
    pub fn tolerance_holds(t: f64) -> bool {
        t.is_finite() && (TOPOLOGY_TOLERANCES.0..=TOPOLOGY_TOLERANCES.1).contains(&t)
    }

    /// Whether `v` is a value a kind may take: finite and above zero; an
    /// angle below a right angle.
    pub fn value_holds(kind: TopologyValue, v: f64) -> bool {
        v.is_finite()
            && v > 0.0
            && match kind {
                TopologyValue::Length => true,
                TopologyValue::Angle => v < std::f64::consts::FRAC_PI_2,
            }
    }

    /// What is wrong with the settings as a file holds them, in the words
    /// the project file says it; none when they hold.
    pub fn problem(&self) -> Option<String> {
        if self.rules.is_empty() && self.exceptions.is_empty() && self.tolerance.is_none() {
            return Some("topoloji ayarı boş; ayarı olmayan proje alanı yazmaz".to_owned());
        }
        if let Some(t) = self.tolerance.filter(|t| !Self::tolerance_holds(*t)) {
            return Some(format!(
                "topoloji toleransı {t} m; 0,000001 ile 1 arasında olmalı"
            ));
        }
        for (i, r) in self.rules.iter().enumerate() {
            if r.id.is_empty() {
                return Some("topoloji kuralının kimliği boş".to_owned());
            }
            if self.rules[..i].iter().any(|s| s.id == r.id) {
                return Some(format!("“{}” kimlikli topoloji kuralı iki kez var", r.id));
            }
            if r.layer.is_empty() {
                return Some(format!("“{}” kuralının katmanı boş", r.id));
            }
            match (&r.other, r.kind.between()) {
                (None, true) => {
                    return Some(format!("“{}” kuralının öbür katmanı yok", r.id));
                }
                (Some(o), true) if o.is_empty() || *o == r.layer => {
                    return Some(format!(
                        "“{}” kuralının öbür katmanı boş ya da kendi katmanı",
                        r.id
                    ));
                }
                (Some(_), false) => {
                    return Some(format!(
                        "“{}” kuralı tek katmanlıdır; öbür katmanı olmaz",
                        r.id
                    ));
                }
                _ => {}
            }
            match (r.value, r.kind.value()) {
                (Some(_), None) => {
                    return Some(format!("“{}” kuralı değer almaz", r.id));
                }
                (Some(v), Some(kind)) if !Self::value_holds(kind, v) => {
                    return Some(format!(
                        "“{}” kuralının değeri {v}; sıfırdan büyük olmalı (açı dik açıdan küçük)",
                        r.id
                    ));
                }
                _ => {}
            }
        }
        for x in &self.exceptions {
            if !self.rules.iter().any(|r| r.id == x.rule) {
                return Some(format!("istisnanın kuralı “{}” yok", x.rule));
            }
            if x.objects.is_empty() {
                return Some("istisnanın nesnesi yok".to_owned());
            }
            if !(x.at.x.is_finite() && x.at.y.is_finite()) {
                return Some("istisnanın yeri sonlu değil".to_owned());
            }
        }
        None
    }

    /// The settings as a project keeps them: the tolerance that holds, the
    /// rules that hold (of the same id the first), the exceptions of the
    /// rules kept with objects; none when nothing is left.
    pub fn sanitized(self) -> Option<Self> {
        let mut rules: Vec<TopologyRule> = Vec::with_capacity(self.rules.len());
        for mut r in self.rules {
            let other_holds = match (&r.other, r.kind.between()) {
                (Some(o), true) => !o.is_empty() && *o != r.layer,
                (None, false) => true,
                _ => false,
            };
            if r.id.is_empty()
                || r.layer.is_empty()
                || !other_holds
                || rules.iter().any(|s| s.id == r.id)
            {
                continue;
            }
            r.value = match (r.value, r.kind.value()) {
                (Some(v), Some(kind)) if Self::value_holds(kind, v) => Some(v),
                _ => None,
            };
            rules.push(r);
        }
        let exceptions: Vec<TopologyException> = self
            .exceptions
            .into_iter()
            .filter(|x| {
                rules.iter().any(|r| r.id == x.rule)
                    && !x.objects.is_empty()
                    && x.at.x.is_finite()
                    && x.at.y.is_finite()
            })
            .collect();
        let kept = Self {
            tolerance: self.tolerance.filter(|t| Self::tolerance_holds(*t)),
            rules,
            exceptions,
        };
        (kept != Self::default()).then_some(kept)
    }
}
