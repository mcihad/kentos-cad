//! KentOS Mimari's building as IFC 4.3 (docs/adr/8047; Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması
//! Hakkında Yönetmelik, RG 5.8.2026/33331, EK-5, EK-6). The application gives the model the sections read ([`Input`]:
//! the walls joined and cut as the plan has them, their openings, the slabs, columns, beams, roofs, railings and stairs
//! as prisms, flights and sloped faces over ±0.00) and the drawing's objects; the spaces are its rooms. Each object
//! becomes the class EK-5 Tablo 5.1 names for it, its name EK-5 §5.5's `Disiplin-KategoriKodu-Açıklama`
//! ([`crate::naming`]), its mandatory properties EK-6's where the drawing knows them.

use std::collections::BTreeMap;

use kentos_contracts::{
    ArchSettings, BeamRole, Entity, OpeningBuild, OpeningForm, OpeningSide, SlabRole, SpaceFields,
    Vec2, WallComponent,
};
use kentos_geometry_core::Vec2 as P;
use kentos_geometry_core::geom::opening::HoleFrame;
use kentos_geometry_core::geom::section::{Flight, Height, Sloped, Solid};

use crate::building::{
    Address, Building, Class, Element, Opening, Place, Pset, Qto, Shape, Space, Storey, Value,
};
use crate::naming::name;

/// An opening through a wall as the section model has it: its outline through the wall (its jambs' face points), its
/// sill and head over ±0.00, its form and its object's persistent id.
#[derive(Clone, Debug, PartialEq)]
pub struct Gap {
    pub ring: Vec<P>,
    pub sill: f64,
    pub head: f64,
    pub form: OpeningForm,
    pub owner: Option<String>,
}

/// What the application gives: the drawing's title, the file's time and the application's name; the section model's
/// storeys (id, name, floor over ±0.00, height), solids with their objects' persistent ids, and openings; the drawing's
/// objects with their ids (the UUID's bits and its text); a wall style's name by its id; a room's net area as the plan
/// measures it; the project's architecture settings (İmar durumu, Vaziyet planı).
pub struct Input<'a> {
    pub title: &'a str,
    pub stamp: &'a str,
    pub application: &'a str,
    pub levels: Vec<(String, String, f64, Option<f64>)>,
    pub solids: &'a [Solid],
    pub owners: &'a [Option<String>],
    pub gaps: Vec<Gap>,
    pub entities: Vec<(u128, String, &'a Entity)>,
    pub wall_style_name: &'a dyn Fn(Option<&str>) -> String,
    pub net_area: &'a mut dyn FnMut(&SpaceFields) -> Option<f64>,
    pub arch: Option<&'a ArchSettings>,
}

/// Why an object of the drawing's building is not in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Why {
    /// The section model gives it no body (a wall without a height, a degenerate object).
    NoBody,
    /// An opening without its outline through a written wall.
    NoWall,
    /// A room without an area.
    NoArea,
    /// Its class is not written yet: a ramp (the section model draws no slope for it).
    NotYet,
}

/// An object of the drawing's building the file does not hold: its persistent id, what it is ("duvar", "kapı" …) and
/// why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Left {
    pub id: String,
    pub what: &'static str,
    pub why: Why,
}

/// What was written, for the command's answer and the acceptance, and what was not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written {
    pub storeys: usize,
    pub walls: usize,
    pub doors: usize,
    pub windows: usize,
    pub openings: usize,
    pub spaces: usize,
    pub slabs: usize,
    pub columns: usize,
    pub beams: usize,
    pub roofs: usize,
    pub railings: usize,
    pub stairs: usize,
    pub left: Vec<Left>,
}

fn pt(p: P) -> [f64; 2] {
    [p.x, p.y]
}

/// Which side of the line `(n, c)` (`n·p ≥ c`) a polygon keeps (Sutherland–Hodgman against one half-plane).
fn clip(ring: &[P], n: P, c: f64) -> Vec<P> {
    let mut out = Vec::new();
    let k = ring.len();
    for i in 0..k {
        let (a, b) = (ring[i], ring[(i + 1) % k]);
        let (da, db) = (n.x * a.x + n.y * a.y - c, n.x * b.x + n.y * b.y - c);
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out.push(P::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    out
}

/// A flight's steps as prisms (docs/adr/8042 §2): each tread's slice of its footprint, from the flight's underside under
/// the slice's far edge to the tread.
fn flight(rings: &[Vec<P>], f: &Flight, floor: f64) -> Vec<Shape> {
    let mut out = Vec::new();
    let Some(ring) = rings.first() else {
        return out;
    };
    let reach = ring.iter().map(|p| f.s(*p)).fold(f64::MIN, f64::max);
    for (i, &(from, top)) in f.steps.iter().enumerate() {
        let to = f.steps.get(i + 1).map_or(reach, |n| n.0);
        if to <= from {
            continue;
        }
        let c0 = f.dir.x * f.start.x + f.dir.y * f.start.y;
        let a = clip(ring, f.dir, c0 + from);
        let b = clip(&a, P::new(-f.dir.x, -f.dir.y), -(c0 + to));
        if b.len() < 3 {
            continue;
        }
        let bottom = f.bottom(to).min(top - 1e-3);
        out.push(Shape::Prism {
            rings: vec![b.into_iter().map(pt).collect()],
            bottom: bottom - floor,
            top: top - floor,
        });
    }
    out
}

/// A roof's face (docs/adr/8044 §5) as a closed set of faces: its footprint's outer ring under its top plane and
/// `thickness` lower, and its sides.
fn sloped(rings: &[Vec<P>], s: &Sloped, floor: f64) -> Vec<Shape> {
    rings
        .iter()
        .filter(|r| r.len() >= 3)
        .map(|r| {
            let n = r.len();
            let mut vertices: Vec<[f64; 3]> =
                r.iter().map(|p| [p.x, p.y, s.z(*p) - floor]).collect();
            vertices.extend(r.iter().map(|p| [p.x, p.y, s.z(*p) - s.thickness - floor]));
            // Counterclockwise from over it: the top as it is, the bottom reversed, the sides outward.
            let ccw = kentos_geometry_core::geometry::signed_area(r) > 0.0;
            let mut top: Vec<usize> = (0..n).collect();
            let mut bottom: Vec<usize> = (n..2 * n).rev().collect();
            if !ccw {
                top.reverse();
                bottom.reverse();
            }
            let mut faces = vec![top, bottom];
            for i in 0..n {
                let j = (i + 1) % n;
                let side = if ccw {
                    vec![i, i + n, j + n, j]
                } else {
                    vec![i, j, j + n, i + n]
                };
                faces.push(side);
            }
            Shape::Faces { vertices, faces }
        })
        .collect()
}

/// A model solid's shapes over its storey's floor.
fn shapes(solid: &Solid, floor: f64) -> Vec<Shape> {
    let rings: Vec<Vec<P>> = solid
        .rings
        .iter()
        .filter(|r| r.len() >= 3)
        .cloned()
        .collect();
    if rings.is_empty() {
        return Vec::new();
    }
    match &solid.height {
        Height::Prism { bottom, top } => vec![Shape::Prism {
            rings: rings
                .iter()
                .map(|r| r.iter().map(|p| pt(*p)).collect())
                .collect(),
            bottom: bottom - floor,
            top: top - floor,
        }],
        Height::Flight(f) => flight(&rings, f, floor),
        Height::Sloped(s) => sloped(&rings, s, floor),
    }
}

fn level_of(e: &Entity) -> Option<&str> {
    match e {
        Entity::Wall(w) => w.wall.arch.level.as_deref(),
        Entity::Slab(s) => s.slab.arch.level.as_deref(),
        Entity::Column(c) => c.column.arch.level.as_deref(),
        Entity::Beam(b) => b.beam.arch.level.as_deref(),
        Entity::Roof(r) => r.roof.arch.level.as_deref(),
        Entity::Railing(r) => r.railing.arch.level.as_deref(),
        Entity::Stair(s) => s.stair.arch.level.as_deref(),
        Entity::Space(s) => s.space.arch.level.as_deref(),
        _ => None,
    }
}

fn label(s: &str) -> Value {
    Value::Label(s.to_owned())
}

/// A layered wall's material for EK-6's single name: its core's, the layer of the strongest join priority (1, a core;
/// absent 3; 5, a finish), of the ones as strong the one taking what the others leave of the width.
fn core_material(components: &[WallComponent]) -> Option<String> {
    components
        .iter()
        .min_by_key(|c| (c.priority.unwrap_or(3), c.thickness.is_some()))
        .and_then(|c| c.material.clone())
}

/// A door's or a window's body across its wall: the band its frame stands in when its construction gives the frame's
/// depth (`HoleFrame`, the rule the plan draws the frame by: centred, or `frameOffset` from the face it opens to), else
/// the opening through the whole wall, as the plan draws one without. `through` is the opening's outline through the
/// wall; the band is cut from its jambs (`place`: the start jamb on the wall's left and right face, then the end's);
/// `right`: it opens to the wall's right face.
fn filler_ring(
    place: &[Vec2],
    build: Option<&OpeningBuild>,
    right: bool,
    through: &[[f64; 2]],
) -> Vec<[f64; 2]> {
    let band = || -> Option<Vec<[f64; 2]>> {
        let [sl, sr, el, er] = place else {
            return None;
        };
        let t = libm::hypot(sr.x - sl.x, sr.y - sl.y);
        let frame = HoleFrame {
            depth: build?.frame_depth?,
            offset: build?.frame_offset,
            right,
        };
        let (near, far) = frame.across(t)?;
        let at =
            |a: &Vec2, c: &Vec2, d: f64| [a.x + (c.x - a.x) * d / t, a.y + (c.y - a.y) * d / t];
        Some(vec![
            at(sl, sr, near),
            at(el, er, near),
            at(el, er, far),
            at(sl, sr, far),
        ])
    };
    band().unwrap_or_else(|| through.to_vec())
}

/// A material's name as EK-6 asks for it ("Malzeme İsmi_Dayanım"): the drawing's, which knows no strength class.
fn material(m: Option<String>) -> Option<String> {
    m.map(|m| m.trim().to_owned()).filter(|m| !m.is_empty())
}

/// The building to write.
pub fn building(input: Input<'_>) -> (Building, Written) {
    let Input {
        title,
        stamp,
        application,
        levels,
        solids,
        owners,
        gaps,
        entities,
        wall_style_name,
        net_area,
        arch,
    } = input;
    let mut written = Written::default();
    // The storeys, by their level id; a drawing without levels has one, Zemin kat at ±0.00.
    let mut levels = levels;
    if levels.is_empty() {
        levels.push((String::new(), "Zemin kat".into(), 0.0, None));
    }
    let storey_of = |level: Option<&str>| -> usize {
        level
            .and_then(|l| levels.iter().position(|(id, ..)| id == l))
            .unwrap_or(0)
    };
    let mut storeys: Vec<Storey> = levels
        .iter()
        .map(|(_, n, floor, height)| Storey {
            id: None,
            name: name("BNK", n),
            long_name: Some(n.clone()),
            elevation: *floor,
            psets: Vec::new(),
            qtos: height
                .map(|h| {
                    vec![Qto {
                        name: "Qto_BuildingStoreyBaseQuantities".into(),
                        quantities: vec![("GrossHeight".into(), Value::Length(h))],
                    }]
                })
                .unwrap_or_default(),
        })
        .collect();
    written.storeys = storeys.len();
    let by_uid: BTreeMap<String, (u128, &Entity)> = entities
        .iter()
        .map(|(bits, u, e)| (u.clone(), (*bits, *e)))
        .collect();
    // The model's solids by their object, in the model's order.
    let mut owned: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, owner) in owners.iter().enumerate() {
        let Some(o) = owner else { continue };
        match owned.iter_mut().find(|(k, _)| k == o) {
            Some((_, list)) => list.push(i),
            None => owned.push((o.clone(), vec![i])),
        }
    }
    let mut elements: Vec<Element> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    for (owner, indices) in &owned {
        let Some(&(id, e)) = by_uid.get(owner) else {
            continue;
        };
        let storey = storey_of(level_of(e));
        let floor = levels[storey].2;
        let mut shapes_of: Vec<Shape> = Vec::new();
        for &i in indices {
            shapes_of.extend(shapes(&solids[i], floor));
        }
        if shapes_of.is_empty() {
            continue;
        }
        let (class, predefined, words): (Class, &'static str, String) = match e {
            // Its style's name (Dış duvar 25), as the plan's legend and the layer key name it.
            Entity::Wall(w) => (
                Class::Wall,
                "STANDARD",
                wall_style_name(w.wall.style.as_deref()),
            ),
            Entity::Slab(_) => (Class::Slab, "FLOOR", "Döşeme".into()),
            Entity::Column(c) => (
                Class::Column,
                "COLUMN",
                c.column.kind.clone().unwrap_or_else(|| "Kolon".into()),
            ),
            Entity::Beam(b) => match b.beam.role {
                BeamRole::Lintel => (Class::Beam, "LINTEL", "Lento".into()),
                BeamRole::BondBeam => (Class::Beam, "BEAM", "Hatıl".into()),
                _ => (Class::Beam, "BEAM", "Kiriş".into()),
            },
            Entity::Roof(_) => (Class::Roof, "NOTDEFINED", "Çatı".into()),
            Entity::Railing(_) => (Class::Railing, "BALUSTRADE", "Korkuluk".into()),
            Entity::Stair(_) => (Class::Stair, "NOTDEFINED", "Merdiven".into()),
            _ => continue,
        };
        match class {
            Class::Wall => written.walls += 1,
            Class::Slab => written.slabs += 1,
            Class::Column => written.columns += 1,
            Class::Beam => written.beams += 1,
            Class::Roof => written.roofs += 1,
            Class::Railing => written.railings += 1,
            Class::Stair => written.stairs += 1,
            _ => {}
        }
        // EK-6's properties the drawing knows: a balcony's or a terrace's slab is outside (Tablo 6.56), a roof is
        // (Tablo 6.52), a column carries (Tablo 6.16: KentOS's column is the frame's, Betonarme unless said).
        let psets = match e {
            Entity::Slab(s) => vec![Pset {
                name: "Pset_SlabCommon".into(),
                props: vec![(
                    "IsExternal".into(),
                    Value::Bool(s.slab.role != SlabRole::Floor),
                )],
            }],
            Entity::Column(_) => vec![Pset {
                name: "Pset_ColumnCommon".into(),
                props: vec![("LoadBearing".into(), Value::Bool(true))],
            }],
            Entity::Roof(_) => vec![Pset {
                name: "Pset_RoofCommon".into(),
                props: vec![("IsExternal".into(), Value::Bool(true))],
            }],
            _ => Vec::new(),
        };
        // Its material (EK-6, `IfcRelAssociatesMaterial`): a column's and a beam's, Betonarme unless said; a slab's;
        // a wall's core's.
        let made_of = material(match e {
            Entity::Wall(w) => core_material(&w.wall.components),
            Entity::Slab(s) => s.slab.material.clone(),
            Entity::Column(c) => Some(
                c.column
                    .material
                    .clone()
                    .unwrap_or_else(|| "Betonarme".into()),
            ),
            Entity::Beam(b) => Some(
                b.beam
                    .material
                    .clone()
                    .unwrap_or_else(|| "Betonarme".into()),
            ),
            _ => None,
        });
        index.insert(owner.clone(), elements.len());
        elements.push(Element {
            class,
            id: Some(id),
            name: name(class.code(), &words),
            description: None,
            tag: Some(owner.clone()),
            predefined,
            storey,
            shapes: shapes_of,
            overall: None,
            material: made_of,
            psets,
            qtos: Vec::new(),
        });
    }
    // Pset_WallCommon's IsExternal (EK-6 Tablo 6.66): a wall with a corner on the outline of its storey's walls and
    // columns together. The columns close it: the section model stops the walls at them, and the walls of a framed
    // storey alone are pieces with an outline each, an inner wall's end on it (the section building: all 8 inner
    // walls outside).
    let mut exterior: Vec<bool> = vec![false; elements.len()];
    for s in 0..storeys.len() {
        let mut rings: Vec<Vec<P>> = Vec::new();
        let mut whose: Vec<usize> = Vec::new();
        for (k, el) in elements
            .iter()
            .enumerate()
            .filter(|(_, el)| matches!(el.class, Class::Wall | Class::Column) && el.storey == s)
        {
            for sh in &el.shapes {
                if let Shape::Prism { rings: rs, .. } = sh {
                    for r in rs {
                        rings.push(r.iter().map(|p| P::new(p[0], p[1])).collect());
                        whose.push(k);
                    }
                }
            }
        }
        for i in kentos_geometry_core::geom::wall::exterior(&rings) {
            exterior[whose[i]] = elements[whose[i]].class == Class::Wall;
        }
    }
    for (k, el) in elements
        .iter_mut()
        .enumerate()
        .filter(|(_, el)| el.class == Class::Wall)
    {
        el.psets.push(Pset {
            name: "Pset_WallCommon".into(),
            props: vec![("IsExternal".into(), Value::Bool(exterior[k]))],
        });
    }
    // The openings, each with its door or window (EK-6 Tablo 6.25, 6.67).
    let mut openings: Vec<Opening> = Vec::new();
    let mut opened: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for gap in &gaps {
        let Some(owner) = gap.owner.as_ref() else {
            continue;
        };
        // One opening element per opening: a second outline of it would give its door a second GlobalId.
        if opened.contains(owner.as_str()) {
            continue;
        }
        let Some(&(id, Entity::Opening(o))) = by_uid.get(owner) else {
            continue;
        };
        let Some(&host) = index.get(&o.opening.host) else {
            continue;
        };
        let storey = elements[host].storey;
        let floor = levels[storey].2;
        let ring: Vec<[f64; 2]> = gap.ring.iter().map(|p| pt(*p)).collect();
        let (bottom, top) = (gap.sill - floor, gap.head - floor);
        let mark = o.opening.number.clone();
        let filler = match gap.form {
            OpeningForm::Door | OpeningForm::Window => {
                let door = gap.form == OpeningForm::Door;
                let (class, pset) = if door {
                    written.doors += 1;
                    (Class::Door, "Pset_DoorCommon")
                } else {
                    written.windows += 1;
                    (Class::Window, "Pset_WindowCommon")
                };
                let words = mark
                    .clone()
                    .unwrap_or_else(|| if door { "Kapı" } else { "Pencere" }.into());
                elements.push(Element {
                    class,
                    id: Some(id),
                    name: name(class.code(), &words),
                    description: None,
                    tag: Some(mark.clone().unwrap_or_else(|| owner.clone())),
                    predefined: if door { "DOOR" } else { "WINDOW" },
                    storey,
                    shapes: vec![Shape::Prism {
                        rings: vec![filler_ring(
                            &o.opening.place,
                            o.opening.build.as_ref(),
                            o.opening.side == OpeningSide::Right,
                            &ring,
                        )],
                        bottom,
                        top,
                    }],
                    overall: Some((gap.head - gap.sill, o.opening.width)),
                    material: None,
                    psets: vec![Pset {
                        name: pset.into(),
                        props: vec![(
                            "IsExternal".into(),
                            Value::Bool(exterior.get(host).copied().unwrap_or(false)),
                        )],
                    }],
                    qtos: Vec::new(),
                });
                Some(elements.len() - 1)
            }
            _ => None,
        };
        written.openings += 1;
        opened.insert(owner.as_str());
        openings.push(Opening {
            id: None,
            name: name("ACL", mark.as_deref().unwrap_or("Açıklık")),
            host,
            ring,
            bottom,
            top,
            filler,
            predefined: if gap.form == OpeningForm::Niche {
                "RECESS"
            } else {
                "OPENING"
            },
        });
    }
    // The spaces (EK-6 Tablo 6.58): their numbers and names, their finishes as the room list's.
    let mut spaces = Vec::new();
    let mut net_floor: Vec<Option<f64>> = vec![None; storeys.len()];
    for (bits, uid, e) in &entities {
        let Entity::Space(s) = e else { continue };
        let f = &s.space;
        let storey = storey_of(f.arch.level.as_deref());
        let rings: Vec<Vec<[f64; 2]>> = f
            .parts
            .iter()
            .flat_map(|part| std::iter::once(&part.outer).chain(&part.holes))
            .map(|r| {
                kentos_geometry_core::entity::polygon_ring(
                    &r.pts.iter().map(|q| P::new(q.x, q.y)).collect::<Vec<_>>(),
                    r.bulges.as_deref(),
                )
                .into_iter()
                .map(pt)
                .collect()
            })
            .filter(|r: &Vec<[f64; 2]>| r.len() >= 3)
            .collect();
        if rings.is_empty() {
            written.left.push(Left {
                id: uid.clone(),
                what: "mahal",
                why: Why::NoArea,
            });
            continue;
        }
        let bottom = f.floor_offset.unwrap_or(0.0);
        let top = bottom + f.height.or(levels[storey].3).unwrap_or(2.8);
        let words = [f.number.as_deref(), f.name.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        let mut props = Vec::new();
        for (key, v) in [
            ("FloorCovering", &f.finishes.floor),
            ("WallCovering", &f.finishes.wall),
            ("CeilingCovering", &f.finishes.ceiling),
        ] {
            if let Some(v) = v.as_deref().filter(|v| !v.trim().is_empty()) {
                props.push((key.to_owned(), label(v)));
            }
        }
        let psets = if props.is_empty() {
            Vec::new()
        } else {
            vec![Pset {
                name: "Pset_SpaceCoveringRequirements".into(),
                props,
            }]
        };
        let net = net_area(f);
        if let Some(a) = net {
            *net_floor[storey].get_or_insert(0.0) += a;
        }
        let qtos = net
            .map(|a| {
                vec![Qto {
                    name: "Qto_SpaceBaseQuantities".into(),
                    quantities: vec![("NetFloorArea".into(), Value::Area(a))],
                }]
            })
            .unwrap_or_default();
        spaces.push(Space {
            id: Some(*bits),
            name: name("MHL", if words.is_empty() { "Mahal" } else { &words }),
            long_name: f.name.clone(),
            storey,
            rings,
            bottom,
            top,
            predefined: "INTERNAL",
            psets,
            qtos,
        });
    }
    written.spaces = spaces.len();
    for (_, uid, e) in &entities {
        let what = match e {
            Entity::Wall(_) => "duvar",
            Entity::Slab(_) => "döşeme",
            Entity::Column(_) => "kolon",
            Entity::Beam(_) => "kiriş",
            Entity::Roof(_) => "çatı",
            Entity::Railing(_) => "korkuluk",
            Entity::Stair(_) => "merdiven",
            Entity::Ramp(_) => "rampa",
            Entity::Opening(o) => match o.opening.form {
                OpeningForm::Door => "kapı",
                OpeningForm::Window => "pencere",
                _ => "açıklık",
            },
            _ => continue,
        };
        let (held, why) = match e {
            Entity::Opening(_) => (opened.contains(uid.as_str()), Why::NoWall),
            Entity::Ramp(_) => (false, Why::NotYet),
            _ => (index.contains_key(uid), Why::NoBody),
        };
        if !held {
            written.left.push(Left {
                id: uid.clone(),
                what,
                why,
            });
        }
    }
    // A storey's and the building's net floor area (EK-6 Tablo 6.8, 6.7): its rooms' net areas summed; the building's
    // height its storeys', only where it is the same for all (Tablo 6.7: "sadece yüksekliğin sabit olduğu durumda").
    for (storey, net) in storeys.iter_mut().zip(&net_floor) {
        if let Some(a) = net {
            match storey
                .qtos
                .iter_mut()
                .find(|q| q.name == "Qto_BuildingStoreyBaseQuantities")
            {
                Some(q) => q.quantities.push(("NetFloorArea".into(), Value::Area(*a))),
                None => storey.qtos.push(Qto {
                    name: "Qto_BuildingStoreyBaseQuantities".into(),
                    quantities: vec![("NetFloorArea".into(), Value::Area(*a))],
                }),
            }
        }
    }
    let mut building_quantities = Vec::new();
    let heights: Vec<Option<f64>> = levels.iter().map(|l| l.3).collect();
    if let Some(Some(h)) = heights.first()
        && heights
            .iter()
            .all(|x| x.is_some_and(|x| (x - h).abs() < 1e-9))
    {
        building_quantities.push(("Height".to_owned(), Value::Length(*h)));
    }
    if net_floor.iter().any(Option::is_some) {
        let total: f64 = net_floor.iter().flatten().sum();
        building_quantities.push(("NetFloorArea".to_owned(), Value::Area(total)));
    }
    // The project, its site and its building (EK-6 Tablo 6.7, 6.55; EK-7).
    let mut site_psets = Vec::new();
    let mut site_qtos = Vec::new();
    if let Some(z) = arch.and_then(|a| a.zoning.as_ref()) {
        let mut props = Vec::new();
        if let Some(far) = z.far {
            props.push(("FloorAreaRatio".to_owned(), Value::Ratio(far)));
        }
        if let Some(c) = z.coverage {
            props.push(("SiteCoverageRatio".to_owned(), Value::Ratio(c)));
        }
        if !props.is_empty() {
            site_psets.push(Pset {
                name: "Pset_SiteCommon".into(),
                props,
            });
        }
        if z.parcel_area > 0.0 {
            site_qtos.push(Qto {
                name: "Qto_SiteBaseQuantities".into(),
                quantities: vec![("GrossArea".into(), Value::Area(z.parcel_area))],
            });
        }
    }
    let title = title.to_owned();
    let building = Building {
        file: title.clone(),
        stamp: stamp.to_owned(),
        application: application.to_owned(),
        phase: None,
        project: Place {
            name: title.clone(),
            long_name: Some(title.clone()),
            ..Place::default()
        },
        site: Place {
            name: name("SAH", "Parsel"),
            psets: site_psets,
            qtos: site_qtos,
            ..Place::default()
        },
        site_elevation: arch.and_then(|a| a.site.as_ref()).and_then(|s| s.datum),
        building: Place {
            name: name("BNA", &title),
            long_name: Some(title),
            psets: vec![Pset {
                name: "Pset_BuildingCommon".into(),
                props: vec![("NumberOfStoreys".into(), Value::Count(storeys.len() as i64))],
            }],
            qtos: if building_quantities.is_empty() {
                Vec::new()
            } else {
                vec![Qto {
                    name: "Qto_BuildingBaseQuantities".into(),
                    quantities: building_quantities,
                }]
            },
            address: None::<Address>,
            ..Place::default()
        },
        storeys,
        elements,
        openings,
        spaces,
        crs: None,
    };
    (building, written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    /// A 1.2 m opening in a 0.25 m wall along x, its left face at y 0.125: the start jamb at x 0, the end at 1.2.
    fn place() -> Vec<Vec2> {
        vec![v(0.0, 0.125), v(0.0, -0.125), v(1.2, 0.125), v(1.2, -0.125)]
    }

    fn close(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(p, q)| (p[0] - q[0]).abs() < 1e-12 && (p[1] - q[1]).abs() < 1e-12)
    }

    #[test]
    fn a_door_s_or_a_window_s_body_is_its_frame_s_band_across_the_wall() {
        let through = vec![[0.0, 0.125], [1.2, 0.125], [1.2, -0.125], [0.0, -0.125]];
        // No construction, or one without the frame's depth: the opening through the whole wall.
        assert_eq!(filler_ring(&place(), None, false, &through), through);
        let plain = OpeningBuild::default();
        assert_eq!(
            filler_ring(&place(), Some(&plain), false, &through),
            through
        );
        // A 7 cm frame centred: 0.09 to 0.16 from the left face.
        let framed = OpeningBuild {
            frame_depth: Some(0.07),
            ..OpeningBuild::default()
        };
        let centred = filler_ring(&place(), Some(&framed), false, &through);
        assert!(close(
            &centred,
            &[[0.0, 0.035], [1.2, 0.035], [1.2, -0.035], [0.0, -0.035]]
        ));
        // 2 cm from the face it opens to: the left's, then the right's.
        let set = OpeningBuild {
            frame_depth: Some(0.07),
            frame_offset: Some(0.02),
            ..OpeningBuild::default()
        };
        assert!(close(
            &filler_ring(&place(), Some(&set), false, &through),
            &[[0.0, 0.105], [1.2, 0.105], [1.2, 0.035], [0.0, 0.035]]
        ));
        assert!(close(
            &filler_ring(&place(), Some(&set), true, &through),
            &[[0.0, -0.035], [1.2, -0.035], [1.2, -0.105], [0.0, -0.105]]
        ));
        // A frame as deep as the wall fills it.
        let deep = OpeningBuild {
            frame_depth: Some(0.3),
            ..OpeningBuild::default()
        };
        assert_eq!(filler_ring(&place(), Some(&deep), false, &through), through);
    }
}
