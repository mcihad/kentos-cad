//! The error ellipses an accepted GNSS network draws (plan/map-cad.md MAP-05, docs/adr/2960): each new or moved
//! point's horizontal confidence region, as `map.survey.network` gives it (docs/adr/2560), magnified and drawn on a
//! layer around the point's object, in the same step as the points. Its major axis runs along the map's direction of
//! its azimuth: the azimuth is from true north, the map's up is grid north, the meridian's convergence at the point
//! (`System::true_north`) between them. The ellipses are recorded as made from the points (a use, `kb-N`), so they go
//! out of date when a point is made again. With `heights` (docs/adr/3360) each point's height half-interval is drawn
//! too: a vertical bar through the point, ± the half-interval magnified as the ellipse, ticks a fifth of it each way at
//! its ends, recorded with the ellipses. With `labels` (docs/adr/3760) each ellipse's semi-axes in millimetres are
//! written beside it, its height on paper by the project's plot scale, recorded with them. Docs/adr/4160: a label goes
//! to the first of its places round the ellipse (east first, then QGIS's cartographic order) whose box meets no ellipse,
//! no height bar and no label placed before it; with none free it stays east and counts as crowded.

use std::collections::BTreeMap;
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

use kentos_commands::{
    EllipseEntity, Entity, EntityBase, ErrorEllipse, NetworkEllipses, PathEntity, TextAlign, TextEntity, Vec2,
};
use kentos_domain::Document;
use kentos_geometry_core::entity::TextPlace;
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

/// The places round an ellipse its label tries, in this order (docs/adr/4160): east (ADR 3760's, QGIS's middle right),
/// then QGIS's cartographic order: top right, top left, bottom right, bottom left, middle left, top, bottom. Each is the
/// unit direction from the point and the label's alignment there; the label's point lies that way at the magnified
/// semi-major axis and one label height (no turned ellipse reaches past that circle).
pub(crate) const PLACES: [(f64, f64, TextAlign); 8] = [
    (1.0, 0.0, TextAlign::MiddleLeft),
    (FRAC_1_SQRT_2, FRAC_1_SQRT_2, TextAlign::BottomLeft),
    (-FRAC_1_SQRT_2, FRAC_1_SQRT_2, TextAlign::BottomRight),
    (FRAC_1_SQRT_2, -FRAC_1_SQRT_2, TextAlign::TopLeft),
    (-FRAC_1_SQRT_2, -FRAC_1_SQRT_2, TextAlign::TopRight),
    (-1.0, 0.0, TextAlign::MiddleRight),
    (0.0, 1.0, TextAlign::BottomCenter),
    (0.0, -1.0, TextAlign::TopCenter),
];

/// The label of `mark`'s ellipse (docs/adr/3760): “a 8.4 · b 3.1 mm” (its semi-axes, a tenth of a millimetre; with
/// `heights` “· h ±12.0 mm” too), `with`'s label height on paper (mm) times the plot scale's denominator over 1000
/// high, at `place` of [`PLACES`] (0, east: its middle left at the point's place plus the magnified semi-major axis and
/// the height). Its attributes name the mark and the adjustment.
pub(crate) fn label(
    mark: &str,
    da: &str,
    at: Vec2,
    e: &ErrorEllipse,
    with: &NetworkEllipses,
    plot_scale: f64,
    place: usize,
) -> Entity {
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
        p: {
            let (ux, uy, _) = PLACES[place];
            let d = e.major * with.scale + h;
            Vec2 { x: at.x + ux * d, y: at.y + uy * d }
        },
        text,
        height: h,
        rotation: 0.0,
        align: Some(PLACES[place].2),
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

/// An axis-aligned box: least x, least y, most x, most y.
type Frame = [f64; 4];

/// A label's box as the drawing measures it (its typeface, its alignment; docs/adr/0145).
fn frame_of(doc: &Document, label: &Entity) -> Option<Frame> {
    let shape = crate::geometry::shape(label);
    let ring = TextPlace::of(&shape)?.outline(crate::geometry::drawing_font(doc.settings().drawing_font));
    let mut f = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for p in ring {
        f = [f[0].min(p.x), f[1].min(p.y), f[2].max(p.x), f[3].max(p.y)];
    }
    f[0].is_finite().then_some(f)
}

/// Whether two boxes overlap (touching is not).
fn frames_meet(a: &Frame, b: &Frame) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// The distance from the origin to segment pq.
fn from_origin(p: (f64, f64), q: (f64, f64)) -> f64 {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let len = dx * dx + dy * dy;
    let t = if len > 0.0 { (-(p.0 * dx + p.1 * dy) / len).clamp(0.0, 1.0) } else { 0.0 };
    (p.0 + t * dx).hypot(p.1 + t * dy)
}

/// Whether segment pq passes through box `f`'s inside (Liang–Barsky; along an edge is not).
fn segment_meets(f: &Frame, p: Vec2, q: Vec2) -> bool {
    let (dx, dy) = (q.x - p.x, q.y - p.y);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (d, lo, hi, at) in [(dx, f[0], f[2], p.x), (dy, f[1], f[3], p.y)] {
        if d == 0.0 {
            if at <= lo || at >= hi {
                return false;
            }
            continue;
        }
        let (a, b) = ((lo - at) / d, (hi - at) / d);
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        t0 = t0.max(a);
        t1 = t1.min(b);
        if t0 >= t1 {
            return false;
        }
    }
    true
}

/// Whether box `f` meets an ellipse (centre `c`, semi-major axis vector `major`, minor over major `ratio`): in the
/// ellipse's own frame scaled to the unit circle, the box is a parallelogram; they meet when the centre is inside the
/// box or an edge comes nearer than 1. A flat ellipse is its major axis.
fn ellipse_meets(f: &Frame, c: Vec2, major: Vec2, ratio: f64) -> bool {
    let a = major.x.hypot(major.y);
    let b = a * ratio;
    if !(a > 0.0 && b > 0.0) {
        return segment_meets(f, Vec2 { x: c.x - major.x, y: c.y - major.y }, Vec2 { x: c.x + major.x, y: c.y + major.y });
    }
    if c.x > f[0] && c.x < f[2] && c.y > f[1] && c.y < f[3] {
        return true;
    }
    let (ux, uy) = (major.x / a, major.y / a);
    let to = |x: f64, y: f64| {
        let (dx, dy) = (x - c.x, y - c.y);
        ((dx * ux + dy * uy) / a, (-dx * uy + dy * ux) / b)
    };
    let corners = [to(f[0], f[1]), to(f[2], f[1]), to(f[2], f[3]), to(f[0], f[3])];
    (0..4).any(|i| from_origin(corners[i], corners[(i + 1) % 4]) < 1.0)
}

/// Whether a label's box meets an obstacle: an ellipse, a height bar's segments, or another label's box.
fn meets(doc: &Document, f: &Frame, obstacle: &Entity) -> bool {
    match obstacle {
        Entity::Ellipse(e) => ellipse_meets(f, e.c, e.major, e.ratio),
        Entity::Polyline(p) => p.pts.windows(2).any(|w| segment_meets(f, w[0], w[1])),
        Entity::Text(_) => frame_of(doc, obstacle).is_some_and(|g| frames_meet(f, &g)),
        _ => false,
    }
}

/// Docs/adr/4160: each station's label (in their order) at the first of [`PLACES`] whose box meets none of
/// `obstacles` (the ellipses and the height bars) and no label placed before it; with none free, east, and it counts
/// as crowded. The labels, how many left east, how many are crowded.
pub(crate) fn placed_labels(
    doc: &Document,
    stations: &[(String, Vec2, ErrorEllipse)],
    obstacles: &[Entity],
    with: &NetworkEllipses,
    da: &str,
    plot_scale: f64,
) -> (Vec<Entity>, u32, u32) {
    let (mut out, mut moved, mut crowded): (Vec<Entity>, u32, u32) = (Vec::new(), 0, 0);
    for (mark, at, e) in stations {
        let free = (0..PLACES.len()).find(|&place| {
            let candidate = label(mark, da, *at, e, with, plot_scale, place);
            frame_of(doc, &candidate).is_some_and(|f| {
                !obstacles.iter().chain(out.iter()).any(|o| meets(doc, &f, o))
            })
        });
        match free {
            Some(0) => {}
            Some(_) => moved += 1,
            None => crowded += 1,
        }
        out.push(label(mark, da, *at, e, with, plot_scale, free.unwrap_or(0)));
    }
    (out, moved, crowded)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Document {
        let snapshot = kentos_kcad::decode(include_bytes!("../../../../fixtures/kcad/v2/minimal.kcad")).unwrap();
        Document::from_snapshot_v2(snapshot).unwrap()
    }

    fn ellipse_at(x: f64, y: f64, major: Vec2, ratio: f64) -> Entity {
        let with = NetworkEllipses { layer: "0".into(), scale: 1.0, heights: false, labels: false, label_height: None };
        let mut e = ellipse("N", "da-1", Vec2 { x, y }, &region(1.0, 1.0), &with, 0.0);
        if let Entity::Ellipse(el) = &mut e {
            el.major = major;
            el.ratio = ratio;
        }
        e
    }

    fn region(major: f64, minor: f64) -> ErrorEllipse {
        ErrorEllipse { major, minor, azimuth: None, up: 0.5, confidence: 0.95 }
    }

    /// Docs/adr/4160: a box meets a turned ellipse where its edge comes within it, not where only its axis-aligned
    /// frame would; a flat ellipse is its axis; a segment along a box's edge does not pass through it.
    #[test]
    fn a_box_meets_an_ellipse_by_its_own_frame() {
        // An ellipse 10 long, 1 wide, turned 45°: its frame reaches (7.1, 7.1), the ellipse itself not.
        let m = Vec2 { x: 10.0 * FRAC_1_SQRT_2, y: 10.0 * FRAC_1_SQRT_2 };
        assert!(!ellipse_meets(&[5.0, -6.0, 7.0, -4.0], Vec2 { x: 0.0, y: 0.0 }, m, 0.1));
        assert!(!ellipse_meets(&[4.0, -2.0, 6.0, 0.0], Vec2 { x: 0.0, y: 0.0 }, m, 0.1));
        assert!(ellipse_meets(&[4.0, 3.0, 6.0, 5.0], Vec2 { x: 0.0, y: 0.0 }, m, 0.1));
        // The centre inside the box.
        assert!(ellipse_meets(&[-1.0, -1.0, 1.0, 1.0], Vec2 { x: 0.0, y: 0.0 }, m, 0.1));
        // Flat: its axis.
        assert!(ellipse_meets(&[6.0, 6.0, 8.0, 8.0], Vec2 { x: 0.0, y: 0.0 }, m, 0.0));
        assert!(!segment_meets(&[0.0, 0.0, 2.0, 2.0], Vec2 { x: 0.0, y: -1.0 }, Vec2 { x: 0.0, y: 3.0 }));
        assert!(segment_meets(&[0.0, 0.0, 2.0, 2.0], Vec2 { x: 1.0, y: -1.0 }, Vec2 { x: 1.0, y: 3.0 }));
    }

    /// Docs/adr/4160: a label with every place taken stays east and counts as crowded; one whose east is taken goes
    /// to the top right.
    #[test]
    fn a_label_goes_to_its_first_free_place_or_stays_east_crowded() {
        let doc = doc();
        let with = NetworkEllipses { layer: "0".into(), scale: 1.0, heights: false, labels: true, label_height: None };
        let e = region(1.0, 1.0);
        let at = Vec2 { x: 0.0, y: 0.0 };
        // An ellipse 2 high east: east is taken, the top right (its box from 2.4 up) is free.
        let east = ellipse_at(12.0, 0.0, Vec2 { x: 0.0, y: 2.0 }, 0.5);
        let (labels, moved, crowded) =
            placed_labels(&doc, &[("N".into(), at, e.clone())], std::slice::from_ref(&east), &with, "da-1", 1000.0);
        assert_eq!((moved, crowded), (1, 0));
        let Entity::Text(t) = &labels[0] else { panic!("a text") };
        assert_eq!(t.align, Some(TextAlign::BottomLeft));
        // Taller, it takes the top right too: the top left then.
        let tall = ellipse_at(12.0, 0.0, Vec2 { x: 0.0, y: 6.0 }, 0.5);
        let (labels, _, _) =
            placed_labels(&doc, &[("N".into(), at, e.clone())], std::slice::from_ref(&tall), &with, "da-1", 1000.0);
        let Entity::Text(t) = &labels[0] else { panic!("a text") };
        assert_eq!(t.align, Some(TextAlign::BottomRight));
        // A ring of big circles all round: no place is free.
        let ring: Vec<Entity> = (0..16)
            .map(|k| {
                let a = f64::from(k) * TAU / 16.0;
                ellipse_at(14.0 * a.cos(), 14.0 * a.sin(), Vec2 { x: 0.0, y: 11.0 }, 1.0)
            })
            .collect();
        let (labels, moved, crowded) = placed_labels(&doc, &[("N".into(), at, e)], &ring, &with, "da-1", 1000.0);
        assert_eq!((moved, crowded), (0, 1));
        let Entity::Text(t) = &labels[0] else { panic!("a text") };
        assert_eq!(t.align, Some(TextAlign::MiddleLeft), "east");
    }
}
