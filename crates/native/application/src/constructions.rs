//! `cad.constructions.create` v1 (docs/adr/0622 §9): the construction helpers' objects computed from what the user
//! gives (Paralel çizgi, Dik in, Dik çık, İki daireye teğet, Dördüncü köşe, Menzil halkaları, Ara nokta, Kesişim
//! noktası, Yardımcı çizgi's Açıortay and Paralel) with the shared core, written as `cad.entities.create` writes them,
//! one undo step named after the tool. The shared cases are `fixtures/commands/v1/cad.constructions.create.json`
//! (`scripts/fixtures/construction_cases.py`).

use kentos_commands::{
    CommandResult, Construction, ConstructionRound, ConstructionsCreate, CreateOperation,
    EntitiesCreate, EntitiesCreatePlan, EntitiesCreated, EntityGeometry, Meeting, NewObject,
    PerpendicularReference, Vec2 as Wire,
};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::intersect::line_line;
use kentos_geometry_core::geom::parallel::{clean_axis, corridor_area, parallel_sides};
use kentos_geometry_core::geom::survey::{distance_intersection, side_offsets, side_point};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::construct::{Between, bearing_bearing, points_between};
use kentos_geometry_core::tools::drawing::{radial_point, xline_direction, xline_offset};
use kentos_geometry_core::tools::drawing_extras::{
    Round, chosen_tangent, common_tangents, fourth_corner, range_rings,
};
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes, create};

fn v(p: Wire) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn w(p: Vec2) -> Wire {
    Wire { x: p.x, y: p.y }
}

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn point(p: Wire, whose: &str, path: &str) -> Result<Vec2, Stop> {
    checks::point(p, whose, path)?;
    Ok(v(p))
}

fn number(value: f64, what: &str, path: &str) -> Result<f64, Stop> {
    checks::finite(value, what, "Sonlu bir sayı verin.", path)?;
    Ok(value)
}

fn line(a: Vec2, b: Vec2) -> EntityGeometry {
    EntityGeometry::Line {
        ms: None,
        a: w(a),
        b: w(b),
        zs: None,
    }
}

fn dot(p: Vec2) -> EntityGeometry {
    EntityGeometry::Point {
        p: w(p),
        z: None,
        parts: None,
    }
}

fn path_geometry(pts: &[Vec2], closed: bool) -> EntityGeometry {
    let pts = pts.iter().copied().map(w).collect();
    if closed {
        EntityGeometry::Polygon {
            pts,
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
        }
    } else {
        EntityGeometry::Polyline {
            ms: None,
            pts,
            bulges: None,
            zs: None,
            parts: None,
            closed: false,
            widths: None,
        }
    }
}

fn area_geometry(area: &Area) -> EntityGeometry {
    let ring = |r: &kentos_geometry_core::geom::arrangement::Ring| kentos_commands::RingGeometry {
        pts: r.pts.iter().copied().map(w).collect(),
        bulges: r.bulges.clone(),
        zs: None,
    };
    let outer = ring(&area.outer);
    EntityGeometry::Polygon {
        pts: outer.pts,
        bulges: outer.bulges,
        holes: (!area.holes.is_empty()).then(|| area.holes.iter().map(ring).collect()),
        zs: None,
        parts: None,
    }
}

const NO_LENGTH: &str = "Çizginin iki noktası çakışıyor; uzunluğu olan bir çizgi verin.";

/// The objects of the construction and the step they are written in.
fn objects(c: &Construction) -> Result<(Vec<EntityGeometry>, Option<CreateOperation>), Stop> {
    let at = |field: &str| format!("construction.{field}");
    match c {
        Construction::Parallel {
            axis,
            closed,
            left,
            right,
            with_axis,
            area,
        } => {
            let mut pts = Vec::with_capacity(axis.len());
            for (i, p) in axis.iter().enumerate() {
                pts.push(point(*p, "Eksenin köşesinin", &at(&format!("axis[{i}]")))?);
            }
            let left = number(*left, "Sol mesafe", &at("left"))?;
            let right = number(*right, "Sağ mesafe", &at("right"))?;
            if left < 0.0 || right < 0.0 {
                return Err(refuse(
                    codes::INVALID_DISTANCE,
                    "Sol ve sağ mesafe sıfır ya da daha büyük olmalı.",
                    &at(if left < 0.0 { "left" } else { "right" }),
                ));
            }
            let axis = clean_axis(&pts, *closed);
            if axis.len() < if *closed { 3 } else { 2 } {
                return Err(refuse(
                    codes::DEGENERATE_SHAPE,
                    if *closed {
                        "Kapalı eksenin en az üç ayrı köşesi olmalı."
                    } else {
                        "Eksenin en az iki ayrı noktası olmalı."
                    },
                    &at("axis"),
                ));
            }
            let mut out = Vec::new();
            if *area {
                if let Some(a) = corridor_area(&axis, left, right, *closed) {
                    out.push(area_geometry(&a));
                }
            } else {
                let sides = parallel_sides(&axis, left, right, *closed);
                out.extend(
                    [sides.left, sides.right]
                        .iter()
                        .flatten()
                        .map(|s| path_geometry(s, *closed)),
                );
            }
            if *with_axis {
                out.push(path_geometry(&axis, *closed));
            }
            if out.is_empty() {
                return Err(refuse(
                    codes::INVALID_DISTANCE,
                    "Sol ve sağ mesafe sıfır ve eksen çizilmiyor: çizilecek bir şey yok.",
                    &at("left"),
                ));
            }
            Ok((out, Some(CreateOperation::Parallel)))
        }
        Construction::PerpendicularIn { reference, points } => {
            let foot: Box<dyn Fn(Vec2) -> Option<Vec2>> = match reference {
                PerpendicularReference::Line { a, b } => {
                    let a = point(*a, "Hattın başının", &at("reference.a"))?;
                    let b = point(*b, "Hattın sonunun", &at("reference.b"))?;
                    if dist(a, b) < 1e-9 {
                        return Err(refuse(codes::DEGENERATE_SHAPE, NO_LENGTH, &at("reference")));
                    }
                    Box::new(move |p| {
                        side_offsets(a, b, p).and_then(|o| side_point(a, b, o.absis, 0.0))
                    })
                }
                PerpendicularReference::Circle { c, r } => {
                    let c = point(*c, "Dairenin merkezinin", &at("reference.c"))?;
                    let r = number(*r, "Yarıçap", &at("reference.r"))?;
                    if r <= 0.0 {
                        return Err(refuse(
                            codes::INVALID_DISTANCE,
                            "Yarıçap sıfırdan büyük olmalı.",
                            &at("reference.r"),
                        ));
                    }
                    Box::new(move |p| radial_point(c, r, p))
                }
            };
            if points.is_empty() {
                return Err(refuse(
                    codes::TOO_FEW_POINTS,
                    "Dik inilecek en az bir nokta verin.",
                    &at("points"),
                ));
            }
            let mut out = Vec::new();
            for (i, p) in points.iter().enumerate() {
                let path = at(&format!("points[{i}]"));
                let p = point(*p, "Noktanın", &path)?;
                let Some(f) = foot(p) else {
                    return Err(refuse(
                        codes::DEGENERATE_SHAPE,
                        "Bu noktadan dik inilemez: nokta dairenin merkezinde.",
                        &path,
                    ));
                };
                if dist(p, f) < 1e-9 {
                    return Err(refuse(
                        codes::DEGENERATE_SHAPE,
                        "Nokta zaten hattın üzerinde.",
                        &path,
                    ));
                }
                out.push(line(p, f));
            }
            Ok((out, Some(CreateOperation::PerpendicularIn)))
        }
        Construction::PerpendicularOut { a, b, stations } => {
            let a = point(*a, "Hattın başının", &at("a"))?;
            let b = point(*b, "Hattın sonunun", &at("b"))?;
            if dist(a, b) < 1e-9 {
                return Err(refuse(codes::DEGENERATE_SHAPE, NO_LENGTH, &at("b")));
            }
            if stations.is_empty() {
                return Err(refuse(
                    codes::TOO_FEW_POINTS,
                    "En az bir dik ayak ve dik boy verin.",
                    &at("stations"),
                ));
            }
            let mut out = Vec::new();
            for (i, s) in stations.iter().enumerate() {
                let along = number(s.along, "Dik ayak", &at(&format!("stations[{i}].along")))?;
                let offset = number(s.offset, "Dik boy", &at(&format!("stations[{i}].offset")))?;
                if offset == 0.0 {
                    return Err(refuse(
                        codes::INVALID_DISTANCE,
                        "Dik boy sıfır olamaz.",
                        &at(&format!("stations[{i}].offset")),
                    ));
                }
                let (Some(foot), Some(end)) = (
                    side_point(a, b, along, 0.0),
                    side_point(a, b, along, offset),
                ) else {
                    return Err(refuse(codes::DEGENERATE_SHAPE, NO_LENGTH, &at("b")));
                };
                out.push(line(foot, end));
            }
            Ok((out, Some(CreateOperation::PerpendicularOut)))
        }
        Construction::TangentLine {
            first,
            second,
            near_first,
            near_second,
        } => {
            let round = |r: &ConstructionRound, name: &str| -> Result<Round, Stop> {
                let c = point(r.c, "Merkezin", &at(&format!("{name}.c")))?;
                let rad = number(r.r, "Yarıçap", &at(&format!("{name}.r")))?;
                if rad <= 0.0 {
                    return Err(refuse(
                        codes::INVALID_DISTANCE,
                        "Yarıçap sıfırdan büyük olmalı.",
                        &at(&format!("{name}.r")),
                    ));
                }
                let span = match (r.a0, r.a1) {
                    (Some(a0), Some(a1)) => {
                        let a0 = number(a0, "Yayın başlangıç açısı", &at(&format!("{name}.a0")))?;
                        let a1 = number(a1, "Yayın bitiş açısı", &at(&format!("{name}.a1")))?;
                        let shape = kentos_geometry_core::entity::Shape::Arc { c, r: rad, a0, a1 };
                        return Round::of(&shape).ok_or_else(|| {
                            refuse(
                                codes::INVALID_DISTANCE,
                                "Yarıçap sıfırdan büyük olmalı.",
                                &at(&format!("{name}.r")),
                            )
                        });
                    }
                    _ => None,
                };
                Ok(Round { c, r: rad, span })
            };
            let (one, two) = (round(first, "first")?, round(second, "second")?);
            let p1 = point(*near_first, "Birinci dairedeki noktanın", &at("nearFirst"))?;
            let p2 = point(*near_second, "İkinci dairedeki noktanın", &at("nearSecond"))?;
            let t = chosen_tangent(&common_tangents(&one, &two), p1, p2).ok_or_else(|| {
                refuse(
                    codes::NO_INTERSECTION,
                    "İki dairenin ortak teğeti yok: biri ötekinin içinde ya da teğetin noktası yayın dışında.",
                    &at("second"),
                )
            })?;
            Ok((vec![line(t.a, t.b)], Some(CreateOperation::TangentLine)))
        }
        Construction::FourthCorner { a, b, c, area } => {
            let a = point(*a, "Birinci köşenin", &at("a"))?;
            let b = point(*b, "Ortadaki köşenin", &at("b"))?;
            let c = point(*c, "Üçüncü köşenin", &at("c"))?;
            // The tool's rule (docs/adr/0197 §2): the turn at b against the arms' lengths.
            let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
            if cross.abs() <= 1e-9 * dist(a, b) * dist(b, c) {
                return Err(refuse(
                    codes::DEGENERATE_SHAPE,
                    "Üç köşe bir doğru üzerinde: paralelkenar olmaz. Başka bir köşe verin.",
                    &at("c"),
                ));
            }
            let d = fourth_corner(a, b, c);
            let g = if *area {
                path_geometry(&[a, b, c, d], true)
            } else {
                dot(d)
            };
            Ok((vec![g], Some(CreateOperation::FourthCorner)))
        }
        Construction::RangeRings {
            centre,
            spacing,
            count,
            rays,
        } => {
            let c = point(*centre, "Merkezin", &at("centre"))?;
            let s = number(*spacing, "Aralık", &at("spacing"))?;
            if s <= 0.0 {
                return Err(refuse(
                    codes::INVALID_DISTANCE,
                    "Halkaların aralığı sıfırdan büyük olmalı.",
                    &at("spacing"),
                ));
            }
            if !(1..=100).contains(count) {
                return Err(refuse(
                    codes::INVALID_COUNT,
                    "Halka sayısı 1 ile 100 arasında olmalı.",
                    &at("count"),
                ));
            }
            if *rays > 360 {
                return Err(refuse(
                    codes::INVALID_COUNT,
                    "Işın sayısı 0 ile 360 arasında olmalı.",
                    &at("rays"),
                ));
            }
            let rings = range_rings(c, s, *count, *rays).ok_or_else(|| {
                refuse(
                    codes::INVALID_COUNT,
                    "Halka sayısı 1 ile 100 arasında olmalı.",
                    &at("count"),
                )
            })?;
            let mut out: Vec<EntityGeometry> = rings
                .radii
                .iter()
                .map(|r| EntityGeometry::Circle { c: w(c), r: *r })
                .collect();
            out.extend(rings.rays.iter().map(|end| line(c, *end)));
            Ok((out, Some(CreateOperation::RangeRings)))
        }
        Construction::PointsBetween {
            a,
            b,
            parts,
            distances,
            ratios,
        } => {
            let a = point(*a, "Başlangıç noktasının", &at("a"))?;
            let b = point(*b, "Bitiş noktasının", &at("b"))?;
            if dist(a, b) < 1e-9 {
                return Err(refuse(
                    codes::DEGENERATE_SHAPE,
                    "İki nokta çakışıyor; aralarında nokta konmaz.",
                    &at("b"),
                ));
            }
            let given = [parts.is_some(), distances.is_some(), ratios.is_some()]
                .iter()
                .filter(|g| **g)
                .count();
            if given != 1 {
                return Err(refuse(
                    codes::INVALID_COUNT,
                    "Parça sayısı (parts), uzaklıklar (distances) ya da oranlardan (ratios) yalnız birini verin.",
                    &at("parts"),
                ));
            }
            let how = if let Some(n) = parts {
                if !(2..=10_000).contains(n) {
                    return Err(refuse(
                        codes::INVALID_COUNT,
                        "Parça sayısı 2 ile 10 000 arasında olmalı.",
                        &at("parts"),
                    ));
                }
                Between::Parts(f64::from(*n))
            } else if let Some(ds) = distances {
                for (i, d) in ds.iter().enumerate() {
                    number(*d, "Uzaklık", &at(&format!("distances[{i}]")))?;
                }
                Between::Distances(ds.clone())
            } else {
                let ts = ratios.clone().unwrap_or_default();
                for (i, t) in ts.iter().enumerate() {
                    number(*t, "Oran", &at(&format!("ratios[{i}]")))?;
                }
                Between::Ratios(ts)
            };
            let pts = points_between(a, b, &how).unwrap_or_default();
            if pts.is_empty() || pts.len() > 10_000 {
                return Err(refuse(
                    codes::INVALID_COUNT,
                    "1 ile 10 000 arasında nokta verin.",
                    &at("parts"),
                ));
            }
            Ok((
                pts.into_iter().map(dot).collect(),
                Some(CreateOperation::PointsBetween),
            ))
        }
        Construction::Intersection { meeting } => {
            let p = match meeting {
                Meeting::Distances { a, da, b, db, near } => {
                    let a = point(*a, "A noktasının", &at("meeting.a"))?;
                    let da = number(*da, "A'dan uzaklık", &at("meeting.da"))?;
                    let b = point(*b, "B noktasının", &at("meeting.b"))?;
                    let db = number(*db, "B'den uzaklık", &at("meeting.db"))?;
                    if da <= 0.0 || db <= 0.0 {
                        return Err(refuse(
                            codes::INVALID_DISTANCE,
                            "Uzaklıklar sıfırdan büyük olmalı.",
                            &at(if da <= 0.0 {
                                "meeting.da"
                            } else {
                                "meeting.db"
                            }),
                        ));
                    }
                    if dist(a, b) < 1e-9 {
                        return Err(refuse(
                            codes::DEGENERATE_SHAPE,
                            "A ve B çakışıyor.",
                            &at("meeting.b"),
                        ));
                    }
                    let near = match near {
                        Some(n) => Some(point(*n, "Yakın noktanın", &at("meeting.near"))?),
                        None => None,
                    };
                    let meets = distance_intersection(a, b, da, db);
                    let chosen = match near {
                        Some(n) => meets
                            .iter()
                            .copied()
                            .min_by(|p, q| dist(*p, n).total_cmp(&dist(*q, n))),
                        None => meets.first().copied(),
                    };
                    chosen.ok_or_else(|| {
                        refuse(
                            codes::NO_INTERSECTION,
                            "Uzaklıklar kesişmiyor: daireler birbirine ulaşmıyor ya da biri ötekinin içinde.",
                            &at("meeting.db"),
                        )
                    })?
                }
                Meeting::Bearings { a, ta, b, tb } => {
                    let a = point(*a, "A noktasının", &at("meeting.a"))?;
                    let ta = number(*ta, "A'dan doğrultu", &at("meeting.ta"))?;
                    let b = point(*b, "B noktasının", &at("meeting.b"))?;
                    let tb = number(*tb, "B'den doğrultu", &at("meeting.tb"))?;
                    bearing_bearing(a, ta, b, tb).ok_or_else(|| {
                        refuse(
                            codes::NO_INTERSECTION,
                            "Doğrultular kesişmiyor: paralel ya da kesişim noktalardan birinin arkasında kalıyor.",
                            &at("meeting.tb"),
                        )
                    })?
                }
                Meeting::Lines { a, b, c, d } => {
                    let a = point(*a, "Birinci doğrunun ilk noktasının", &at("meeting.a"))?;
                    let b = point(*b, "Birinci doğrunun ikinci noktasının", &at("meeting.b"))?;
                    let c = point(*c, "İkinci doğrunun ilk noktasının", &at("meeting.c"))?;
                    let d = point(*d, "İkinci doğrunun ikinci noktasının", &at("meeting.d"))?;
                    if dist(a, b) < 1e-9 || dist(c, d) < 1e-9 {
                        return Err(refuse(
                            codes::DEGENERATE_SHAPE,
                            NO_LENGTH,
                            &at(if dist(a, b) < 1e-9 {
                                "meeting.b"
                            } else {
                                "meeting.d"
                            }),
                        ));
                    }
                    line_line(a, b, c, d).map(|h| h.p).ok_or_else(|| {
                        refuse(
                            codes::NO_INTERSECTION,
                            "Doğrular paralel; kesişmiyorlar.",
                            &at("meeting.d"),
                        )
                    })?
                }
            };
            Ok((vec![dot(p)], Some(CreateOperation::IntersectPoint)))
        }
        Construction::Bisector { vertex, p1, p2 } => {
            let o = point(*vertex, "Köşenin", &at("vertex"))?;
            let p1 = point(*p1, "Birinci kolun", &at("p1"))?;
            let p2 = point(*p2, "İkinci kolun", &at("p2"))?;
            let dir = xline_direction("bisect", &[o, p1], p2, 0.0).ok_or_else(|| {
                refuse(
                    codes::DEGENERATE_SHAPE,
                    "Kolun noktası köşeyle çakışıyor; kollar uzunluğu olan doğrular olmalı.",
                    &at("p2"),
                )
            })?;
            Ok((
                vec![EntityGeometry::Xline {
                    p: w(o),
                    dir: w(dir),
                }],
                None,
            ))
        }
        Construction::XlineOffset {
            a,
            b,
            side,
            distance,
        } => {
            let a = point(*a, "Kenarın başının", &at("a"))?;
            let b = point(*b, "Kenarın sonunun", &at("b"))?;
            let side = point(*side, "Yanın noktasının", &at("side"))?;
            if let Some(d) = distance {
                let d = number(*d, "Uzaklık", &at("distance"))?;
                if d < 0.0 {
                    return Err(refuse(
                        codes::INVALID_DISTANCE,
                        "Uzaklık sıfır ya da daha büyük olmalı.",
                        &at("distance"),
                    ));
                }
            }
            if dist(a, b) < 1e-9 {
                return Err(refuse(codes::DEGENERATE_SHAPE, NO_LENGTH, &at("b")));
            }
            let (p, dir) = xline_offset(a, b, *distance, side).ok_or_else(|| {
                refuse(
                    codes::DEGENERATE_SHAPE,
                    "Yanın noktası kenarın doğrusunun üstünde; bir yan gösterin.",
                    &at("side"),
                )
            })?;
            Ok((
                vec![EntityGeometry::Xline {
                    p: w(p),
                    dir: w(dir),
                }],
                None,
            ))
        }
    }
}

/// The objects as `cad.entities.create`'s input.
fn create_input(input: &ConstructionsCreate) -> Result<EntitiesCreate, Stop> {
    let (geometries, operation) = objects(&input.construction)?;
    Ok(EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: geometries
            .into_iter()
            .map(|geometry| NewObject {
                // The weight and line type go to what is drawn with lines, as the tools gave them (docs/adr/0139,
                // 0260 §2).
                line_weight: geometry.draws_lines().then_some(input.line_weight).flatten(),
                line_type: geometry.draws_lines().then_some(input.line_type).flatten(),
                geometry,
                color: input.color.clone(),
                attrs: input.attrs.clone(),
                label: input.label.clone(),
                symbol: input.symbol.clone(),
                label_of: None,
                label_scale: None,
                order: None,
                transparency: None,
            })
            .collect(),
        // A construction line is written in the document's own step, “Ekle”, as the tool's was.
        operation,
        expected_revision: input.expected_revision.clone(),
    })
}

/// `cad.entities.create`'s answer in this command's words: a refusal of one object's look (`objects[2].lineWeight`) is
/// the input's own field (`lineWeight`), since every object takes the look the input gives.
fn ours<T>(result: CommandResult<T>) -> CommandResult<T> {
    fn path(error: &mut kentos_commands::CommandError) {
        if let Some(p) = &error.path
            && let Some(rest) = p.strip_prefix("objects[")
            && let Some((_, field)) = rest.split_once("].")
        {
            error.path = Some(field.to_owned());
        }
    }
    match result {
        CommandResult::Failed { mut error } => {
            path(&mut error);
            CommandResult::Failed { error }
        }
        CommandResult::Conflict { mut error } => {
            path(&mut error);
            CommandResult::Conflict { error }
        }
        CommandResult::NeedsInput { mut error } => {
            path(&mut error);
            CommandResult::NeedsInput { error }
        }
        other => other,
    }
}

/// The construction's checks and `cad.entities.create`'s.
pub fn validate(cx: &ExecutionContext<'_>, input: &ConstructionsCreate) -> CommandResult<()> {
    match create_input(input) {
        Ok(create) => ours(create::validate(cx, &create)),
        Err(stop) => stop.into(),
    }
}

/// The objects as they would be written.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ConstructionsCreate,
) -> CommandResult<EntitiesCreatePlan> {
    match create_input(input) {
        Ok(create) => ours(create::plan(cx, &create)),
        Err(stop) => stop.into(),
    }
}

/// The objects a tool's preview draws: what the command would write, or its refusal.
pub fn geometries(
    input: &ConstructionsCreate,
) -> Result<Vec<EntityGeometry>, Box<kentos_commands::CommandError>> {
    objects(&input.construction)
        .map(|(g, _)| g)
        .map_err(|stop| Box::new(stop.into_error()))
}

/// Writes the objects, one undo step named after the tool.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ConstructionsCreate,
) -> CommandResult<EntitiesCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_CONSTRUCTIONS_CREATE,
        kentos_commands::CAD_CONSTRUCTIONS_CREATE_VERSION,
        input,
        |cx, input| match create_input(&input) {
            Ok(create) => ours(create::write_in(cx, create)),
            Err(stop) => stop.into(),
        },
    )
}
