//! Geometri üretimi (docs/adr/0219; the web's `builtin/generate/`): the
//! points' Voronoi cells and Delaunay triangles, bounding geometries and
//! concave hulls, the areas' poles of inaccessibility. The geometry is the
//! core's (`ops::generate`); what the tools share is here: their scopes, the
//! points they take, the grouping and the notes about the inputs.

pub mod bounding;
pub mod pole;
pub mod triangulation;

use std::collections::BTreeMap;

use kentos_contracts::Entity;
use kentos_geometry_core::vec2::Vec2;
use serde_json::{Value, json};

use super::geometry::has_elevation;
use crate::types::{EnumOption, Feedback, ParamDef, ParamKind, ScopeKind};

/// The scopes the tools offer, a layer first.
pub fn scopes() -> Option<Vec<ScopeKind>> {
    Some(vec![
        ScopeKind::Layer,
        ScopeKind::Selection,
        ScopeKind::Visible,
        ScopeKind::All,
    ])
}

/// A site: a point of an object, its object's place in the input, its part
/// (1, 2 … for a multi-point object) and elevation.
pub struct Site {
    pub p: Vec2,
    pub at: usize,
    pub part: Option<usize>,
    pub z: Option<f64>,
}

/// The points of the input in order, each point of a multi-point object one.
pub fn sites_of(inputs: &[&Entity]) -> Vec<Site> {
    let mut out = Vec::new();
    for (at, e) in inputs.iter().enumerate() {
        if let Entity::Point(p) = e {
            let parts = p.parts.as_deref().unwrap_or(&[]);
            let many = !parts.is_empty();
            out.push(Site {
                p: Vec2::new(p.p.x, p.p.y),
                at,
                part: many.then_some(1),
                z: p.z,
            });
            for (k, q) in parts.iter().enumerate() {
                out.push(Site {
                    p: Vec2::new(q.p.x, q.p.y),
                    at,
                    part: Some(k + 2),
                    z: q.z,
                });
            }
        }
    }
    out
}

fn by_field(v: &crate::types::Values) -> bool {
    v.get("group").and_then(Value::as_str) == Some("field")
}

/// How objects are gathered (§4): each its own, all together, or by a field's value.
pub fn group_params(fallback: &str) -> Vec<ParamDef> {
    vec![
        ParamDef::new(
            "group",
            "Gruplama",
            ParamKind::Choice {
                options: vec![
                    EnumOption::new("each", "Her nesne").hint("Her nesnenin kendisi"),
                    EnumOption::new("all", "Hepsi").hint("Bütün nesneler birlikte"),
                    EnumOption::new("field", "Alana göre")
                        .hint("Alanın değeri aynı olanlar birlikte"),
                ],
            },
        )
        .default_value(json!(fallback)),
        ParamDef::new(
            "field",
            "Alan",
            ParamKind::Field {
                of: vec!["input".into()],
                allow_new: false,
                multiple: false,
            },
        )
        .optional()
        .shown_when(by_field)
        .describe("Değeri aynı olan nesneler bir grup olur; boş değer de bir gruptur."),
    ]
}

/// The groups: each object's group, how many, and each group's attributes
/// (the object's; Nesne sayısı and the field's value).
pub struct Groups {
    pub groups: Vec<usize>,
    pub count: usize,
    pub attrs: Vec<BTreeMap<String, String>>,
}

pub fn grouping(inputs: &[&Entity], group: &str, field: &str) -> Groups {
    let name = crate::text::js_trim(field);
    if group == "each" {
        return Groups {
            groups: (0..inputs.len()).collect(),
            count: inputs.len(),
            attrs: inputs.iter().map(|e| e.base().attrs.clone()).collect(),
        };
    }
    if group == "field" && !name.is_empty() {
        let mut keys: Vec<&str> = Vec::new();
        let groups: Vec<usize> = inputs
            .iter()
            .map(|e| {
                let key = e.base().attrs.get(name).map(String::as_str).unwrap_or("");
                match keys.iter().position(|k| *k == key) {
                    Some(k) => k,
                    None => {
                        keys.push(key);
                        keys.len() - 1
                    }
                }
            })
            .collect();
        let attrs = keys
            .iter()
            .enumerate()
            .map(|(k, key)| {
                let count = groups.iter().filter(|g| **g == k).count();
                BTreeMap::from([
                    (name.to_owned(), (*key).to_owned()),
                    ("Nesne sayısı".to_owned(), count.to_string()),
                ])
            })
            .collect();
        return Groups {
            groups,
            count: keys.len(),
            attrs,
        };
    }
    Groups {
        groups: vec![0; inputs.len()],
        count: 1,
        attrs: vec![BTreeMap::from([(
            "Nesne sayısı".to_owned(),
            inputs.len().to_string(),
        )])],
    }
}

/// The summary of a grouped run.
pub fn grouped_summary(
    inputs: usize,
    count: usize,
    group: &str,
    written: usize,
    what: &str,
) -> String {
    if group == "each" {
        format!("{inputs} nesneden {written} {what} yazıldı.")
    } else {
        format!("{inputs} nesne {count} grupta; {written} {what} yazıldı.")
    }
}

/// The notes about the inputs (§1): curves taken as points within 0.1 mm, elevations not carried.
pub fn input_notes(feedback: &mut dyn Feedback, inputs: &[&Entity], curved: usize) {
    if curved > 0 {
        feedback.warn(format!(
            "{curved} nesnenin yayları ya da eğrileri 0,1 mm içinde noktalarla alındı."
        ));
    }
    let heights = inputs.iter().filter(|e| has_elevation(e)).count();
    if heights > 0 {
        feedback.warn(format!("{heights} nesnenin kotları sonuca taşınmadı."));
    }
}

/// The note about results that are no area.
pub fn empty_note(feedback: &mut dyn Feedback, empty: usize) {
    if empty > 0 {
        feedback.warn(format!(
            "{empty} sonuç alan değil (bir nokta ya da bir doğru); yazılmadı."
        ));
    }
}
