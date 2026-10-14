//! `cad.curves.construct` v1 (docs/adr/0289): Daire's, Yay's and Elips's methods from what they are made of: the
//! circle on two points, through three, tangent to two objects with a radius or to three; the arc through three
//! points, from a start round a centre to an end, an angle or a chord, from a start to an end with an angle, a
//! direction or a radius, and on from an object's end (Devam); the ellipse from its axis ends or its centre,
//! whole or an arc. It writes nothing: the answer is what `cad.circle.create`, `cad.arc.create` and
//! `cad.entities.create`'s ellipse take. The tools compute their curves here too ([`curve`]), so a tool and a
//! script refuse the same input with the same words. The shared cases are
//! `fixtures/commands/v1/cad.curves.construct.json` (`scripts/fixtures/curve_method_cases.py`).
//!
//! The checks, construction by construction in order (the first that fails answers): one to 10 000
//! constructions; the construction's numbers finite, in its fields' order; its objects' ids, the objects and
//! their places; then what its kind asks: points apart, not on a line, angles, radii, chords and half-axes that
//! make a curve, objects a circle can touch, a circle that touches them.

use kentos_contracts::{
    CommandResult, ConstructedCurve, CurveConstruction, CurvesConstruct, CurvesConstructed,
    EdgeNear, MeasuredPoint, ObjectEnd, SubPlace,
};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arc::{ArcGeom, arc_through, circle_through};
use kentos_geometry_core::geom::ellipse::{
    EllipseGeom, ellipse_from_axis, ellipse_from_center, param_at_polar,
};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::shapes::{
    arc_start_center_chord, arc_start_center_sweep, arc_start_center_to, arc_start_end_direction,
    arc_start_end_radius, arc_start_end_sweep,
};
use kentos_geometry_core::geom::tangent_circle::{
    parallel_gap, tangent_tangent_radius, tangent_tangent_tangent,
};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::{PI, cos, sin};
use kentos_geometry_core::ops::edges::nearest_edge;
use kentos_geometry_core::tools::drawing::{circle_on_diameter, end_tangent, start_tangent};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;
use crate::dimension_create::{object, point_of};

/// Two points closer than this are one place, metres.
const SAME: f64 = 1e-9;
/// The largest rotation of an ellipse's first axis (AutoCAD's 89.4°).
pub const MAX_ROTATION: f64 = (89.4 * PI) / 180.0;

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &CurvesConstruct) -> CommandResult<()> {
    match construct(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The curves; the same answer as [`execute`].
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &CurvesConstruct,
) -> CommandResult<CurvesConstructed> {
    answer(cx.doc, input)
}

/// The curves. Writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: CurvesConstruct,
) -> CommandResult<CurvesConstructed> {
    answer(cx.doc, &input)
}

/// [`execute`] over the document alone.
pub fn answer(doc: &Document, input: &CurvesConstruct) -> CommandResult<CurvesConstructed> {
    match construct(doc, input) {
        Ok(curves) => CommandResult::Completed {
            output: CurvesConstructed { curves },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// One construction's curve, or the refusal's message: what the tools draw and say (docs/adr/0289 §2).
pub fn curve(doc: &Document, c: &CurveConstruction) -> Result<ConstructedCurve, String> {
    one(Some(doc), c, "constructions[0]").map_err(|stop| stop.into_error().message)
}

/// [`curve`] for a construction of coordinates alone, without a drawing: a tool's preview. One that names an
/// object is refused (`entity_not_found`).
pub fn curve_at(c: &CurveConstruction) -> Result<ConstructedCurve, String> {
    one(None, c, "constructions[0]").map_err(|stop| stop.into_error().message)
}

/// The drawing a place or an object is read in; none without one.
fn drawing<'d>(doc: Option<&'d Document>, path: &str) -> Result<&'d Document, Stop> {
    doc.ok_or_else(|| {
        refuse(
            codes::ENTITY_NOT_FOUND,
            "Nesneye başvuran yapım bir çizimde hesaplanır.",
            &format!("{path}.uid"),
        )
    })
}

/// A point now, in the drawing when it names a place.
fn point_in(doc: Option<&Document>, p: &MeasuredPoint, path: &str) -> Result<Vec2, Stop> {
    match p {
        MeasuredPoint::At { at } => Ok(v(*at)),
        MeasuredPoint::Place { .. } => point_of(drawing(doc, path)?, p, path),
    }
}

/// An arc as the core keeps it.
pub fn arc_geom(c: &ConstructedCurve) -> Option<ArcGeom> {
    match *c {
        ConstructedCurve::Arc { c, r, a0, a1 } => Some(ArcGeom { c: v(c), r, a0, a1 }),
        _ => None,
    }
}

/// An ellipse as the core keeps it.
pub fn ellipse_geom(c: &ConstructedCurve) -> Option<EllipseGeom> {
    match *c {
        ConstructedCurve::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => Some(EllipseGeom {
            c: v(c),
            major: v(major),
            ratio,
            t0,
            t1,
        }),
        _ => None,
    }
}

fn v(p: kentos_contracts::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn w(p: Vec2) -> kentos_contracts::Vec2 {
    kentos_contracts::Vec2 { x: p.x, y: p.y }
}

fn refuse(code: &str, message: &str, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn construct(doc: &Document, input: &CurvesConstruct) -> Result<Vec<ConstructedCurve>, Stop> {
    let n = input.constructions.len();
    if n == 0 || n > kentos_contracts::MAX_CONSTRUCTIONS {
        return Err(refuse(
            codes::NO_CONSTRUCTIONS,
            &format!("{n} yapım verildi: bir ile 10 000 arasında yapım verin."),
            "constructions",
        ));
    }
    input
        .constructions
        .iter()
        .enumerate()
        .map(|(i, c)| one(Some(doc), c, &format!("constructions[{i}]")))
        .collect()
}

/// A point's numbers, before the drawing is read.
fn numbers(p: &MeasuredPoint, path: &str) -> Result<(), Stop> {
    match p {
        MeasuredPoint::At { at } => checks::point(*at, "Noktanın", &format!("{path}.at")),
        MeasuredPoint::Place { .. } => Ok(()),
    }
}

fn number(value: f64, what: &str, fix: &str, path: &str) -> Result<(), Stop> {
    checks::finite(value, what, fix, path)
}

fn angle_number(value: f64, path: &str) -> Result<(), Stop> {
    number(value, "Açı", "Radyan olarak bir sayı verin.", path)
}

fn length_number(value: f64, what: &str, path: &str) -> Result<(), Stop> {
    number(value, what, "Metre olarak bir sayı verin.", path)
}

fn uid_text(uid: &str, path: &str) -> Result<(), Stop> {
    if is_uid_text(uid) {
        return Ok(());
    }
    Err(refuse(
        codes::INVALID_UID,
        &format!(
            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
        ),
        &format!("{path}.uid"),
    ))
}

fn place_uid(p: &MeasuredPoint, path: &str) -> Result<(), Stop> {
    match p {
        MeasuredPoint::Place { uid, .. } => uid_text(uid, path),
        MeasuredPoint::At { .. } => Ok(()),
    }
}

/// The points named `names` in `path`, their numbers then their ids checked first, then read.
fn points<const N: usize>(
    doc: Option<&Document>,
    path: &str,
    named: [(&str, &MeasuredPoint); N],
    more: impl FnOnce() -> Result<(), Stop>,
) -> Result<[Vec2; N], Stop> {
    for (name, p) in named {
        numbers(p, &format!("{path}.{name}"))?;
    }
    more()?;
    for (name, p) in named {
        place_uid(p, &format!("{path}.{name}"))?;
    }
    let mut out = [Vec2::new(0.0, 0.0); N];
    for (i, (name, p)) in named.into_iter().enumerate() {
        out[i] = point_in(doc, p, &format!("{path}.{name}"))?;
    }
    Ok(out)
}

fn apart(a: Vec2, b: Vec2, message: &str, path: &str) -> Result<(), Stop> {
    if dist(a, b) < SAME {
        return Err(refuse(codes::SAME_POINTS, message, path));
    }
    Ok(())
}

const THREE_APART: &str =
    "Üç noktadan ikisi aynı yerde; aralarından daire geçmez. Ayrı üç nokta verin.";
const ENDS_APART: &str = "Başlangıç ve bitiş aynı yerde; aralarında yay yok. Ayrı iki nokta verin.";
const START_CENTRE: &str =
    "Başlangıç merkezle aynı yerde; yarıçap sıfır olur. Ayrı noktalar verin.";
const ON_TANGENT: &str = "Bitiş noktası başlangıçtaki teğetin doğrusunda; yay değil doğru olur. Doğrunun dışında bir bitiş verin.";
const ARC_ANGLE: &str = "Yay açısı sıfır ya da tam tur ve daha büyük olamaz; ikisinin arasında bir açı verin (eksi: saat yönünde).";

fn arc(g: ArcGeom) -> ConstructedCurve {
    ConstructedCurve::Arc {
        c: w(g.c),
        r: g.r,
        a0: g.a0,
        a1: g.a1,
    }
}

fn circle(c: Vec2, r: f64) -> ConstructedCurve {
    ConstructedCurve::Circle { c: w(c), r }
}

fn one(
    doc: Option<&Document>,
    c: &CurveConstruction,
    path: &str,
) -> Result<ConstructedCurve, Stop> {
    match c {
        CurveConstruction::CircleTwoPoints { a, b } => {
            let [a, b] = points(doc, path, [("a", a), ("b", b)], || Ok(()))?;
            apart(
                a,
                b,
                "Çapın iki ucu aynı yerde; daire oluşmaz. Ayrı iki nokta verin.",
                &format!("{path}.b"),
            )?;
            let round = circle_on_diameter(a, b);
            Ok(circle(round.c, round.r))
        }
        CurveConstruction::CircleThreePoints { a, b, c } => {
            let [a, b, c] = points(doc, path, [("a", a), ("b", b), ("c", c)], || Ok(()))?;
            three_apart(a, b, c, path, ["b", "c"])?;
            let round = circle_through(a, b, c).ok_or_else(|| {
                refuse(
                    codes::COLLINEAR_POINTS,
                    "Üç nokta aynı doğru üzerinde; daire geçmez. Doğrunun dışında bir nokta verin.",
                    &format!("{path}.c"),
                )
            })?;
            Ok(circle(round.c, round.r))
        }
        CurveConstruction::CircleTangentTangentRadius { first, second, r } => {
            near_numbers(first, &format!("{path}.first"))?;
            near_numbers(second, &format!("{path}.second"))?;
            length_number(*r, "Yarıçap", &format!("{path}.r"))?;
            if *r <= 0.0 {
                return Err(refuse(
                    codes::INVALID_RADIUS,
                    "Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.",
                    &format!("{path}.r"),
                ));
            }
            uid_text(&first.uid, &format!("{path}.first"))?;
            uid_text(&second.uid, &format!("{path}.second"))?;
            let e1 = tangent_edge(doc, first, &format!("{path}.first"))?;
            let e2 = tangent_edge(doc, second, &format!("{path}.second"))?;
            let round = tangent_tangent_radius(&e1, v(first.near), &e2, v(second.near), *r)
                .ok_or_else(|| {
                    let message = if parallel_gap(&e1, &e2).is_some() {
                        "İki doğru paralel; yalnız aralarındaki uzaklığın yarısı olan yarıçapla ikisine birden teğet daire olur."
                    } else {
                        "Bu yarıçapla iki nesneye birden teğet bir daire yok; yarıçapı değiştirin."
                    };
                    refuse(codes::NO_TANGENT_CIRCLE, message, &format!("{path}.r"))
                })?;
            Ok(circle(round.c, round.r))
        }
        CurveConstruction::CircleTangents {
            first,
            second,
            third,
        } => {
            near_numbers(first, &format!("{path}.first"))?;
            near_numbers(second, &format!("{path}.second"))?;
            near_numbers(third, &format!("{path}.third"))?;
            uid_text(&first.uid, &format!("{path}.first"))?;
            uid_text(&second.uid, &format!("{path}.second"))?;
            uid_text(&third.uid, &format!("{path}.third"))?;
            let e1 = tangent_edge(doc, first, &format!("{path}.first"))?;
            let e2 = tangent_edge(doc, second, &format!("{path}.second"))?;
            let e3 = tangent_edge(doc, third, &format!("{path}.third"))?;
            let round = tangent_tangent_tangent(
                &[e1, e2, e3],
                &[v(first.near), v(second.near), v(third.near)],
            )
            .ok_or_else(|| {
                let message = if parallel_gap(&e1, &e2).is_some() && parallel_gap(&e1, &e3).is_some() {
                    "Üç doğru paralel; üçüne birden teğet daire yok."
                } else {
                    "Üç nesneye birden teğet bir daire bulunamadı; nesnelere teğet noktalarının yakınından tıklayın."
                };
                refuse(codes::NO_TANGENT_CIRCLE, message, path)
            })?;
            Ok(circle(round.c, round.r))
        }
        CurveConstruction::ArcThreePoints { start, mid, end } => {
            let [s, m, e] = points(
                doc,
                path,
                [("start", start), ("mid", mid), ("end", end)],
                || Ok(()),
            )?;
            three_apart(s, m, e, path, ["mid", "end"])?;
            let g = arc_through(s, m, e).ok_or_else(|| {
                refuse(
                    codes::COLLINEAR_POINTS,
                    "Üç nokta aynı doğru üzerinde; yay geçmez. Doğrunun dışında bir nokta verin.",
                    &format!("{path}.end"),
                )
            })?;
            Ok(arc(g))
        }
        CurveConstruction::ArcStartCentreEnd { start, centre, end } => {
            let [s, c, e] = points(
                doc,
                path,
                [("start", start), ("centre", centre), ("end", end)],
                || Ok(()),
            )?;
            apart(s, c, START_CENTRE, &format!("{path}.centre"))?;
            apart(
                e,
                c,
                "Bitiş merkezle aynı yerde; bitişin doğrultusu yok. Merkezden ayrı bir bitiş verin.",
                &format!("{path}.end"),
            )?;
            let g = arc_start_center_to(s, c, e).ok_or_else(|| {
                refuse(
                    codes::INVALID_ANGLE,
                    "Bitiş başlangıçla aynı doğrultuda; yay oluşmaz (tam daire Daire'yle çizilir).",
                    &format!("{path}.end"),
                )
            })?;
            Ok(arc(g))
        }
        CurveConstruction::ArcStartCentreAngle {
            start,
            centre,
            angle,
        } => {
            let [s, c] = points(doc, path, [("start", start), ("centre", centre)], || {
                angle_number(*angle, &format!("{path}.angle"))
            })?;
            apart(s, c, START_CENTRE, &format!("{path}.centre"))?;
            let g = arc_start_center_sweep(s, c, *angle)
                .ok_or_else(|| refuse(codes::INVALID_ANGLE, ARC_ANGLE, &format!("{path}.angle")))?;
            Ok(arc(g))
        }
        CurveConstruction::ArcStartCentreChord {
            start,
            centre,
            chord,
        } => {
            let [s, c] = points(doc, path, [("start", start), ("centre", centre)], || {
                length_number(*chord, "Kiriş", &format!("{path}.chord"))
            })?;
            apart(s, c, START_CENTRE, &format!("{path}.centre"))?;
            if chord.abs() < SAME {
                return Err(refuse(
                    codes::INVALID_DISTANCE,
                    "Kiriş sıfır olamaz; sıfırdan farklı bir kiriş verin (eksi: büyük yay).",
                    &format!("{path}.chord"),
                ));
            }
            let g = arc_start_center_chord(s, c, *chord).ok_or_else(|| {
                refuse(
                    codes::CHORD_TOO_LONG,
                    "Kiriş dairenin çapından uzun; en çok çap kadar bir kiriş verin.",
                    &format!("{path}.chord"),
                )
            })?;
            Ok(arc(g))
        }
        CurveConstruction::ArcStartEndAngle { start, end, angle } => {
            let [s, e] = points(doc, path, [("start", start), ("end", end)], || {
                angle_number(*angle, &format!("{path}.angle"))
            })?;
            apart(s, e, ENDS_APART, &format!("{path}.end"))?;
            let g = arc_start_end_sweep(s, e, *angle)
                .ok_or_else(|| refuse(codes::INVALID_ANGLE, ARC_ANGLE, &format!("{path}.angle")))?;
            Ok(arc(g))
        }
        CurveConstruction::ArcStartEndDirection {
            start,
            end,
            direction,
        } => {
            let [s, e] = points(doc, path, [("start", start), ("end", end)], || {
                angle_number(*direction, &format!("{path}.direction"))
            })?;
            apart(s, e, ENDS_APART, &format!("{path}.end"))?;
            let dir = Vec2::new(cos(*direction), sin(*direction));
            let g = arc_start_end_direction(s, e, dir).ok_or_else(|| {
                refuse(codes::COLLINEAR_POINTS, ON_TANGENT, &format!("{path}.end"))
            })?;
            Ok(arc(g))
        }
        CurveConstruction::ArcStartEndRadius { start, end, r } => {
            let [s, e] = points(doc, path, [("start", start), ("end", end)], || {
                length_number(*r, "Yarıçap", &format!("{path}.r"))
            })?;
            apart(s, e, ENDS_APART, &format!("{path}.end"))?;
            if *r == 0.0 {
                return Err(refuse(
                    codes::INVALID_RADIUS,
                    "Yarıçap sıfır olamaz; sıfırdan farklı bir yarıçap verin (eksi: büyük yay).",
                    &format!("{path}.r"),
                ));
            }
            let g = arc_start_end_radius(s, e, *r).ok_or_else(|| {
                refuse(
                    codes::RADIUS_TOO_SMALL,
                    "Yarıçap iki uç arasındaki kirişin yarısından küçük; en az kirişin yarısı kadar bir yarıçap verin.",
                    &format!("{path}.r"),
                )
            })?;
            Ok(arc(g))
        }
        CurveConstruction::ArcContinue { from, end } => {
            numbers(end, &format!("{path}.end"))?;
            uid_text(&from.uid, &format!("{path}.from"))?;
            place_uid(end, &format!("{path}.end"))?;
            let tangent = continued(doc, from, &format!("{path}.from"))?;
            let e = point_in(doc, end, &format!("{path}.end"))?;
            apart(tangent.p, e, ENDS_APART, &format!("{path}.end"))?;
            let g = arc_start_end_direction(tangent.p, e, tangent.dir).ok_or_else(|| {
                refuse(codes::COLLINEAR_POINTS, ON_TANGENT, &format!("{path}.end"))
            })?;
            Ok(arc(g))
        }
        CurveConstruction::EllipseAxis {
            a,
            b,
            other,
            rotation,
            start_angle,
            end_angle,
        } => {
            let [a, b] = points(doc, path, [("a", a), ("b", b)], || {
                ellipse_numbers(*other, *rotation, *start_angle, *end_angle, path)
            })?;
            apart(
                a,
                b,
                "Eksenin iki ucu aynı yerde; eksen sıfır uzunlukta. Ayrı iki nokta verin.",
                &format!("{path}.b"),
            )?;
            let half = half_axis(dist(a, b) / 2.0, *other, *rotation, path)?;
            let g = ellipse_from_axis(a, b, half).ok_or_else(|| tiny_axis(path))?;
            elliptical(g, *start_angle, *end_angle, path)
        }
        CurveConstruction::EllipseCentre {
            centre,
            axis_end,
            other,
            rotation,
            start_angle,
            end_angle,
        } => {
            let [c, e] = points(
                doc,
                path,
                [("centre", centre), ("axisEnd", axis_end)],
                || ellipse_numbers(*other, *rotation, *start_angle, *end_angle, path),
            )?;
            apart(
                c,
                e,
                "Eksenin ucu merkezle aynı yerde; eksen sıfır uzunlukta. Ayrı iki nokta verin.",
                &format!("{path}.axisEnd"),
            )?;
            let half = half_axis(dist(c, e), *other, *rotation, path)?;
            let g = ellipse_from_center(c, e, half).ok_or_else(|| tiny_axis(path))?;
            elliptical(g, *start_angle, *end_angle, path)
        }
    }
}

/// Three points pairwise apart; a pair in one place answers at the later's field.
fn three_apart(a: Vec2, b: Vec2, c: Vec2, path: &str, later: [&str; 2]) -> Result<(), Stop> {
    apart(a, b, THREE_APART, &format!("{path}.{}", later[0]))?;
    apart(a, c, THREE_APART, &format!("{path}.{}", later[1]))?;
    apart(b, c, THREE_APART, &format!("{path}.{}", later[1]))
}

fn near_numbers(e: &EdgeNear, path: &str) -> Result<(), Stop> {
    checks::point(
        e.near,
        "Teğet noktasının yakınındaki noktanın",
        &format!("{path}.near"),
    )
}

/// The edge of `e`'s object nearest its near point, when a circle can touch the object.
fn tangent_edge(doc: Option<&Document>, e: &EdgeNear, path: &str) -> Result<Edge, Stop> {
    let entity = object(drawing(doc, path)?, &e.uid, path)?;
    let not_tangent = || {
        refuse(
            codes::NOT_TANGENT_KIND,
            "Teğet daire yalnız çizgiye, çoklu çizgiye, alana, yaya, daireye, yardımcı çizgiye ya da ışına değer; bu nesneye değmez.",
            path,
        )
    };
    if !kentos_contracts::capabilities::TANGENT_KINDS.contains(&entity.kind()) {
        return Err(not_tangent());
    }
    nearest_edge(&crate::geometry::shape(entity), v(e.near)).ok_or_else(not_tangent)
}

/// Where Devam starts and which way: the object's end on its way, or its start backwards.
fn continued(
    doc: Option<&Document>,
    from: &ObjectEnd,
    path: &str,
) -> Result<kentos_geometry_core::tools::drawing::EndTangent, Stop> {
    let doc = drawing(doc, path)?;
    let e = object(doc, &from.uid, path)?;
    let shape = crate::blocks::place_shape(doc.blocks(), e, &from.path).ok_or_else(|| {
        refuse(
            codes::PIECE_GONE,
            crate::blocks::NO_PIECE,
            &format!("{path}.path"),
        )
    })?;
    let not_an_end = |at: &str| {
        refuse(
            codes::NOT_AN_END,
            "Devam çizginin, çoklu çizginin ya da yayın bir ucundan sürer: başlangıcı (start) ya da bitişi (end); kapalı çoklu çizginin yalnız bitişi.",
            at,
        )
    };
    if !matches!(
        shape,
        Shape::Line { .. } | Shape::Polyline { .. } | Shape::Arc { .. }
    ) {
        return Err(not_an_end(path));
    }
    let count = match &shape {
        Shape::Polyline { pts, .. } => pts.len() as u32,
        _ => 0,
    };
    let at_end = match from.place {
        SubPlace::End => true,
        SubPlace::Start => false,
        SubPlace::Vertex {
            part: 0,
            ring: 0,
            index,
        } if count >= 2 && index + 1 == count => true,
        SubPlace::Vertex {
            part: 0,
            ring: 0,
            index: 0,
        } if count >= 2 => false,
        _ => return Err(not_an_end(&format!("{path}.place"))),
    };
    let tangent = if at_end {
        end_tangent(&shape)
    } else {
        start_tangent(&shape)
    };
    tangent.ok_or_else(|| not_an_end(&format!("{path}.place")))
}

fn ellipse_numbers(
    other: Option<f64>,
    rotation: Option<f64>,
    start: Option<f64>,
    end: Option<f64>,
    path: &str,
) -> Result<(), Stop> {
    if let Some(o) = other {
        length_number(o, "Diğer yarı eksen", &format!("{path}.other"))?;
    }
    if let Some(r) = rotation {
        angle_number(r, &format!("{path}.rotation"))?;
    }
    if let Some(s) = start {
        angle_number(s, &format!("{path}.startAngle"))?;
    }
    if let Some(e) = end {
        angle_number(e, &format!("{path}.endAngle"))?;
    }
    Ok(())
}

/// The other half-axis: given, or the first one (`first`, its half) seen tilted by the rotation.
fn half_axis(
    first: f64,
    other: Option<f64>,
    rotation: Option<f64>,
    path: &str,
) -> Result<f64, Stop> {
    match (other, rotation) {
        (Some(_), Some(_)) => Err(refuse(
            codes::INVALID_HALF_AXIS,
            "Diğer yarı eksen ya uzunluğuyla (other) ya döndürme açısıyla (rotation) verilir; ikisi birlikte verilmez.",
            &format!("{path}.rotation"),
        )),
        (None, None) => Err(refuse(
            codes::INVALID_HALF_AXIS,
            "Diğer yarı ekseni uzunluğuyla (other) ya da döndürme açısıyla (rotation) verin.",
            &format!("{path}.other"),
        )),
        (Some(o), None) if o <= 0.0 => Err(refuse(
            codes::INVALID_HALF_AXIS,
            "Diğer yarı eksen sıfırdan büyük olmalı.",
            &format!("{path}.other"),
        )),
        (Some(o), None) => Ok(o),
        (None, Some(r)) if !(0.0..=MAX_ROTATION).contains(&r) => Err(refuse(
            codes::INVALID_ANGLE,
            "Döndürme açısı 0 ile 89,4° (1,5603 radyan) arasında olmalı.",
            &format!("{path}.rotation"),
        )),
        (None, Some(r)) => Ok(first * cos(r)),
    }
}

fn tiny_axis(path: &str) -> Stop {
    refuse(
        codes::INVALID_HALF_AXIS,
        "Elipsin bir yarı ekseni sıfır uzunlukta; ayrı noktalar ve sıfırdan büyük bir yarı eksen verin.",
        &format!("{path}.other"),
    )
}

/// The whole ellipse, or its arc between the polar angles from its major axis.
fn elliptical(
    g: EllipseGeom,
    start: Option<f64>,
    end: Option<f64>,
    path: &str,
) -> Result<ConstructedCurve, Stop> {
    let (t0, t1) = match (start, end) {
        (None, None) => (0.0, 0.0),
        (Some(s), Some(e)) => {
            let (t0, t1) = (param_at_polar(&g, s), param_at_polar(&g, e));
            let turn = kentos_geometry_core::geom::arc::norm_angle(t1 - t0);
            if !(1e-9..=2.0 * PI - 1e-9).contains(&turn) {
                return Err(refuse(
                    codes::INVALID_ANGLE,
                    "Eliptik yayın bitiş açısı başlangıcıyla aynı yerde olamaz; başka bir bitiş açısı verin.",
                    &format!("{path}.endAngle"),
                ));
            }
            (t0, t1)
        }
        (Some(_), None) | (None, Some(_)) => {
            let missing = if start.is_none() {
                "startAngle"
            } else {
                "endAngle"
            };
            return Err(refuse(
                codes::INVALID_ANGLE,
                "Eliptik yayın başlangıç ve bitiş açısı birlikte verilir; ikisini de verin ya da ikisini de kaldırın.",
                &format!("{path}.{missing}"),
            ));
        }
    };
    Ok(ConstructedCurve::Ellipse {
        c: w(g.c),
        major: w(g.major),
        ratio: g.ratio,
        t0,
        t1,
    })
}
