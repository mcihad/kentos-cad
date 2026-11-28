//! The elements' base quantities (docs/adr/8071 §2; EK-6's optional rows: Tablo 6.133 the wall's, 6.123 the slab's, the
//! column's, the beam's and 6.88 the covering's; IFC 4.3's `Qto_…BaseQuantities`), from their bodies as the file writes
//! them. Every area and volume comes from the rings with their arcs, exactly: a ring's shoelace plus each arc's circular
//! segment, an opening taken out by the geometry core's overlay, which keeps arcs as arcs. None comes from a flattened
//! outline or from triangles (the drawing's step is the view's, never the measure's). Lengths are nominal, the source's:
//! a wall's along its centre line (EK-6: "orta hat boyunca", its arcs' own lengths), a beam's along its line, a column's
//! its height.

use kentos_contracts::{BeamFields, WallFields};
use kentos_geometry_core::Vec2 as P;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::bulge::{
    bulge_path_edges, bulge_path_length, bulge_ring_area, has_bulges, reverse_bulge_path,
};
use kentos_geometry_core::geom::intersect::{Edge, intersect_edges, point_at};
use kentos_geometry_core::geom::region::{
    inside_area, net_area, ring_area, ring_edges, subtract_areas,
};
use kentos_geometry_core::geom::wall::line_at;
use kentos_geometry_core::ops::edges::edge_length;

use crate::building::{Opening, Qto, Shape, Value, flat_rings, nest};

/// A prism's footprint as the overlay's areas: each outer ring with its holes (even-odd, nested as the file nests them),
/// arcs kept.
pub fn areas(rings: &[Vec<[f64; 2]>], bulges: &[Vec<f64>]) -> Vec<Area> {
    let flat = flat_rings(rings, bulges);
    let kept: Vec<usize> = (0..rings.len()).filter(|&i| flat[i].len() >= 3).collect();
    let nested: Vec<Vec<[f64; 2]>> = kept.iter().map(|&i| flat[i].clone()).collect();
    let ring = |i: usize| Ring {
        pts: rings[i].iter().map(|p| P::new(p[0], p[1])).collect(),
        bulges: bulges.get(i).filter(|b| has_bulges(Some(b))).cloned(),
    };
    nest(&nested)
        .into_iter()
        .map(|(o, holes)| Area {
            outer: ring(kept[o]),
            holes: holes.into_iter().map(|h| ring(kept[h])).collect(),
        })
        .collect()
}

/// A footprint's area: its outer rings less their holes.
pub fn area(rings: &[Vec<[f64; 2]>], bulges: &[Vec<f64>]) -> f64 {
    areas(rings, bulges).iter().map(net_area).sum()
}

/// A footprint's outer rings' area, its holes not taken out: a slab's openings are not its gross area's business (EK-6
/// Tablo 6.123: "Açıklıklar, girintiler, çıkıntılar ve kesit boşlukları dikkate alınmaz").
pub fn outer_area(rings: &[Vec<[f64; 2]>], bulges: &[Vec<f64>]) -> f64 {
    areas(rings, bulges)
        .iter()
        .map(|a| ring_area(&a.outer).abs())
        .sum()
}

/// A shape's volume: a prism's footprint by its depth; a closed set of plane faces by the divergence theorem, each face
/// a fan from its first vertex (exact for plane faces).
pub fn volume(shape: &Shape) -> f64 {
    match shape {
        Shape::Prism {
            rings,
            bulges,
            bottom,
            top,
        } => area(rings, bulges) * (top - bottom),
        Shape::Faces { vertices, faces } => {
            let mut v = 0.0;
            for f in faces {
                for k in 1..f.len().saturating_sub(1) {
                    let (a, b, c) = (vertices[f[0]], vertices[f[k]], vertices[f[k + 1]]);
                    v += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                        + a[2] * (b[0] * c[1] - b[1] * c[0]))
                        / 6.0;
                }
            }
            v.abs()
        }
    }
}

/// An opening's footprint as the overlay's area.
fn cut_area(o: &Opening) -> Vec<Area> {
    areas(
        std::slice::from_ref(&o.ring),
        std::slice::from_ref(&o.bulges),
    )
}

/// `shapes` with `cuts` taken out (docs/adr/8071 §4; each opening's prism from its bottom to its top, over the same
/// floor): each prism cut in bands at the openings' sills and heads, each band's footprint less the openings standing
/// in it, by the overlay, arcs kept; the bands no opening stands in stay whole, as one prism each run. Faces stay as
/// they are. The cut body of a Reference View and of the 3D view, and the net volume's.
pub fn cut(shapes: &[Shape], cuts: &[&Opening]) -> Vec<Shape> {
    let mut out = Vec::new();
    for s in shapes {
        let Shape::Prism {
            rings,
            bulges,
            bottom,
            top,
        } = s
        else {
            out.push(s.clone());
            continue;
        };
        let mut levels = vec![*bottom, *top];
        for c in cuts {
            for z in [c.bottom, c.top] {
                if z > bottom + 1e-9 && z < top - 1e-9 {
                    levels.push(z);
                }
            }
        }
        levels.sort_by(f64::total_cmp);
        levels.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        let whole = |from: f64, to: f64| Shape::Prism {
            rings: rings.clone(),
            bulges: bulges.clone(),
            bottom: from,
            top: to,
        };
        let base = areas(rings, bulges);
        let mut run: Option<f64> = None; // where the bands no opening stands in began
        for w in levels.windows(2) {
            let mid = (w[0] + w[1]) / 2.0;
            let cutters: Vec<Area> = cuts
                .iter()
                .filter(|c| c.bottom < mid && mid < c.top)
                .flat_map(|c| cut_area(c))
                .collect();
            if cutters.is_empty() {
                run.get_or_insert(w[0]);
                continue;
            }
            if let Some(from) = run.take() {
                out.push(whole(from, w[0]));
            }
            out.extend(
                subtract_areas(&base, &cutters)
                    .iter()
                    .map(|a| prism_of(a, w[0], w[1])),
            );
        }
        if let Some(from) = run {
            out.push(whole(from, *top));
        }
    }
    out
}

/// An overlay's area as a prism between two heights: its outer ring and its holes, their arcs.
fn prism_of(a: &Area, bottom: f64, top: f64) -> Shape {
    let rings: Vec<&Ring> = std::iter::once(&a.outer).chain(&a.holes).collect();
    Shape::Prism {
        rings: rings
            .iter()
            .map(|r| r.pts.iter().map(|p| [p.x, p.y]).collect())
            .collect(),
        bulges: rings
            .iter()
            .map(|r| r.bulges.clone().unwrap_or_else(|| vec![0.0; r.pts.len()]))
            .collect(),
        bottom,
        top,
    }
}

/// What is left of `shapes` once `cuts` are taken out: the volume of their cut body.
pub fn net_volume(shapes: &[Shape], cuts: &[&Opening]) -> f64 {
    cut(shapes, cuts).iter().map(volume).sum()
}

/// How long the path of `edges` runs inside `area`: each edge cut where it crosses the area's rings, each piece whose
/// middle is inside counted (exact on arcs: an arc's parameter runs with its angle).
fn length_inside(edges: &[Edge], area: &Area) -> f64 {
    let rings: Vec<Edge> = std::iter::once(&area.outer)
        .chain(&area.holes)
        .flat_map(ring_edges)
        .collect();
    let mut total = 0.0;
    for e in edges {
        let mut ts = vec![0.0, 1.0];
        for r in &rings {
            ts.extend(
                intersect_edges(e, r)
                    .into_iter()
                    .map(|h| h.t)
                    .filter(|t| *t > 0.0 && *t < 1.0),
            );
        }
        ts.sort_by(f64::total_cmp);
        for w in ts.windows(2) {
            if w[1] - w[0] > 1e-12 && inside_area(area, point_at(e, (w[0] + w[1]) / 2.0)) {
                total += edge_length(e) * (w[1] - w[0]);
            }
        }
    }
    total
}

/// The prisms' common bottom and top, when they share them.
fn span(shapes: &[Shape]) -> Option<(f64, f64)> {
    let mut out: Option<(f64, f64)> = None;
    for s in shapes {
        let Shape::Prism { bottom, top, .. } = s else {
            return None;
        };
        match out {
            None => out = Some((*bottom, *top)),
            Some((b, t)) if (b - bottom).abs() > 1e-9 || (t - top).abs() > 1e-9 => return None,
            Some(_) => {}
        }
    }
    out
}

/// The line `d` to the left of a path, as edges; none when it cannot be offset (an arc it would collapse).
fn line_edges(
    pts: &[kentos_contracts::Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    d: f64,
) -> Option<Vec<Edge>> {
    let pts: Vec<P> = pts.iter().map(|q| P::new(q.x, q.y)).collect();
    let line = line_at(&pts, bulges, closed, d).ok()?;
    Some(bulge_path_edges(&line.pts, Some(&line.bulges), closed))
}

/// A wall's own body in plan, without its joins: the band between its two faces (an open wall's left face on, straight
/// across its end, its right face back; a closed wall's ring between them), exact on its arcs.
fn own_area(fields: &WallFields) -> Option<f64> {
    let pts: Vec<P> = fields.pts.iter().map(|q| P::new(q.x, q.y)).collect();
    let left = fields.justify.left_face(fields.width) + fields.offset;
    let b = fields.bulges.as_deref();
    let l = line_at(&pts, b, fields.closed, left).ok()?;
    let r = line_at(&pts, b, fields.closed, left - fields.width).ok()?;
    if fields.closed {
        return Some(
            (bulge_ring_area(&l.pts, Some(&l.bulges)).abs()
                - bulge_ring_area(&r.pts, Some(&r.bulges)).abs())
            .abs(),
        );
    }
    let back = reverse_bulge_path(&r.pts, Some(&r.bulges), false);
    let mut ring = l.pts.clone();
    let mut bulges = l.bulges.clone();
    if let Some(x) = bulges.last_mut() {
        *x = 0.0;
    }
    let n = back.pts.len();
    ring.extend(back.pts);
    bulges.extend(back.bulges.unwrap_or_else(|| vec![0.0; n]));
    Some(bulge_ring_area(&ring, Some(&bulges)).abs())
}

/// A wall's base quantities (EK-6 Tablo 6.133; `Qto_WallBaseQuantities`, IFC 4.3's definitions): its centre line's
/// length; its width and its height (a KentOS wall's are constant along it); its middle plane's area seen square-on,
/// less where its openings go through that plane; its gross volume its own body's, "without taking into account the
/// openings and the connection geometry" (its joins, the columns that cut it); its net volume its body's as the file
/// writes it (joined and cut) less what its openings and recesses take out.
pub fn wall(fields: &WallFields, shapes: &[Shape], cuts: &[&Opening]) -> Qto {
    let mut q: Vec<(String, Value)> = Vec::new();
    let middle = fields.justify.left_face(fields.width) + fields.offset - fields.width / 2.0;
    let centre = line_edges(&fields.pts, fields.bulges.as_deref(), fields.closed, middle);
    let length = centre
        .as_ref()
        .map(|edges| edges.iter().map(edge_length).sum::<f64>());
    if let Some(l) = length {
        q.push(("Length".into(), Value::Length(l)));
    }
    q.push(("Width".into(), Value::Length(fields.width)));
    let height = span(shapes);
    if let Some((bottom, top)) = height {
        q.push(("Height".into(), Value::Length(top - bottom)));
        if let (Some(l), Some(edges)) = (length, &centre) {
            let through: f64 = cuts
                .iter()
                .map(|c| {
                    let rise = c.top.min(top) - c.bottom.max(bottom);
                    if rise <= 0.0 {
                        0.0
                    } else {
                        cut_area(c)
                            .iter()
                            .map(|a| length_inside(edges, a))
                            .sum::<f64>()
                            * rise
                    }
                })
                .sum();
            q.push((
                "NetSideArea".into(),
                Value::Area(l * (top - bottom) - through),
            ));
        }
    }
    if let (Some(a), Some((bottom, top))) = (own_area(fields), height) {
        q.push(("GrossVolume".into(), Value::Volume(a * (top - bottom))));
    }
    q.push(("NetVolume".into(), Value::Volume(net_volume(shapes, cuts))));
    Qto {
        name: "Qto_WallBaseQuantities".into(),
        quantities: q,
    }
}

/// A slab's base quantities (EK-6 Tablo 6.123; `Qto_SlabBaseQuantities`): its thickness (its layers' together); its
/// footprint's area with its holes in (gross) and taken out (net), less its openings; its volume likewise.
pub fn slab(shapes: &[Shape], cuts: &[&Opening]) -> Option<Qto> {
    let (rings, bulges) = shapes.iter().find_map(|s| match s {
        Shape::Prism { rings, bulges, .. } => Some((rings, bulges)),
        Shape::Faces { .. } => None,
    })?;
    let (bottom, top) = shapes
        .iter()
        .try_fold((f64::MAX, f64::MIN), |(b, t), s| match s {
            Shape::Prism { bottom, top, .. } => Some((b.min(*bottom), t.max(*top))),
            Shape::Faces { .. } => None,
        })?;
    let gross = outer_area(rings, bulges);
    let base = areas(rings, bulges);
    let through: Vec<Area> = cuts
        .iter()
        .filter(|c| c.bottom <= bottom + 1e-9 && c.top >= top - 1e-9)
        .flat_map(|c| cut_area(c))
        .collect();
    let net: f64 = if through.is_empty() {
        base.iter().map(net_area).sum()
    } else {
        subtract_areas(&base, &through).iter().map(net_area).sum()
    };
    Some(Qto {
        name: "Qto_SlabBaseQuantities".into(),
        quantities: vec![
            ("Width".into(), Value::Length(top - bottom)),
            ("GrossArea".into(), Value::Area(gross)),
            ("NetArea".into(), Value::Area(net)),
            ("GrossVolume".into(), Value::Volume(gross * (top - bottom))),
            ("NetVolume".into(), Value::Volume(net_volume(shapes, cuts))),
        ],
    })
}

/// A sloped slab's base quantities (docs/adr/8071 §6; `Qto_SlabBaseQuantities`): its thickness square to its top (its
/// layers' together); its area as it lies, its plan's (exact on its arcs) by √(1 + grade²); its volume its plan's area by
/// its upright thickness. A sloped slab has no hole.
pub fn sloped_slab(fields: &kentos_contracts::SlabFields) -> Option<Qto> {
    let slope = fields.slope?;
    let k = slope.upright();
    let plan: f64 = fields
        .parts
        .iter()
        .map(|part| {
            let pts: Vec<[f64; 2]> = part.outer.pts.iter().map(|q| [q.x, q.y]).collect();
            let bulges = part.outer.bulges.clone().unwrap_or_default();
            area(&[pts], &[bulges])
        })
        .sum();
    let width = fields.whole_thickness();
    Some(Qto {
        name: "Qto_SlabBaseQuantities".into(),
        quantities: vec![
            ("Width".into(), Value::Length(width)),
            ("GrossArea".into(), Value::Area(plan * k)),
            ("NetArea".into(), Value::Area(plan * k)),
            ("GrossVolume".into(), Value::Volume(plan * width * k)),
            ("NetVolume".into(), Value::Volume(plan * width * k)),
        ],
    })
}

/// A covering's base quantities (EK-6 Tablo 6.88; `Qto_CoveringBaseQuantities`): its area with its holes in (gross) and
/// taken out (net).
pub fn covering(shapes: &[Shape]) -> Option<Qto> {
    let (rings, bulges) = shapes.iter().find_map(|s| match s {
        Shape::Prism { rings, bulges, .. } => Some((rings, bulges)),
        Shape::Faces { .. } => None,
    })?;
    Some(Qto {
        name: "Qto_CoveringBaseQuantities".into(),
        quantities: vec![
            ("GrossArea".into(), Value::Area(outer_area(rings, bulges))),
            ("NetArea".into(), Value::Area(area(rings, bulges))),
        ],
    })
}

/// A column's base quantities (EK-6, `Qto_ColumnBaseQuantities`): its height, its section's area, its volume. A column
/// the slab over it cuts (docs/adr/8171 §2.1) is prisms of more than one height: its height runs from its lowest foot
/// to its highest top, its section is that of the prisms standing on its foot (together its whole section).
pub fn column(shapes: &[Shape], cuts: &[&Opening]) -> Option<Qto> {
    let mut prisms = Vec::new();
    for s in shapes {
        let Shape::Prism {
            rings,
            bulges,
            bottom,
            top,
        } = s
        else {
            return None;
        };
        prisms.push((area(rings, bulges), *bottom, *top));
    }
    let bottom = prisms.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let top = prisms.iter().map(|p| p.2).fold(f64::NEG_INFINITY, f64::max);
    if prisms.is_empty() {
        return None;
    }
    let section: f64 = prisms
        .iter()
        .filter(|p| (p.1 - bottom).abs() <= 1e-9)
        .map(|p| p.0)
        .sum();
    Some(Qto {
        name: "Qto_ColumnBaseQuantities".into(),
        quantities: vec![
            ("Length".into(), Value::Length(top - bottom)),
            ("CrossSectionArea".into(), Value::Area(section)),
            (
                "GrossVolume".into(),
                Value::Volume(shapes.iter().map(volume).sum()),
            ),
            ("NetVolume".into(), Value::Volume(net_volume(shapes, cuts))),
        ],
    })
}

/// A beam's base quantities (EK-6, `Qto_BeamBaseQuantities`): its length along its centre line, its section (its width
/// by its depth), its body's volume.
pub fn beam(fields: &BeamFields, shapes: &[Shape], cuts: &[&Opening]) -> Qto {
    let middle = fields.justify.left_face(fields.width) + fields.offset - fields.width / 2.0;
    let mut q: Vec<(String, Value)> = Vec::new();
    if let Some(edges) = line_edges(&fields.pts, None, false, middle) {
        q.push((
            "Length".into(),
            Value::Length(edges.iter().map(edge_length).sum()),
        ));
    }
    q.push((
        "CrossSectionArea".into(),
        Value::Area(fields.width * fields.height),
    ));
    q.push((
        "GrossVolume".into(),
        Value::Volume(shapes.iter().map(volume).sum()),
    ));
    q.push(("NetVolume".into(), Value::Volume(net_volume(shapes, cuts))));
    Qto {
        name: "Qto_BeamBaseQuantities".into(),
        quantities: q,
    }
}

/// A footing's base quantities (IFC 4.3 `Qto_FootingBaseQuantities`; docs/adr/8172): its length along its axis, its
/// width, its height (its thickness), its section, its body's volume.
pub fn footing(fields: &BeamFields, shapes: &[Shape], cuts: &[&Opening]) -> Qto {
    let mut q = beam(fields, shapes, cuts);
    q.name = "Qto_FootingBaseQuantities".into();
    // After Length: its width and its height, then its section and volumes as a beam's.
    let at = q
        .quantities
        .iter()
        .position(|(n, _)| n == "Length")
        .map_or(0, |k| k + 1);
    q.quantities
        .insert(at, ("Width".into(), Value::Length(fields.width)));
    q.quantities
        .insert(at + 1, ("Height".into(), Value::Length(fields.height)));
    q
}

/// A path's length with its arcs' own lengths.
pub fn path_length(pts: &[[f64; 2]], bulges: &[f64], closed: bool) -> f64 {
    let pts: Vec<P> = pts.iter().map(|p| P::new(p[0], p[1])).collect();
    bulge_path_length(&pts, Some(bulges), closed)
}

#[cfg(test)]
// A test's own arithmetic may use the platform's functions: its values are compared with a tolerance, never written.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn opening(ring: Vec<[f64; 2]>, bulges: Vec<f64>, bottom: f64, top: f64) -> Opening {
        Opening {
            id: None,
            name: String::new(),
            host: 0,
            ring,
            bulges,
            bottom,
            top,
            filler: None,
            predefined: "OPENING",
        }
    }

    /// A quarter of a ring 0.30 m wide round a 6 m radius, 3 m high, with a window between 30° and 40° from 0.9 to 2.1 m:
    /// its gross volume the closed form's (r₁² − r₂²)/2 · π/2 · h, its net that less the window's piece of the ring.
    #[test]
    fn a_curved_wall_s_volumes_are_the_closed_form_s() {
        let (r1, r2) = (6.15, 5.85);
        let quarter = (std::f64::consts::PI / 8.0).tan();
        let shapes = vec![Shape::Prism {
            rings: vec![vec![[r1, 0.0], [0.0, r1], [0.0, r2], [r2, 0.0]]],
            bulges: vec![vec![quarter, 0.0, -quarter, 0.0]],
            bottom: 0.0,
            top: 3.0,
        }];
        let gross = (r1 * r1 - r2 * r2) / 2.0 * std::f64::consts::FRAC_PI_2 * 3.0;
        assert!(
            (volume(&shapes[0]) - gross).abs() < 1e-12,
            "{}",
            volume(&shapes[0])
        );
        let at = |r: f64, deg: f64| {
            let a: f64 = deg.to_radians();
            [r * a.cos(), r * a.sin()]
        };
        let ten = (10f64.to_radians() / 4.0).tan();
        let window = opening(
            vec![at(r1, 30.0), at(r2, 30.0), at(r2, 40.0), at(r1, 40.0)],
            vec![0.0, ten, 0.0, -ten],
            0.9,
            2.1,
        );
        let piece = (r1 * r1 - r2 * r2) / 2.0 * 10f64.to_radians() * 1.2;
        let net = net_volume(&shapes, &[&window]);
        assert!(
            (net - (gross - piece)).abs() < 1e-9,
            "{net} {}",
            gross - piece
        );
    }

    /// The same wall cut (docs/adr/8071 §4): whole under the window's sill and over its head, two pieces of the ring
    /// beside it, each with its arcs; together the net volume, each a closed body.
    #[test]
    fn a_curved_wall_s_cut_body_is_its_bands_with_their_arcs() {
        let (r1, r2) = (6.15, 5.85);
        let quarter = (std::f64::consts::PI / 8.0).tan();
        let shapes = vec![Shape::Prism {
            rings: vec![vec![[r1, 0.0], [0.0, r1], [0.0, r2], [r2, 0.0]]],
            bulges: vec![vec![quarter, 0.0, -quarter, 0.0]],
            bottom: 0.0,
            top: 3.0,
        }];
        let at = |r: f64, deg: f64| {
            let a: f64 = deg.to_radians();
            [r * a.cos(), r * a.sin()]
        };
        let ten = (10f64.to_radians() / 4.0).tan();
        let window = opening(
            vec![at(r1, 30.0), at(r2, 30.0), at(r2, 40.0), at(r1, 40.0)],
            vec![0.0, ten, 0.0, -ten],
            0.9,
            2.1,
        );
        let body = cut(&shapes, &[&window]);
        let spans: Vec<(f64, f64)> = body
            .iter()
            .map(|s| match s {
                Shape::Prism { bottom, top, .. } => (*bottom, *top),
                Shape::Faces { .. } => panic!("faces"),
            })
            .collect();
        assert_eq!(
            spans,
            vec![(0.0, 0.9), (0.9, 2.1), (0.9, 2.1), (2.1, 3.0)],
            "{body:?}"
        );
        for s in &body {
            assert_eq!(crate::closed::faults(s), Vec::<String>::new(), "{s:?}");
            let Shape::Prism { bulges, .. } = s else {
                unreachable!()
            };
            assert!(
                bulges.iter().flatten().any(|b| *b != 0.0),
                "its arcs kept: {s:?}"
            );
        }
        let gross = (r1 * r1 - r2 * r2) / 2.0 * std::f64::consts::FRAC_PI_2 * 3.0;
        let piece = (r1 * r1 - r2 * r2) / 2.0 * 10f64.to_radians() * 1.2;
        let v: f64 = body.iter().map(volume).sum();
        assert!((v - (gross - piece)).abs() < 1e-9, "{v}");
    }

    /// A straight wall 5 m long, 0.25 m wide, 3 m high on its centre line, a 1 m window from 0.9 to 2.1 m through it and
    /// a niche 0.10 m deep: its middle plane's area less the window only, its volume less both.
    #[test]
    fn a_wall_s_quantities_take_out_what_goes_through() {
        let fields: WallFields = serde_json::from_value(serde_json::json!({
            "pts": [{"x": 0.0, "y": 0.0}, {"x": 5.0, "y": 0.0}],
            "width": 0.25,
            "justify": "center"
        }))
        .expect("a wall");
        let shapes = vec![Shape::Prism {
            rings: vec![vec![
                [0.0, -0.125],
                [5.0, -0.125],
                [5.0, 0.125],
                [0.0, 0.125],
            ]],
            bulges: vec![],
            bottom: 0.0,
            top: 3.0,
        }];
        let window = opening(
            vec![[2.0, -0.125], [2.0, 0.125], [3.0, 0.125], [3.0, -0.125]],
            vec![],
            0.9,
            2.1,
        );
        let niche = opening(
            vec![[0.5, 0.025], [0.5, 0.125], [1.5, 0.125], [1.5, 0.025]],
            vec![],
            0.0,
            2.0,
        );
        let q = wall(&fields, &shapes, &[&window, &niche]);
        let get = |k: &str| {
            q.quantities
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(get("Length"), Some(Value::Length(5.0)));
        assert_eq!(get("Width"), Some(Value::Length(0.25)));
        assert_eq!(get("Height"), Some(Value::Length(3.0)));
        let num = |v: Option<Value>| match v {
            Some(Value::Area(x) | Value::Volume(x) | Value::Length(x)) => x,
            other => panic!("{other:?}"),
        };
        assert!((num(get("NetSideArea")) - (15.0 - 1.2)).abs() < 1e-12);
        assert!((num(get("GrossVolume")) - 3.75).abs() < 1e-12);
        assert!((num(get("NetVolume")) - (3.75 - 0.3 - 0.2)).abs() < 1e-12);
    }
}
