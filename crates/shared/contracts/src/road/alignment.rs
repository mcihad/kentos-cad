//! The horizontal alignments: their intersection points, offsets and stationing. A part of the road project's settings (docs/adr/8054 §9).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[allow(unused_imports)]
use super::*;

/// A parallel axis's source (docs/adr/0901 §8): the alignment and how far right of it (left negative).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentOffset {
    pub alignment: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
}

/// A station equation (docs/adr/0901 §6): the km where it is (read before it), the km after it, and whether the km
/// grows after it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StationEquationDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub back: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub ahead: f64,
    /// Absent: true.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub increasing: bool,
}

/// An alignment's km: the km at its start and its equations in order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentStationing {
    /// Absent: 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub start: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<StationEquationDef>>", optional))]
    pub equations: Vec<StationEquationDef>,
}

impl AlignmentStationing {
    pub fn is_empty(&self) -> bool {
        self.start == 0.0 && self.equations.is_empty()
    }
}

/// A horizontal alignment's definition (docs/adr/0901): its PIs, or the alignment it is parallel to; its km.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentDef {
    /// Unique in the project; what a profile, a corridor or a parallel axis names.
    pub id: String,
    /// Its name (“Ana aks”), unique in the project.
    pub name: String,
    /// Grows by one with every change of its geometry or km; absent: 0.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    /// Its start, PIs and end; empty for a parallel axis.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AlignmentPi>>", optional))]
    pub pis: Vec<AlignmentPi>,
    /// A parallel axis's source; absent for an alignment of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub offset_of: Option<AlignmentOffset>,
    /// Its km; absent: from 0 with no equation. A parallel axis has its source's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stationing: Option<AlignmentStationing>,
    /// The road's design speed (proje hızı, km/h; docs/adr/0985, KCAD schema 193): what its curves are designed for
    /// and the V of a curve's data on the plan (KGM 2019 şartname s. 5: “Yatay kurblar proje hızına göre
    /// seçilmelidir”; the 2024 sample sheets write `V = 80`). A PI's own `speed` stands for its curve. A parallel axis
    /// has its source's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub design_speed: Option<f64>,
    /// Where the road leads beyond its start (docs/adr/1028, KCAD schema 194): the place KGM's 2024 sample sheets name
    /// on a red arrow at the plan's end toward it ("← KAMAN"). At most 80 characters. A parallel axis has its source's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub start_place: Option<String>,
    /// Where the road leads beyond its end ("ANKARA-NİĞDE OTOYOLU →"); as `start_place`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end_place: Option<String>,
}

/// The longest place name an alignment's end takes (docs/adr/1028): a sheet's arrow label, not a description.
pub const MAX_PLACE_CHARS: usize = 80;

/// Whether `name` is a place name an alignment's end takes: trimmed, not empty, at most [`MAX_PLACE_CHARS`] characters,
/// no control character.
pub fn place_ok(name: &str) -> bool {
    name == name.trim() && !name.is_empty() && name.chars().count() <= MAX_PLACE_CHARS && !name.chars().any(char::is_control)
}

/// A PVI of a vertical alignment (docs/adr/0902 §2): its persistent id, its station along the alignment (its own
/// horizontal length from the start, not its km), its elevation, and the curve on it: a symmetric parabola (`length`),
/// an asymmetric one (`lengthIn` and `lengthOut` together) or a circle (`radius`); none of them: no curve. The first
/// and the last carry none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilePvi {
    /// Unique in the profile.
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: f64,
    /// A symmetric parabola's horizontal length; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    /// An asymmetric parabola's horizontal length before the PVI; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length_in: f64,
    /// … and after it.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length_out: f64,
    /// A circular curve's radius in the station–elevation plane; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
}

/// What is wrong with the alignments, by the field's path and in words; their geometry is judged where it is built.
pub(super) fn alignments_problem(list: &[AlignmentDef]) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if list.len() > MAX_ALIGNMENTS {
        return fail("alignments".into(), &format!("en çok {MAX_ALIGNMENTS} aks olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, a) in list.iter().enumerate() {
        let at = |field: &str| format!("alignments[{i}].{field}");
        if !word_ok(&a.id) {
            return fail(at("id"), "aksın kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(a.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir aks var");
        }
        let name = a.name.trim();
        if name.is_empty() || name != a.name || a.name.chars().count() > 120 {
            return fail(at("name"), "aksın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir aks var");
        }
        match (&a.offset_of, a.pis.is_empty()) {
            (Some(o), true) => {
                if a.stationing.is_some() {
                    return fail(at("stationing"), "paralel aksın km'si kaynağınındır");
                }
                if a.design_speed.is_some() {
                    return fail(at("designSpeed"), "paralel aksın proje hızı kaynağınındır");
                }
                if a.start_place.is_some() || a.end_place.is_some() {
                    return fail(at("startPlace"), "paralel aksın gittiği yerler kaynağınındır");
                }
                let source = list.iter().find(|b| b.id == o.alignment);
                match source {
                    None => return fail(at("offsetOf.alignment"), "kaynak aks projede yok"),
                    Some(b) if b.offset_of.is_some() || b.id == a.id => {
                        return fail(at("offsetOf.alignment"), "paralel aks yalnız PI'larıyla kurulan aksa bağlanır");
                    }
                    _ => {}
                }
                if !(o.offset.is_finite() && o.offset != 0.0 && o.offset.abs() <= MAX_ALIGNMENT_OFFSET) {
                    return fail(
                        at("offsetOf.offset"),
                        &format!("öteleme 0 olmayan, en çok {MAX_ALIGNMENT_OFFSET} m bir sayı olmalı"),
                    );
                }
            }
            (None, false) => {
                if a.pis.len() < 2 || a.pis.len() > MAX_ALIGNMENT_PIS {
                    return fail(at("pis"), &format!("aksın 2 ile {MAX_ALIGNMENT_PIS} arası noktası olur"));
                }
                let mut pis = std::collections::BTreeSet::new();
                for (j, p) in a.pis.iter().enumerate() {
                    let pat = |field: &str| format!("alignments[{i}].pis[{j}].{field}");
                    if !word_ok(&p.id) || !pis.insert(p.id.as_str()) {
                        return fail(pat("id"), "noktanın kimliği aksta tek ve boşluksuz olmalı");
                    }
                    if ![p.x, p.y, p.radius, p.spiral_in, p.spiral_out].iter().all(|v| v.is_finite()) {
                        return fail(pat("x"), "sayılar sonlu olmalı");
                    }
                    if p.radius < 0.0 || p.spiral_in < 0.0 || p.spiral_out < 0.0 {
                        return fail(pat("radius"), "yarıçap ve klotoid boyları eksi olamaz");
                    }
                    if p.speed.is_some_and(|v| !speed_ok(v)) {
                        return fail(pat("speed"), &format!("proje hızı 0'dan büyük, en çok {MAX_DESIGN_SPEED} km/sa olmalı"));
                    }
                }
            }
            _ => return fail(at("pis"), "aks ya PI'larıyla ya bir aksa paralel kurulur, ikisi birden değil"),
        }
        if a.design_speed.is_some_and(|v| !speed_ok(v)) {
            return fail(at("designSpeed"), &format!("proje hızı 0'dan büyük, en çok {MAX_DESIGN_SPEED} km/sa olmalı"));
        }
        for (field, place) in [("startPlace", &a.start_place), ("endPlace", &a.end_place)] {
            if place.as_deref().is_some_and(|n| !place_ok(n)) {
                return fail(at(field), &format!("yer adı boş olmayan, kenarında boşluk olmayan, en çok {MAX_PLACE_CHARS} harf olmalı"));
            }
        }
        if let Some(st) = &a.stationing {
            if !st.start.is_finite() {
                return fail(at("stationing.start"), "başlangıç km'si sonlu olmalı");
            }
            if st.equations.len() > MAX_STATION_EQUATIONS {
                return fail(at("stationing.equations"), &format!("en çok {MAX_STATION_EQUATIONS} km eşitliği olur"));
            }
            if st.equations.iter().any(|q| !(q.back.is_finite() && q.ahead.is_finite())) {
                return fail(at("stationing.equations"), "km eşitliğinin değerleri sonlu olmalı");
            }
        }
    }
    None
}
