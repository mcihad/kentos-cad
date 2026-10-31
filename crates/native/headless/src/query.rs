//! Reading a drawing without writing it: its objects by page (the web's
//! and the server's wire form, each with its persistent id) and an object's
//! measures from the shared geometry core (TODOS.md AI-05, AI-06).

use kentos_commands::{AngleUnit, Bounds, DrawingUnit, Entity, ProjectSettings, Workspace};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::crs::System;
use kentos_geometry_core::crs::ground::ground_measures;
use kentos_geometry_core::crs::measure::Ring;
use kentos_geometry_core::entity::{ellipse_geom, entity_area, entity_bounds, entity_length};
use kentos_geometry_core::geom::curve_outline::{ellipse_outline, spline_outline};
use kentos_geometry_core::geom::ellipse::is_full_ellipse;
use kentos_geometry_core::vec2::Vec2 as P;
use kentos_native_application::geometry::shape;
use serde::{Deserialize, Serialize};

use crate::HeadlessError;

/// The most objects one page gives; a program reads on with the cursor.
pub const PAGE_MAX: usize = 10_000;

/// Which objects a page reads.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filter {
    /// Only on this layer (a layer's id).
    pub layer: Option<String>,
    /// Only of these kinds (`line`, `polygon`, `text`…).
    pub kinds: Option<Vec<String>>,
    /// Only those whose box meets this one.
    pub bbox: Option<Bounds>,
    /// After this object (its persistent id), in the drawing's order.
    pub after: Option<String>,
    /// At most this many, at most [`PAGE_MAX`].
    pub limit: Option<usize>,
}

/// One object as a page gives it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Item<'a> {
    /// Its persistent id (lowercase UUID text with hyphens).
    pub uid: String,
    /// Its state stamp (docs/adr/0253 §2): what a call expects of it.
    pub stamp: String,
    pub entity: &'a Entity,
}

/// A page of objects in the drawing's order; `next` continues it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Page<'a> {
    pub items: Vec<Item<'a>>,
    /// The cursor of the next page (the last object's id); none at the end.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

/// The units of the values a drawing gives and takes (docs/adr/0245):
/// coordinates, lengths and areas are in the project's plane (metres, or
/// degrees in a geographic system) whatever the drawing unit, which is only
/// how the windows read and write them (docs/adr/0165 §2).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Units {
    /// Coordinates and lengths: `m`, or `degree` in a geographic system.
    pub length: &'static str,
    /// Areas: `m2`, or `degree2`.
    pub area: &'static str,
    /// The project's drawing unit (`mm`, `cm`, `m`): a local project's own, else metres.
    pub drawing: DrawingUnit,
}

/// The units of a project with these settings.
pub fn units(settings: &ProjectSettings) -> Units {
    let degrees = kentos_project::systems::own(settings).is_some_and(|s| s.geographic());
    Units {
        length: if degrees { "degree" } else { "m" },
        area: if degrees { "degree2" } else { "m2" },
        drawing: settings.drawing_unit.unwrap_or(DrawingUnit::M),
    }
}

/// How a measure is taken (docs/adr/0256 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    /// In the project's plane: metres, degrees in a geographic system; arcs,
    /// circles and ellipses from their source (CLAUDE.md §23.3).
    #[default]
    Plane,
    /// On the project's ellipsoid: geodesics (docs/adr/0171), metres.
    Ellipsoid,
    /// On the ground at the project's mean ellipsoidal height, metres.
    Ground,
}

impl Method {
    /// The method a request names; none: the plane.
    pub fn named(name: Option<&str>) -> Result<Method, HeadlessError> {
        match name {
            None | Some("plane") => Ok(Method::Plane),
            Some("ellipsoid") => Ok(Method::Ellipsoid),
            Some("ground") => Ok(Method::Ground),
            Some(other) => Err(HeadlessError::new(
                "invalid_input",
                format!("method plane, ellipsoid ya da ground olmalı (“{other}” verildi)."),
            )),
        }
    }
}

/// What a measure was taken in and how (docs/adr/0256 §4).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Basis {
    /// The project's system: its EPSG code; 0 for none (a local project) or
    /// for a definition of the project's own.
    pub srid: u32,
    /// Its name: “TUREF / TM33”, a definition's, “Yerel (koordinat sistemi yok)”.
    pub system: String,
    /// `local`, `projected` or `geographic`.
    pub kind: &'static str,
    /// The frame the coordinates and the box are in: `world`, the drawing's
    /// own (the wire's; docs/adr/0256 §5).
    pub frame: &'static str,
    pub method: Method,
    /// The ground's mean ellipsoidal height, m (method `ground`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

/// The basis of a measure taken by `method` in a project with these settings.
pub fn basis(settings: &ProjectSettings, method: Method) -> Basis {
    let named = kentos_project::systems::own(settings);
    let (system, kind) = match &named {
        None => ("Yerel (koordinat sistemi yok)".to_owned(), "local"),
        Some(n) if n.geographic() => (n.name.clone(), "geographic"),
        Some(n) => (n.name.clone(), "projected"),
    };
    Basis {
        srid: settings.srid,
        system,
        kind,
        frame: "world",
        method,
        height: (method == Method::Ground)
            .then(|| settings.ground_height())
            .flatten(),
    }
}

/// The units of a measure taken by `method`: the plane's, or metres on the
/// ellipsoid and the ground whatever the system.
pub fn units_by(settings: &ProjectSettings, method: Method) -> Units {
    match method {
        Method::Plane => units(settings),
        Method::Ellipsoid | Method::Ground => Units {
            length: "m",
            area: "m2",
            drawing: settings.drawing_unit.unwrap_or(DrawingUnit::M),
        },
    }
}

/// An object's measures from its source geometry (not a drawn
/// approximation; CLAUDE.md §23.3), in the units `units` names, taken as
/// `basis` says: by default metres in the project's plane, never the
/// drawing unit.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    pub uid: String,
    pub kind: String,
    /// Area of a closed shape (`units.area`); none for an open one, and on
    /// the ellipsoid or the ground for an object no ring stands for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<f64>,
    /// Length of a line or path, perimeter of a closed shape (`units.length`); none for a point or text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    pub bounds: Bounds,
    pub units: Units,
    pub basis: Basis,
}

/// One object of a batch: its measures without the batch's units and basis.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasureItem {
    pub uid: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    pub bounds: Bounds,
}

/// A batch's sums: of the objects that have an area, of those that have a length.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub count: usize,
    pub area: f64,
    pub length: f64,
}

/// Many objects' measures taken one way (docs/adr/0256 §4): each object,
/// the sums, and the units and basis once.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measures {
    pub items: Vec<MeasureItem>,
    pub totals: Totals,
    pub units: Units,
    pub basis: Basis,
}

/// The most objects a batch measures.
pub const MEASURE_MAX: usize = 1000;

/// The axes and angles a project's values and windows speak (docs/adr/0165
/// §4, docs/adr/0256 §5): on the wire x is east and y north, always; the
/// windows name them X, Y in CAD and Y, X in CBS; polar input and readings
/// go from east counter-clockwise in CAD, from north clockwise in CBS.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conventions {
    /// `cad` or `gis` (a project whose type is not asked yet shows as CBS).
    pub workspace: &'static str,
    /// What the wire's x and y are: `east`, `north`.
    pub axes: Axes,
    /// What the windows call the wire's x and y.
    pub labels: Axes,
    /// Where polar angles start and which way they turn.
    pub polar: Polar,
    /// The angle unit the windows show and take: `deg` or `grad`.
    pub angle_unit: AngleUnit,
    /// The drawing's user coordinate system (docs/adr/0261 §5): how its
    /// windows take and show points; absent, the world's. The wire stays
    /// the world: a caller working in the UCS turns its points itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ucs: Option<kentos_commands::UcsNow>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Axes {
    pub x: &'static str,
    pub y: &'static str,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Polar {
    /// `east` or `north`.
    pub zero: &'static str,
    /// `ccw` (counter-clockwise) or `cw`.
    pub turn: &'static str,
}

/// The conventions of a project with these settings.
pub fn conventions(settings: &ProjectSettings) -> Conventions {
    let cad = settings.project_type() == Some(Workspace::Cad);
    Conventions {
        workspace: if cad { "cad" } else { "gis" },
        axes: Axes {
            x: "east",
            y: "north",
        },
        labels: if cad {
            Axes { x: "X", y: "Y" }
        } else {
            Axes { x: "Y", y: "X" }
        },
        polar: if cad {
            Polar {
                zero: "east",
                turn: "ccw",
            }
        } else {
            Polar {
                zero: "north",
                turn: "cw",
            }
        },
        angle_unit: settings.angle_unit,
        ucs: settings.ucs.map(|u| kentos_commands::UcsNow {
            origin: u.origin,
            angle: u.angle,
            name: settings
                .named_ucs
                .iter()
                .find(|n| n.origin == u.origin && n.angle == u.angle)
                .map(|n| n.name.clone()),
        }),
    }
}

pub fn slot_of(doc: &Document, uid: &str) -> Result<Slot, HeadlessError> {
    let id = Uuid::parse_str(uid).map_err(|_| {
        HeadlessError::new(
            "invalid_input",
            format!("“{uid}” bir nesne kimliği değil: küçük harfli, tireli UUID olmalı."),
        )
    })?;
    doc.slot_of(id).ok_or_else(|| {
        HeadlessError::new(
            "unknown_object",
            format!("{uid} kimlikli nesne bu çizimde yok (silinmiş ya da başka bir çizimin)."),
        )
    })
}

fn uid_text(doc: &Document, slot: Slot) -> String {
    doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
}

fn meets(a: &Bounds, b: &Bounds) -> bool {
    a.min_x <= b.max_x && b.min_x <= a.max_x && a.min_y <= b.max_y && b.min_y <= a.max_y
}

pub fn page<'a>(doc: &'a Document, filter: &Filter) -> Result<Page<'a>, HeadlessError> {
    let limit = filter.limit.unwrap_or(1000).clamp(1, PAGE_MAX);
    let start = match &filter.after {
        Some(uid) => Some(slot_of(doc, uid)?),
        None => None,
    };
    let objects: Box<dyn Iterator<Item = &Entity>> = match &filter.layer {
        Some(layer) => Box::new(doc.by_layer(layer)),
        None => Box::new(doc.entities()),
    };
    let mut past = start.is_none();
    let mut items = Vec::new();
    let mut more = false;
    for e in objects {
        let slot = Slot(e.base().id);
        if !past {
            past = Some(slot) == start;
            continue;
        }
        if let Some(kinds) = &filter.kinds
            && !kinds.iter().any(|k| k == e.kind())
        {
            continue;
        }
        if let Some(bbox) = &filter.bbox {
            let b = entity_bounds(&shape(e));
            let b = Bounds {
                min_x: b.min_x,
                min_y: b.min_y,
                max_x: b.max_x,
                max_y: b.max_y,
            };
            if !meets(&b, bbox) {
                continue;
            }
        }
        if items.len() == limit {
            more = true;
            break;
        }
        items.push(Item {
            uid: uid_text(doc, slot),
            stamp: doc.stamp(slot).unwrap_or_default(),
            entity: e,
        });
    }
    let next = if more {
        items.last().map(|i| i.uid.clone())
    } else {
        None
    };
    Ok(Page { items, next })
}

pub fn measure(doc: &Document, uid: &str) -> Result<Measure, HeadlessError> {
    measure_by(doc, uid, Method::Plane)
}

/// An object's measures taken by `method`.
pub fn measure_by(doc: &Document, uid: &str, method: Method) -> Result<Measure, HeadlessError> {
    let way = Way::of(doc.settings(), method)?;
    let item = item(doc, uid, &way)?;
    Ok(Measure {
        uid: item.uid,
        kind: item.kind,
        area: item.area,
        length: item.length,
        bounds: item.bounds,
        units: units_by(doc.settings(), method),
        basis: basis(doc.settings(), method),
    })
}

/// Many objects' measures taken by `method`, at most [`MEASURE_MAX`].
pub fn measure_many(
    doc: &Document,
    uids: &[String],
    method: Method,
) -> Result<Measures, HeadlessError> {
    if uids.len() > MEASURE_MAX {
        return Err(HeadlessError::new(
            "invalid_input",
            format!(
                "Bir seferde en çok {MEASURE_MAX} nesne ölçülür ({} verildi); listeyi bölün.",
                uids.len()
            ),
        ));
    }
    let way = Way::of(doc.settings(), method)?;
    let items = uids
        .iter()
        .map(|uid| item(doc, uid, &way))
        .collect::<Result<Vec<_>, _>>()?;
    let totals = Totals {
        count: items.len(),
        area: items.iter().filter_map(|i| i.area).sum(),
        length: items.iter().filter_map(|i| i.length).sum(),
    };
    Ok(Measures {
        items,
        totals,
        units: units_by(doc.settings(), method),
        basis: basis(doc.settings(), method),
    })
}

/// How the measures are taken: in the plane, or on the project's system's
/// ellipsoid with the ground's height for `ground`.
enum Way {
    Plane,
    Ellipsoid(System, Option<f64>),
}

impl Way {
    fn of(settings: &ProjectSettings, method: Method) -> Result<Way, HeadlessError> {
        if method == Method::Plane {
            return Ok(Way::Plane);
        }
        let Some(system) = kentos_project::systems::own(settings).and_then(|n| n.system) else {
            return Err(HeadlessError::new(
                "method_unavailable",
                "Projenin koordinat sistemi yok: elipsoit ve zemin ölçüsü verilemez; method plane kullanın.",
            ));
        };
        let height = match method {
            Method::Ground => Some(settings.ground_height().ok_or_else(|| {
                HeadlessError::new(
                    "method_unavailable",
                    "Projenin ortalama elipsoit yüksekliği yazılı değil (Proje ayarları › Ölçme): zemin ölçüsü verilemez.",
                )
            })?),
            _ => None,
        };
        Ok(Way::Ellipsoid(system, height))
    }
}

fn item(doc: &Document, uid: &str, way: &Way) -> Result<MeasureItem, HeadlessError> {
    let slot = slot_of(doc, uid)?;
    let Some(e) = doc.get(slot) else {
        return Err(HeadlessError::new(
            "unknown_object",
            format!("{uid} kimlikli nesne bu çizimde yok."),
        ));
    };
    let s = shape(e);
    let b = entity_bounds(&s);
    let (area, length) = match way {
        // A space's or a slab's rings as one area object: its area net of its holes and its rings' length, what
        // Metraj reads (docs/adr/0776 §2: the API gave none, an agent had to ask cad.spaces.quantities).
        Way::Plane => match &s {
            kentos_geometry_core::entity::Shape::Space { parts, .. } => {
                let p = kentos_geometry_core::geom::space::as_polygon(parts);
                (entity_area(&p), entity_length(&p))
            }
            _ => (entity_area(&s), entity_length(&s)),
        },
        Way::Ellipsoid(system, height) => on_ellipsoid(e, system, *height)?,
    };
    Ok(MeasureItem {
        uid: uid.to_owned(),
        kind: e.kind().to_owned(),
        area,
        length,
        bounds: Bounds {
            min_x: b.min_x,
            min_y: b.min_y,
            max_x: b.max_x,
            max_y: b.max_y,
        },
    })
}

fn pt(v: kentos_commands::Vec2) -> P {
    P::new(v.x, v.y)
}

fn ring(pts: &[kentos_commands::Vec2], bulges: Option<&Vec<f64>>) -> Ring {
    Ring {
        pts: pts.iter().copied().map(pt).collect(),
        bulges: bulges.cloned(),
    }
}

/// The rings an object stands on the ellipsoid as: each part's (its outer
/// ring first, then its holes) and whether it is closed; none for an
/// object no ring stands for (a point, a text …). An ellipse and a spline
/// by their chords within 0.1 mm (docs/adr/0149 §5.3).
fn rings_of(e: &Entity) -> Option<Vec<(Vec<Ring>, bool)>> {
    Some(match e {
        Entity::Line(l) => vec![(vec![ring(&[l.a, l.b], None)], false)],
        Entity::Polyline(p) => {
            let mut parts = vec![(vec![ring(&p.pts, p.bulges.as_ref())], false)];
            for part in p.parts.iter().flatten() {
                parts.push((vec![ring(&part.pts, part.bulges.as_ref())], false));
            }
            parts
        }
        Entity::Polygon(p) => {
            let mut outer = vec![ring(&p.pts, p.bulges.as_ref())];
            outer.extend(
                p.holes
                    .iter()
                    .flatten()
                    .map(|h| ring(&h.pts, h.bulges.as_ref())),
            );
            let mut parts = vec![(outer, true)];
            for part in p.parts.iter().flatten() {
                let mut rings = vec![ring(&part.pts, part.bulges.as_ref())];
                rings.extend(
                    part.holes
                        .iter()
                        .flatten()
                        .map(|h| ring(&h.pts, h.bulges.as_ref())),
                );
                parts.push((rings, true));
            }
            parts
        }
        // A whole circle: two half circles (a bulge of 1 is a half turn).
        Entity::Circle(c) => vec![(
            vec![Ring {
                pts: vec![P::new(c.c.x + c.r, c.c.y), P::new(c.c.x - c.r, c.c.y)],
                bulges: Some(vec![1.0, 1.0]),
            }],
            true,
        )],
        Entity::Arc(a) => {
            let mut sweep = a.a1 - a.a0;
            while sweep <= 0.0 {
                sweep += std::f64::consts::TAU;
            }
            let at = |t: f64| P::new(a.c.x + a.r * t.cos(), a.c.y + a.r * t.sin());
            vec![(
                vec![Ring {
                    pts: vec![at(a.a0), at(a.a1)],
                    bulges: Some(vec![(sweep / 4.0).tan()]),
                }],
                false,
            )]
        }
        Entity::Hatch(h) => {
            let mut rings = vec![ring(&h.ring, None)];
            rings.extend(h.holes.iter().flatten().map(|hole| ring(hole, None)));
            vec![(rings, true)]
        }
        Entity::Ellipse(el) => {
            let g = ellipse_geom(pt(el.c), pt(el.major), el.ratio, el.t0, el.t1);
            let closed = is_full_ellipse(&g);
            vec![(
                vec![Ring {
                    pts: ellipse_outline(&g),
                    bulges: None,
                }],
                closed,
            )]
        }
        // A NURBS curve's chords within 0,1 mm (docs/adr/0259 §5).
        Entity::Nurbs(_) => {
            let s = shape(e);
            let closed = matches!(
                s,
                kentos_geometry_core::entity::Shape::Nurbs { closed: true, .. }
            );
            // A closed curve's ring leaves out the end that returns to its start.
            let mut pts = kentos_geometry_core::geom::nurbs::outline(&s);
            if closed && pts.len() > 3 {
                pts.pop();
            }
            vec![(vec![Ring { pts, bulges: None }], closed)]
        }
        Entity::Spline(sp) => {
            let pts: Vec<P> = sp.pts.iter().copied().map(pt).collect();
            vec![(
                vec![Ring {
                    pts: spline_outline(&pts, sp.closed),
                    bulges: None,
                }],
                sp.closed,
            )]
        }
        _ => return None,
    })
}

/// An object's area and length on the ellipsoid, or on the ground at `height`.
fn on_ellipsoid(
    e: &Entity,
    system: &System,
    height: Option<f64>,
) -> Result<(Option<f64>, Option<f64>), HeadlessError> {
    let Some(parts) = rings_of(e) else {
        return Ok((None, None));
    };
    let (mut area, mut length, mut closed_any) = (0.0, 0.0, false);
    for (rings, closed) in &parts {
        let m = ground_measures(system, rings, *closed, height).map_err(|_| {
            HeadlessError::new(
                "out_of_system",
                "Ölçülen yerin bir noktası projenin sisteminin ulaştığı yerin dışında; değer verilemedi.",
            )
        })?;
        let (a, l) = match height {
            Some(_) => (m.ground_area, m.ground_length.unwrap_or(0.0)),
            None => (m.ellipsoid_area, m.ellipsoid_length),
        };
        if *closed {
            closed_any = true;
            area += a.unwrap_or(0.0);
        }
        length += l;
    }
    Ok((closed_any.then_some(area), Some(length)))
}
