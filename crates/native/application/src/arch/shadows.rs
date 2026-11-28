//! Gölge (docs/adr/8177 §3.2): the building's shadow on the ground at ±0.00 under the sun of a day and an hour. Each
//! solid of the section model (`sections::model`) above the ground is swept along the sun's rays down to it: a prism
//! between its bottom (the ground at most) and its top, a roof's face by each corner's own height, a stair's flight
//! from its floor to its top tread; their union, less the ground the building stands on, is the shadow. That ground:
//! the solids down to ±0.00, the building's outline there (`site_ground::building_corners`) and every gap between them
//! a solid covers (a room under its floor or roof is the building, not the ground in its shadow).
//!
//! The sun's place (`geom::sun`) at the building's middle: its latitude and longitude from the project's coordinate
//! system, else the ones given; its true north the model's (`north::model_north`: the system's, else the drawing's
//! north arrows), else the plan's north.
//!
//! Drawn: one hour filled with a light hatch, its outline over it; several hours each its outline and its time at the
//! shadow's tip. Each object carries the attribute `Gölge`; drawn again, the old shadows go first. Plain objects: they
//! do not follow the building; they are drawn again after a change.

use kentos_commands::{
    CommandResult, EntitiesCreate, EntityGeometry, HatchPattern, HatchPatternType, NewObject,
    TextAlign, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region::{intersect_area_sets, subtract_areas, union_areas};
use kentos_geometry_core::geom::section::Height;
use kentos_geometry_core::geom::sun::{self, Sun};
use kentos_geometry_core::geometry::point_in_polygon;
use kentos_geometry_core::ops::generate::bounding::convex_hull;
use kentos_geometry_core::predicates::orient2d;
use kentos_geometry_core::vec2::Vec2 as P;

use crate::{ExecutionContext, create};

/// The attribute the shadows carry: what a new drawing takes away.
pub const MARK_ATTR: &str = "Gölge";
/// The undo step's name.
pub const STEP: &str = "Gölge";
/// A shadow's outline's and time's colour, and its hatch's: lighter, a shade over the drawing rather than a block.
pub const COLOR: &str = "#616161";
pub const HATCH_COLOR: &str = "#9E9E9E";

/// One time's shadow: its words (09:00), the sun's direction in the plan (x, y, up) and its areas.
pub type Hour = (String, [f64; 3], Vec<Area>);

/// Where the sun is reckoned from: the place (latitude, longitude, degrees), the time zone (UTC+hours) and true
/// north (radians counter-clockwise from the plan's north), with what was taken for want of better.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub lat: f64,
    pub lon: f64,
    pub zone: f64,
    pub north: f64,
    /// What the place took from the project, in words; and what it could not (the plan's north taken for true north).
    pub notes: Vec<String>,
}

/// The middle of the drawing's walls: where the building stands.
fn middle(doc: &Document) -> Option<P> {
    let pts: Vec<&kentos_commands::Vec2> = doc
        .entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Wall(w) => Some(w.wall.pts.iter()),
            _ => None,
        })
        .flatten()
        .collect();
    let first = pts.first()?;
    let (mut lo, mut hi) = ((first.x, first.y), (first.x, first.y));
    for p in &pts {
        lo = (lo.0.min(p.x), lo.1.min(p.y));
        hi = (hi.0.max(p.x), hi.1.max(p.y));
    }
    Some(P::new((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0))
}

/// The latitude and longitude of the building's middle by the project's coordinate system; none without one.
fn located_at(doc: &Document, at: P) -> Option<(f64, f64)> {
    kentos_project::systems::own(doc.settings())
        .and_then(|n| n.system)
        .and_then(|s| s.geographic_of(at))
}

/// Whether the project's coordinate system gives the building's latitude and longitude: else they are asked for.
pub fn located(doc: &Document) -> bool {
    middle(doc).and_then(|at| located_at(doc, at)).is_some()
}

/// The building's place: the project's coordinate system's latitude and longitude at its middle, else `given`; why
/// none without either.
pub fn place(doc: &Document, given: Option<(f64, f64)>, zone: f64) -> Result<Place, String> {
    let at = middle(doc).ok_or_else(|| "Çizimde duvar yok: binanın yeri bilinmiyor.".to_owned())?;
    let mut notes = Vec::new();
    let (lat, lon) = match (located_at(doc, at), given) {
        (Some(ll), _) => {
            notes.push("konum projenin koordinat sisteminden".to_owned());
            ll
        }
        (None, Some(ll)) => ll,
        (None, None) => {
            return Err(
                "Projenin koordinat sistemi yok: binanın enlemini (E) ve boylamını (B) verin."
                    .into(),
            );
        }
    };
    let north = match crate::north::model_north(doc) {
        Ok(n) => n,
        Err(_) => {
            notes.push("kuzey oku yok, plan kuzeyi coğrafi kuzey sayıldı".to_owned());
            0.0
        }
    };
    Ok(Place {
        lat,
        lon,
        zone,
        north,
        notes,
    })
}

/// The sun at `place` on day `day` (1–366) at clock time `hours`, and towards it in the plan.
pub fn sun_at(place: &Place, day: u32, hours: f64) -> (Sun, [f64; 3]) {
    let s = sun::at(place.lat, place.lon, place.zone, day, hours);
    let in_plan = s.in_plan(place.north);
    (s, in_plan)
}

fn area(pts: Vec<P>) -> Area {
    Area {
        outer: Ring { pts, bulges: None },
        holes: Vec::new(),
    }
}

/// Whether `ring` turns one way only (a straight run is no turn).
fn convex(ring: &[P]) -> bool {
    let n = ring.len();
    let mut way = 0.0;
    for i in 0..n {
        let turn = orient2d(ring[i], ring[(i + 1) % n], ring[(i + 2) % n]);
        if turn != 0.0 {
            if way == 0.0 {
                way = turn.signum();
            } else if turn.signum() != way {
                return false;
            }
        }
    }
    true
}

/// `ring` swept from height `b` to `t` along `v` (the plan offset of a unit of height down the sun's rays). A convex
/// ring's sweep is the convex hull of its two ends, exactly: one piece, not one for each end and edge. The union of a
/// day's thousand pieces took two seconds an hour (a debug build, the section building's 168 solids).
fn sweep(ring: &[P], b: f64, t: f64, v: (f64, f64), out: &mut Vec<Area>) {
    if ring.len() < 3 {
        return;
    }
    let at = |p: &P, z: f64| P::new(p.x + v.0 * z, p.y + v.1 * z);
    if convex(ring) {
        let hull = convex_hull(
            &ring
                .iter()
                .map(|p| at(p, b))
                .chain(ring.iter().map(|p| at(p, t)))
                .collect::<Vec<_>>(),
        );
        if hull.len() >= 3 {
            out.push(area(hull));
        }
        return;
    }
    out.push(area(ring.iter().map(|p| at(p, b)).collect()));
    out.push(area(ring.iter().map(|p| at(p, t)).collect()));
    for i in 0..ring.len() {
        let (p, q) = (&ring[i], &ring[(i + 1) % ring.len()]);
        out.push(area(vec![at(p, b), at(q, b), at(q, t), at(p, t)]));
    }
}

/// What a building's shadow is cast from, made once for a day's hours: the section model's solids and the building's
/// outline at ±0.00.
pub struct Caster {
    model: crate::sections::Model,
    building: Option<Vec<P>>,
}

/// The caster of the drawing's building.
pub fn caster(doc: &Document) -> Caster {
    let building = crate::site_ground::building_corners(doc)
        .map(|c| c.iter().map(|p| P::new(p.x, p.y)).collect());
    Caster {
        model: crate::sections::model(doc),
        building,
    }
}

/// The building's shadow on the ground at ±0.00 with the sun towards `sun` (plan x, y, up): its areas. None with the
/// sun down (under 0.5°: a shadow kilometres long says nothing).
pub fn shadow(doc: &Document, sun: [f64; 3]) -> Vec<Area> {
    shadow_of(&caster(doc), sun)
}

/// A point inside `ring`: its centroid when that is, else the middle of the first stretch inside along the line across
/// its middle height.
fn inside_of(ring: &[P]) -> Option<P> {
    let n = ring.len();
    if n < 3 {
        return None;
    }
    let (mut a2, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let c = p.x * q.y - q.x * p.y;
        (a2, cx, cy) = (a2 + c, cx + (p.x + q.x) * c, cy + (p.y + q.y) * c);
    }
    if a2.abs() > 1e-12 {
        let c = P::new(cx / (3.0 * a2), cy / (3.0 * a2));
        if point_in_polygon(c, ring) {
            return Some(c);
        }
    }
    let (lo, hi) = ring.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.y), hi.max(p.y))
    });
    let y = lo + (hi - lo) * 0.5003;
    let mut xs: Vec<f64> = (0..n)
        .filter_map(|i| {
            let (p, q) = (ring[i], ring[(i + 1) % n]);
            ((p.y > y) != (q.y > y)).then(|| p.x + (y - p.y) / (q.y - p.y) * (q.x - p.x))
        })
        .collect();
    xs.sort_by(f64::total_cmp);
    (xs.len() >= 2).then(|| P::new((xs[0] + xs[1]) / 2.0, y))
}

/// Which of a solid's `rings` are holes, by the even-odd rule its rings follow: one whose corner lies inside an odd
/// number of the others (a corner, not a middle: a courtyard's outer ring has its middle in the courtyard).
fn holes_of(rings: &[Vec<P>]) -> Vec<bool> {
    rings
        .iter()
        .enumerate()
        .map(|(i, r)| {
            r.first().is_some_and(|&p| {
                rings
                    .iter()
                    .enumerate()
                    .filter(|&(j, o)| j != i && o.len() >= 3 && point_in_polygon(p, o))
                    .count()
                    % 2
                    == 1
            })
        })
        .collect()
}

/// `ground` with every hole a solid of `model` covers filled.
fn covered(model: &crate::sections::Model, ground: Vec<Area>) -> Vec<Area> {
    let mut filled: Vec<Area> = ground
        .iter()
        .flat_map(|a| &a.holes)
        .filter(|h| {
            inside_of(&h.pts).is_some_and(|p| {
                model.solids.iter().any(|s| {
                    s.rings
                        .iter()
                        .any(|r| r.len() >= 3 && point_in_polygon(p, r))
                })
            })
        })
        .map(|h| area(h.pts.clone()))
        .collect();
    if filled.is_empty() {
        return ground;
    }
    filled.extend(ground);
    union_areas(&filled)
}

/// [`shadow`] of `caster`: an hour's of a day's diagram made once.
pub fn shadow_of(caster: &Caster, sun: [f64; 3]) -> Vec<Area> {
    let [sx, sy, up] = sun;
    if up <= 0.5f64.to_radians().sin() {
        return Vec::new();
    }
    let v = (-sx / up, -sy / up);
    let model = &caster.model;
    let (mut pieces, mut footprint) = (Vec::new(), Vec::new());
    footprint.extend(
        caster
            .building
            .as_ref()
            .filter(|b| b.len() >= 3)
            .map(|b| area(b.clone())),
    );
    for solid in &model.solids {
        match &solid.height {
            Height::Prism { bottom, top } => {
                if *top <= 0.0 {
                    continue;
                }
                let b = bottom.max(0.0);
                let holes = holes_of(&solid.rings);
                if !holes.contains(&true) {
                    for ring in &solid.rings {
                        sweep(ring, b, *top, v, &mut pieces);
                        if *bottom <= 0.01 && ring.len() >= 3 {
                            footprint.push(area(ring.clone()));
                        }
                    }
                    continue;
                }
                // A solid with holes (a courtyard, a stair's opening): its outer rings' sweep less the light through
                // each hole, the rays its bottom and its top both let by. Swept whole, a slab's hole cast a shadow.
                let (mut own, mut light, mut ground, mut open) =
                    (Vec::new(), Vec::new(), Vec::new(), Vec::new());
                for (ring, hole) in solid.rings.iter().zip(&holes).filter(|(r, _)| r.len() >= 3) {
                    if *hole {
                        let at = |z: f64| {
                            area(
                                ring.iter()
                                    .map(|p| P::new(p.x + v.0 * z, p.y + v.1 * z))
                                    .collect(),
                            )
                        };
                        light.extend(intersect_area_sets(&[vec![at(b)], vec![at(*top)]]));
                        if *bottom <= 0.01 {
                            open.push(area(ring.clone()));
                        }
                    } else {
                        sweep(ring, b, *top, v, &mut own);
                        if *bottom <= 0.01 {
                            ground.push(area(ring.clone()));
                        }
                    }
                }
                pieces.extend(subtract_areas(&union_areas(&own), &union_areas(&light)));
                if !ground.is_empty() {
                    footprint.extend(subtract_areas(&union_areas(&ground), &union_areas(&open)));
                }
            }
            Height::Sloped(s) => {
                for ring in solid.rings.iter().filter(|r| r.len() >= 3) {
                    let at = |p: &P| {
                        let z = s.z(*p).max(0.0);
                        P::new(p.x + v.0 * z, p.y + v.1 * z)
                    };
                    pieces.push(area(ring.iter().map(at).collect()));
                }
            }
            Height::Flight(f) => {
                let top = f.steps.iter().map(|(_, h)| *h).fold(f.floor, f64::max);
                if top > 0.0 {
                    for ring in &solid.rings {
                        sweep(ring, f.floor.max(0.0), top, v, &mut pieces);
                    }
                }
            }
        }
    }
    if pieces.is_empty() {
        return Vec::new();
    }
    let all = union_areas(&pieces);
    if footprint.is_empty() {
        return all;
    }
    subtract_areas(&all, &covered(model, union_areas(&footprint)))
}

/// [`shadow_of`] for each of `suns`, side by side on their own threads: the hours do not wait for each other.
pub fn shadows_of(caster: &Caster, suns: &[[f64; 3]]) -> Vec<Vec<Area>> {
    if suns.len() < 2 {
        return suns.iter().map(|s| shadow_of(caster, *s)).collect();
    }
    std::thread::scope(|scope| {
        let hours: Vec<_> = suns
            .iter()
            .map(|s| scope.spawn(move || shadow_of(caster, *s)))
            .collect();
        hours
            .into_iter()
            .map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

/// A clock time as a label writes it: 09:00.
pub fn clock(hours: f64) -> String {
    let minutes = (hours * 60.0).round() as i64;
    format!(
        "{:02}:{:02}",
        minutes.div_euclid(60).rem_euclid(24),
        minutes.rem_euclid(60)
    )
}

/// Draws `shadows` (each a time's words, the sun's plan direction and its areas) on `layer`, the old shadows taken
/// away first; how many areas. One time: filled with a light hatch under its outline; several: each its outline and its
/// time at its tip, the point farthest down the sun's rays. One undo step when the caller holds a group.
pub fn draw(doc: &mut Document, layer: &str, shadows: &[Hour]) -> Result<usize, String> {
    let old: Vec<Slot> = doc
        .entities()
        .filter(|e| e.base().attrs.contains_key(MARK_ATTR))
        .map(|e| Slot(e.base().id))
        .collect();
    if !old.is_empty() {
        doc.remove(&old);
    }
    let h = 2.5 / 1000.0 * doc.settings().plot_scale;
    let filled = shadows.len() == 1;
    let object = |geometry, what: &str, color: &str| NewObject {
        geometry,
        color: Some(color.to_owned()),
        line_weight: None,
        line_type: None,
        line_pattern: None,
        attrs: Some(std::iter::once((MARK_ATTR.to_owned(), what.to_owned())).collect()),
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    };
    let pts = |r: &Ring| {
        r.pts
            .iter()
            .map(|p| Vec2 { x: p.x, y: p.y })
            .collect::<Vec<_>>()
    };
    let mut objects = Vec::new();
    let mut count = 0;
    for (time, sun, areas) in shadows {
        for a in areas {
            count += 1;
            if filled {
                // Lines 45° apart by 3 mm on paper: what lies under the shadow stays readable.
                let hatch = EntityGeometry::Hatch {
                    ring: pts(&a.outer),
                    holes: (!a.holes.is_empty()).then(|| a.holes.iter().map(pts).collect()),
                    pattern: HatchPattern::user(
                        HatchPatternType::Lines,
                        45.0,
                        0.003 * doc.settings().plot_scale,
                    ),
                    assoc: None,
                    boundary: None,
                    annotative: None,
                };
                objects.push(object(hatch, time, HATCH_COLOR));
            }
            for r in std::iter::once(&a.outer).chain(&a.holes) {
                let outline = EntityGeometry::Polyline {
                    pts: pts(r),
                    bulges: None,
                    zs: None,
                    ms: None,
                    parts: None,
                    closed: true,
                    widths: None,
                };
                objects.push(object(outline, time, COLOR));
            }
        }
        // The time at the tip: the farthest point down the sun's rays, of all its areas.
        let down = (-sun[0], -sun[1]);
        let tip = areas
            .iter()
            .flat_map(|a| a.outer.pts.iter())
            .max_by(|a, b| (a.x * down.0 + a.y * down.1).total_cmp(&(b.x * down.0 + b.y * down.1)));
        if let Some(t) = tip {
            let text = EntityGeometry::Text {
                layout: None,
                annotative: None,
                p: Vec2 { x: t.x, y: t.y },
                text: time.clone(),
                height: h,
                rotation: 0.0,
                align: Some(TextAlign::BottomLeft),
                width_factor: None,
                mask: true,
                box_width: None,
                line_spacing: None,
                runs: Vec::new(),
                face: Default::default(),
                path: None,
            };
            objects.push(object(text, time, COLOR));
        }
    }
    if objects.is_empty() {
        return Ok(0);
    }
    let input = EntitiesCreate {
        layer_id: layer.to_owned(),
        objects,
        operation: None,
        expected_revision: None,
    };
    match create::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { .. } => Ok(count),
        CommandResult::Failed { error } | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Gölge yazılamadı.".into()),
    }
}
