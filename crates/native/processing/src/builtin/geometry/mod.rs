//! Geometri işlemleri (docs/adr/0201; the web's `builtin/geometry/`): Tampon,
//! Kırp, Gruplayarak birleştir, Kesişim, Fark, Simetrik fark, Birleşim,
//! Geçerliliği denetle, Onar, Sadeleştir and Koordinat sistemine dönüştür.
//! The geometry is the shared core's (`ops::geoprocess::calls`), the same
//! functions the web reaches through WASM; what the tools share is here: the
//! kinds they take, their output layers' look, a result as a new object and
//! the notes every run gives about its inputs.

pub mod buffer;
pub mod clip;
pub mod dissolve;
pub mod overlay;
pub mod reproject;
pub mod simplify;
pub mod validity;

use std::collections::BTreeMap;

use kentos_contracts::{Entity, EntityBase};
use kentos_geometry_core::entity::Shape;
use kentos_native_application::geometry::{edit_geometry, entity_of, shape};

use crate::types::{Feedback, NewLayerStyle, ScopeKind};

/// The kinds that are areas: a closed area, a circle, a whole ellipse, a
/// closed curve (an open ellipse or curve is a path).
pub const AREA_KINDS: [&str; 4] = ["polygon", "circle", "ellipse", "spline"];

/// What the geometry tools take: areas, paths and points.
pub const GEO_KINDS: [&str; 8] = [
    "polygon", "circle", "ellipse", "spline", "line", "polyline", "arc", "point",
];

/// What has written vertices to check and to repair: areas, polylines and lines.
pub const VERTEX_KINDS: [&str; 3] = ["polygon", "polyline", "line"];

/// What Sadeleştir thins: areas and polylines.
pub const SIMPLIFY_KINDS: [&str; 2] = ["polygon", "polyline"];

/// The scopes the geometry tools offer, a layer first.
pub fn geo_scopes() -> Option<Vec<ScopeKind>> {
    Some(vec![
        ScopeKind::Layer,
        ScopeKind::Selection,
        ScopeKind::Visible,
        ScopeKind::All,
    ])
}

/// An output layer's look when the tool makes it: its colour, a 0.25 mm
/// line, the colour faint inside areas (the web's `outputStyle`).
pub fn output_style(color: &str) -> NewLayerStyle {
    NewLayerStyle {
        color: Some(color.to_owned()),
        line_weight: Some(0.25),
        fill: Some(format!("{color}26")),
        ..NewLayerStyle::default()
    }
}

/// Objects as the core takes them.
pub fn shapes(list: &[&Entity]) -> Vec<Shape> {
    list.iter().map(|e| shape(e)).collect()
}

/// A result as a new object on the output layer, with its attributes; none
/// for a shape the contract does not know (never one of these tools').
pub fn new_object(s: Shape, layer_id: &str, attrs: BTreeMap<String, String>) -> Option<Entity> {
    let geometry = edit_geometry(s)?;
    Some(entity_of(
        &geometry,
        EntityBase {
            id: 0,
            layer_id: layer_id.to_owned(),
            color: None,
            attrs,
            label: None,
            symbol: None,
            line_weight: None,
        },
    ))
}

fn held(zs: Option<&Vec<Option<f64>>>) -> bool {
    zs.is_some_and(|z| z.iter().any(Option::is_some))
}

/// Whether an object has elevations: a point's, a line's ends', a path's or
/// an area's vertices' (docs/adr/0142).
pub fn has_elevation(e: &Entity) -> bool {
    match e {
        Entity::Point(p) => p.z.is_some() || p.parts.iter().flatten().any(|q| q.z.is_some()),
        Entity::Line(l) => l.za.is_some() || l.zb.is_some(),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            held(p.zs.as_ref())
                || p.holes.iter().flatten().any(|h| held(h.zs.as_ref()))
                || p.parts.iter().flatten().any(|q| {
                    held(q.zs.as_ref()) || q.holes.iter().flatten().any(|h| held(h.zs.as_ref()))
                })
        }
        _ => false,
    }
}

/// The notes about a run's inputs (§1): ellipses and curves that entered as
/// chords within 0.1 mm (the inputs' and the other side's, each object once),
/// and inputs whose elevations the results do not carry (`points`: a
/// point's elevation is carried, as Koordinat sistemine dönüştür does).
pub fn input_notes(
    feedback: &mut dyn Feedback,
    inputs: &[&Entity],
    others: &[&Entity],
    points: bool,
) {
    let mut seen = std::collections::BTreeSet::new();
    let chorded = inputs
        .iter()
        .chain(others)
        .filter(|e| seen.insert(e.base().id))
        .filter(|e| matches!(e, Entity::Ellipse(_) | Entity::Spline(_)))
        .count();
    if chorded > 0 {
        feedback.warn(format!(
            "{chorded} elips ya da eğri 0,1 mm içinde doğru parçalarına çevrildi."
        ));
    }
    let heights = inputs
        .iter()
        .filter(|e| has_elevation(e) && !(points && matches!(e, Entity::Point(_))))
        .count();
    if heights > 0 {
        feedback.warn(format!("{heights} nesnenin kotları sonuca taşınmadı."));
    }
}

/// The note about objects whose result is nothing (§1).
pub fn empty_note(count: usize, feedback: &mut dyn Feedback) {
    if count > 0 {
        feedback.warn(format!("{count} nesnenin sonucu boş; yazılmadı."));
    }
}

/// An object's attributes, copied.
pub fn attrs_of(e: &Entity) -> BTreeMap<String, String> {
    e.base().attrs.clone()
}

/// The output layer's parameter (the web's `{ name: 'layer', … }`).
pub fn layer_param(name: &str, color: &str) -> crate::types::ParamDef {
    crate::types::ParamDef::new(
        "layer",
        "Çıktı katmanı",
        crate::types::ParamKind::Layer {
            new_layer_style: output_style(color),
        },
    )
    .default_value(serde_json::json!({ "newName": name }))
    .describe("Bu adda katman yoksa oluşturulur.")
}

/// A features parameter of the geometry tools.
pub fn features_param(
    name: &str,
    label: &str,
    kinds: &[&str],
    scopes: Option<Vec<ScopeKind>>,
    description: &str,
) -> crate::types::ParamDef {
    crate::types::ParamDef::new(
        name,
        label,
        crate::types::ParamKind::Features {
            kinds: Some(kinds.iter().map(|k| (*k).to_owned()).collect()),
            scopes,
            writes: false,
        },
    )
    .describe(description)
}
