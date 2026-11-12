//! `cad.points.construct` v1 (docs/adr/0270): the points the drafting aids give a click, from what they are
//! made of: İki nokta ortası, Uzantı, Paralel, Nesne izleme's two alignments, a polar point and a distance along
//! a line. It writes nothing: Python and an agent take the same point the tools take. The shared cases are
//! `fixtures/commands/v1/cad.points.construct.json` (`scripts/fixtures/point_construct_cases.py`).
//!
//! The checks, construction by construction in order (the first that fails answers): one to 10 000
//! constructions; the numbers finite; an extension's distance zero or more; the points' and the edge's objects
//! (`invalid_uid`, `entity_not_found`, `no_such_place`, `piece_gone`); an extension's end an open end; Paralel's
//! object has a straight edge; Hat üzerinde's points apart.

use kentos_commands::{
    CommandResult, MeasuredPoint, ObjectEnd, PointConstruction, PointsConstruct, PointsConstructed,
};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::jsmath::{cos, js_hypot, js_sign, sin};
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::tools::ucs::Frame;
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;
use crate::dimension_create::{object, point_of, straight_edge_at};

/// How near an edge's end must be to the place to be its end, metres.
const AT_END: f64 = 1e-6;

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &PointsConstruct) -> CommandResult<()> {
    match construct(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The points; the same answer as [`execute`].
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &PointsConstruct,
) -> CommandResult<PointsConstructed> {
    answer(cx.doc, input)
}

/// The points. Writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: PointsConstruct,
) -> CommandResult<PointsConstructed> {
    answer(cx.doc, &input)
}

/// [`execute`] over the document alone.
pub fn answer(doc: &Document, input: &PointsConstruct) -> CommandResult<PointsConstructed> {
    match construct(doc, input) {
        Ok(points) => CommandResult::Completed {
            output: PointsConstructed {
                points: points
                    .into_iter()
                    .map(|p| kentos_commands::Vec2 { x: p.x, y: p.y })
                    .collect(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The drawing's UCS (docs/adr/0261 §1).
fn frame(doc: &Document) -> Frame {
    match doc.settings().ucs {
        Some(u) => Frame::new(Vec2::new(u.origin.x, u.origin.y), u.angle),
        None => Frame::WORLD,
    }
}

fn construct(doc: &Document, input: &PointsConstruct) -> Result<Vec<Vec2>, Stop> {
    let n = input.constructions.len();
    if n == 0 || n > kentos_commands::MAX_CONSTRUCTIONS {
        return Err(refuse(
            codes::NO_CONSTRUCTIONS,
            format!("{n} yapım verildi: bir ile 10 000 arasında yapım verin."),
            "constructions",
        ));
    }
    input
        .constructions
        .iter()
        .enumerate()
        .map(|(i, c)| one(doc, c, &format!("constructions[{i}]")))
        .collect()
}

/// A point's numbers and object id, before the drawing is read.
fn check_point(p: &MeasuredPoint, path: &str) -> Result<(), Stop> {
    match p {
        MeasuredPoint::At { at } => checks::point(*at, "Noktanın", &format!("{path}.at")),
        MeasuredPoint::Place { uid, .. } => uid_text(uid, path),
    }
}

fn uid_text(uid: &str, path: &str) -> Result<(), Stop> {
    if is_uid_text(uid) {
        return Ok(());
    }
    Err(refuse(
        codes::INVALID_UID,
        format!(
            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
        ),
        &format!("{path}.uid"),
    ))
}

fn distance(d: f64, path: &str) -> Result<(), Stop> {
    checks::finite(
        d,
        "Uzaklık",
        "Metre olarak bir sayı verin.",
        &format!("{path}.distance"),
    )
}

fn one(doc: &Document, c: &PointConstruction, path: &str) -> Result<Vec2, Stop> {
    match c {
        PointConstruction::Mid { a, b } => {
            let (pa, pb) = (format!("{path}.a"), format!("{path}.b"));
            check_point(a, &pa)?;
            check_point(b, &pb)?;
            let (a, b) = (point_of(doc, a, &pa)?, point_of(doc, b, &pb)?);
            Ok(Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0))
        }
        PointConstruction::Extension { end, distance: d } => {
            distance(*d, path)?;
            if *d < 0.0 {
                return Err(refuse(
                    codes::INVALID_DISTANCE,
                    "Uzantının uzaklığı sıfır ya da daha büyük olmalı: uç geride kalmaz.".into(),
                    &format!("{path}.distance"),
                ));
            }
            let pe = format!("{path}.end");
            uid_text(&end.uid, &pe)?;
            extension(doc, end, *d, &pe)
        }
        PointConstruction::Parallel {
            through,
            edge,
            distance: d,
        } => {
            let pt = format!("{path}.through");
            check_point(through, &pt)?;
            let pe = format!("{path}.edge");
            checks::point(
                edge.near,
                "Kenarın yakınındaki noktanın",
                &format!("{pe}.near"),
            )?;
            distance(*d, path)?;
            uid_text(&edge.uid, &pe)?;
            let from = point_of(doc, through, &pt)?;
            let e = object(doc, &edge.uid, &pe)?;
            // A curve's chords are not its edges: an ellipse's or a spline's direction is no straight edge's.
            let curved = matches!(
                crate::geometry::shape(e),
                Shape::Ellipse { .. } | Shape::Spline { .. } | Shape::Nurbs { .. }
            );
            let straight = (!curved)
                .then(|| straight_edge_at(e, Vec2::new(edge.near.x, edge.near.y)))
                .flatten();
            let (a, b) = straight.ok_or_else(|| {
                refuse(
                    codes::NO_STRAIGHT_EDGE,
                    "Nesnenin düz kenarı yok: paralelin doğrultusu düz bir kenardan alınır; çizgi, çoklu çizgi ya da alan verin.".into(),
                    &pe,
                )
            })?;
            let len = js_hypot(b.x - a.x, b.y - a.y);
            let u = Vec2::new((b.x - a.x) / len, (b.y - a.y) / len);
            Ok(Vec2::new(from.x + u.x * d, from.y + u.y * d))
        }
        PointConstruction::Track { x, y } => {
            let (px, py) = (format!("{path}.x"), format!("{path}.y"));
            check_point(x, &px)?;
            check_point(y, &py)?;
            let (x, y) = (point_of(doc, x, &px)?, point_of(doc, y, &py)?);
            let f = frame(doc);
            Ok(f.to_world(Vec2::new(f.to_frame(x).x, f.to_frame(y).y)))
        }
        PointConstruction::Polar {
            from,
            angle,
            distance: d,
        } => {
            let pf = format!("{path}.from");
            check_point(from, &pf)?;
            checks::finite(
                *angle,
                "Açı",
                "Radyan olarak bir sayı verin.",
                &format!("{path}.angle"),
            )?;
            distance(*d, path)?;
            let o = point_of(doc, from, &pf)?;
            let u = frame(doc).dir_to_world(Vec2::new(cos(*angle), sin(*angle)));
            Ok(Vec2::new(o.x + u.x * d, o.y + u.y * d))
        }
        PointConstruction::Along { a, b, distance: d } => {
            let (pa, pb) = (format!("{path}.a"), format!("{path}.b"));
            check_point(a, &pa)?;
            check_point(b, &pb)?;
            distance(*d, path)?;
            let (a, b) = (point_of(doc, a, &pa)?, point_of(doc, b, &pb)?);
            let len = js_hypot(b.x - a.x, b.y - a.y);
            if len < 1e-9 {
                return Err(refuse(
                    codes::SAME_POINTS,
                    "Hattın iki noktası aynı yerde; aralarında bir doğrultu yok. Ayrı iki nokta verin.".into(),
                    &format!("{path}.b"),
                ));
            }
            let t = d / len;
            Ok(Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t))
        }
    }
}

/// The point `d` past the open end `end` names, along the one edge that ends there.
fn extension(doc: &Document, end: &ObjectEnd, d: f64, path: &str) -> Result<Vec2, Stop> {
    let at = point_of(
        doc,
        &MeasuredPoint::Place {
            uid: end.uid.clone(),
            place: end.place,
            path: end.path.clone(),
        },
        path,
    )?;
    let e = object(doc, &end.uid, path)?;
    let shape = crate::blocks::place_shape(doc.blocks(), e, &end.path).ok_or_else(|| {
        refuse(
            codes::PIECE_GONE,
            crate::blocks::NO_PIECE.into(),
            &format!("{path}.path"),
        )
    })?;
    let not_an_end = || {
        refuse(
            codes::NOT_AN_END,
            "Yer nesnenin açık bir ucu değil: uzantı çizginin, açık çoklu çizginin ya da yayın ucundan başlar (kapalı yolun ya da yolun içindeki köşe olmaz).".into(),
            &format!("{path}.place"),
        )
    };
    if !matches!(
        shape,
        Shape::Line { .. } | Shape::Polyline { .. } | Shape::Arc { .. }
    ) {
        return Err(not_an_end());
    }
    let near = |p: Vec2| js_hypot(p.x - at.x, p.y - at.y) <= AT_END;
    let mut found: Vec<(Edge, bool)> = Vec::new();
    for edge in entity_edges(&shape) {
        let (s, t) = match edge {
            Edge::Seg { a, b } => (a, b),
            Edge::Arc { c, r, a0, sweep } => (
                Vec2::new(c.x + r * cos(a0), c.y + r * sin(a0)),
                Vec2::new(c.x + r * cos(a0 + sweep), c.y + r * sin(a0 + sweep)),
            ),
        };
        if near(s) {
            found.push((edge, true));
        }
        if near(t) {
            found.push((edge, false));
        }
    }
    let [(edge, at_start)] = found[..] else {
        return Err(not_an_end());
    };
    Ok(match edge {
        Edge::Seg { a, b } => {
            let (from, to) = if at_start { (b, a) } else { (a, b) };
            let len = js_hypot(to.x - from.x, to.y - from.y);
            Vec2::new(
                to.x + (to.x - from.x) / len * d,
                to.y + (to.y - from.y) / len * d,
            )
        }
        Edge::Arc { c, r, a0, sweep } => {
            let turn = d / r * js_sign(sweep);
            let t = if at_start {
                a0 - turn
            } else {
                a0 + sweep + turn
            };
            Vec2::new(c.x + r * cos(t), c.y + r * sin(t))
        }
    })
}
