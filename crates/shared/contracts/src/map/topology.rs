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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
    /// The rules, in the order they are checked and listed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TopologyRule>>", optional))]
    pub rules: Vec<TopologyRule>,
    /// Findings marked as left on purpose (docs/adr/0202 §3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TopologyException>>", optional))]
    pub exceptions: Vec<TopologyException>,
    /// The institution's or the job's quality profile (docs/adr/1260 §2, `.kcad` schema 189): its name, the storage
    /// precision of the coordinates and the acceptance threshold, apart from the check's tolerance above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<TopologyProfile>,
}

/// A quality profile (docs/adr/1260 §2): three lengths kept apart. The check's tolerance (`TopologySettings::tolerance`)
/// is the CAD nearness within which two places are one; `precision` is the step the coordinates are stored at (ArcGIS's
/// XY resolution: the tolerance is at least twice it); `acceptance` is the institution's threshold: no repair moves a
/// vertex of the source geometry farther than it. With a profile every exception and every error mark names its reason.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TopologyProfile {
    /// The profile's name: the institution's or the job's (“Kadastro güncelleme”), 1–200 characters.
    pub name: String,
    /// The profile's own edition as its owner writes it (“2026/1”), at most 50 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub edition: Option<String>,
    /// Metres, within [`TOPOLOGY_PRECISIONS`]; the tolerance is at least twice it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub precision: f64,
    /// Metres, finite, at least the precision.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub acceptance: f64,
}

/// The least and the greatest storage precision a profile may name (metres).
pub const TOPOLOGY_PRECISIONS: (f64, f64) = (0.000_000_001, 0.5);

/// The longest reason an exception or an error mark keeps (characters).
pub const TOPOLOGY_REASON_MAX: usize = 500;

/// The rules' engine edition the rule digest is taken with (docs/adr/1260 §3): it changes when what a kind finds
/// changes, so the marks made under the old one are reviewed again.
pub const TOPOLOGY_RULES_ENGINE: u32 = 1;

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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// The objects' geometry when it was marked (docs/adr/0589 §3): 16 lowercase hexadecimal digits. The geometry
    /// changed since, the exception lapses and its finding is open again; absent (marked before schema 99): kept while
    /// the rule, the objects and the place are the same.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub digest: Option<String>,
    /// What the review decided (docs/adr/1260 §1): absent, an exception (left on purpose); `error`, a confirmed
    /// error to be put right. Schema 189.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub decision: Option<TopologyDecision>,
    /// Why, as the reviewer wrote it (not blank, at most [`TOPOLOGY_REASON_MAX`] characters). Schema 189.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
    /// The rule's version when it was marked ([`rule_digest`]): the rule changed since, the mark lapses. Absent (marked
    /// before schema 189): kept whatever the rule. Schema 189.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rule_digest: Option<String>,
    /// What it covers: absent, the finding at its place; `objects`, every finding of the rule between these objects
    /// in this order wherever it is. Schema 189.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scope: Option<TopologyScope>,
}

/// A review's decision on a finding (docs/adr/1260 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TopologyDecision {
    /// İstisna: left on purpose.
    Exception,
    /// Hata: confirmed, to be put right.
    Error,
}

/// What an exception covers (docs/adr/1260 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TopologyScope {
    /// The finding at its place (within the tolerance).
    Finding,
    /// Every finding of the rule between the same objects in the same order.
    Objects,
}

impl TopologyException {
    /// Whether it is an exception (and not an error mark).
    pub fn is_exception(&self) -> bool {
        self.decision != Some(TopologyDecision::Error)
    }

    /// Whether it covers the finding wherever it is among its objects.
    pub fn anywhere(&self) -> bool {
        self.scope == Some(TopologyScope::Objects)
    }
}

/// A rule's version (docs/adr/1260 §3): the SHA-256 of the engine edition, the kind's key, the layer, the other layer
/// and the value in effect (the kind's default when the rule names none; as the 16 hexadecimal digits of its IEEE 754
/// bits, `-` for none), each closed by a zero byte; its first 8 bytes in lowercase hexadecimal. The same rule checked
/// by the same engine keeps it; a changed kind, layer or value is another rule.
pub fn rule_digest(rule: &TopologyRule) -> String {
    use sha2::{Digest, Sha256};
    let value = rule
        .value
        .or(rule.kind.default_value())
        .map_or_else(|| "-".to_owned(), |v| format!("{:016x}", v.to_bits()));
    let mut h = Sha256::new();
    for piece in [
        format!("kentos.topology-rule/{TOPOLOGY_RULES_ENGINE}"),
        rule.kind.key().to_owned(),
        rule.layer.clone(),
        rule.other.clone().unwrap_or_default(),
        value,
    ] {
        h.update(piece.as_bytes());
        h.update([0u8]);
    }
    h.finalize()[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// What is wrong with a reason, in the words the commands say it; none when it holds.
pub fn reason_problem(reason: &str) -> Option<String> {
    if reason.trim().is_empty() {
        return Some("gerekçe boş".to_owned());
    }
    if reason.chars().count() > TOPOLOGY_REASON_MAX {
        return Some(format!("gerekçe {TOPOLOGY_REASON_MAX} karakteri aşıyor"));
    }
    None
}

impl TopologyProfile {
    /// What is wrong with the profile beside the tolerance `tolerance`, in the words the project file says it; none
    /// when it holds.
    pub fn problem(&self, tolerance: f64) -> Option<String> {
        if self.name.trim().is_empty() || self.name.chars().count() > 200 {
            return Some("kalite profilinin adı boş ya da 200 karakterden uzun".to_owned());
        }
        if self
            .edition
            .as_deref()
            .is_some_and(|e| e.trim().is_empty() || e.chars().count() > 50)
        {
            return Some("kalite profilinin sürümü boş ya da 50 karakterden uzun".to_owned());
        }
        let p = self.precision;
        if !(p.is_finite() && (TOPOLOGY_PRECISIONS.0..=TOPOLOGY_PRECISIONS.1).contains(&p)) {
            return Some(format!(
                "kalite profilinin depolama hassasiyeti {p} m; 0,000000001 ile 0,5 arasında olmalı"
            ));
        }
        if tolerance < 2.0 * p {
            return Some(format!(
                "topoloji toleransı {tolerance} m, depolama hassasiyetinin ({p} m) iki katından küçük; tolerans hassasiyetin en az iki katı olmalı"
            ));
        }
        let a = self.acceptance;
        if !(a.is_finite() && a >= p) {
            return Some(format!(
                "kalite profilinin kabul eşiği {a} m; sonlu ve depolama hassasiyetinden ({p} m) küçük olmayan bir uzunluk olmalı"
            ));
        }
        None
    }
}

/// Whether `d` is an exception's geometry digest: 16 lowercase hexadecimal digits.
pub fn exception_digest_holds(d: &str) -> bool {
    d.len() == 16
        && d.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
        if self.rules.is_empty()
            && self.exceptions.is_empty()
            && self.tolerance.is_none()
            && self.profile.is_none()
        {
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
            if x.digest
                .as_deref()
                .is_some_and(|d| !exception_digest_holds(d))
            {
                return Some("istisnanın geometri özeti 16 küçük onaltılık rakam değil".to_owned());
            }
            if x.rule_digest
                .as_deref()
                .is_some_and(|d| !exception_digest_holds(d))
            {
                return Some("istisnanın kural sürümü 16 küçük onaltılık rakam değil".to_owned());
            }
            if let Some(why) = x.reason.as_deref().and_then(reason_problem) {
                return Some(format!("istisnanın {why}"));
            }
        }
        if let Some(p) = &self.profile {
            return p.problem(self.tolerance());
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
                    && x.digest.as_deref().is_none_or(exception_digest_holds)
                    && x.rule_digest.as_deref().is_none_or(exception_digest_holds)
                    && x.reason.as_deref().is_none_or(|r| reason_problem(r).is_none())
            })
            .collect();
        let tolerance = self.tolerance.filter(|t| Self::tolerance_holds(*t));
        let t = tolerance.unwrap_or(TOPOLOGY_TOLERANCE);
        let kept = Self {
            tolerance,
            rules,
            exceptions,
            profile: self.profile.filter(|p| p.problem(t).is_none()),
        };
        (kept != Self::default()).then_some(kept)
    }
}
