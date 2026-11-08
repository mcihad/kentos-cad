//! The error ellipses an accepted GNSS network draws (plan/map-cad.md MAP-05, docs/adr/2960): each new or moved
//! point's horizontal confidence region, as `map.survey.network` gives it (docs/adr/2560), magnified and drawn on a
//! layer around the point's object, in the same step as the points. Its major axis runs along the map's direction of
//! its azimuth: the azimuth is from true north, the map's up is grid north, the meridian's convergence at the point
//! (`System::true_north`) between them. The ellipses are recorded as made from the points (a use, `kb-N`), so they go
//! out of date when a point is made again.

use std::collections::BTreeMap;
use std::f64::consts::TAU;

use kentos_commands::{EllipseEntity, Entity, EntityBase, ErrorEllipse, NetworkEllipses, Vec2};
use kentos_domain::Document;
use kentos_geometry_core::jsmath::{cos, sin};

use crate::checks::Stop;
use crate::codes;
use crate::map_survey::fail;

/// A number as the attributes write it: four decimals.
fn four(v: f64) -> String {
    format!("{v:.4}")
}

/// The ellipse of `mark`'s region `e` at its object's place `at` (the project's map), magnified as `with` says, made
/// by the adjustment `da`; the convergence at `at` (radians, counter-clockwise from the map's up to the meridian).
pub(crate) fn ellipse(mark: &str, da: &str, at: Vec2, e: &ErrorEllipse, with: &NetworkEllipses, convergence: f64) -> Entity {
    // A circle has no azimuth: its "major" axis is drawn up the map.
    let bearing = e.azimuth.map_or(0.0, |az| az.to_radians() - convergence);
    let a = e.major * with.scale;
    let major = Vec2 { x: a * sin(bearing), y: a * cos(bearing) };
    let ratio = if e.major > 0.0 { e.minor / e.major } else { 1.0 };
    let mut attrs = BTreeMap::new();
    attrs.insert("İşaret".to_owned(), mark.to_owned());
    attrs.insert("Dengeleme".to_owned(), da.to_owned());
    attrs.insert("Güven".to_owned(), format!("{}", e.confidence));
    attrs.insert("Büyük yarı eksen (m)".to_owned(), four(e.major));
    attrs.insert("Küçük yarı eksen (m)".to_owned(), four(e.minor));
    attrs.insert("Yön (°)".to_owned(), e.azimuth.map(four).unwrap_or_default());
    attrs.insert("Büyütme".to_owned(), format!("{}", with.scale));
    Entity::Ellipse(EllipseEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: with.layer.clone(),
            color: None,
            attrs,
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        c: at,
        major,
        ratio,
        t0: 0.0,
        t1: TAU,
    })
}

/// The magnification and the layer, checked: 1 to 100 000, a layer of the drawing that is not locked.
pub(crate) fn check(doc: &Document, with: &NetworkEllipses) -> Result<(), Stop> {
    let (lo, hi) = kentos_commands::ELLIPSE_SCALES;
    if !(with.scale.is_finite() && (lo..=hi).contains(&with.scale)) {
        return Err(fail(
            codes::INVALID_SURVEY_VALUE,
            format!("Elipslerin büyütmesi 1 ile 100 000 arasında olmalı; {} verildi.", with.scale),
            "operation.ellipses.scale",
        ));
    }
    if doc.layers().get(&with.layer).is_none() {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!("“{}” kimlikli katman çizimde yok.", with.layer),
            "operation.ellipses.layer",
        ));
    }
    if doc.layers().is_locked(&with.layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{}” katmanı kilitli: hata elipsleri kilitli katmana çizilmez. Kilidi açın.", doc.layers().path(&with.layer)),
            "operation.ellipses.layer",
        ));
    }
    Ok(())
}

/// The meridian's convergence at a point of the project's map; the refusal where the system does not reach it.
pub(crate) fn convergence(doc: &Document, at: Vec2, mark: &str) -> Result<f64, Stop> {
    kentos_project::systems::own(doc.settings())
        .and_then(|n| n.system)
        .and_then(|s| s.true_north(kentos_geometry_core::Vec2::new(at.x, at.y)))
        .ok_or_else(|| {
            fail(
                codes::INVALID_SURVEY_VALUE,
                format!("“{mark}” noktasında projenin sisteminin kuzeyi bulunamadı; elipsi haritanın yönüne çevrilemez."),
                "operation.ellipses",
            )
        })
}
