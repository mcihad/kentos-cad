//! The error ellipses an accepted GNSS network draws (plan/map-cad.md MAP-05, docs/adr/2960): each new or moved
//! point's horizontal confidence region, as `map.survey.network` gives it (docs/adr/2560), magnified and drawn on a
//! layer around the point's object, in the same step as the points. Its major axis runs along the map's direction of
//! its azimuth: the azimuth is from true north, the map's up is grid north, the meridian's convergence at the point
//! (`System::true_north`) between them. The ellipses are recorded as made from the points (a use, `kb-N`), so they go
//! out of date when a point is made again. With `heights` (docs/adr/3360) each point's height half-interval is drawn
//! too: a vertical bar through the point, ± the half-interval magnified as the ellipse, ticks a fifth of it each way at
//! its ends, recorded with the ellipses. With `labels` (docs/adr/3760) each ellipse's semi-axes in millimetres are
//! written beside it, its height on paper by the project's plot scale, recorded with them.

use std::collections::BTreeMap;
use std::f64::consts::TAU;

use kentos_commands::{
    EllipseEntity, Entity, EntityBase, ErrorEllipse, NetworkEllipses, PathEntity, TextAlign, TextEntity, Vec2,
};
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

/// The height bar of `mark`'s region `e` at its object's place `at` (docs/adr/3360): a polyline down the bar's top
/// tick, the bar itself and its bottom tick, the half-interval u magnified as the ellipse: from (x − u/5, y + u) to
/// (x + u/5, y + u), back to (x, y + u), down to (x, y − u), and the bottom tick from (x − u/5, y − u) to
/// (x + u/5, y − u). Its attributes name the mark, the adjustment, the confidence, the half-interval and the
/// magnification.
pub(crate) fn height_bar(mark: &str, da: &str, at: Vec2, e: &ErrorEllipse, with: &NetworkEllipses) -> Entity {
    let u = e.up * with.scale;
    let t = u / 5.0;
    let p = |dx: f64, dy: f64| Vec2 { x: at.x + dx, y: at.y + dy };
    let mut attrs = BTreeMap::new();
    attrs.insert("İşaret".to_owned(), mark.to_owned());
    attrs.insert("Dengeleme".to_owned(), da.to_owned());
    attrs.insert("Güven".to_owned(), format!("{}", e.confidence));
    attrs.insert("Kot yarı aralığı (m)".to_owned(), four(e.up));
    attrs.insert("Büyütme".to_owned(), format!("{}", with.scale));
    Entity::Polyline(PathEntity {
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
        pts: vec![p(-t, u), p(t, u), p(0.0, u), p(0.0, -u), p(-t, -u), p(t, -u)],
        space_separator: None,
        bulges: None,
        holes: None,
        zs: None,
        ms: None,
        parts: None,
        closed: false,
        widths: None,
    })
}

/// The label of `mark`'s ellipse (docs/adr/3760): “a 8.4 · b 3.1 mm” (its semi-axes, a tenth of a millimetre; with
/// `heights` “· h ±12.0 mm” too), `with`'s label height on paper (mm) times the plot scale's denominator over 1000
/// high, its middle left at the right of the ellipse: the point's place plus the magnified semi-major axis and the
/// height (no turned ellipse reaches further east). Its attributes name the mark and the adjustment.
pub(crate) fn label(mark: &str, da: &str, at: Vec2, e: &ErrorEllipse, with: &NetworkEllipses, plot_scale: f64) -> Entity {
    let h = with.label_height.unwrap_or(kentos_commands::ELLIPSE_LABEL_HEIGHT) * plot_scale / 1000.0;
    let mut text = format!("a {:.1} · b {:.1} mm", e.major * 1000.0, e.minor * 1000.0);
    if with.heights {
        text.push_str(&format!(" · h ±{:.1} mm", e.up * 1000.0));
    }
    let mut attrs = BTreeMap::new();
    attrs.insert("İşaret".to_owned(), mark.to_owned());
    attrs.insert("Dengeleme".to_owned(), da.to_owned());
    Entity::Text(TextEntity {
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
        p: Vec2 { x: at.x + e.major * with.scale + h, y: at.y },
        text,
        height: h,
        rotation: 0.0,
        align: Some(TextAlign::MiddleLeft),
        width_factor: None,
        mask: false,
        label_of: None,
        label_scale: None,
        paragraph: Default::default(),
        face: Default::default(),
        path: None,
        annotative: None,
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
    // docs/adr/3760: a label's height on paper, over 0.
    if let Some(h) = with.label_height
        && !(h.is_finite() && h > 0.0)
    {
        return Err(fail(
            codes::INVALID_SURVEY_VALUE,
            format!("Elips etiketinin kâğıttaki yüksekliği 0'dan büyük olmalı (mm); {h} verildi."),
            "operation.ellipses.labelHeight",
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
