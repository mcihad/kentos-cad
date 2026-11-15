//! What KentOS measures in a drawing for an audit rule: `ProjectAuditFacts`, `kentos.auditFacts` 1 (plan/eklenti.md
//! ADIM 20, docs/adr/8057 §36). "Geometriyi Rust çıkarır, kararı aynı Python betiği verir": the geometry is worked out
//! once, in Rust (each layer's objects counted by their geometry, its invalid geometries, the overlaps of its areas and
//! the gaps their union closes in), and a rule's Python reads the summary (`facts.project`) instead of measuring it
//! again. The desktop and a cloud job give it byte for byte the same.
//!
//! What makes it the same everywhere is written down here, and every producer keeps it by building the facts through
//! [`ProjectAuditFacts::new`]:
//!
//! - **Order:** the layers in the layer tree's order; a layer's invalid geometries by object id, then problem, then
//!   place; its overlaps by their pair of ids (the smaller first), then place; its gaps north to south, then west to
//!   east, then by area. No map's order, clock or thread count enters it.
//! - **Rounding:** areas to 0.0001 m², lengths and places to 0.001 m: the value times 10⁴ (10³) rounded half away from
//!   zero, divided back. −0 is written 0, a value that is not a finite number 0. The tolerance is the project's own.
//! - **Bounds:** a list holds at most [`MOST_ITEMS`] items, the first in its order; `totals` counts them all.
//!
//! The drawing's architecture (storeys, rooms, the program's article 4 areas, the escape routes) is an optional section
//! of the same version, [`architecture`] (docs/adr/8057 §39): absent when the drawing has no room. So is the permit's
//! (the dated scope decision's particulars and the building's area and storeys), [`permit`] (docs/adr/8063): absent
//! when none of its particulars has a value. And the CAD drawing's names and styles EK-3 and EK-4 read, [`cad`]
//! (docs/adr/8064): absent when the drawing has neither a layer holding objects nor a room.
//!
//! Pure, as the crate: the producer (`kentos_processing::audit_facts`) measures with the geometry core; this module
//! holds the shape, its rules and its JSON Schema (`sdk/kentos-audit-facts.schema.json`, the SDK's `factsSchema`).

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

pub mod architecture;
pub mod cad;
pub mod permit;

pub use architecture::ArchitectureFacts;
pub use cad::CadFacts;
pub use permit::PermitFacts;

/// The facts' format name.
pub const FORMAT: &str = "kentos.auditFacts";

/// The facts' version this KentOS writes.
pub const VERSION: u32 = 1;

/// The most items a layer's list holds (the schema's words say 10 000); `totals` counts the rest.
pub const MOST_ITEMS: usize = 10_000;

/// Areas are written to 0.0001 m², lengths and places to 1 mm.
const AREA: f64 = 1e4;
const LENGTH: f64 = 1e3;

/// A drawing's facts, as an audit rule reads them (`facts.project`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ProjectAuditFacts {
    /// `kentos.auditFacts`.
    pub format: String,
    /// The facts' version, 1. A reader ignores a field it does not know: a later KentOS may add facts in the same
    /// version, and a change of meaning is a new one.
    pub schema_version: u32,
    /// The project's topology tolerance, m (its Topoloji settings): areas nearer than it touch, and a piece no wider
    /// than it is neither an overlap nor a gap.
    pub tolerance: f64,
    /// The layers holding objects, in the layer tree's order.
    pub layers: Vec<LayerFacts>,
    /// The drawing's architecture: storeys, rooms, the program's groups, the escape routes. Absent when the drawing has
    /// no room (or the producer was not asked for it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architecture: Option<ArchitectureFacts>,
    /// The permit's particulars the dated scope decision reads and the building's area and storeys. Absent when none of
    /// the particulars has a value (or the producer was not asked for it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permit: Option<PermitFacts>,
    /// The CAD drawing's layers, blocks, dimension typefaces, grids and rooms' numbers, as EK-3 and EK-4 read them.
    /// Absent when the drawing has neither a layer holding objects nor a room (or the producer was not asked for it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cad: Option<CadFacts>,
    /// Whether objects were left out: past the producer's limit, those after it by persistent id (a room or a door
    /// past a list's bound too).
    pub cut: bool,
}

/// One layer's facts. Places are `[x, y]` in the project's plane (m), as the objects' GeoJSON shapes give them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LayerFacts {
    /// The layer's id, what its objects' `layerId` names.
    pub id: String,
    /// Its name, as the layer tree shows it.
    pub name: String,
    /// Its objects counted by their geometry.
    pub objects: ObjectCounts,
    /// Its areas' area summed, m²: holes out, arcs exact (not the tessellated GeoJSON's).
    pub area: f64,
    /// Its lines' length summed, m, arcs exact.
    pub length: f64,
    /// How many invalid geometries, overlaps and gaps it has in all: a list holds at most 10 000, the first in its
    /// order.
    pub totals: Totals,
    /// Its objects whose shape is not valid, each problem once per ring (or line) at its first place.
    pub invalid: Vec<InvalidGeometry>,
    /// The pairs of its areas that cover each other wider than the tolerance.
    pub overlaps: Vec<Overlap>,
    /// What the union of its areas closes in and none of them covers: a gap between parcels, an area's hole.
    pub gaps: Vec<Gap>,
}

/// A layer's objects by their geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ObjectCounts {
    /// Points (a multi-point object once).
    pub points: u64,
    /// Lines: lines, polylines (a closed one too), arcs, open curves.
    pub lines: u64,
    /// Areas: polygons (with holes, of several parts), circles, whole ellipses, closed curves.
    pub areas: u64,
    /// The rest, not measured: texts, dimensions, blocks, hatches, architectural objects.
    pub other: u64,
}

/// How many of each a layer has in all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Totals {
    pub invalid: u64,
    pub overlaps: u64,
    pub gaps: u64,
}

/// What is wrong with a shape, as the geometry core's Geçerliliği denetle says it. An area's ring closes on itself by
/// construction; a ring that comes to nothing is `zeroArea`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ValidityProblem {
    /// Two vertices one after another at the same place (an edge of no length).
    Repeated,
    /// A ring whose area is nothing (its vertices on one line).
    ZeroArea,
    /// A ring that crosses, touches or runs back over itself.
    RingCrossing,
    /// A line that crosses, touches or runs back over itself.
    PathCrossing,
    /// A hole reaching out of its outer ring.
    HoleOutside,
    /// Two holes over each other.
    HolesOverlap,
}

impl ValidityProblem {
    /// Every problem, in the order a check finds them.
    pub const ALL: [ValidityProblem; 6] = [
        ValidityProblem::Repeated,
        ValidityProblem::ZeroArea,
        ValidityProblem::RingCrossing,
        ValidityProblem::PathCrossing,
        ValidityProblem::HoleOutside,
        ValidityProblem::HolesOverlap,
    ];

    /// Its name in the facts (the geometry core's key).
    pub fn key(self) -> &'static str {
        match self {
            ValidityProblem::Repeated => "repeated",
            ValidityProblem::ZeroArea => "zeroArea",
            ValidityProblem::RingCrossing => "ringCrossing",
            ValidityProblem::PathCrossing => "pathCrossing",
            ValidityProblem::HoleOutside => "holeOutside",
            ValidityProblem::HolesOverlap => "holesOverlap",
        }
    }

    /// The problem a key names.
    pub fn of_key(key: &str) -> Option<ValidityProblem> {
        Self::ALL.into_iter().find(|p| p.key() == key)
    }
}

/// An object whose shape is not valid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InvalidGeometry {
    /// The object's persistent id.
    pub uid: String,
    pub problem: ValidityProblem,
    /// Where the problem first shows.
    pub at: [f64; 2],
}

/// Two areas of one layer covering each other.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Overlap {
    /// The first area's persistent id: the smaller of the two.
    pub a: String,
    /// The second area's.
    pub b: String,
    /// The area both cover, m²: its pieces wider than the tolerance.
    pub area: f64,
    /// The centroid of its largest piece.
    pub at: [f64; 2],
}

/// A gap the layer's areas close in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Gap {
    /// Its area, m².
    pub area: f64,
    /// Its centroid.
    pub at: [f64; 2],
    /// The areas around it by persistent id, the one sharing the longest boundary with it first.
    pub around: Vec<String>,
}

impl ProjectAuditFacts {
    /// The facts of `layers` as measured, in the order, rounding and bounds this module writes down: what every
    /// producer builds them with.
    pub fn new(tolerance: f64, layers: Vec<LayerFacts>, cut: bool) -> ProjectAuditFacts {
        ProjectAuditFacts {
            format: FORMAT.to_owned(),
            schema_version: VERSION,
            tolerance: finite(tolerance),
            layers: layers.into_iter().map(LayerFacts::normalized).collect(),
            architecture: None,
            permit: None,
            cad: None,
            cut,
        }
    }

    /// These facts with the drawing's architecture, in its section's order, rounding and bounds; a list it cuts makes
    /// the facts `cut`.
    pub fn with_architecture(mut self, architecture: ArchitectureFacts) -> ProjectAuditFacts {
        let (architecture, cut) = architecture.normalized();
        self.architecture = Some(architecture);
        self.cut |= cut;
        self
    }

    /// These facts with the CAD drawing's, in its section's order, rounding and bounds; a list it cuts makes the facts
    /// `cut`. An empty section is left out.
    pub fn with_cad(mut self, cad: CadFacts) -> ProjectAuditFacts {
        if cad.is_empty() {
            return self;
        }
        let (cad, cut) = cad.normalized();
        self.cad = Some(cad);
        self.cut |= cut;
        self
    }

    /// These facts with the permit's, trimmed, bounded and rounded as its section says.
    pub fn with_permit(mut self, permit: PermitFacts) -> ProjectAuditFacts {
        self.permit = Some(permit.normalized());
        self
    }

    /// The facts as JSON text, the bytes every computer gives for the same drawing.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl LayerFacts {
    /// This layer's facts in the module's order, rounding and bounds; `totals` counts every item before a list is
    /// cut. Doing it again changes nothing.
    pub fn normalized(mut self) -> LayerFacts {
        self.area = round(self.area, AREA);
        self.length = round(self.length, LENGTH);
        for x in &mut self.invalid {
            x.at = place(x.at);
        }
        for o in &mut self.overlaps {
            if o.b < o.a {
                std::mem::swap(&mut o.a, &mut o.b);
            }
            o.area = round(o.area, AREA);
            o.at = place(o.at);
        }
        for g in &mut self.gaps {
            g.area = round(g.area, AREA);
            g.at = place(g.at);
        }
        self.invalid.sort_by(|x, y| {
            x.uid
                .cmp(&y.uid)
                .then(x.problem.cmp(&y.problem))
                .then_with(|| by_place(x.at, y.at))
        });
        self.overlaps.sort_by(|x, y| {
            x.a.cmp(&y.a)
                .then_with(|| x.b.cmp(&y.b))
                .then_with(|| by_place(x.at, y.at))
                .then_with(|| x.area.total_cmp(&y.area))
        });
        // North to south, then west to east.
        self.gaps.sort_by(|x, y| {
            y.at[1]
                .total_cmp(&x.at[1])
                .then_with(|| x.at[0].total_cmp(&y.at[0]))
                .then_with(|| x.area.total_cmp(&y.area))
                .then_with(|| x.around.cmp(&y.around))
        });
        let count = |n: usize, before: u64| before.max(n as u64);
        self.totals = Totals {
            invalid: count(self.invalid.len(), self.totals.invalid),
            overlaps: count(self.overlaps.len(), self.totals.overlaps),
            gaps: count(self.gaps.len(), self.totals.gaps),
        };
        self.invalid.truncate(MOST_ITEMS);
        self.overlaps.truncate(MOST_ITEMS);
        self.gaps.truncate(MOST_ITEMS);
        self
    }
}

/// `x` to `1 / scale`, half away from zero; 0 for −0 and for what is not a finite number.
fn round(x: f64, scale: f64) -> f64 {
    finite((x * scale).round() / scale)
}

fn finite(x: f64) -> f64 {
    if x.is_finite() && x != 0.0 { x } else { 0.0 }
}

fn place(p: [f64; 2]) -> [f64; 2] {
    [round(p[0], LENGTH), round(p[1], LENGTH)]
}

fn by_place(a: [f64; 2], b: [f64; 2]) -> Ordering {
    a[0].total_cmp(&b[0]).then_with(|| a[1].total_cmp(&b[1]))
}

/// The facts' JSON Schema, made from their types and committed as the SDK's `factsSchema` part
/// (`sdk/kentos-audit-facts.schema.json`).
#[cfg(feature = "schema")]
pub fn json_schema() -> serde_json::Value {
    use serde_json::{Value, json};
    let mut schema = schemars::schema_for!(ProjectAuditFacts).to_value();
    if let Some(o) = schema.as_object_mut() {
        o.insert(
            "title".into(),
            Value::from("KentOS audit facts (kentos.auditFacts 1)"),
        );
        if let Some(Value::Object(props)) = o.get_mut("properties") {
            props.insert("format".into(), json!({ "const": FORMAT }));
            props.insert("schemaVersion".into(), json!({ "const": VERSION }));
        }
    }
    schema
}

#[cfg(test)]
mod tests;
