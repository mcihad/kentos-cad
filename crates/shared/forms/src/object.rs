//! What the rules see of an object beside its attributes (docs/adr/0402 §5): its kind as the language
//! names it, its layer, its id and label, and its measures (`$uzunluk`, `$alan`, `$köşe_sayısı`),
//! from its own shape. The commands and the document's following ask it the same way.

use kentos_contracts::Entity;
use kentos_expression::Measured;
use kentos_expression::geometry::{value, vertices};
use kentos_expression::host::Geometry;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Entity as CoreEntity;

use crate::record::Object;

/// The kind of an object as the expression language names it (`$tür`).
pub fn kind_label(e: &Entity) -> &'static str {
    match e {
        Entity::Point(_) => "Nokta",
        Entity::Line(_) => "Çizgi",
        Entity::Polyline(_) => "Çoklu çizgi",
        Entity::Polygon(_) => "Kapalı alan",
        Entity::Circle(_) => "Daire",
        Entity::Arc(_) => "Yay",
        Entity::Ellipse(_) => "Elips",
        Entity::Spline(_) => "Eğri",
        Entity::Nurbs(_) => "NURBS eğrisi",
        Entity::Xline(_) => "Yardımcı çizgi",
        Entity::Ray(_) => "Işın",
        Entity::Text(_) => "Yazı",
        Entity::Dimension(_) => "Ölçü",
        Entity::Hatch(_) => "Tarama",
        Entity::Insert(_) => "Blok",
        Entity::Leader(_) => "Kılavuz",
        Entity::Table(_) => "Tablo",
        Entity::Image(_) => "Resim",
        Entity::Raster(_) => "Raster",
        Entity::PointCloud(_) => "Nokta bulutu",
    }
}

/// The object `e` of the layer named `layer`, numbered `id`, as the rules see it; its measures in
/// metres (the schema's unit turns them into the drawing's).
pub fn object_of(e: &Entity, layer: &str, id: f64) -> Object {
    // The geometry core reads the contract's own JSON for an object.
    let shape = serde_json::to_string(e)
        .ok()
        .and_then(|json| Json::parse(&json).ok())
        .and_then(|json| CoreEntity::from_json(&json).ok())
        .map(|core| core.shape);
    let (measured, corners) = shape.as_ref().map_or((Measured::default(), None), |s| {
        let anchor = value(s, Geometry::AnchorY).zip(value(s, Geometry::AnchorX));
        (
            Measured {
                length: value(s, Geometry::Length),
                area: value(s, Geometry::Area),
                anchor,
            },
            vertices(s),
        )
    });
    Object {
        kind: kind_label(e).to_owned(),
        layer: layer.to_owned(),
        id,
        label: e.base().label.clone(),
        measured,
        vertices: corners,
    }
}
