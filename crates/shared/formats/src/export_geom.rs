//! An object as a file without curves, texts or blocks writes it (GeoJSON,
//! KML, GPX; docs/adr/0046, 0222): curves (circles, arcs, ellipses, splines,
//! bulged paths) sampled exactly as the app samples its own; hatches as their
//! areas; area rings turned to the right-hand rule (outlines counter-clockwise,
//! holes clockwise, as RFC 7946 and KML have them); an insert its block's
//! objects placed; a multi-part object one of several members (docs/adr/0143,
//! 0174). What a format cannot hold is said in its own words ([`Words`]).

use kentos_contracts::{Entity, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::entity::{ellipse_geom, polygon_ring, tessellate_circle};
use kentos_geometry_core::geom::arc::{ArcGeom, DEFAULT_STEP, tessellate_arc};
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::geom::ellipse::{is_full_ellipse, tessellate_ellipse};
use kentos_geometry_core::geom::nurbs::{Nurbs, TOLERANCE};
use kentos_geometry_core::geom::spline::catmull_rom;

use crate::blocks::Placing;
use crate::geom::{bulge_path_zs, has_arcs, to_core};
use crate::gis::{Ring, Zs, open_ring, shoelace};
use crate::report::Report;

/// What a format says of what it cannot hold.
pub(crate) struct Words {
    pub ring_turn: &'static str,
    pub curve: &'static str,
    pub path_holes: &'static str,
    pub arc_area: &'static str,
    pub hatch: &'static str,
    pub text: &'static str,
    pub dimension: &'static str,
    pub table: &'static str,
    pub image: &'static str,
    pub cloud: &'static str,
    /// A record without a geometry (docs/adr/0410 §4).
    pub record: &'static str,
    pub raster: &'static str,
    pub leader: &'static str,
    pub block_empty: &'static str,
    pub block: &'static str,
    pub infinite: &'static str,
    /// A route's measures (M, docs/adr/0224), which the format does not hold.
    pub measure: &'static str,
    /// A vertex without an elevation beside ones with: none when the format writes heights its own way.
    pub mixed_z: Option<&'static str>,
}

/// GeoJSON's words (docs/adr/0046).
pub(crate) const GEOJSON: Words = Words {
    ring_turn: "RFC 7946'nın sağ el kuralına çevrildi (dış sınır saat yönünün tersine, delikler saat yönünde); köşeler aynı",
    curve: "GeoJSON'da eğri yok: uygulamanın kendi örneklemesiyle (turda 72 adım) çizgi olarak yazıldı",
    path_holes: "açık çizginin delikleri GeoJSON'da gösterilemez; yazılmadı",
    arc_area: "GeoJSON'da eğri yok: yayları uygulamanın kendi örneklemesiyle (turda 72 adım) yazıldı",
    hatch: "deseni GeoJSON'da gösterilemez; sınırı alan (Polygon) olarak yazıldı",
    text: "GeoJSON'da yazı nesnesi yok; yazılmadı",
    dimension: "GeoJSON'da ölçü nesnesi yok; yazılmadı",
    table: "GeoJSON'da tablo nesnesi yok; yazılmadı",
    image: "GeoJSON'da resim nesnesi yok; yazılmadı",
    cloud: "GeoJSON'da nokta bulutu yok; yazılmadı",
    record: "geometrisiz kayıt boş geometriyle yazıldı",
    raster: "GeoJSON'da raster yok; yazılmadı",
    leader: "kırık çizgisi çizgi (LineString) olarak yazıldı; ok başı ve not GeoJSON'da gösterilemez",
    block_empty: "bloğunda GeoJSON'a yazılabilecek nesne yok; yazılmadı",
    block: "yerleştirmeler bloklarının nesneleriyle, açılmış olarak (GeometryCollection) yazıldı; GeoJSON'da blok yoktur, KentOS'a ayrı nesneler olarak geri okunur",
    infinite: "uçsuz doğru ve ışın GeoJSON'da gösterilemez; yazılmadı",
    measure: "GeoJSON köşenin ölçüsünü (M) taşımaz; çizgi yazıldı, ölçüleri yazılmadı",
    mixed_z: Some(
        "kotu olmayan köşeler iki sayıyla yazıldı (RFC 7946 iki ve üç sayılı konumların bir dizide karışmasına izin verir; bazı programlar onları 0 kotlu okur)",
    ),
};

impl Geometry {
    /// Every vertex moved by `f` (its elevation kept); false when `f` refuses one.
    pub(crate) fn map(&mut self, f: &mut dyn FnMut(Vec2) -> Option<Vec2>) -> bool {
        fn one(p: &mut Vec2, f: &mut dyn FnMut(Vec2) -> Option<Vec2>) -> bool {
            match f(*p) {
                Some(q) => {
                    *p = q;
                    true
                }
                None => false,
            }
        }
        fn ring(r: &mut Ring, f: &mut dyn FnMut(Vec2) -> Option<Vec2>) -> bool {
            r.pts.iter_mut().all(|p| one(p, f))
        }
        match self {
            Geometry::Point(p, _) => one(p, f),
            Geometry::Points(ps) => ps.iter_mut().all(|(p, _)| one(p, f)),
            Geometry::Line(r) => ring(r, f),
            Geometry::Lines(rs) | Geometry::Area(rs) => rs.iter_mut().all(|r| ring(r, f)),
            Geometry::Areas(parts) => parts.iter_mut().flatten().all(|r| ring(r, f)),
            Geometry::Collection(members) => members.iter_mut().all(|m| m.map(f)),
        }
    }
}

/// Segments per turn of a sampled curve: the app's own (72, the core's `DEFAULT_STEP`).
/// Each curve is sampled by the core function the app's outline uses for it
/// (`entity_outline` with 72 segments), called directly so the formats module
/// does not take in the outline of every other kind (dimensions, text).
const SEGMENTS: f64 = 72.0;
/// Points per turn of an ellipse in that outline (`max(64, 2 × segments)`).
const ELLIPSE_PER_TURN: f64 = 2.0 * SEGMENTS;
/// Points per span of a spline in that outline.
const SPLINE_PER_SPAN: f64 = 16.0;

/// An object's geometry as a file without curves writes it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Geometry {
    Point(Vec2, Option<f64>),
    /// A multi-point object's points, each with its elevation (docs/adr/0174).
    Points(Vec<(Vec2, Option<f64>)>),
    Line(Ring),
    /// A multi-part polyline's paths, the polyline's own first (docs/adr/0174).
    Lines(Vec<Ring>),
    /// Closed rings, the outline first, turned to the right-hand rule.
    Area(Vec<Ring>),
    /// A multi-part area: each part's rings as [`Geometry::Area`]'s.
    Areas(Vec<Vec<Ring>>),
    /// An insert's placed objects (docs/adr/0144 §5).
    Collection(Vec<Geometry>),
}

pub(crate) fn from_core(pts: Vec<CoreVec2>) -> Vec<Vec2> {
    pts.into_iter().map(|p| Vec2 { x: p.x, y: p.y }).collect()
}

pub(crate) fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

/// A ring closed by repeating its first point (and its elevation).
pub(crate) fn closed(mut ring: Ring) -> Ring {
    if let Some(&first) = ring.pts.first()
        && ring.pts.last() != Some(&first)
    {
        ring.pts.push(first);
        if let Some(zs) = ring.zs.as_mut() {
            let z = zs.first().copied().flatten();
            zs.push(z);
        }
    }
    ring
}

/// A ring (without its closing point) turned counter-clockwise (`ccw`) or
/// clockwise, keeping its first vertex (each vertex its elevation); the ring closed.
fn oriented(ring: Ring, ccw: bool, turned: &mut bool) -> Ring {
    let (mut ring, _, _) = open_ring(ring);
    let s = shoelace(&ring.pts);
    if (ccw && s < 0.0) || (!ccw && s > 0.0) {
        if let Some(rest) = ring.pts.get_mut(1..) {
            rest.reverse();
        }
        if let Some(rest) = ring.zs.as_mut().and_then(|zs| zs.get_mut(1..)) {
            rest.reverse();
        }
        *turned = true;
    }
    closed(ring)
}

/// An area from its outline and holes; None when the outline has fewer than three points.
fn area(outline: Ring, holes: Vec<Ring>, rep: &mut Report, w: &Words) -> Option<Geometry> {
    let mut turned = false;
    if open_ring(outline.clone()).0.pts.len() < 3 {
        return None;
    }
    let mut rings = vec![oriented(outline, true, &mut turned)];
    for h in holes {
        if open_ring(h.clone()).0.pts.len() >= 3 {
            rings.push(oriented(h, false, &mut turned));
        } else {
            rep.skip("Delik", "üçten az köşesi var; yazılmadı", 0);
        }
    }
    if turned {
        rep.note("Halka yönü", w.ring_turn, 0);
    }
    Some(Geometry::Area(rings))
}

fn curve(rep: &mut Report, what: &str, ring: Ring, w: &Words) -> Option<Geometry> {
    rep.note(what, w.curve, 0);
    (ring.pts.len() >= 2).then_some(Geometry::Line(ring))
}

/// A path's elevations when it has as many as vertices (else it has none).
fn heights(zs: &Zs, vertices: usize) -> Option<&[Option<f64>]> {
    zs.as_deref().filter(|z| z.len() == vertices)
}

/// Whether some elevations are not numbers a file can hold.
pub(crate) fn zs_finite(zs: &Zs) -> bool {
    zs.iter().flatten().flatten().all(|z| z.is_finite())
}

/// Whether a geometry has vertices with an elevation and vertices without, in a
/// ring or between its rings (a hole of a 3D area that has none).
fn mixed(rings: &[&Ring]) -> bool {
    let has = |r: &&Ring| r.zs.iter().flatten().any(Option::is_some);
    let lacks = |r: &&Ring| r.zs.as_ref().is_none_or(|z| z.iter().any(Option::is_none));
    rings.iter().any(has) && rings.iter().any(lacks)
}

/// What an object becomes, or None (said in the report in the format's words).
pub(crate) fn geometry(
    e: &Entity,
    blocks: &Placing,
    rep: &mut Report,
    w: &Words,
) -> Option<Geometry> {
    // A route's measures (docs/adr/0224): the line is written, its measures are not.
    if crate::import::has_measure(e) {
        rep.note("Ölçü (M)", w.measure, 0);
    }
    let g = match e {
        Entity::Point(p) => Some(match p.parts.as_deref() {
            Some(parts) if !parts.is_empty() => Geometry::Points(
                std::iter::once((p.p, p.z))
                    .chain(parts.iter().map(|q| (q.p, q.z)))
                    .collect(),
            ),
            _ => Geometry::Point(p.p, p.z),
        }),
        Entity::Line(l) => Some(Geometry::Line(Ring::new(vec![l.a, l.b], [l.za, l.zb]))),
        Entity::Polyline(p) => {
            if p.holes.as_ref().is_some_and(|h| !h.is_empty()) {
                rep.note("Çoklu çizginin delikleri", w.path_holes, 0);
            }
            // A wide polyline's widths are drawing, not geometry: the line goes (docs/adr/0258 §2).
            if p.widths.is_some() {
                rep.note(
                    "Genişlikli çoklu çizgi",
                    "genişlik bu biçimde yok; çizgi ekseniyle yazıldı",
                    0,
                );
            }
            let one =
                |pts: &[Vec2], bulges: Option<&[f64]>, zs: &Zs, closed: bool, rep: &mut Report| {
                    let zs = heights(zs, pts.len());
                    // A closed line runs back to its first vertex: a line string whose last point is its first.
                    let back = |mut out: Vec<Vec2>, mut zs: Option<Vec<Option<f64>>>| {
                        if closed && let Some(&first) = out.first() {
                            out.push(first);
                            if let Some(z) = zs.as_mut() {
                                let z0 = z.first().copied().flatten();
                                z.push(z0);
                            }
                        }
                        (out, zs)
                    };
                    match bulges {
                        Some(b) if has_arcs(b) => {
                            let out = from_core(bulge_path_outline(
                                &to_core(pts),
                                Some(b),
                                closed,
                                DEFAULT_STEP,
                            ));
                            let zs = zs.map(|z| bulge_path_zs(pts, b, z, closed));
                            let (out, zs) = back(out, zs);
                            curve(
                                rep,
                                "Yaylı çoklu çizgi",
                                Ring::new(out, zs.into_iter().flatten()),
                                w,
                            )
                        }
                        _ => (pts.len() >= 2).then(|| {
                            let (out, zs) = back(pts.to_vec(), zs.map(<[Option<f64>]>::to_vec));
                            Geometry::Line(Ring::new(out, zs.into_iter().flatten()))
                        }),
                    }
                };
            let first = one(&p.pts, p.bulges.as_deref(), &p.zs, p.closed, rep);
            match p.parts.as_deref() {
                // A multi-part polyline is a MultiLineString, its own path the first member (docs/adr/0174).
                Some(parts) if !parts.is_empty() => {
                    let mut paths: Vec<Ring> = Vec::new();
                    let members = std::iter::once(first).chain(
                        parts
                            .iter()
                            .map(|q| one(&q.pts, q.bulges.as_deref(), &q.zs, false, rep)),
                    );
                    for member in members.collect::<Vec<_>>() {
                        match member {
                            Some(Geometry::Line(r)) => paths.push(r),
                            _ => rep.skip("Parça", "iki köşesi yok; yazılmadı", 0),
                        }
                    }
                    match paths.len() {
                        0 => None,
                        1 => paths.pop().map(Geometry::Line),
                        _ => Some(Geometry::Lines(paths)),
                    }
                }
                _ => first,
            }
        }
        Entity::Polygon(p) => {
            let mut arcs = false;
            let mut ring = |pts: &[Vec2], bulges: Option<&[f64]>, zs: &Zs| {
                arcs |= bulges.is_some_and(has_arcs);
                let outline = from_core(polygon_ring(&to_core(pts), bulges));
                let zs = heights(zs, pts.len())
                    .map(|z| bulge_path_zs(pts, bulges.unwrap_or(&[]), z, true));
                Ring::new(outline, zs.into_iter().flatten())
            };
            // The area's own fields are its first part (docs/adr/0143).
            let mut parts = vec![(
                ring(&p.pts, p.bulges.as_deref(), &p.zs),
                p.holes
                    .iter()
                    .flatten()
                    .map(|h| ring(&h.pts, h.bulges.as_deref(), &h.zs))
                    .collect::<Vec<_>>(),
            )];
            for part in p.parts.iter().flatten() {
                let holes = part.holes.iter().flatten();
                let holes = holes.map(|h| ring(&h.pts, h.bulges.as_deref(), &h.zs));
                let holes: Vec<Ring> = holes.collect();
                parts.push((ring(&part.pts, part.bulges.as_deref(), &part.zs), holes));
            }
            if arcs {
                rep.note("Yaylı alan", w.arc_area, 0);
            }
            let several = parts.len() > 1;
            let mut areas: Vec<Vec<Ring>> = Vec::new();
            for (outline, holes) in parts {
                match area(outline, holes, rep, w) {
                    Some(Geometry::Area(rings)) => areas.push(rings),
                    _ if several => rep.skip("Parça", "üçten az köşesi var; yazılmadı", 0),
                    _ => {}
                }
            }
            match areas.len() {
                0 => None,
                1 => areas.pop().map(Geometry::Area),
                _ => Some(Geometry::Areas(areas)),
            }
        }
        Entity::Circle(c) => {
            let pts = from_core(tessellate_circle(core(c.c), c.r, SEGMENTS));
            curve(rep, "Daire", closed(Ring::flat(pts)), w)
        }
        Entity::Arc(a) => curve(
            rep,
            "Yay",
            Ring::flat(from_core(tessellate_arc(
                &ArcGeom {
                    c: core(a.c),
                    r: a.r,
                    a0: a.a0,
                    a1: a.a1,
                },
                DEFAULT_STEP,
            ))),
            w,
        ),
        Entity::Ellipse(el) => {
            let g = ellipse_geom(core(el.c), core(el.major), el.ratio, el.t0, el.t1);
            let pts = from_core(tessellate_ellipse(&g, ELLIPSE_PER_TURN.max(64.0)));
            let ring = Ring::flat(pts);
            curve(
                rep,
                "Elips",
                if is_full_ellipse(&g) {
                    closed(ring)
                } else {
                    ring
                },
                w,
            )
        }
        Entity::Spline(s) => {
            let mut pts = from_core(catmull_rom(&to_core(&s.pts), s.closed, SPLINE_PER_SPAN));
            // The curve passes through its fit points: its ends are the source's own
            // coordinates, not a sample off by the last bit.
            if let (Some(first), Some(&start)) = (pts.first_mut(), s.pts.first()) {
                *first = start;
            }
            if s.closed {
                if let Some(first) = pts.first().copied() {
                    pts.pop();
                    pts.push(first);
                }
            } else if let (Some(last), Some(&end)) = (pts.last_mut(), s.pts.last()) {
                *last = end;
            }
            curve(rep, "Eğri (spline)", Ring::flat(pts), w)
        }
        Entity::Nurbs(n) => {
            // Chords within the drawing's 0,1 mm of the curve; its ends exact (docs/adr/0259 §4).
            let ctrl = to_core(&n.ctrl);
            let c = Nurbs {
                degree: n.degree as usize,
                ctrl: &ctrl,
                knots: &n.knots,
                weights: n.weights.as_deref(),
            };
            if !c.valid() {
                rep.skip("NURBS eğrisi", "sayıları bir eğri tanımlamıyor", 0);
                return None;
            }
            curve(
                rep,
                "NURBS eğrisi",
                Ring::flat(from_core(c.tessellate(TOLERANCE))),
                w,
            )
        }
        Entity::Hatch(h) => {
            rep.note("Tarama", w.hatch, 0);
            let holes = h.holes.iter().flatten().cloned().map(Ring::flat);
            area(Ring::flat(h.ring.clone()), holes.collect(), rep, w)
        }
        Entity::Text(_) => {
            rep.skip("Yazı", w.text, 0);
            return None;
        }
        Entity::Dimension(_) => {
            rep.skip("Ölçü", w.dimension, 0);
            return None;
        }
        Entity::Table(_) => {
            rep.skip("Tablo", w.table, 0);
            return None;
        }
        Entity::Image(_) => {
            rep.skip("Resim", w.image, 0);
            return None;
        }
        // Nor a point cloud (docs/adr/0207 §11).
        Entity::PointCloud(_) => {
            rep.skip("Nokta bulutu", w.cloud, 0);
            return None;
        }
        // Its body, an area (docs/adr/0339 §4): GeoJSON, KML and GPX have no wall.
        Entity::Wall(e) => {
            let pts: Vec<CoreVec2> = e.wall.pts.iter().map(|p| core(*p)).collect();
            let Ok((outer, holes)) = kentos_geometry_core::geom::wall::body(
                &pts,
                e.wall.bulges.as_deref(),
                &crate::walls::spec(&e.wall),
            ) else {
                rep.skip(
                    "Duvar",
                    "çizilemeyen duvar (yayı duvarın genişliğinden dar) yazılmadı",
                    0,
                );
                return None;
            };
            let plain = |pts| Ring::new(from_core(pts), std::iter::empty::<Option<f64>>());
            let holes = holes.into_iter().flatten().map(plain).collect();
            area(plain(outer), holes, rep, w)
        }
        // Its jambs' footprint, an area (docs/adr/0341 §2): GeoJSON, KML and GPX have no opening.
        Entity::Opening(e) => {
            let p = &e.opening.place;
            if p.len() != 4 {
                rep.skip("Açıklık", "pervazları olmayan açıklık yazılmadı", 0);
                return None;
            }
            let ring = Ring::new(
                vec![p[0], p[2], p[3], p[1]],
                std::iter::empty::<Option<f64>>(),
            );
            area(ring, Vec::new(), rep, w)
        }
        // A record has no geometry (docs/adr/0410 §4); GeoJSON writes it itself, with a null one.
        Entity::Record(_) => {
            rep.skip("Kayıt", w.record, 0);
            return None;
        }
        // No raster (docs/adr/0204 §10).
        Entity::Raster(_) => {
            rep.skip("Raster", w.raster, 0);
            return None;
        }
        // Its line, as the server projects it (docs/adr/0146 §8).
        Entity::Leader(l) => {
            rep.note("Kılavuz", w.leader, 0);
            Some(Geometry::Line(Ring::flat(l.pts.clone())))
        }
        Entity::Insert(i) => {
            let pieces = blocks.placed(i);
            if pieces.is_empty() {
                rep.skip(
                    "Blok",
                    "bloğunun tanımı dışa aktarılanlarda yok ya da blok boş; yazılmadı",
                    0,
                );
                return None;
            }
            let members: Vec<Geometry> = pieces
                .iter()
                .filter_map(|p| geometry(p, blocks, rep, w))
                .collect();
            if members.is_empty() {
                rep.skip("Blok", w.block_empty, 0);
                return None;
            }
            rep.note("Blok", w.block, 0);
            Some(Geometry::Collection(members))
        }
        Entity::Xline(_) | Entity::Ray(_) => {
            rep.skip("Sonsuz doğru", w.infinite, 0);
            return None;
        }
    };
    let Some(g) = g else {
        rep.skip("Geçersiz nesne", "yeterli köşesi yok; yazılmadı", 0);
        return None;
    };
    let finite = |p: &Vec2| p.x.is_finite() && p.y.is_finite();
    let ok = match &g {
        Geometry::Point(p, z) => finite(p) && z.is_none_or(f64::is_finite),
        Geometry::Points(ps) => ps
            .iter()
            .all(|(p, z)| finite(p) && z.is_none_or(f64::is_finite)),
        Geometry::Line(r) => r.pts.iter().all(finite) && zs_finite(&r.zs),
        Geometry::Lines(paths) => paths
            .iter()
            .all(|r| r.pts.iter().all(finite) && zs_finite(&r.zs)),
        Geometry::Area(rings) => rings
            .iter()
            .all(|r| r.pts.iter().all(finite) && zs_finite(&r.zs)),
        Geometry::Areas(parts) => parts
            .iter()
            .flatten()
            .all(|r| r.pts.iter().all(finite) && zs_finite(&r.zs)),
        // Each member was checked (and said) as the object it is.
        Geometry::Collection(_) => true,
    };
    if !ok {
        rep.skip(
            "Sonlu olmayan koordinat",
            "koordinatı sayı değil ya da sonsuz; yazılmadı",
            0,
        );
        return None;
    }
    let mixes = match &g {
        Geometry::Line(r) => mixed(&[r]),
        Geometry::Lines(paths) => mixed(&paths.iter().collect::<Vec<_>>()),
        // Points with an elevation and points without (docs/adr/0174).
        Geometry::Points(ps) => {
            ps.iter().any(|(_, z)| z.is_some()) && ps.iter().any(|(_, z)| z.is_none())
        }
        Geometry::Area(rings) => mixed(&rings.iter().collect::<Vec<_>>()),
        Geometry::Areas(parts) => mixed(&parts.iter().flatten().collect::<Vec<_>>()),
        Geometry::Point(..) | Geometry::Collection(_) => false,
    };
    if mixes && let Some(said) = w.mixed_z {
        rep.note("Kotsuz köşe", said, 0);
    }
    Some(g)
}
