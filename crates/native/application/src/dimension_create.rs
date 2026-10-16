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
    CommandResult, DimensionEntity, DimensionLook, DimensionMeasure, DimensionRefs, DimensionRule,
    DimensionStyle, DimensionsCreate, EdgeNear, EntitiesCreatePlan, EntitiesCreated, Entity,
    EntityGeometry, EntityId, MeasuredPoint, STANDARD_STYLE, SubRef, Vec2 as Point,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::dimension::{
    DimensionGeom, dimension_offset_at, layout_dimension, linear_angle_for, ordinate_axis_for,
    signed_offset,
};
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::subref::{EdgeKind, edge_place_near};
use kentos_geometry_core::tools::construct::{baseline_dimension, continue_dimension};
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
    /// Zincir ölçü and Baz ölçü (docs/adr/0320 §3): linear dimensions from a base.
    Continue,
    Baseline,
    /// Merkez işareti, Eksen çizgisi and Kot (docs/adr/0322).
    CenterMark,
    CenterLine,
    Elevation,
}

/// A baseline level is this many times the base's value height (AutoCAD's DIMDLI, docs/adr/0140).
pub const LEVEL_TEXT_HEIGHTS: f64 = 3.0;

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
            DimensionMeasure::Continue { .. } => Kind::Continue,
            DimensionMeasure::Baseline { .. } => Kind::Baseline,
            DimensionMeasure::CenterMark { .. } => Kind::CenterMark,
            DimensionMeasure::CenterLine { .. } => Kind::CenterLine,
            DimensionMeasure::Elevation { .. } => Kind::Elevation,
        }
    }

    /// The style as the shared layout reads it; none for aligned.
    fn layout_style(self) -> Option<&'static str> {
        match self {
            Kind::Aligned => None,
            Kind::Linear | Kind::Continue | Kind::Baseline => Some("linear"),
            Kind::Angle => Some("angular"),
            Kind::Radius => Some("radius"),
            Kind::Diameter => Some("diameter"),
            Kind::Ordinate => Some("ordinate"),
            Kind::ArcLength => Some("arcLength"),
            Kind::Jogged => Some("jogged"),
            Kind::Azimuth => Some("azimuth"),
            Kind::Slope => Some("slope"),
            Kind::CenterMark => Some("centerMark"),
            Kind::CenterLine => Some("centerLine"),
            Kind::Elevation => Some("elevation"),
        }
    }

    /// The style as the drawing keeps it; none for aligned (the web leaves it out).
    fn contract(self) -> Option<DimensionStyle> {
        match self {
            Kind::Aligned => None,
            Kind::Linear | Kind::Continue | Kind::Baseline => Some(DimensionStyle::Linear),
            Kind::Angle => Some(DimensionStyle::Angular),
            Kind::Radius => Some(DimensionStyle::Radius),
            Kind::Diameter => Some(DimensionStyle::Diameter),
            Kind::Ordinate => Some(DimensionStyle::Ordinate),
            Kind::ArcLength => Some(DimensionStyle::ArcLength),
            Kind::Jogged => Some(DimensionStyle::Jogged),
            Kind::Azimuth => Some(DimensionStyle::Azimuth),
            Kind::Slope => Some(DimensionStyle::Slope),
            Kind::CenterMark => Some(DimensionStyle::CenterMark),
            Kind::CenterLine => Some(DimensionStyle::CenterLine),
            Kind::Elevation => Some(DimensionStyle::Elevation),
        }
    }

    /// Whether a typed distance places it (Yarıçap, Çap, Koordinat and Kot are placed by `at` alone; Zincir and Baz
    /// by their base). A centre mark's and an axis's is how far they reach past their objects.
    fn takes_offset(self) -> bool {
        !matches!(
            self,
            Kind::Radius
                | Kind::Diameter
                | Kind::Ordinate
                | Kind::Continue
                | Kind::Baseline
                | Kind::Elevation
        )
    }

    /// Whether it writes no value but reaches past its objects (docs/adr/0322).
    fn is_mark(self) -> bool {
        matches!(self, Kind::CenterMark | Kind::CenterLine)
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
        Kind::Continue | Kind::Baseline => {
            "Bu yerde ölçü oluşmuyor; nokta ölçülen noktayla çakışıyor."
        }
        Kind::CenterMark => "Merkez işareti oluşmuyor: eğrinin yarıçapı sıfır.",
        Kind::CenterLine => {
            "Eksen çizgisi oluşmuyor: iki kenar aynı ya da eksenin uzunluğu sıfır. İki ayrı düz kenar gösterin."
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
    /// Zincir's and Baz's base dimension, at `measure.base`.
    Base(&'a String),
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
        DimensionMeasure::Continue { base, to } | DimensionMeasure::Baseline { base, to } => {
            vec![Item::Base(base), point(to, "to")]
        }
        DimensionMeasure::CenterMark { of } => vec![edge(of, "of")],
        DimensionMeasure::CenterLine { first, second } => {
            vec![edge(first, "first"), edge(second, "second")]
        }
        DimensionMeasure::Elevation { a, z: level } => vec![point(a, "a"), z(level, "z")],
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
            Item::Base(_) => {}
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
        let (uid, path): (&String, &str) = match item {
            Item::Point(MeasuredPoint::Place { uid, .. }, path) => (uid, path),
            Item::Edge(e, path) => (&e.uid, path),
            Item::Base(uid) => (uid, "measure"),
            _ => continue,
        };
        let path = if matches!(item, Item::Base(_)) {
            "measure.base".to_owned()
        } else {
            format!("{path}.uid")
        };
        if !is_uid_text(uid) {
            return Err(refuse(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                ),
                &path,
            ));
        }
    }
    let kind = Kind::of(&input.measure);
    if input.offset.is_some() && !kind.takes_offset() {
        let words = if matches!(kind, Kind::Continue | Kind::Baseline) {
            "Zincir ve baz ölçü uzaklık almaz; yerini temel ölçü verir. offset'i kaldırın."
        } else {
            "Yarıçap, çap, koordinat ve kot ölçüsü uzaklık almaz; yerini at verir. offset'i kaldırın."
        };
        return Err(refuse(codes::INVALID_OFFSET, words.into(), "offset"));
    }
    if kind.is_mark() && input.offset.is_some_and(|o| o < 0.0) {
        return Err(refuse(
            codes::INVALID_OFFSET,
            "Merkez işaretinin ve eksen çizgisinin uzantısı eksi olamaz; 0 ya da daha büyük bir uzaklık verin (metre)."
                .into(),
            "offset",
        ));
    }
    Ok(())
}

/// The object a place or an edge names.
pub(crate) fn object<'d>(doc: &'d Document, uid: &str, path: &str) -> Result<&'d Entity, Stop> {
    object_at(doc, uid, &format!("{path}.uid"))
}

/// The object `uid` names, refused at `path` when the drawing has none.
fn object_at<'d>(doc: &'d Document, uid: &str, path: &str) -> Result<&'d Entity, Stop> {
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
                path,
            )
        })
}

/// A measured point now: its coordinates, or its place's on its object.
pub(crate) fn point_of(doc: &Document, p: &MeasuredPoint, path: &str) -> Result<Vec2, Stop> {
    match p {
        MeasuredPoint::At { at } => Ok(v(*at)),
        MeasuredPoint::Place {
            uid,
            place,
            path: pieces,
        } => {
            let e = object(doc, uid, path)?;
            crate::blocks::place_point(doc.blocks(), e, pieces, *place).map_err(|miss| match miss {
                crate::blocks::PlaceMiss::Piece => refuse(
                    codes::PIECE_GONE,
                    crate::blocks::NO_PIECE.into(),
                    &format!("{path}.path"),
                ),
                crate::blocks::PlaceMiss::Place => refuse(
                    codes::NO_SUCH_PLACE,
                    "Nesnede böyle bir yer yok: köşe sırası halkanın dışında ya da yer bu nesne türünde bulunmuyor.".into(),
                    &format!("{path}.place"),
                ),
            })
        }
    }
}

/// A slope's or a level's elevation: given, else its place's.
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
        MeasuredPoint::Place {
            uid,
            place,
            path: pieces,
        } if pieces.is_empty() => object(doc, uid, path)
            .ok()
            .and_then(|e| kentos_domain::dimension_ties::place_elevation(e, *place)),
        MeasuredPoint::Place { .. } => None,
        MeasuredPoint::At { .. } => None,
    };
    kept.ok_or_else(|| {
        refuse(
            codes::MISSING_ELEVATION,
            format!(
                "{} için noktanın kotu yok: kotu metre olarak verin ya da noktayı kotlu bir köşeye bağlayın.",
                if path == "measure.z" { "Kot" } else { "Eğim" }
            ),
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
const NO_AXIS_EDGE: &str = "Nesnenin düz kenarı yok: eksen çizgisi iki düz kenar (çizgi ya da çoklu çizginin, alanın düz kenarı) arasında çizilir.";
const NO_CIRCLE_MARK: &str =
    "Nesne daire değil: merkez işareti daireye, yaya ya da çoklu çizginin yay parçasına konur.";
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
            Item::Base(uid) => {
                object_at(doc, uid, "measure.base")?;
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
        DimensionMeasure::Elevation { a, z } => {
            (Some(elevation_of(doc, *z, a, "measure.z")?), None)
        }
        _ => (None, None),
    };
    // Zincir and Baz take their base's style, look and height unless a style is named (AutoCAD's DIMCONTINUEMODE 1).
    let base = match &edges {
        Edges::Base(b) => Some(&b.0),
        _ => None,
    };
    let (mut look, style_height) = match (base, input.dimension_style.as_deref()) {
        (Some(d), None) => (d.look.clone(), d.height),
        (_, name) => style_of(doc, name)?,
    };
    // A centre mark's axis lines and an axis are centre lines (docs/adr/0322).
    if kind.is_mark() && look.ext_line_type.is_none() {
        look.ext_line_type = Some(kentos_contracts::LineType::Dashdot);
    }
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
    let g = match (&input.measure, &edges) {
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
            let (s, t) = (*s, *t);
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
            let ends = arc_length_ends(edge, None).ok_or_else(none)?;
            let offset = typed.map_or_else(|| dist(ends.c, at), f64::abs);
            geom(ends.a, ends.b, offset, None, Some(ends.c))
        }
        (
            DimensionMeasure::Radius { .. } | DimensionMeasure::Diameter { .. },
            Edges::Circle(c, r),
        ) => {
            let radial = radial_dimension(*c, *r, at);
            geom(*c, radial.b, radial.offset, None, None)
        }
        (DimensionMeasure::ArcLength { between, .. }, Edges::Arc(edge)) => {
            let ends =
                arc_length_ends(edge, between.map(|[p, q]| (v(p), v(q)))).ok_or_else(none)?;
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
            let (c, r) = (*c, *r);
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
        (DimensionMeasure::Continue { to, .. }, Edges::Base(b)) => {
            let base = &b.1;
            let to = pt(to, "to")?;
            let g = continue_dimension(base, base.b, to).ok_or_else(none)?;
            DimensionGeom {
                height,
                look: core_look(&look),
                ..g
            }
        }
        (DimensionMeasure::Baseline { to, .. }, Edges::Base(b)) => {
            let base = &b.1;
            let to = pt(to, "to")?;
            let g = baseline_dimension(base, to, 1.0, LEVEL_TEXT_HEIGHTS * base.height)
                .ok_or_else(none)?;
            DimensionGeom {
                height,
                look: core_look(&look),
                ..g
            }
        }
        (DimensionMeasure::CenterMark { .. }, Edges::Circle(c, r)) => {
            // Its point on the curve east of the centre: the cross upright.
            let b = Vec2::new(c.x + r, c.y);
            geom(*c, b, typed.unwrap_or(height), None, None)
        }
        (DimensionMeasure::CenterLine { first, second }, Edges::Two(s, t)) => {
            if first.uid == second.uid && (s == t || (s.0 == t.1 && s.1 == t.0)) {
                return Err(none());
            }
            let [a, b] = kentos_domain::dimension_ties::pair_axis(*s, *t);
            geom(a, b, typed.unwrap_or(height), None, None)
        }
        (DimensionMeasure::Elevation { a, .. }, _) => geom(pt(a, "a")?, at, 0.0, None, None),
        _ => return Err(none()),
    };
    if layout_dimension(&g).is_none() {
        return Err(none());
    }
    Ok(Measured {
        refs: bind(doc, kind, &input.measure, &g, &edges),
        geom: g,
        style: kind.contract(),
        look,
    })
}

/// What the measure's edges are, found on their objects; Zincir's and Baz's base dimension.
enum Edges {
    None,
    Two((Vec2, Vec2), (Vec2, Vec2)),
    Arc(Edge),
    Circle(Vec2, f64),
    Base(Box<(DimensionEntity, DimensionGeom)>),
}

/// A straight dimension as the core lays it out: aligned (no style) or linear; none for any other, or one that
/// does not lay out. Zincir ölçü and Baz ölçü go on from it (docs/adr/0140).
pub fn straight(d: &DimensionEntity) -> Option<DimensionGeom> {
    let style = match d.style {
        None => None,
        Some(DimensionStyle::Linear) => Some("linear".to_owned()),
        Some(_) => return None,
    };
    let g = DimensionGeom {
        a: v(d.a),
        b: v(d.b),
        offset: d.offset,
        height: d.height,
        style,
        angle: d.angle,
        c: None,
        za: None,
        zb: None,
        look: core_look(&d.look),
    };
    layout_dimension(&g).map(|_| g)
}

const NO_BASE: &str = "Temel ölçü hizalı ya da doğrusal bir ölçü değil: zincir ve baz ölçü yalnız hizalı ya da doğrusal ölçüden sürer.";

impl Edges {
    fn of(doc: &Document, m: &DimensionMeasure) -> Result<Edges, Stop> {
        Ok(match m {
            DimensionMeasure::Angle { first, second } => Edges::Two(
                edge_of(doc, first, "measure.first", straight_edge_at, NO_STRAIGHT)?,
                edge_of(doc, second, "measure.second", straight_edge_at, NO_STRAIGHT)?,
            ),
            DimensionMeasure::CenterLine { first, second } => Edges::Two(
                edge_of(doc, first, "measure.first", straight_edge_at, NO_AXIS_EDGE)?,
                edge_of(
                    doc,
                    second,
                    "measure.second",
                    straight_edge_at,
                    NO_AXIS_EDGE,
                )?,
            ),
            DimensionMeasure::CenterMark { of } => {
                let (c, r) = edge_of(doc, of, "measure.of", circle_at, NO_CIRCLE_MARK)?;
                Edges::Circle(c, r)
            }
            DimensionMeasure::AngleArc { of } | DimensionMeasure::ArcLength { of, .. } => {
                Edges::Arc(edge_of(doc, of, "measure.of", arc_edge_at, NO_ARC)?)
            }
            DimensionMeasure::Radius { of }
            | DimensionMeasure::Diameter { of }
            | DimensionMeasure::Jogged { of, .. } => {
                let (c, r) = edge_of(doc, of, "measure.of", circle_at, NO_CIRCLE)?;
                Edges::Circle(c, r)
            }
            DimensionMeasure::Continue { base, .. } | DimensionMeasure::Baseline { base, .. } => {
                let found = match object_at(doc, base, "measure.base")? {
                    Entity::Dimension(d) => straight(d).map(|g| (d.clone(), g)),
                    _ => None,
                };
                let (d, g) = found
                    .ok_or_else(|| refuse(codes::NOT_STRAIGHT, NO_BASE.into(), "measure.base"))?;
                Edges::Base(Box::new((d, g)))
            }
            _ => Edges::None,
        })
    }
}

/// A measured point's reference: its place's, when it is given as one.
fn reference(p: &MeasuredPoint) -> Option<SubRef> {
    match p {
        MeasuredPoint::Place { uid, place, path } => Uuid::parse_str(uid).ok().map(|u| SubRef {
            uid: EntityId(*u.as_bytes()),
            place: *place,
            path: path.clone(),
            broken: None,
        }),
        MeasuredPoint::At { .. } => None,
    }
}

/// The reference of the point of `e`'s edge of `kind` nearest to `near` (docs/adr/0320 §1).
fn on_edge(doc: &Document, e: &EdgeNear, kind: EdgeKind, near: Vec2) -> Option<SubRef> {
    let u = Uuid::parse_str(&e.uid).ok()?;
    let object = doc.get(doc.slot_of(u)?)?;
    let place = edge_place_near(&crate::blocks::core_entity(object), kind, near)?;
    Some(SubRef {
        uid: EntityId(*u.as_bytes()),
        place: kentos_domain::dimension_ties::contract_place(place),
        path: Vec::new(),
        broken: None,
    })
}

/// What the dimension's points follow (docs/adr/0249 §3, 0320 §1): the points given as places; an angle's two
/// edges, a radius's, a diameter's or a jogged radius's curve, an arc length's or an arc's angle's two ends on their
/// arc (with the rule that derives the rest); Zincir's first point what its base's second follows, Baz's what its
/// base's first follows.
fn bind(
    doc: &Document,
    kind: Kind,
    m: &DimensionMeasure,
    g: &DimensionGeom,
    edges: &Edges,
) -> Option<DimensionRefs> {
    let r = reference;
    let refs = match m {
        DimensionMeasure::Aligned { a, b }
        | DimensionMeasure::Linear { a, b, .. }
        | DimensionMeasure::Azimuth { a, b }
        | DimensionMeasure::Slope { a, b, .. } => DimensionRefs {
            a: r(a),
            b: r(b),
            ..Default::default()
        },
        DimensionMeasure::Ordinate { a, .. } => DimensionRefs {
            a: r(a),
            ..Default::default()
        },
        // The arms may have swapped for the sector `at` is in.
        DimensionMeasure::AngleVertex { c, a, b } => {
            let first = point_of(doc, a, "measure.a").ok();
            let (ra, rb) = if first == Some(g.a) {
                (r(a), r(b))
            } else {
                (r(b), r(a))
            };
            DimensionRefs {
                a: ra,
                b: rb,
                c: r(c),
                rule: None,
            }
        }
        DimensionMeasure::Angle { first, second } => {
            let Edges::Two(s, _) = edges else {
                return None;
            };
            let c = g.c?;
            // Which edge each arm lies along: the one its direction is nearer parallel to.
            let across = |p: Vec2, (a, b): (Vec2, Vec2)| {
                let (ux, uy) = (b.x - a.x, b.y - a.y);
                let (vx, vy) = (p.x - c.x, p.y - c.y);
                (ux * vy - uy * vx).abs()
                    / (kentos_geometry_core::jsmath::js_hypot(ux, uy)
                        * kentos_geometry_core::jsmath::js_hypot(vx, vy))
                    .max(f64::MIN_POSITIVE)
            };
            let a_on_first = across(g.a, *s) <= across(g.b, *s);
            let (ea, eb) = if a_on_first {
                (first, second)
            } else {
                (second, first)
            };
            DimensionRefs {
                a: on_edge(doc, ea, EdgeKind::Straight, v(ea.near)),
                b: on_edge(doc, eb, EdgeKind::Straight, v(eb.near)),
                c: None,
                rule: Some(DimensionRule::Edges),
            }
        }
        DimensionMeasure::AngleArc { of } | DimensionMeasure::ArcLength { of, .. } => {
            DimensionRefs {
                a: on_edge(doc, of, EdgeKind::Arc, g.a),
                b: on_edge(doc, of, EdgeKind::Arc, g.b),
                c: None,
                rule: Some(DimensionRule::Curve),
            }
        }
        DimensionMeasure::Radius { of }
        | DimensionMeasure::Diameter { of }
        | DimensionMeasure::Jogged { of, .. } => DimensionRefs {
            b: on_edge(doc, of, EdgeKind::Round, g.b),
            rule: Some(DimensionRule::Curve),
            ..Default::default()
        },
        DimensionMeasure::Continue { base, to } | DimensionMeasure::Baseline { base, to } => {
            let Edges::Base(b) = edges else {
                return None;
            };
            let d = &b.0;
            let from = d.refs.as_ref().and_then(|refs| {
                if kind == Kind::Continue {
                    refs.b.clone()
                } else {
                    refs.a.clone()
                }
            });
            let _ = base;
            DimensionRefs {
                a: from.filter(SubRef::live),
                b: r(to),
                ..Default::default()
            }
        }
        DimensionMeasure::CenterMark { of } => DimensionRefs {
            b: on_edge(doc, of, EdgeKind::Round, g.b),
            rule: Some(DimensionRule::Curve),
            ..Default::default()
        },
        DimensionMeasure::CenterLine { first, second } => DimensionRefs {
            a: on_edge(doc, first, EdgeKind::Straight, v(first.near)),
            b: on_edge(doc, second, EdgeKind::Straight, v(second.near)),
            c: None,
            rule: Some(DimensionRule::Pair),
        },
        DimensionMeasure::Elevation { a, .. } => DimensionRefs {
            a: r(a),
            ..Default::default()
        },
    };
    if refs.a.is_none() && refs.b.is_none() && refs.c.is_none() {
        return None;
    }
    // A rule wants all of its places: an angle both edges, an arc its two ends.
    if refs.rule.is_some()
        && !matches!(
            kind,
            Kind::Radius | Kind::Diameter | Kind::Jogged | Kind::CenterMark
        )
        && (refs.a.is_none() || refs.b.is_none())
    {
        return None;
    }
    Some(refs)
}

fn wire(p: Vec2) -> Point {
    Point { x: p.x, y: p.y }
}

/// The objects `cad.entities.create` writes for the dimension.
fn objects(input: &DimensionsCreate, m: &Measured) -> kentos_contracts::EntitiesCreate {
    let g = &m.geom;
    let geometry = EntityGeometry::Dimension {
        text_shift: None,
        breaks: Vec::new(),
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
        refs: m.refs.clone().map(Box::new),
        look: m.look.clone(),
    };
    // Zincir ölçü and Baz ölçü keep their tools' step names (docs/adr/0140).
    let operation = match input.measure {
        DimensionMeasure::Continue { .. } => {
            Some(kentos_contracts::CreateOperation::DimensionChain)
        }
        DimensionMeasure::Baseline { .. } => {
            Some(kentos_contracts::CreateOperation::DimensionBaseline)
        }
        _ => None,
    };
    creating::input_as(&input.layer_id, vec![geometry], &input.look, operation)
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
