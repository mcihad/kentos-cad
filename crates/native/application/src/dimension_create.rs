//! `cad.dimensions.create` v1 (docs/adr/0255): a dimension made from what it
//! measures (points or places, an object's edge) and where it goes, as one
//! undo step “Ekle”. The desktop's handler over the native document, run by
//! Ölçülendirme (its preview is [`measured_in`], its click the execute), the
//! headless server, Python and MCP. The shared cases are
//! `fixtures/commands/v1/cad.dimensions.create.json`.
//!
//! The edges are found here as a click finds them (the straight edge, the
//! arc or the circle nearest the point, docs/adr/0061, 0147 §7) and the
//! dimension is the shared core's (`radial_dimension`, `arc_length_ends`,
//! `edge_arms`, `vertex_arms`, the offsets of `geom::dimension`): what the
//! tool computed for itself before. It is written through
//! `cad.entities.create` ([`crate::creating`]), its bound points read from
//! their places there again (docs/adr/0249, 0250 §5).

use kentos_contracts::{
    CommandResult, DimensionLook, DimensionMeasure, DimensionRefs, DimensionStyle,
    DimensionsCreate, EdgeNear, EntitiesCreatePlan, EntitiesCreated, Entity, EntityGeometry,
    EntityId, MeasuredPoint, STANDARD_STYLE, SubPlace, SubRef, Vec2 as Point,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::dimension::{
    DimensionGeom, dimension_offset_at, layout_dimension, linear_angle_for, ordinate_axis_for,
    signed_offset,
};
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::tools::editing::{
    PickedEdge, arc_length_ends, edge_arms, radial_dimension, vertex_arms,
};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;
use crate::creating;
use crate::geometry::{core_look, shape};

// ── The edges a click picks ────────────────────────────────────────────

/// The straight edge of `e` nearest to `p` (Açı's, Semt's and Eğim's pick).
pub fn straight_edge_at(e: &Entity, p: Vec2) -> Option<(Vec2, Vec2)> {
    let mut best: Option<((Vec2, Vec2), f64)> = None;
    for edge in entity_edges(&shape(e)) {
        let Edge::Seg { a, b } = edge else {
            continue;
        };
        if dist(a, b) < 1e-9 {
            continue;
        }
        let d = closest_on_edge(&edge, p).d;
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some(((a, b), d));
        }
    }
    best.map(|(s, _)| s)
}

/// The arc of an arc, or the arc segment of a path nearest to `p`: Yay
/// uzunluğu's and Yaydan's pick (a circle has no ends to measure between).
pub fn arc_edge_at(e: &Entity, p: Vec2) -> Option<Edge> {
    let mut best: Option<(Edge, f64)> = None;
    for edge in entity_edges(&shape(e)) {
        if !matches!(edge, Edge::Arc { .. }) || arc_length_ends(&edge, None).is_none() {
            continue;
        }
        let d = closest_on_edge(&edge, p).d;
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((edge, d));
        }
    }
    best.map(|(e, _)| e)
}

/// The circle of a circle, an arc, or the arc segment of a path nearest to
/// `p`: Yarıçap's, Çap's and Kırıklı yarıçap's pick.
pub fn circle_at(e: &Entity, p: Vec2) -> Option<(Vec2, f64)> {
    match e {
        Entity::Circle(c) => return Some((Vec2::new(c.c.x, c.c.y), c.r)),
        Entity::Arc(a) => return Some((Vec2::new(a.c.x, a.c.y), a.r)),
        _ => {}
    }
    let mut best: Option<((Vec2, f64), f64)> = None;
    for edge in entity_edges(&shape(e)) {
        let Edge::Arc { c, r, .. } = edge else {
            continue;
        };
        let d = closest_on_edge(&edge, p).d;
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some(((c, r), d));
        }
    }
    best.map(|(c, _)| c)
}

// ── The kinds ──────────────────────────────────────────────────────────

/// A measure's kind: its style as the layout reads it and as the drawing
/// keeps it, whether its points bind, why none forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Aligned,
    Linear,
    Ordinate,
    Azimuth,
    Slope,
    Angle,
    Radius,
    Diameter,
    ArcLength,
    Jogged,
}

impl Kind {
    fn of(m: &DimensionMeasure) -> Kind {
        match m {
            DimensionMeasure::Aligned { .. } => Kind::Aligned,
            DimensionMeasure::Linear { .. } => Kind::Linear,
            DimensionMeasure::Ordinate { .. } => Kind::Ordinate,
            DimensionMeasure::Azimuth { .. } => Kind::Azimuth,
            DimensionMeasure::Slope { .. } => Kind::Slope,
            DimensionMeasure::Angle { .. }
            | DimensionMeasure::AngleVertex { .. }
            | DimensionMeasure::AngleArc { .. } => Kind::Angle,
            DimensionMeasure::Radius { .. } => Kind::Radius,
            DimensionMeasure::Diameter { .. } => Kind::Diameter,
            DimensionMeasure::ArcLength { .. } => Kind::ArcLength,
            DimensionMeasure::Jogged { .. } => Kind::Jogged,
        }
    }

    /// The style as the shared layout reads it; none for aligned.
    fn layout_style(self) -> Option<&'static str> {
        match self {
            Kind::Aligned => None,
            Kind::Linear => Some("linear"),
            Kind::Angle => Some("angular"),
            Kind::Radius => Some("radius"),
            Kind::Diameter => Some("diameter"),
            Kind::Ordinate => Some("ordinate"),
            Kind::ArcLength => Some("arcLength"),
            Kind::Jogged => Some("jogged"),
            Kind::Azimuth => Some("azimuth"),
            Kind::Slope => Some("slope"),
        }
    }

    /// The style as the drawing keeps it; none for aligned (the web leaves it out).
    fn contract(self) -> Option<DimensionStyle> {
        match self {
            Kind::Aligned => None,
            Kind::Linear => Some(DimensionStyle::Linear),
            Kind::Angle => Some(DimensionStyle::Angular),
            Kind::Radius => Some(DimensionStyle::Radius),
            Kind::Diameter => Some(DimensionStyle::Diameter),
            Kind::Ordinate => Some(DimensionStyle::Ordinate),
            Kind::ArcLength => Some(DimensionStyle::ArcLength),
            Kind::Jogged => Some(DimensionStyle::Jogged),
            Kind::Azimuth => Some(DimensionStyle::Azimuth),
            Kind::Slope => Some(DimensionStyle::Slope),
        }
    }

    /// Whether its points given as places follow them (docs/adr/0249 §3): the point methods.
    fn binds(self) -> bool {
        matches!(
            self,
            Kind::Aligned | Kind::Linear | Kind::Ordinate | Kind::Azimuth | Kind::Slope
        )
    }

    /// Whether a typed distance places it (Yarıçap, Çap and Koordinat are placed by `at` alone).
    fn takes_offset(self) -> bool {
        !matches!(self, Kind::Radius | Kind::Diameter | Kind::Ordinate)
    }
}

/// Why a dimension of `m`'s kind does not form where it is placed (Ölçülendirme's words).
pub fn no_dimension_words(m: &DimensionMeasure) -> &'static str {
    match Kind::of(m) {
        Kind::Ordinate => {
            "Çizginin ucu noktaya çok yakın; imleci noktadan eksene dik yönde uzaklaştırın."
        }
        Kind::ArcLength => {
            "Bu yerde ölçü oluşmuyor; ölçü yayı merkeze ulaşıyor ya da iki nokta aynı yerde."
        }
        Kind::Jogged => {
            "Gösterilen merkez, yarıçap boyunca yaydaki noktadan geride ve yarıçapa yakın olmalı; başka bir yer gösterin."
        }
        Kind::Azimuth | Kind::Slope => {
            "Kenarın iki ucu aynı nokta; ölçülecek kenar yok. Başka bir nokta gösterin."
        }
        _ => "Bu yerde ölçü oluşmuyor; ölçülen noktalar çakışıyor ya da yay yarıçapı sıfır.",
    }
}

// ── What the input names, in order ─────────────────────────────────────

/// One of the things a measure names, with its path: the checks go over them in this order.
enum Item<'a> {
    Point(&'a MeasuredPoint, String),
    Edge(&'a EdgeNear, String),
    Number(Option<f64>, String, &'static str, &'static str),
    Coord(Point, String, &'static str),
}

fn items(m: &DimensionMeasure) -> Vec<Item<'_>> {
    let point = |p, name: &str| Item::Point(p, format!("measure.{name}"));
    let edge = |e, name: &str| Item::Edge(e, format!("measure.{name}"));
    let angle = |a: &Option<f64>| {
        Item::Number(
            *a,
            "measure.angle".into(),
            "Ölçünün doğrultusu",
            "Açıyı derece olarak sonlu bir sayıyla verin.",
        )
    };
    let z = |v: &Option<f64>, name: &str| {
        Item::Number(
            *v,
            format!("measure.{name}"),
            "Kot",
            "Kotu metre olarak sonlu bir sayıyla verin.",
        )
    };
    match m {
        DimensionMeasure::Aligned { a, b } | DimensionMeasure::Azimuth { a, b } => {
            vec![point(a, "a"), point(b, "b")]
        }
        DimensionMeasure::Linear { a, b, angle: an } => {
            vec![point(a, "a"), point(b, "b"), angle(an)]
        }
        DimensionMeasure::Ordinate { a, angle: an } => vec![point(a, "a"), angle(an)],
        DimensionMeasure::Slope { a, b, za, zb } => {
            vec![point(a, "a"), point(b, "b"), z(za, "za"), z(zb, "zb")]
        }
        DimensionMeasure::Angle { first, second } => {
            vec![edge(first, "first"), edge(second, "second")]
        }
        DimensionMeasure::AngleVertex { c, a, b } => {
            vec![point(c, "c"), point(a, "a"), point(b, "b")]
        }
        DimensionMeasure::AngleArc { of }
        | DimensionMeasure::Radius { of }
        | DimensionMeasure::Diameter { of } => vec![edge(of, "of")],
        DimensionMeasure::ArcLength { of, between } => {
            let mut out = vec![edge(of, "of")];
            if let Some([p, q]) = between {
                out.push(Item::Coord(
                    *p,
                    "measure.between[0]".into(),
                    "Kısmi noktanın",
                ));
                out.push(Item::Coord(
                    *q,
                    "measure.between[1]".into(),
                    "Kısmi noktanın",
                ));
            }
            out
        }
        DimensionMeasure::Jogged {
            of,
            shown_centre,
            on,
        } => vec![
            edge(of, "of"),
            Item::Coord(
                *shown_centre,
                "measure.shownCentre".into(),
                "Gösterilen merkezin",
            ),
            Item::Coord(*on, "measure.on".into(), "Yaydaki noktanın"),
        ],
    }
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn v(p: Point) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// The checks that need no drawing: numbers, ids, the offset's kind.
fn check_input(input: &DimensionsCreate) -> Result<(), Stop> {
    checks::point(input.at, "Ölçünün yerinin", "at")?;
    if let Some(o) = input.offset {
        checks::finite(
            o,
            "Uzaklık",
            "Uzaklığı metre olarak sonlu bir sayıyla verin.",
            "offset",
        )?;
    }
    let list = items(&input.measure);
    for item in &list {
        match item {
            Item::Point(MeasuredPoint::At { at }, path) => {
                checks::point(*at, "Ölçülen noktanın", &format!("{path}.at"))?
            }
            Item::Point(MeasuredPoint::Place { .. }, _) => {}
            Item::Edge(e, path) => {
                checks::point(e.near, "Tıklanan noktanın", &format!("{path}.near"))?
            }
            Item::Number(Some(n), path, what, fix) => checks::finite(*n, what, fix, path)?,
            Item::Number(None, ..) => {}
            Item::Coord(p, path, whose) => checks::point(*p, whose, path)?,
        }
    }
    if let Some(h) = input.height
        && !(h.is_finite() && h > 0.0)
    {
        return Err(refuse(
            codes::INVALID_HEIGHT,
            "Ölçü yazısının yüksekliği sıfırdan büyük, sonlu bir sayı olmalı (metre).".into(),
            "height",
        ));
    }
    for item in &list {
        let (uid, path) = match item {
            Item::Point(MeasuredPoint::Place { uid, .. }, path) => (uid, path),
            Item::Edge(e, path) => (&e.uid, path),
            _ => continue,
        };
        if !is_uid_text(uid) {
            return Err(refuse(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                ),
                &format!("{path}.uid"),
            ));
        }
    }
    if input.offset.is_some() && !Kind::of(&input.measure).takes_offset() {
        return Err(refuse(
            codes::INVALID_OFFSET,
            "Yarıçap, çap ve koordinat ölçüsü uzaklık almaz; yerini at verir. offset'i kaldırın."
                .into(),
            "offset",
        ));
    }
    Ok(())
}

/// The object a place or an edge names.
fn object<'d>(doc: &'d Document, uid: &str, path: &str) -> Result<&'d Entity, Stop> {
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .ok_or_else(|| {
            refuse(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                &format!("{path}.uid"),
            )
        })
}

/// A measured point now: its coordinates, or its place's on its object.
fn point_of(doc: &Document, p: &MeasuredPoint, path: &str) -> Result<Vec2, Stop> {
    match p {
        MeasuredPoint::At { at } => Ok(v(*at)),
        MeasuredPoint::Place { uid, place } => {
            let e = object(doc, uid, path)?;
            let core = kentos_domain::dimension_ties::core_place(*place);
            kentos_geometry_core::ops::subref::point(&crate::blocks::core_entity(e), core).ok_or_else(|| {
                refuse(
                    codes::NO_SUCH_PLACE,
                    "Nesnede böyle bir yer yok: köşe sırası halkanın dışında ya da yer bu nesne türünde bulunmuyor.".into(),
                    &format!("{path}.place"),
                )
            })
        }
    }
}

/// The elevation a place keeps (docs/adr/0142): a point's, a line's end's, a path's vertex's.
fn elevation_at(e: &Entity, place: SubPlace) -> Option<f64> {
    let nth = |zs: &Option<Vec<Option<f64>>>, i: u32| {
        zs.as_ref()
            .and_then(|z| z.get(i as usize).copied())
            .flatten()
    };
    match (e, place) {
        (Entity::Point(p), SubPlace::Node { part: 0 }) => p.z,
        (Entity::Point(p), SubPlace::Node { part }) => {
            p.parts.as_ref()?.get(part as usize - 1).and_then(|q| q.z)
        }
        (Entity::Line(l), SubPlace::Start) => l.za,
        (Entity::Line(l), SubPlace::End) => l.zb,
        (Entity::Polyline(p) | Entity::Polygon(p), SubPlace::Vertex { part, ring, index }) => {
            let (zs, holes) = if part == 0 {
                (&p.zs, &p.holes)
            } else {
                let q = p.parts.as_ref()?.get(part as usize - 1)?;
                (&q.zs, &q.holes)
            };
            if ring == 0 {
                nth(zs, index)
            } else {
                nth(&holes.as_ref()?.get(ring as usize - 1)?.zs, index)
            }
        }
        _ => None,
    }
}

/// A slope's elevation: given, else its place's.
fn elevation_of(
    doc: &Document,
    given: Option<f64>,
    p: &MeasuredPoint,
    path: &str,
) -> Result<f64, Stop> {
    if let Some(z) = given {
        return Ok(z);
    }
    let kept = match p {
        MeasuredPoint::Place { uid, place } => object(doc, uid, path)
            .ok()
            .and_then(|e| elevation_at(e, *place)),
        MeasuredPoint::At { .. } => None,
    };
    kept.ok_or_else(|| {
        refuse(
            codes::MISSING_ELEVATION,
            "Eğim için noktanın kotu yok: kotu metre olarak verin ya da noktayı kotlu bir köşeye bağlayın.".into(),
            path,
        )
    })
}

/// The edge an `EdgeNear` names: `find` on its object, or `not_measurable` in `words`.
fn edge_of<T>(
    doc: &Document,
    e: &EdgeNear,
    path: &str,
    find: impl Fn(&Entity, Vec2) -> Option<T>,
    words: &str,
) -> Result<T, Stop> {
    let object = object(doc, &e.uid, path)?;
    find(object, v(e.near)).ok_or_else(|| refuse(codes::NOT_MEASURABLE, words.into(), path))
}

const NO_STRAIGHT: &str = "Nesnenin düz kenarı yok: açı iki düz kenar arasında ölçülür; yaydan ölçmek için angleArc, köşe noktasından angleVertex verin.";
const NO_ARC: &str = "Nesnenin uçları olan yayı yok: yaydan ya da çoklu çizginin ya da alanın yaylı kenarından ölçülür; tam daire için Yarıçap ya da Çap'ı kullanın.";
const NO_CIRCLE: &str =
    "Nesne daire değil: daireden, yaydan ya da çoklu çizginin yay parçasından ölçülür.";

/// The dimension the input makes: its layout's geometry, the style as kept,
/// the look, the references of its bound points.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub geom: DimensionGeom,
    pub style: Option<DimensionStyle>,
    pub look: DimensionLook,
    pub refs: Option<DimensionRefs>,
}

/// The project's dimension style by name: its look and its value's height at
/// the project's scale; Standart's when none is named (docs/adr/0183 §4, 0205).
fn style_of(doc: &Document, name: Option<&str>) -> Result<(DimensionLook, f64), Stop> {
    let settings = doc.settings();
    let standard = settings.annotation_height(kentos_contracts::AnnotationKind::Dimension);
    let same = |a: &str, b: &str| a.trim().to_lowercase() == b.trim().to_lowercase();
    match name {
        None => Ok((DimensionLook::default(), standard)),
        Some(n) if same(n, STANDARD_STYLE) => Ok((DimensionLook::default(), standard)),
        Some(n) => settings
            .dimension_styles
            .iter()
            .find(|s| same(&s.name, n))
            .map(|s| (s.look(), s.height_at(settings.plot_scale)))
            .ok_or_else(|| {
                refuse(
                    codes::UNKNOWN_STYLE,
                    format!(
                        "“{}” adlı ölçü stili yok. Projenin ölçü stillerinden birinin adını verin ya da alanı kaldırın (Standart).",
                        n.trim()
                    ),
                    "dimensionStyle",
                )
            }),
    }
}

/// The checks of `cad.dimensions.create` in the contract's order, and the
/// dimension it makes.
fn check(doc: &Document, input: &DimensionsCreate) -> Result<Measured, Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    // The objects and places, in order; then what each edge is.
    for item in items(&input.measure) {
        match item {
            Item::Point(p, path) => {
                point_of(doc, p, &path)?;
            }
            Item::Edge(e, path) => {
                object(doc, &e.uid, &path)?;
            }
            _ => {}
        }
    }
    let kind = Kind::of(&input.measure);
    let at = v(input.at);
    let typed = input.offset;
    // The edges (not_measurable), the elevations (missing_elevation), then the style (unknown_style).
    let edges = Edges::of(doc, &input.measure)?;
    let (za, zb) = match &input.measure {
        DimensionMeasure::Slope { a, b, za, zb } => (
            Some(elevation_of(doc, *za, a, "measure.za")?),
            Some(elevation_of(doc, *zb, b, "measure.zb")?),
        ),
        _ => (None, None),
    };
    let (look, style_height) = style_of(doc, input.dimension_style.as_deref())?;
    let height = input.height.unwrap_or(style_height);
    let geom = |a, b, offset, angle, c| DimensionGeom {
        a,
        b,
        offset,
        height,
        style: kind.layout_style().map(str::to_owned),
        angle,
        c,
        za,
        zb,
        look: core_look(&look),
    };
    let none = || {
        refuse(
            codes::NO_DIMENSION,
            no_dimension_words(&input.measure).into(),
            "at",
        )
    };
    let pt = |p: &MeasuredPoint, name: &str| point_of(doc, p, &format!("measure.{name}"));
    let g = match (&input.measure, edges) {
        (DimensionMeasure::Aligned { a, b }, _)
        | (DimensionMeasure::Azimuth { a, b }, _)
        | (DimensionMeasure::Slope { a, b, .. }, _) => {
            let (a, b) = (pt(a, "a")?, pt(b, "b")?);
            let offset = typed.unwrap_or_else(|| signed_offset(a, b, at));
            geom(a, b, offset, None, None)
        }
        (DimensionMeasure::Linear { a, b, angle }, _) => {
            let (a, b) = (pt(a, "a")?, pt(b, "b")?);
            let angle = angle.unwrap_or_else(|| linear_angle_for(a, b, at));
            let g = geom(a, b, 0.0, Some(angle), None);
            let offset = typed.unwrap_or_else(|| dimension_offset_at(&g, at));
            DimensionGeom { offset, ..g }
        }
        (DimensionMeasure::Ordinate { a, angle }, _) => {
            let a = pt(a, "a")?;
            let angle = angle.unwrap_or_else(|| ordinate_axis_for(a, at));
            geom(a, at, 0.0, Some(angle), None)
        }
        (DimensionMeasure::AngleVertex { c, a, b }, _) => {
            let arms = vertex_arms(pt(c, "c")?, pt(a, "a")?, pt(b, "b")?, at);
            let offset = typed.map_or_else(|| dist(arms.c, at), f64::abs);
            geom(arms.a, arms.b, offset, None, Some(arms.c))
        }
        (DimensionMeasure::Angle { first, second }, Edges::Two(s, t)) => {
            let picked = |(a, b): (Vec2, Vec2), e: &EdgeNear| PickedEdge {
                a,
                b,
                at: v(e.near),
            };
            let arms = edge_arms(&picked(s, first), &picked(t, second), at).ok_or_else(|| {
                refuse(
                    codes::NO_DIMENSION,
                    "Kenarlar paralel; aralarında açı yok.".into(),
                    "measure.second",
                )
            })?;
            let offset = typed.map_or_else(|| dist(arms.c, at), f64::abs);
            geom(arms.a, arms.b, offset, None, Some(arms.c))
        }
        (DimensionMeasure::AngleArc { .. }, Edges::Arc(edge)) => {
            let ends = arc_length_ends(&edge, None).ok_or_else(none)?;
            let offset = typed.map_or_else(|| dist(ends.c, at), f64::abs);
            geom(ends.a, ends.b, offset, None, Some(ends.c))
        }
        (
            DimensionMeasure::Radius { .. } | DimensionMeasure::Diameter { .. },
            Edges::Circle(c, r),
        ) => {
            let radial = radial_dimension(c, r, at);
            geom(c, radial.b, radial.offset, None, None)
        }
        (DimensionMeasure::ArcLength { between, .. }, Edges::Arc(edge)) => {
            let ends =
                arc_length_ends(&edge, between.map(|[p, q]| (v(p), v(q)))).ok_or_else(none)?;
            let g = geom(ends.a, ends.b, 0.0, None, Some(ends.c));
            let offset = typed.unwrap_or_else(|| dimension_offset_at(&g, at));
            DimensionGeom { offset, ..g }
        }
        (
            DimensionMeasure::Jogged {
                shown_centre, on, ..
            },
            Edges::Circle(c, r),
        ) => {
            let on = v(*on);
            let l = dist(c, on);
            if l < 1e-9 {
                return Err(none());
            }
            let b = Vec2::new(c.x + (on.x - c.x) / l * r, c.y + (on.y - c.y) / l * r);
            let g = geom(c, b, 0.0, None, Some(v(*shown_centre)));
            let offset = typed.unwrap_or_else(|| dimension_offset_at(&g, at));
            DimensionGeom { offset, ..g }
        }
        _ => return Err(none()),
    };
    if layout_dimension(&g).is_none() {
        return Err(none());
    }
    Ok(Measured {
        geom: g,
        style: kind.contract(),
        look,
        refs: refs_of(kind, &input.measure),
    })
}

/// What the measure's edges are, found on their objects.
enum Edges {
    None,
    Two((Vec2, Vec2), (Vec2, Vec2)),
    Arc(Edge),
    Circle(Vec2, f64),
}

impl Edges {
    fn of(doc: &Document, m: &DimensionMeasure) -> Result<Edges, Stop> {
        Ok(match m {
            DimensionMeasure::Angle { first, second } => Edges::Two(
                edge_of(doc, first, "measure.first", straight_edge_at, NO_STRAIGHT)?,
                edge_of(doc, second, "measure.second", straight_edge_at, NO_STRAIGHT)?,
            ),
            DimensionMeasure::AngleArc { of } | DimensionMeasure::ArcLength { of, .. } => {
                Edges::Arc(edge_of(doc, of, "measure.of", arc_edge_at, NO_ARC)?)
            }
            DimensionMeasure::Radius { of }
            | DimensionMeasure::Diameter { of }
            | DimensionMeasure::Jogged { of, .. } => {
                let (c, r) = edge_of(doc, of, "measure.of", circle_at, NO_CIRCLE)?;
                Edges::Circle(c, r)
            }
            _ => Edges::None,
        })
    }
}

/// The references of the points given as places, on the kinds whose points bind (docs/adr/0249 §3): `a` and `b`,
/// Koordinat's `a` (its `b` is the leader's end).
fn refs_of(kind: Kind, m: &DimensionMeasure) -> Option<DimensionRefs> {
    if !kind.binds() {
        return None;
    }
    let r = |p: &MeasuredPoint| match p {
        MeasuredPoint::Place { uid, place } => Uuid::parse_str(uid).ok().map(|u| SubRef {
            uid: EntityId(*u.as_bytes()),
            place: *place,
            broken: None,
        }),
        MeasuredPoint::At { .. } => None,
    };
    let refs = match m {
        DimensionMeasure::Aligned { a, b }
        | DimensionMeasure::Linear { a, b, .. }
        | DimensionMeasure::Azimuth { a, b }
        | DimensionMeasure::Slope { a, b, .. } => DimensionRefs {
            a: r(a),
            b: r(b),
            c: None,
        },
        DimensionMeasure::Ordinate { a, .. } => DimensionRefs {
            a: r(a),
            b: None,
            c: None,
        },
        _ => return None,
    };
    (!refs.is_empty()).then_some(refs)
}

fn wire(p: Vec2) -> Point {
    Point { x: p.x, y: p.y }
}

/// The objects `cad.entities.create` writes for the dimension.
fn objects(input: &DimensionsCreate, m: &Measured) -> kentos_contracts::EntitiesCreate {
    let g = &m.geom;
    let geometry = EntityGeometry::Dimension {
        a: wire(g.a),
        b: wire(g.b),
        offset: g.offset,
        height: g.height,
        text: None,
        style: m.style,
        angle: g.angle,
        c: g.c.map(wire),
        mask: input.mask.unwrap_or(false),
        za: g.za,
        zb: g.zb,
        refs: m.refs.clone(),
        look: m.look.clone(),
    };
    creating::input_as(&input.layer_id, vec![geometry], &input.look, None)
}

/// The dimension `input` makes in `doc`, for a tool's preview: its geometry
/// as the layout reads it; the refusal when none forms or the input is wrong.
pub fn measured_in(
    doc: &Document,
    input: &DimensionsCreate,
) -> Result<Measured, Box<kentos_contracts::CommandError>> {
    check(doc, input).map_err(|stop| Box::new(stop.into_error()))
}

/// `cad.dimensions.create` v1: Ölçülendirme.
pub mod dimensions_create {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &DimensionsCreate) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(m) => creating::validate(cx, &objects(input, &m)),
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &DimensionsCreate,
    ) -> CommandResult<EntitiesCreatePlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over the document.
    pub fn plan_in(doc: &Document, input: &DimensionsCreate) -> CommandResult<EntitiesCreatePlan> {
        match check(doc, input) {
            Ok(m) => creating::plan(doc, &objects(input, &m)),
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: DimensionsCreate,
    ) -> CommandResult<EntitiesCreated> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_DIMENSIONS_CREATE,
            kentos_contracts::CAD_DIMENSIONS_CREATE_VERSION,
            input,
            |cx, input| match check(cx.doc, &input) {
                Ok(m) => creating::execute(cx, objects(&input, &m)),
                Err(stop) => stop.into(),
            },
        )
    }
}
