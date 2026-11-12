//! The vertical alignments: their intersection points and crossings. A part of the road project's settings (docs/adr/8054 §9).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[allow(unused_imports)]
use super::*;

/// A vertical alignment's definition (docs/adr/0902): the alignment it lies on and that alignment's revision when it
/// was last made or rebound (a newer one makes it stale), its PVIs, and the surface whose terrain profile it is read
/// against.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileDef {
    /// Unique in the project.
    pub id: String,
    /// Its name (“Proje kotu”), unique in the project.
    pub name: String,
    /// The alignment's id.
    pub alignment: String,
    /// The alignment's revision it was made or rebound against; absent: 0.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    /// Grows by one with every change; absent: 0.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub pvis: Vec<ProfilePvi>,
    /// The terrain surface's id; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    /// What the road crosses (docs/adr/0946; KCAD schema 172).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProfileCrossingDef>>", optional))]
    pub crossings: Vec<ProfileCrossingDef>,
}

/// Where a crossed structure is against the road (docs/adr/0946).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CrossingSide {
    /// Over the road (a bridge, an overpass, a line): its elevation is its underside, the road passes under it.
    Above,
    /// Under the road (a culvert, a pipe): its elevation is its top, the road passes over it.
    Below,
}

/// Something a road crosses (docs/adr/0946): its name, station on the profile's alignment, side, elevation (underside
/// or top) and the clearance under it or the cover over it (m). The values are the project's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileCrossingDef {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    pub side: CrossingSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub clearance: f64,
}

/// A template component's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TemplateComponentKind {
    Lane,
    Shoulder,
}

/// What is wrong with the profiles, by the field's path and in words; each lies on an alignment of its own PIs. Their
/// geometry is judged where it is built.
pub(super) fn profiles_problem(list: &[ProfileDef], alignments: &[AlignmentDef]) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    if list.len() > MAX_PROFILES {
        return fail("profiles".into(), &format!("en çok {MAX_PROFILES} boykesit olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, p) in list.iter().enumerate() {
        let at = |field: &str| format!("profiles[{i}].{field}");
        if !word_ok(&p.id) {
            return fail(at("id"), "boykesitin kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(p.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir boykesit var");
        }
        let name = p.name.trim();
        if name.is_empty() || name != p.name || p.name.chars().count() > 120 {
            return fail(at("name"), "boykesitin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir boykesit var");
        }
        match alignments.iter().find(|a| a.id == p.alignment) {
            None => return fail(at("alignment"), "boykesitin aksı projede yok"),
            Some(a) if a.offset_of.is_some() => {
                return fail(at("alignment"), "boykesit yalnız PI'larıyla kurulan aksa bağlanır");
            }
            _ => {}
        }
        if p.surface.as_deref().is_some_and(|s| !word_ok(s)) {
            return fail(at("surface"), "yüzey bir yüzeyin kimliği olmalı");
        }
        if p.pvis.len() < 2 || p.pvis.len() > MAX_PROFILE_PVIS {
            return fail(at("pvis"), &format!("boykesitin 2 ile {MAX_PROFILE_PVIS} arası PVI'si olur"));
        }
        let mut pvis = std::collections::BTreeSet::new();
        for (j, v) in p.pvis.iter().enumerate() {
            let vat = |field: &str| format!("profiles[{i}].pvis[{j}].{field}");
            if !word_ok(&v.id) || !pvis.insert(v.id.as_str()) {
                return fail(vat("id"), "PVI'nin kimliği boykesitte tek ve boşluksuz olmalı");
            }
            if ![v.station, v.elevation, v.length, v.length_in, v.length_out, v.radius].iter().all(|x| x.is_finite()) {
                return fail(vat("station"), "sayılar sonlu olmalı");
            }
            if v.length < 0.0 || v.length_in < 0.0 || v.length_out < 0.0 || v.radius < 0.0 {
                return fail(vat("length"), "kurp boyu ve yarıçap eksi olamaz");
            }
            let asymmetric = v.length_in > 0.0 || v.length_out > 0.0;
            let kinds = [v.length > 0.0, asymmetric, v.radius > 0.0].iter().filter(|k| **k).count();
            if kinds > 1 {
                return fail(vat("length"), "bir PVI'de tek kurp olur: length, lengthIn ile lengthOut ya da radius");
            }
            if asymmetric && (v.length_in <= 0.0 || v.length_out <= 0.0) {
                return fail(vat("lengthIn"), "asimetrik kurbun iki kolu birlikte verilir");
            }
        }
        if p.crossings.len() > MAX_PROFILE_PVIS {
            return fail(at("crossings"), &format!("en çok {MAX_PROFILE_PVIS} kesişme olur"));
        }
        for (j, c) in p.crossings.iter().enumerate() {
            let cat = format!("profiles[{i}].crossings[{j}]");
            let nm = c.name.trim();
            if nm.is_empty() || nm != c.name || c.name.chars().count() > 120 {
                return fail(cat, "kesişmenin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
            }
            if !(c.station.is_finite() && c.elevation.is_finite()) {
                return fail(cat, "kesişmenin km'si ve kotu sonlu olmalı");
            }
            if !(c.clearance.is_finite() && (0.0..=100.0).contains(&c.clearance)) {
                return fail(cat, "açıklık ya da örtü 0 ile 100 m arası olur");
            }
        }
    }
    None
}
