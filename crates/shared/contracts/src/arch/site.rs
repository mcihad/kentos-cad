//! Vaziyet planı (docs/adr/8032, 8174; Planlı Alanlar İmar Yönetmeliği 23, 57/2-a): the parcel, its edges' kinds and
//! setbacks, ±0.00's absolute elevation, the setback line, and the natural and the graded ground.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A parcel edge's kind (docs/adr/8032; Planlı Alanlar İmar Yönetmeliği 23/1): what its garden distance is measured by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SiteEdgeKind {
    /// Yan bahçe: at least 3.00 m, half a metre more for each storey past four (23/1-b, ç).
    #[default]
    Side,
    /// Ön bahçe, a road's edge: at least 5.00 m (23/1-a).
    Front,
    /// Arka bahçe: at least 3.00 m, half a metre more for each storey past four (23/1-c, ç).
    Back,
    /// A public area's edge (a park, a square): at least 5.00 m, no storey's increase (23/1-a, ç).
    Public,
    /// Bitişik (docs/adr/8142): the building stands against the neighbouring parcel's building on this edge (bitişik
    /// nizam, or a blok or ikiz nizam's attached side; PAİY 4/1-pppp): no garden, its wall a fire wall (BYKHY 25) with no
    /// window or door (PAİY 39/4). In ayrık nizam, attached to no neighbour (4/1-pppp-1), a side garden. `.kcad`
    /// architecture data 29.
    Party,
}

impl SiteEdgeKind {
    pub const ALL: [SiteEdgeKind; 5] = [
        SiteEdgeKind::Side,
        SiteEdgeKind::Front,
        SiteEdgeKind::Back,
        SiteEdgeKind::Public,
        SiteEdgeKind::Party,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SiteEdgeKind::Side => "Yan bahçe",
            SiteEdgeKind::Front => "Ön bahçe",
            SiteEdgeKind::Back => "Arka bahçe",
            SiteEdgeKind::Public => "Kamusal alan",
            SiteEdgeKind::Party => "Bitişik (komşu bina)",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SiteEdgeKind::Side => "side",
            SiteEdgeKind::Front => "front",
            SiteEdgeKind::Back => "back",
            SiteEdgeKind::Public => "public",
            SiteEdgeKind::Party => "party",
        }
    }

    /// Its garden distance with `floors` storeys (23/1, d's count), m: 5 on a road or a public area, 3 at the side and
    /// the back with half a metre for each storey past four; none against the neighbour's building (in ayrık nizam the
    /// site plan reads a party edge as a side garden, docs/adr/8142).
    pub fn setback(self, floors: u32) -> f64 {
        let more = 0.5 * f64::from(floors.saturating_sub(4));
        match self {
            SiteEdgeKind::Front | SiteEdgeKind::Public => 5.0,
            SiteEdgeKind::Side | SiteEdgeKind::Back => 3.0 + more,
            SiteEdgeKind::Party => 0.0,
        }
    }
}

/// A parcel edge (docs/adr/8032): its kind and the plan's own setback where the plan gives one, m.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SiteEdge {
    #[serde(default, skip_serializing_if = "is_side")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SiteEdgeKind>", optional))]
    pub kind: SiteEdgeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub setback: Option<f64>,
}

fn is_side(k: &SiteEdgeKind) -> bool {
    *k == SiteEdgeKind::Side
}

/// The largest number of parcel edges a site plan keeps (docs/adr/8032).
pub const MAX_SITE_EDGES: usize = 1000;

/// The most survey layers a ground is made of.
pub const MAX_GROUND_LAYERS: usize = 16;

/// A site's ground (docs/adr/8174 §3.1): the terrain surface of the points with a height on its layers (a survey's),
/// made when it is read; its heights absolute, as the site's `datum` is. The site's own: an architecture project keeps
/// no road settings, where a road's and a map's surfaces are.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SiteGround {
    /// The layers' ids, each once.
    pub layers: Vec<String>,
}

impl SiteGround {
    /// The ground of the points on `layer`.
    pub fn of_layer(layer: &str) -> Self {
        Self { layers: vec![layer.to_owned()] }
    }

    /// What is wrong with it (`what`: the ground's name, Tabii zemin): the words.
    fn problem(&self, what: &str) -> Option<String> {
        if self.layers.is_empty() {
            return Some(format!("{what} en az bir nokta katmanından kurulur."));
        }
        if self.layers.len() > MAX_GROUND_LAYERS {
            return Some(format!("{what} en çok {MAX_GROUND_LAYERS} katmandan kurulur; {} verildi.", self.layers.len()));
        }
        if self.layers.iter().any(|l| l.trim().is_empty()) {
            return Some(format!("{what} katmanının kimliği boş olamaz."));
        }
        if self.layers.iter().enumerate().any(|(i, l)| self.layers[..i].contains(l)) {
            return Some(format!("{what} katmanları birer kez yazılır."));
        }
        None
    }

    /// Whether it is made of the same layers as `other`, in any order.
    fn same(&self, other: &Self) -> bool {
        self.layers.len() == other.layers.len() && self.layers.iter().all(|l| other.layers.contains(l))
    }
}

/// Vaziyet planı (docs/adr/8032; Planlı Alanlar İmar Yönetmeliği 23, 57/2-a): the parcel drawn as a closed line or a
/// polygon (its persistent id), each edge's kind and setback in the boundary's order (edge i from corner i to i + 1;
/// past the list a side garden), ±0.00's absolute elevation (the röper's) and the setback line the document keeps on
/// the parcel and the storeys.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SitePlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parcel: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SiteEdge>>", optional))]
    pub edges: Vec<SiteEdge>,
    /// ±0.00's absolute elevation, m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub datum: Option<f64>,
    /// Yapı yaklaşma sınırı: the persistent id of the closed line the document keeps on the setbacks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub envelope: Option<String>,
    /// Tabii zemin (docs/adr/8174; PAİY 4: "arazinin hafredilmemiş veya doldurulmamış hali", 57/2-a). Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub natural_ground: Option<SiteGround>,
    /// Tesviye edilmiş zemin (docs/adr/8174; PAİY 4, 15–18): what the parcel is graded to. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub graded_ground: Option<SiteGround>,
}

impl SitePlan {
    /// Edge `i`'s kind and setback; past the list a side garden.
    pub fn edge(&self, i: usize) -> SiteEdge {
        self.edges.get(i).copied().unwrap_or_default()
    }

    /// What is wrong with it: the field and the words; an empty one is not written.
    pub fn problem(&self) -> Option<(String, String)> {
        if self == &SitePlan::default() {
            return Some(("site".into(), "Vaziyet planı boş yazılmaz.".into()));
        }
        if self.parcel.as_deref().is_some_and(|p| p.trim().is_empty()) {
            return Some(("site.parcel".into(), "Parselin kimliği boş olamaz.".into()));
        }
        if self
            .envelope
            .as_deref()
            .is_some_and(|p| p.trim().is_empty())
        {
            return Some((
                "site.envelope".into(),
                "Yapı yaklaşma sınırının kimliği boş olamaz.".into(),
            ));
        }
        if self.edges.len() > MAX_SITE_EDGES {
            return Some((
                "site.edges".into(),
                format!(
                    "Parselin en çok {MAX_SITE_EDGES} kenarı yazılır; {} verildi.",
                    self.edges.len()
                ),
            ));
        }
        for (i, e) in self.edges.iter().enumerate() {
            if e.kind == SiteEdgeKind::Party && e.setback.is_some() {
                return Some((
                    format!("site.edges[{i}].setback"),
                    format!(
                        "{}. kenar bitişik (komşu bina): bahçe mesafesi olmaz. Mesafeyi kaldırın ya da kenarın türünü \
                         değiştirin.",
                        i + 1
                    ),
                ));
            }
            if let Some(d) = e.setback
                && !(d.is_finite() && d > 0.0 && d <= 100.0)
            {
                return Some((
                    format!("site.edges[{i}].setback"),
                    format!("Bahçe mesafesi {d} m; sıfırdan büyük, en çok 100 m olmalı."),
                ));
            }
        }
        if let Some(z) = self.datum
            && !(z.is_finite() && z.abs() <= 10_000.0)
        {
            return Some((
                "site.datum".into(),
                format!("±0,00 kotu {z} m; en çok ±10 000 m olmalı."),
            ));
        }
        // The grounds (docs/adr/8174): each its survey's layers; the earthwork is the two's difference.
        if let Some(why) = self.natural_ground.as_ref().and_then(|g| g.problem("Tabii zemin")) {
            return Some(("site.naturalGround".into(), why));
        }
        if let Some(why) = self.graded_ground.as_ref().and_then(|g| g.problem("Tesviye zemin")) {
            return Some(("site.gradedGround".into(), why));
        }
        if let (Some(n), Some(g)) = (&self.natural_ground, &self.graded_ground)
            && n.same(g)
        {
            return Some((
                "site.gradedGround".into(),
                "Tabii ve tesviye zemin aynı katmanlardan kurulamaz: hafriyat ikisinin farkıdır.".into(),
            ));
        }
        None
    }
}
