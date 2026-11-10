//! Kesit çıkar (docs/adr/8042; M42, ADV-19): a building section drawn from a section mark, from what the model holds.
//!
//! - **The model's solids** (§3): each wall's layers as they are joined in plan (the walls' join solver, the columns
//!   cutting them; docs/adr/8041 §2) between its foot and its top (docs/adr/8040 §1), its openings taken out between
//!   their sills and heads over their storey's floor, their outlines laid on its faces; each slab's layers from its top
//!   down (finish, screed, insulation, waterproofing, its structure, plaster under it; docs/adr/8040 §2); each column
//!   between the structure under and over it; each beam between its underside and its top (docs/adr/8041 §1); each
//!   straight leg of a stair as a flight (its treads over a slope as thick as the slab it arrives at) and each landing
//!   as a slab of that thickness.
//! - **The drawing** (§4), in the section's plane (`x` along the line as the viewer sees it, `y` up, metres over
//!   ±0.00): what is cut joined material by material into one section (ISO 128-3 7.2: a wall's plaster with its
//!   core, the slabs, beams, columns, stairs and concrete walls as one), outlined heavy and hatched as the plan draws
//!   them at 1:50; a floor's finish layers outlined light; what is seen past it light, on its material's layer with its
//!   depth (EK-4 4.14.3: `-1` nearest … `-4` farthest); the storeys' levels at its left (EK-4 Şekil 4.3), the grids the
//!   line crosses (Şekil 4.5), the rooms' numbers and names where the line passes through them (Mimarlar Odası), the
//!   graded ground at ±0.00, its title and scale under it.
//! - **What the model does not hold** is listed, not drawn (§3; ADR 0347 §3): the roof, the natural ground, a curved
//!   stair or wall, a ramp, a block's height.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{BeamRole, Entity, OpeningForm, SlabLayerKind, Vec2};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::section::{self as sec, Flight, Frame, Height, Solid, Void};
use kentos_geometry_core::vec2::Vec2 as P;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The scale a section is drawn for (docs/adr/8042 §4, §6): the drawing's own (Çizim ölçeği), as its plans' words
/// are; Mimarlar Odası's uygulama projesi 1/50 when the drawing has none. A section printed at another scale prints
/// its words smaller or larger, so the set prints it at this one (as its calculation tables, docs/adr/8037).
pub fn scale_of(doc: &Document) -> f64 {
    let s = doc.settings().plot_scale.round();
    if s.is_finite() && s >= 1.0 { s } else { 50.0 }
}

/// Words 2.5 mm high on paper at `scale`; the title 5 mm.
fn text_at(scale: f64) -> f64 {
    2.5 / 1000.0 * scale
}
/// Pens, paper millimetres: the 1:50 line group (docs/adr/8013: 0.5 and 0.25), a fainter one for what is far and the
/// hatches.
const HEAVY: f64 = 0.5;
const LIGHT: f64 = 0.25;
const FAINT: f64 = 0.18;
const HATCH: f64 = 0.13;
/// Reinforced concrete cut at 1:50: the plan's Betonarme perde (docs/adr/8014): solid grey.
const CONCRETE: &str = "betonarme";
const CONCRETE_GREY: &str = "#A6A6A6";

fn p(v: Vec2) -> P {
    P::new(v.x, v.y)
}

/// A hatch as a wall's layer or a material gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct Fill {
    pub pattern: String,
    pub scale: f64,
    pub angle: f64,
    pub color: Option<String>,
}

/// How a solid is drawn when cut and when seen.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    /// What its cut joins with into one section: one material's parts (ISO 128-3 7.2); a wall's plaster with its core.
    pub key: String,
    /// The EK-4 layer its cut and its seen lines go on (4.14.3): `MM-<element>-<material>`.
    pub layer: String,
    /// Its cut's outline heavy (the structure, the masonry) or light (a floor's finish layers).
    pub heavy: bool,
    pub fill: Option<Fill>,
}

/// Why something is not drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Left {
    /// The roof: the model has none (ADV-20).
    Roof,
    /// The natural ground: the model has no survey of it (EK-4 4.30).
    NaturalGround,
    /// A curved wall or stair: drawn as its chords, not as a curve.
    Curved,
    /// A ramp: its slope is not drawn yet.
    Ramp,
    /// A wall with no height.
    NoHeight,
    /// An opening without a head: its gap reaches its wall's top.
    HeadUnknown,
    /// A stair with no slab to take its waist from: drawn solid to its floor.
    StairWaist,
    /// A niche or a flue: inside its wall.
    Recess,
}

impl Left {
    /// What the drawing's note says of it.
    pub fn words(self) -> &'static str {
        match self {
            Left::Roof => "çatı (modelde yok)",
            Left::NaturalGround => "tabii zemin (modelde yok)",
            Left::Curved => "eğri duvar ya da merdiven (kirişleriyle)",
            Left::Ramp => "rampa",
            Left::NoHeight => "yüksekliği olmayan duvar",
            Left::HeadUnknown => "lentosu olmayan açıklık (duvarın tepesine kadar)",
            Left::StairWaist => "merdiven plağı (döşemesi yok: dolu)",
            Left::Recess => "niş ve baca",
        }
    }
}

/// An opening through a wall, for what the section draws in its gap.
#[derive(Clone, Debug)]
pub struct Gap {
    /// Its outline through the wall: its start jamb's left and right face points, its end jamb's right and left.
    pub ring: Vec<P>,
    pub sill: f64,
    pub head: f64,
    pub form: OpeningForm,
    /// The middle of its frame across its wall: two points on the wall's faces' line through it (its frame's middle
    /// line in plan), and the frame's depth across the wall when its type says.
    pub frame: (P, P),
    pub depth: Option<f64>,
}

/// The model as the section reads it.
#[derive(Clone, Debug, Default)]
pub struct Model {
    pub solids: Vec<Solid>,
    pub looks: Vec<Look>,
    /// The persistent id of each solid's object.
    pub owners: Vec<Option<String>>,
    pub gaps: Vec<Gap>,
    pub left: BTreeSet<(Left, Option<String>)>,
    /// The storeys: id, name, floor, height.
    pub levels: Vec<(String, String, f64, Option<f64>)>,
}

fn fold(s: &str) -> String {
    kentos_geometry_core::geom::column_join::material_key(s)
}

fn concrete_look() -> Look {
    Look {
        key: CONCRETE.into(),
        layer: "MM-TASI-BETONARME".into(),
        heavy: true,
        fill: Some(Fill {
            pattern: "SOLID".into(),
            scale: 1.0,
            angle: 0.0,
            color: Some(CONCRETE_GREY.into()),
        }),
    }
}

/// A material's look: concrete for Betonarme (or none said), else the wall's element with the material.
fn material_look(element: &str, material: Option<&str>, fill: Option<Fill>) -> Look {
    let m = material
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or("Betonarme");
    if fold(m) == CONCRETE {
        return concrete_look();
    }
    Look {
        key: format!("{element}:{}", fold(m)),
        layer: kentos_commands::ek4_layer(element, m),
        heavy: true,
        fill,
    }
}

/// A slab layer's look: its element (EK-2 Tablo 2.3) and material; insulation hatched by TS 5319 (SN55).
fn slab_layer_look(kind: SlabLayerKind, material: Option<&str>) -> Look {
    let (element, fill) = match kind {
        SlabLayerKind::Finish => ("ZMNK", None),
        SlabLayerKind::Screed => ("SAPP", None),
        SlabLayerKind::Insulation => (
            "ISYA",
            Some(Fill {
                pattern: "SN55".into(),
                scale: 1.0,
                angle: 0.0,
                color: None,
            }),
        ),
        SlabLayerKind::Waterproofing => ("SUYA", None),
        SlabLayerKind::Plaster => ("SIVA", None),
    };
    let detail = material
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or("Genel");
    Look {
        key: format!("{element}:{}", fold(detail)),
        layer: kentos_commands::ek4_layer(element, detail),
        // Plaster under a slab is the ceiling's skin: it joins nothing heavy and draws as the walls' plaster does.
        heavy: false,
        fill,
    }
}

/// The storeys as the project gives them: id, name, floor (its elevation), height.
fn levels(doc: &Document) -> Vec<(String, String, f64, Option<f64>)> {
    doc.settings()
        .arch
        .as_ref()
        .map(|a| {
            a.levels
                .iter()
                .map(|l| (l.id.clone(), l.name.clone(), l.elevation, l.height))
                .collect()
        })
        .unwrap_or_default()
}

fn floor_of(
    levels: &[(String, String, f64, Option<f64>)],
    level: Option<&str>,
) -> (f64, Option<f64>) {
    level
        .and_then(|id| levels.iter().find(|l| l.0 == id))
        .map_or((0.0, None), |l| (l.2, l.3))
}

/// A slab as the section measures it: its rings, its top, its layers' thicknesses over and under its structure, its
/// structure's thickness.
struct SlabAt {
    rings: Vec<Vec<P>>,
    top: f64,
    over: f64,
    thickness: f64,
}

impl SlabAt {
    fn holds(&self, at: P) -> bool {
        let mut odd = false;
        for r in &self.rings {
            let n = r.len();
            for i in 0..n {
                let (a, b) = (r[i], r[(i + 1) % n]);
                if (a.y > at.y) != (b.y > at.y)
                    && at.x < a.x + (at.y - a.y) * (b.x - a.x) / (b.y - a.y)
                {
                    odd = !odd;
                }
            }
        }
        odd
    }

    fn structure_top(&self) -> f64 {
        self.top - self.over
    }
}

/// The model's solids for a section (docs/adr/8042 §3).
pub fn model(doc: &Document) -> Model {
    let mut m = Model {
        levels: levels(doc),
        ..Model::default()
    };
    m.left.insert((Left::Roof, None));
    m.left.insert((Left::NaturalGround, None));
    let entities: Vec<(Uuid, &Entity)> = doc.entities_with_uids().collect();
    let slabs: Vec<SlabAt> = entities
        .iter()
        .filter_map(|(_, e)| match e {
            Entity::Slab(s) => Some(&s.slab),
            _ => None,
        })
        .map(|s| {
            let (floor, _) = floor_of(&m.levels, s.arch.level.as_deref());
            SlabAt {
                rings: s
                    .parts
                    .iter()
                    .flat_map(|part| std::iter::once(&part.outer).chain(&part.holes))
                    .map(|r| r.pts.iter().map(|q| p(*q)).collect())
                    .collect(),
                top: floor + s.offset,
                over: s.layers.over_thickness(),
                thickness: s.thickness,
            }
        })
        .collect();
    walls(doc, &entities, &mut m);
    for (uid, e) in &entities {
        let owner = Some(uid.to_string());
        match e {
            Entity::Slab(s) => slab(&s.slab, &m.levels.clone(), owner, &mut m),
            Entity::Column(c) => column(&c.column, &slabs, owner, &mut m),
            Entity::Beam(b) => beam(doc, &b.beam, owner, &mut m),
            Entity::Stair(s) => stair(&s.stair, &slabs, owner, &mut m),
            Entity::Ramp(_) => {
                m.left.insert((Left::Ramp, owner));
            }
            _ => {}
        }
    }
    m
}

fn push(m: &mut Model, solid: Solid, look: Look, owner: Option<String>) {
    m.solids.push(solid);
    m.looks.push(look);
    m.owners.push(owner);
}

/// The walls, joined as the plan joins them and cut by the columns on their storeys, with their openings.
fn walls(doc: &Document, entities: &[(Uuid, &Entity)], m: &mut Model) {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::column_join::{Foot, cut};
    use kentos_geometry_core::geom::wall_join::{WallIn, level, solve_with};
    let walls: Vec<(Uuid, &kentos_commands::WallFields, Shape)> = entities
        .iter()
        .filter_map(|(u, e)| match e {
            Entity::Wall(w) => Some((*u, &w.wall, kentos_formats::walls::core_wall(&w.wall))),
            _ => None,
        })
        .collect();
    let ins: Vec<WallIn<'_>> = walls
        .iter()
        .filter_map(|(_, _, s)| match s {
            Shape::Wall { pts, bulges, wall } => Some(WallIn {
                pts,
                bulges: bulges.as_deref(),
                spec: wall,
            }),
            _ => None,
        })
        .collect();
    if ins.len() != walls.len() {
        return;
    }
    let solved = solve_with(&ins, &[], None);
    let feet: Vec<Foot> = entities
        .iter()
        .filter_map(|(_, e)| match e {
            Entity::Column(c) => match kentos_formats::arch_elements::core_column(&c.column) {
                Shape::Column { at, column } => Foot::of(at, &column),
                _ => None,
            },
            _ => None,
        })
        .collect();
    // Each wall's openings, by its persistent id.
    let mut openings: BTreeMap<String, Vec<&kentos_commands::OpeningFields>> = BTreeMap::new();
    for (_, e) in entities {
        if let Entity::Opening(o) = e {
            openings
                .entry(o.opening.host.clone())
                .or_default()
                .push(&o.opening);
        }
    }
    for (((uid, w, shape), win), drawn) in walls.iter().zip(&ins).zip(&solved.drawn) {
        let Some(drawn) = drawn else { continue };
        let owner = Some(uid.to_string());
        if w.bulges
            .as_ref()
            .is_some_and(|b| b.iter().any(|x| *x != 0.0))
        {
            m.left.insert((Left::Curved, owner.clone()));
        }
        let lv = level(win.spec).map(str::to_owned);
        let mine: Vec<&Foot> = feet.iter().filter(|f| f.level == lv).collect();
        let d = cut(drawn, &mine).unwrap_or_else(|| drawn.clone());
        let Some(height) = doc.wall_height(w) else {
            m.left.insert((Left::NoHeight, owner));
            continue;
        };
        let foot = doc.wall_foot(w);
        let top = foot + height;
        let (floor, _) = floor_of(&m.levels, w.arch.level.as_deref());
        let Shape::Wall { wall: spec, .. } = shape else {
            continue;
        };
        // Its core: the layer of the smallest rank (docs/adr/8008), whose material names the wall.
        let core = spec
            .components
            .iter()
            .min_by_key(|c| c.rank())
            .and_then(|c| c.material.clone().or_else(|| c.name.clone()));
        let base = material_look("DUVA", core.as_deref(), None);
        let mut voids = Vec::new();
        let mut marks = Vec::new();
        for o in openings.get(&uid.to_string()).into_iter().flatten() {
            let Ok([l0, r0, l1, r1]) = <[Vec2; 4]>::try_from(o.place.clone()) else {
                continue;
            };
            if matches!(o.form, OpeningForm::Niche | OpeningForm::Flue) {
                m.left.insert((Left::Recess, owner.clone()));
                continue;
            }
            let sill = floor + o.sill.unwrap_or(0.0);
            let head = match o.head {
                Some(h) => floor + h,
                None => {
                    m.left.insert((Left::HeadUnknown, owner.clone()));
                    top
                }
            };
            if head <= sill {
                continue;
            }
            let ring = vec![p(l0), p(l1), p(r1), p(r0)];
            voids.push(Void {
                ring: ring.clone(),
                bottom: sill,
                top: head,
            });
            // Its outline on both faces of the wall; its frame inset by its width when its type gives one.
            for (a, b) in [(p(l0), p(l1)), (p(r0), p(r1))] {
                outline(&mut marks, a, b, sill, head);
                if let Some(f) = o.frame.filter(|f| *f > 0.0) {
                    let len = kentos_geometry_core::jsmath::js_hypot(b.x - a.x, b.y - a.y);
                    if len > 2.0 * f && head - sill > 2.0 * f {
                        let t = f / len;
                        let a2 = P::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
                        let b2 = P::new(b.x - (b.x - a.x) * t, b.y - (b.y - a.y) * t);
                        let low = if o.form == OpeningForm::Door {
                            sill
                        } else {
                            sill + f
                        };
                        outline(&mut marks, a2, b2, low, head - f);
                    }
                }
            }
            let mid = |x: Vec2, y: Vec2| P::new((x.x + y.x) / 2.0, (x.y + y.y) / 2.0);
            m.gaps.push(Gap {
                ring,
                sill,
                head,
                form: o.form,
                frame: (mid(l0, r0), mid(l1, r1)),
                depth: o.build.as_ref().and_then(|b| b.frame_depth),
            });
        }
        let shown = |c: &kentos_geometry_core::geom::wall::Component| {
            c.detail <= kentos_geometry_core::geom::wall::Detail::Standard
        };
        let comps = &spec.components;
        let bands: Vec<(
            Option<&kentos_geometry_core::geom::wall::Component>,
            &Vec<kentos_geometry_core::geom::wall::Body>,
        )> = if d.bands.len() == comps.len() && !comps.is_empty() {
            comps.iter().map(Some).zip(&d.bands).collect()
        } else {
            vec![(None, &d.fill)]
        };
        let mut first = true;
        for (comp, bodies) in bands {
            let fill = comp
                .filter(|c| shown(c))
                .and_then(|c| c.hatch.as_ref())
                .map(|h| Fill {
                    pattern: h.pattern.clone(),
                    scale: h.scale,
                    angle: h.angle,
                    color: h.color.clone(),
                });
            let look = Look {
                fill,
                ..base.clone()
            };
            for (ring, holes) in bodies {
                let mut rings = vec![ring.clone()];
                rings.extend(holes.iter().flatten().cloned());
                let solid = Solid {
                    rings,
                    height: Height::Prism { bottom: foot, top },
                    voids: voids.clone(),
                    // The openings' outlines once, on the wall's first piece.
                    marks: if first {
                        std::mem::take(&mut marks)
                    } else {
                        Vec::new()
                    },
                };
                first = false;
                push(m, solid, look.clone(), owner.clone());
            }
        }
    }
}

/// A rectangle on a wall's face from `a` to `b` between two heights, as marks.
fn outline(marks: &mut Vec<[(P, f64); 2]>, a: P, b: P, low: f64, high: f64) {
    let c = [(a, low), (b, low), (b, high), (a, high)];
    for k in 0..4 {
        marks.push([c[k], c[(k + 1) % 4]]);
    }
}

/// A slab's layers from its top down (docs/adr/8040 §2).
fn slab(
    s: &kentos_commands::SlabFields,
    levels: &[(String, String, f64, Option<f64>)],
    owner: Option<String>,
    m: &mut Model,
) {
    let (floor, _) = floor_of(levels, s.arch.level.as_deref());
    let rings: Vec<Vec<P>> = s
        .parts
        .iter()
        .flat_map(|part| std::iter::once(&part.outer).chain(&part.holes))
        .map(|r| r.pts.iter().map(|q| p(*q)).collect())
        .collect();
    if s.parts.iter().any(|part| {
        std::iter::once(&part.outer).chain(&part.holes).any(|r| {
            r.bulges
                .as_ref()
                .is_some_and(|b| b.iter().any(|x| *x != 0.0))
        })
    }) {
        m.left.insert((Left::Curved, owner.clone()));
    }
    let mut z = floor + s.offset;
    let solid = |bottom: f64, top: f64| Solid {
        rings: rings.clone(),
        height: Height::Prism { bottom, top },
        voids: Vec::new(),
        marks: Vec::new(),
    };
    for l in &s.layers.over {
        push(
            m,
            solid(z - l.thickness, z),
            slab_layer_look(l.kind, l.material.as_deref()),
            owner.clone(),
        );
        z -= l.thickness;
    }
    push(
        m,
        solid(z - s.thickness, z),
        material_look("DOSM", s.material.as_deref(), None),
        owner.clone(),
    );
    z -= s.thickness;
    for l in &s.layers.under {
        push(
            m,
            solid(z - l.thickness, z),
            slab_layer_look(l.kind, l.material.as_deref()),
            owner.clone(),
        );
        z -= l.thickness;
    }
}

/// A column between the structure under it and the structure over its storey.
fn column(
    c: &kentos_commands::ColumnFields,
    slabs: &[SlabAt],
    owner: Option<String>,
    m: &mut Model,
) {
    use kentos_geometry_core::entity::Shape;
    let Shape::Column { at, column } = kentos_formats::arch_elements::core_column(c) else {
        return;
    };
    let Some(foot) = kentos_geometry_core::geom::column_join::Foot::of(at, &column) else {
        return;
    };
    let (floor, height) = floor_of(&m.levels, c.arch.level.as_deref());
    let centre = P::new(
        foot.ring.iter().map(|q| q.x).sum::<f64>() / foot.ring.len() as f64,
        foot.ring.iter().map(|q| q.y).sum::<f64>() / foot.ring.len() as f64,
    );
    let bottom = slabs
        .iter()
        .filter(|s| s.holds(centre) && (s.top - floor).abs() <= 0.5)
        .map(SlabAt::structure_top)
        .fold(floor, kentos_geometry_core::jsmath::js_min);
    let Some(h) = height else {
        m.left.insert((Left::NoHeight, owner));
        return;
    };
    let top = slabs
        .iter()
        .filter(|s| s.holds(centre) && (s.top - (floor + h)).abs() <= 0.5)
        .map(SlabAt::structure_top)
        .fold(floor + h, kentos_geometry_core::jsmath::js_min);
    if top - bottom <= 1e-6 {
        return;
    }
    let solid = Solid {
        rings: vec![foot.ring.clone()],
        height: Height::Prism { bottom, top },
        voids: Vec::new(),
        marks: Vec::new(),
    };
    push(
        m,
        solid,
        material_look("KOLN", c.material.as_deref(), None),
        owner,
    );
}

/// A beam between its underside and its top (docs/adr/8041 §1); a lintel and a bond beam alike.
fn beam(doc: &Document, b: &kentos_commands::BeamFields, owner: Option<String>, m: &mut Model) {
    use kentos_geometry_core::entity::Shape;
    let (Some(top), Some(bottom)) = (doc.beam_top(b), doc.beam_underside(b)) else {
        return;
    };
    let Shape::Beam { pts, beam } = kentos_formats::arch_elements::core_beam(b) else {
        return;
    };
    let ring = kentos_geometry_core::geom::beam::band(&pts, &beam);
    if ring.len() < 3 || top - bottom <= 1e-6 {
        return;
    }
    let element = match b.role {
        BeamRole::Beam => "TASI",
        BeamRole::Lintel => "LENT",
        BeamRole::BondBeam => "HATL",
    };
    let solid = Solid {
        rings: vec![ring],
        height: Height::Prism { bottom, top },
        voids: Vec::new(),
        marks: Vec::new(),
    };
    push(m, solid, material_look(element, None, None), owner);
}

/// A stair: each straight leg a flight, each landing a slab, as thick as the slab it arrives at (docs/adr/8042 §3).
fn stair(s: &kentos_commands::StairFields, slabs: &[SlabAt], owner: Option<String>, m: &mut Model) {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::arch::{Walk, riser_stations, stair_landings};
    let Shape::Stair { pts, stair: spec } = kentos_formats::arch_elements::core_stair(s) else {
        return;
    };
    if spec.bulge != 0.0 || pts.len() < 2 {
        m.left.insert((Left::Curved, owner));
        return;
    }
    let (base, _) = floor_of(&m.levels, s.arch.level.as_deref());
    let rh = s.riser_height;
    let n = s.risers as usize;
    let walk = Walk::of(&pts);
    let (stations, _) = riser_stations(&walk, &spec);
    let landings = stair_landings(&walk, &spec);
    let end = *pts.last().unwrap_or(&pts[0]);
    let start = pts[0];
    // Its waist: the structure of the slab it arrives at, else of the one it starts from.
    let arrive = base + rh * n as f64;
    let waist = slabs
        .iter()
        .find(|sl| sl.holds(end) && (sl.top - arrive).abs() <= 0.5)
        .or_else(|| {
            slabs
                .iter()
                .find(|sl| sl.holds(start) && (sl.top - base).abs() <= 0.5)
        })
        .map(|sl| sl.thickness);
    if waist.is_none() {
        m.left.insert((Left::StairWaist, owner.clone()));
    }
    let look = concrete_look();
    let half = s.width / 2.0;
    let slope = rh / s.tread;
    let upright = waist.map_or(f64::INFINITY, |w| w * (1.0 + slope * slope).sqrt());
    // Each segment of the walking line: its flight's stretch past the landings.
    for k in 0..pts.len() - 1 {
        let (a, b) = (pts[k], pts[k + 1]);
        let (sa, sb) = (walk.at[k], walk.at[k + 1]);
        let len = sb - sa;
        if len <= 1e-9 {
            continue;
        }
        let dir = P::new((b.x - a.x) / len, (b.y - a.y) / len);
        let across = P::new(-dir.y, dir.x);
        // Its flight: from the end of a landing that holds its start (else its start) to the start of one that holds
        // its end (else its end).
        let lo = landings
            .iter()
            .find(|(x, y)| *x <= sa + 1e-9 && *y > sa + 1e-9)
            .map_or(sa, |(_, y)| *y);
        let hi = landings
            .iter()
            .find(|(x, y)| *x < sb - 1e-9 && *y >= sb - 1e-9)
            .map_or(sb, |(x, _)| *x);
        if hi - lo <= 1e-9 {
            continue;
        }
        let at = |s: f64| P::new(a.x + dir.x * (s - sa), a.y + dir.y * (s - sa));
        let (fa, fb) = (at(lo), at(hi));
        let ring = vec![
            P::new(fa.x - across.x * half, fa.y - across.y * half),
            P::new(fb.x - across.x * half, fb.y - across.y * half),
            P::new(fb.x + across.x * half, fb.y + across.y * half),
            P::new(fa.x + across.x * half, fa.y + across.y * half),
        ];
        // Its risers, by their numbers along the whole stair.
        let mine: Vec<(usize, f64)> = stations
            .iter()
            .enumerate()
            .filter(|(_, st)| **st >= lo - 1e-9 && **st < hi - 1e-9)
            .map(|(i, st)| (i, *st))
            .collect();
        let Some(&(first, s0)) = mine.first() else {
            continue;
        };
        let mut steps: Vec<(f64, f64)> = Vec::new();
        if s0 > lo + 1e-9 {
            steps.push((0.0, base + rh * first as f64));
        }
        for (i, st) in &mine {
            steps.push((st - lo, base + rh * (*i + 1) as f64));
        }
        // The steps' roots: the first riser's foot, rising by the riser over the tread.
        let root = base + rh * first as f64 - (s0 - lo) * slope;
        let under_landing = if lo > sa + 1e-9 || k > 0 {
            base + rh * first as f64 - waist.unwrap_or(0.0)
        } else {
            base
        };
        let flight = Flight {
            start: fa,
            dir,
            steps,
            soffit: root - upright,
            slope,
            floor: under_landing,
        };
        push(
            m,
            Solid {
                rings: vec![ring],
                height: Height::Flight(flight),
                voids: Vec::new(),
                marks: Vec::new(),
            },
            look.clone(),
            owner.clone(),
        );
    }
    // Its landings: the band between their ends, as thick as its waist (or down to its floor without one).
    let band = kentos_geometry_core::geom::arch::stair_band(&pts, &spec);
    for (x, y) in &landings {
        let risers_before = stations.iter().filter(|st| **st < *x + 1e-9).count();
        let top = base + rh * risers_before as f64;
        let bottom = waist.map_or(base, |w| top - w);
        let Some(ring) = landing_ring(&walk, &band, *x, *y) else {
            continue;
        };
        push(
            m,
            Solid {
                rings: vec![ring],
                height: Height::Prism { bottom, top },
                voids: Vec::new(),
                marks: Vec::new(),
            },
            look.clone(),
            owner.clone(),
        );
    }
}

/// A landing's outline: the stair's band between the walking line's points `x` and `y` along it, cut square to the
/// line at both ends.
fn landing_ring(
    walk: &kentos_geometry_core::geom::arch::Walk,
    band: &kentos_geometry_core::entity::Shape,
    x: f64,
    y: f64,
) -> Option<Vec<P>> {
    use kentos_geometry_core::entity::Shape;
    let Shape::Polygon { pts: ring, .. } = band else {
        return None;
    };
    let (px, dx) = walk.point(x);
    let (py, dy) = walk.point(y);
    // Keep the band's part after the cross line at x (along dx) and before the one at y (along dy).
    let after = move |q: P| (q.x - px.x) * dx.x + (q.y - px.y) * dx.y;
    let before = move |q: P| (py.x - q.x) * dy.x + (py.y - q.y) * dy.y;
    let mut out = clip(ring, &after);
    out = clip(&out, &before);
    (out.len() >= 3).then_some(out)
}

/// A ring cut to where `keep` is not below 0.
fn clip(ring: &[P], keep: &dyn Fn(P) -> f64) -> Vec<P> {
    let n = ring.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let (ka, kb) = (keep(a), keep(b));
        if ka >= 0.0 {
            out.push(a);
        }
        if (ka >= 0.0) != (kb >= 0.0) {
            let t = ka / (ka - kb);
            out.push(P::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    out
}

/// The block a section mark's section is drawn into: its id from the mark's (a section is its mark's), the same in
/// every file.
pub fn block_id(mark: Uuid) -> kentos_commands::BlockId {
    let digest = Sha256::digest(format!("kentos.section/{mark}").as_bytes());
    let mut b = [0u8; 16];
    b.copy_from_slice(&digest[..16]);
    // A UUID of version 8 (custom), its variant RFC 9562's.
    b[6] = (b[6] & 0x0f) | 0x80;
    b[8] = (b[8] & 0x3f) | 0x80;
    kentos_commands::BlockId(b)
}

/// How a section is called from its mark's words: "A-A" for a mark "A"; "Kesit" for none.
pub fn label(mark: &str) -> String {
    let m = mark.trim();
    if m.is_empty() {
        "Kesit".into()
    } else {
        format!("{m}-{m}")
    }
}

/// Its title as Türkiye writes one under a section: "A-A KESİTİ".
pub fn title(mark: &str) -> String {
    let m = mark.trim();
    if m.is_empty() {
        "KESİT".into()
    } else {
        format!("{}-{} KESİTİ", upper(m), upper(m))
    }
}

/// Turkish upper case: i → İ, ı → I.
fn upper(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'i' => vec!['İ'],
            'ı' => vec!['I'],
            c => c.to_uppercase().collect(),
        })
        .collect()
}

/// The block's name by the digital project regulation's EK-4 Tablo 4.4: MM-XXXX-KESIT and its mark ("MM-XXXX-KESITA").
pub fn block_name(mark: &str) -> String {
    let word = kentos_commands::ek4_word(mark, 10);
    kentos_commands::ek4_layer(kentos_commands::EK4_GENERAL, &format!("KESIT{word}"))
}

/// A level's words as a section writes them (EK-4 Şekil 4.3): "+3.00", "±0.00", "-2.75".
pub fn level_words(z: f64) -> String {
    let t = kentos_geometry_core::geom::arch::level_text(z);
    if t == "+0.00" { "±0.00".into() } else { t }
}

/// What a section draws: its objects in the block's coordinates (its left end at ±0.00 the base point), the layers
/// they are on (EK-4 names), what it left out, and its box.
#[derive(Clone, Debug)]
pub struct Drawn {
    pub entities: Vec<Entity>,
    pub layers: BTreeSet<String>,
    pub left: Vec<(Left, Option<String>)>,
    pub lo: P,
    pub hi: P,
}

/// Writes one object into the drawing on `layer` (a layer's EK-4 name; the command gives it its id).
struct Out {
    entities: Vec<Value>,
    layers: BTreeSet<String>,
}

impl Out {
    fn put(&mut self, layer: &str, mut v: Value) {
        v["id"] = json!(self.entities.len() + 1);
        v["layerId"] = json!(layer);
        v["attrs"] = json!({});
        self.layers.insert(layer.to_owned());
        self.entities.push(v);
    }

    fn line(&mut self, layer: &str, a: P, b: P, weight: f64) {
        self.put(
            layer,
            json!({"kind": "line", "a": {"x": a.x, "y": a.y}, "b": {"x": b.x, "y": b.y}, "lineWeight": weight}),
        );
    }

    fn ring(&mut self, layer: &str, ring: &[P], weight: f64) {
        let pts: Vec<Value> = ring.iter().map(|q| json!({"x": q.x, "y": q.y})).collect();
        self.put(
            layer,
            json!({"kind": "polyline", "pts": pts, "closed": true, "lineWeight": weight}),
        );
    }

    fn text(&mut self, layer: &str, at: P, words: &str, height: f64, align: &str) {
        if words.is_empty() {
            return;
        }
        self.put(
            layer,
            json!({"kind": "text", "p": {"x": at.x, "y": at.y}, "text": words, "height": height, "rotation": 0.0,
                   "align": align, "lineWeight": FAINT}),
        );
    }
}

/// The section of the mark: the line from its first point to its second, looking as its arrows do.
pub fn frame_of(mark: &kentos_commands::ViewMarkFields) -> Option<Frame> {
    let (a, b) = (mark.points.first()?, mark.points.get(1)?);
    Frame::new(p(*a), p(*b), mark.flip, None)
}

/// The objects a mark's line cuts (docs/adr/8042 §7): the persistent ids of the model's solids its first two points'
/// line passes through within its length. Denetim › Mevzuat reads whether a section passes through the common stair
/// with it, the section's own geometry.
pub fn cut_owners(m: &Model, mark: &kentos_commands::ViewMarkFields) -> BTreeSet<String> {
    let Some(frame) = frame_of(mark) else {
        return BTreeSet::new();
    };
    m.solids
        .iter()
        .zip(&m.owners)
        .filter(|(s, _)| !sec::cut_stretches(&frame, &s.rings).is_empty())
        .filter_map(|(_, o)| o.clone())
        .collect()
}

/// The section drawn (docs/adr/8042 §4): what is cut, what is seen, the levels, grids, rooms, ground and title.
pub fn draw(doc: &Document, frame: &Frame, mark_words: &str) -> Drawn {
    let m = model(doc);
    let s = sec::section(frame, &m.solids);
    let mut out = Out {
        entities: Vec::new(),
        layers: BTreeSet::new(),
    };
    let plot = scale_of(doc);
    let text = text_at(plot);
    let title_height = 2.0 * text;
    // What is cut: hatches by material and fill, outlines by material (ISO 128-3 7.2).
    let mut outlines: BTreeMap<String, (Look, Vec<kentos_geometry_core::geom::arrangement::Area>)> =
        BTreeMap::new();
    let mut fills: BTreeMap<
        (String, String),
        (Look, Vec<kentos_geometry_core::geom::arrangement::Area>),
    > = BTreeMap::new();
    for c in &s.cut {
        let look = &m.looks[c.solid];
        let area = area_of(&c.ring);
        outlines
            .entry(look.key.clone())
            .or_insert_with(|| (look.clone(), Vec::new()))
            .1
            .push(area.clone());
        if let Some(f) = &look.fill {
            let k = format!(
                "{}|{}|{}|{}",
                f.pattern,
                f.scale,
                f.angle,
                f.color.clone().unwrap_or_default()
            );
            fills
                .entry((look.key.clone(), k))
                .or_insert_with(|| (look.clone(), Vec::new()))
                .1
                .push(area);
        }
    }
    for (look, areas) in fills.values() {
        let Some(f) = &look.fill else { continue };
        let Some(pattern) =
            kentos_geometry_core::geom::wall::component_hatch(&f.pattern, f.scale, f.angle, plot)
                .and_then(crate::geometry::contract_pattern)
        else {
            continue;
        };
        for a in kentos_geometry_core::geom::region::union_areas(areas) {
            let ring: Vec<Value> = a
                .outer
                .pts
                .iter()
                .map(|q| json!({"x": q.x, "y": q.y}))
                .collect();
            let holes: Vec<Vec<Value>> = a
                .holes
                .iter()
                .map(|h| h.pts.iter().map(|q| json!({"x": q.x, "y": q.y})).collect())
                .collect();
            let mut v =
                json!({"kind": "hatch", "ring": ring, "pattern": pattern, "lineWeight": HATCH});
            if !holes.is_empty() {
                v["holes"] = json!(holes);
            }
            if let Some(c) = &f.color {
                v["color"] = json!(c);
            }
            out.put(&format!("{}-T", look.layer), v);
        }
    }
    for (look, areas) in outlines.values() {
        for a in kentos_geometry_core::geom::region::union_areas(areas) {
            out.ring(
                &look.layer,
                &a.outer.pts,
                if look.heavy { HEAVY } else { LIGHT },
            );
            for h in &a.holes {
                out.ring(&look.layer, &h.pts, if look.heavy { HEAVY } else { LIGHT });
            }
        }
    }
    // What the openings show in their gaps: the frame (its two faces) or, without its depth, the glass's line.
    for g in &m.gaps {
        let gap = sec::cut_stretches(frame, std::slice::from_ref(&g.ring));
        if gap.is_empty() {
            continue;
        }
        let layer = match g.form {
            OpeningForm::Door => "MM-KAPI-GENEL",
            _ => "MM-PENC-GENEL",
        };
        let (a, b) = g.frame;
        // Where the frame's middle line crosses the section line.
        let Some(u) = crossing(frame, a, b) else {
            continue;
        };
        let offsets: Vec<f64> = match g.depth {
            Some(d) if d > 0.0 => vec![-d / 2.0, d / 2.0],
            _ => vec![0.0],
        };
        // The frame's depth runs across the wall, along the section line where the line crosses the wall square.
        for o in offsets {
            let x = u + o;
            if gap
                .iter()
                .any(|(lo, hi)| x >= *lo - 1e-9 && x <= *hi + 1e-9)
            {
                out.line(layer, P::new(x, g.sill), P::new(x, g.head), LIGHT);
            }
        }
    }
    // What is seen: on its material's layer with its depth, 1 nearest … 4 farthest (EK-4 4.14.3).
    let far = s
        .seen
        .iter()
        .map(|e| e.depth)
        .fold(0.0, kentos_geometry_core::jsmath::js_max);
    let mut seen: BTreeMap<String, Vec<(P, P)>> = BTreeMap::new();
    for e in &s.seen {
        let rank = if far > 0.0 {
            (1.0 + (4.0 * e.depth / far).floor()).clamp(1.0, 4.0) as u8
        } else {
            1
        };
        let layer = format!("{}-{rank}", m.looks[e.solid].layer);
        seen.entry(layer).or_default().push((e.a, e.b));
    }
    for (layer, pieces) in seen {
        let weight = if layer.ends_with("-1") || layer.ends_with("-2") {
            LIGHT
        } else {
            FAINT
        };
        for (a, b) in merged(pieces) {
            out.line(&layer, a, b, weight);
        }
    }
    // The extent the annotations sit around.
    let mut lo = P::new(0.0, f64::INFINITY);
    let mut hi = P::new(frame.length, f64::NEG_INFINITY);
    for c in &s.cut {
        for q in &c.ring {
            lo.y = lo.y.min(q.y);
            hi.y = hi.y.max(q.y);
        }
    }
    for e in &s.seen {
        for q in [e.a, e.b] {
            lo.y = lo.y.min(q.y);
            hi.y = hi.y.max(q.y);
        }
    }
    if !lo.y.is_finite() {
        lo.y = 0.0;
        hi.y = 3.0;
    }
    lo.y = lo.y.min(0.0);
    // The building's ends along the line (what is cut and seen; the line's own ends lie past it): the levels, the
    // heights and the ground keep to them, not to the line's ends 1.5 m off.
    let (mut bx0, mut bx1) = (f64::INFINITY, f64::NEG_INFINITY);
    for x in s
        .cut
        .iter()
        .flat_map(|c| c.ring.iter().map(|q| q.x))
        .chain(s.seen.iter().flat_map(|e| [e.a.x, e.b.x]))
    {
        bx0 = bx0.min(x);
        bx1 = bx1.max(x);
    }
    if bx0 > bx1 {
        (bx0, bx1) = (0.0, frame.length);
    }
    // The graded ground at ±0.00 (EK-4 4.30), past the building on both sides, not across what is cut.
    let ground = (bx0 - 2.0, bx1 + 2.0);
    for (a, b) in uncut(&s.cut, ground, 0.0) {
        out.line("MM-CVRE-TESVIYE", P::new(a, 0.0), P::new(b, 0.0), HEAVY);
    }
    // The storeys' levels at the left (EK-4 Şekil 4.3): the line, the triangle on it, the words over it.
    let mut zs: Vec<f64> = m.levels.iter().map(|l| l.2).collect();
    if let Some(top) = m
        .levels
        .iter()
        .filter_map(|l| l.3.map(|h| l.2 + h))
        .fold(None, |a: Option<f64>, b| Some(a.map_or(b, |a| a.max(b))))
    {
        zs.push(top);
    }
    zs.push(0.0);
    zs.sort_by(f64::total_cmp);
    zs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    let x0 = bx0 - 1.0;
    for z in &zs {
        let tip = P::new(x0, *z);
        let w = text * 0.6;
        out.line(
            "MM-XXXX-OLCU",
            P::new(x0 - 0.9, *z),
            P::new(bx0 - 0.15, *z),
            FAINT,
        );
        out.ring(
            "MM-XXXX-OLCU",
            &[
                tip,
                P::new(tip.x + w, tip.y + w * 1.2),
                P::new(tip.x - w, tip.y + w * 1.2),
            ],
            FAINT,
        );
        out.line(
            "MM-XXXX-OLCU",
            P::new(x0 - 0.8, *z + w * 1.2),
            P::new(x0 + 0.1, *z + w * 1.2),
            FAINT,
        );
        out.text(
            "MM-XXXX-OLCU",
            P::new(x0 - 0.75, *z + w * 1.2 + text * 0.4),
            &level_words(*z),
            text,
            "bottomLeft",
        );
        hi.y = hi.y.max(*z);
    }
    lo.x = lo.x.min(x0 - 1.0);
    // The storeys' heights beside it on the right: a chain between the levels.
    let xd = bx1 + 1.0;
    for w in zs.windows(2) {
        let (a, b) = (w[0], w[1]);
        out.line("MM-XXXX-OLCU", P::new(xd, a), P::new(xd, b), FAINT);
        for z in [a, b] {
            out.line(
                "MM-XXXX-OLCU",
                P::new(xd - 0.08, z - 0.08),
                P::new(xd + 0.08, z + 0.08),
                FAINT,
            );
        }
        let words = format!("{:.2}", b - a).replace('.', ",");
        out.put(
            "MM-XXXX-OLCU",
            json!({"kind": "text", "p": {"x": xd - 0.1, "y": (a + b) / 2.0}, "text": words, "height": text,
                   "rotation": std::f64::consts::FRAC_PI_2, "align": "bottomCenter", "lineWeight": FAINT}),
        );
    }
    hi.x = hi.x.max(xd + 0.5);
    // The grids the line crosses (EK-4 Şekil 4.5): a dash-dot line up the section and its bubble under it, the
    // bubbles in one row (the lines' ends fixed before the first: lowering them per grid stepped the bubbles down).
    let mut grid_seen: BTreeSet<String> = BTreeSet::new();
    let (z0, z1) = (lo.y - 0.6, hi.y + 0.4);
    let mut under = lo.y;
    for e in doc.entities() {
        let Entity::Grid(g) = e else { continue };
        if g.grid.pts.len() != 2
            || g.grid
                .bulges
                .as_ref()
                .is_some_and(|b| b.iter().any(|x| *x != 0.0))
        {
            continue;
        }
        let Some(u) = crossing(frame, p(g.grid.pts[0]), p(g.grid.pts[1])) else {
            continue;
        };
        if !(0.0..=frame.length).contains(&u) || !grid_seen.insert(g.grid.label.clone()) {
            continue;
        }
        let r = g.grid.bubble_size.unwrap_or(8.0) / 1000.0 * plot / 2.0;
        out.put(
            "MM-AKSS-GENEL",
            json!({"kind": "line", "a": {"x": u, "y": z0}, "b": {"x": u, "y": z1}, "lineType": "dashdot",
                   "lineWeight": FAINT}),
        );
        out.put(
            "MM-AKSS-GENEL",
            json!({"kind": "circle", "c": {"x": u, "y": z0 - r}, "r": r, "lineWeight": FAINT}),
        );
        out.text(
            "MM-AKSS-GENEL",
            P::new(u, z0 - r),
            &g.grid.label,
            text,
            "middleCenter",
        );
        under = under.min(z0 - 2.0 * r);
    }
    lo.y = under;
    // The rooms the line passes through: their numbers and names in the room's stretch where they touch no line (TS EN
    // ISO 3098-0: no line runs through words; a grid's line often runs through a room's middle, a stair through a
    // stair hall's): mid height in the middle first, then aside, higher or lower; with no such place in the middle,
    // masked (the text's “fon”).
    let mut busy = Busy::of(&out, &s.cut, text);
    for e in doc.entities() {
        let Entity::Space(sp) = e else { continue };
        let rings: Vec<Vec<P>> = sp
            .space
            .parts
            .iter()
            .flat_map(|part| std::iter::once(&part.outer).chain(&part.holes))
            .map(|r| r.pts.iter().map(|q| p(*q)).collect())
            .collect();
        let Some((a, b)) = sec::cut_stretches(frame, &rings)
            .into_iter()
            .max_by(|x, y| (x.1 - x.0).total_cmp(&(y.1 - y.0)))
        else {
            continue;
        };
        let words = crate::elevations::label_of(sp);
        let wide = words.chars().count() as f64 * text * 0.6;
        if b - a < wide + 0.1 {
            continue;
        }
        let floor = doc.room_floor(&sp.space).0;
        let top = doc
            .room_height(&sp.space)
            .map_or(floor + 2.5, |h| h.ceiling);
        let (at, mask) = busy.room_spot((a, b), (floor, top), wide);
        busy.take(at, wide);
        let mut v = json!({"kind": "text", "p": {"x": at.x, "y": at.y}, "text": words, "height": text,
                           "rotation": 0.0, "align": "middleCenter", "lineWeight": FAINT});
        if mask {
            v["mask"] = json!(true);
        }
        out.put("MM-XXXX-ETIKET", v);
    }
    // The title and the scale under it.
    let tx = (bx0 + bx1) / 2.0;
    out.text(
        "MM-XXXX-ETIKET",
        P::new(tx, lo.y - 0.9),
        &title(mark_words),
        title_height,
        "bottomCenter",
    );
    out.text(
        "MM-XXXX-ETIKET",
        P::new(tx, lo.y - 1.2),
        &format!("Ölçek 1/{plot:.0}"),
        text,
        "topCenter",
    );
    lo.y -= 1.6;
    let entities = out
        .entities
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    Drawn {
        entities,
        layers: out.layers,
        left: m.left.into_iter().collect(),
        lo,
        hi,
    }
}

/// What a room's words must not touch: the lines and outlines drawn so far, what is cut, the words placed.
struct Busy {
    lines: Vec<(P, P)>,
    areas: Vec<Vec<P>>,
    /// The words' height.
    text: f64,
}

impl Busy {
    fn of(out: &Out, cut: &[sec::Cut], text: f64) -> Self {
        let pt = |v: &Value| {
            P::new(
                v["x"].as_f64().unwrap_or(0.0),
                v["y"].as_f64().unwrap_or(0.0),
            )
        };
        let mut lines = Vec::new();
        for v in &out.entities {
            match v["kind"].as_str() {
                Some("line") => lines.push((pt(&v["a"]), pt(&v["b"]))),
                Some("polyline") => {
                    let pts: Vec<P> = v["pts"]
                        .as_array()
                        .map(|a| a.iter().map(pt).collect())
                        .unwrap_or_default();
                    lines.extend(pts.windows(2).map(|w| (w[0], w[1])));
                    if v["closed"].as_bool() == Some(true)
                        && let (Some(first), Some(last)) = (pts.first(), pts.last())
                    {
                        lines.push((*last, *first));
                    }
                }
                _ => {}
            }
        }
        Busy {
            lines,
            areas: cut.iter().map(|c| c.ring.clone()).collect(),
            text,
        }
    }

    /// Words `wide` long and their margin, centred on `at`.
    fn frame(&self, at: P, wide: f64) -> (P, P) {
        let (hw, hh) = (wide / 2.0 + self.text * 0.3, self.text * 0.8);
        (P::new(at.x - hw, at.y - hh), P::new(at.x + hw, at.y + hh))
    }

    /// Whether the box touches a line or lies in what is cut.
    fn touches(&self, (lo, hi): (P, P)) -> bool {
        let mid = P::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
        self.lines.iter().any(|(a, b)| meets_box(*a, *b, lo, hi))
            || self
                .areas
                .iter()
                .any(|r| kentos_geometry_core::geometry::point_in_polygon(mid, r))
    }

    /// Where a room's words `wide` long go in its stretch `(a, b)` between its floor and ceiling: the first place,
    /// middle and mid height first, whose box touches nothing; and whether none did (then the middle, to be masked).
    fn room_spot(&self, (a, b): (f64, f64), (floor, top): (f64, f64), wide: f64) -> (P, bool) {
        let mid = (a + b) / 2.0;
        let step = self.text * 2.0;
        let mut xs = vec![mid];
        let mut k = 1.0;
        while mid + k * step + wide / 2.0 <= b {
            xs.extend([mid - k * step, mid + k * step]);
            k += 1.0;
        }
        for f in [0.45, 0.6, 0.3, 0.75, 0.15] {
            let y = floor + (top - floor) * f;
            for x in &xs {
                let (lo, hi) = self.frame(P::new(*x, y), wide);
                if lo.x < a || hi.x > b || lo.y < floor || hi.y > top {
                    continue;
                }
                if !self.touches((lo, hi)) {
                    return (P::new(*x, y), false);
                }
            }
        }
        (P::new(mid, floor + (top - floor) * 0.45), true)
    }

    /// The words placed at `at` taken: the next room's keep off them.
    fn take(&mut self, at: P, wide: f64) {
        let (lo, hi) = self.frame(at, wide);
        let c = [lo, P::new(hi.x, lo.y), hi, P::new(lo.x, hi.y)];
        self.lines.extend((0..4).map(|i| (c[i], c[(i + 1) % 4])));
    }
}

/// Whether the segment `a`–`b` meets the box `lo`–`hi` (Liang–Barsky).
fn meets_box(a: P, b: P, lo: P, hi: P) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, a.x - lo.x),
        (dx, hi.x - a.x),
        (-dy, a.y - lo.y),
        (dy, hi.y - a.y),
    ] {
        if p.abs() < 1e-15 {
            if q < 0.0 {
                return false;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                if r > t1 {
                    return false;
                }
                t0 = t0.max(r);
            } else {
                if r < t0 {
                    return false;
                }
                t1 = t1.min(r);
            }
        }
    }
    true
}

/// An area of a cut ring, counter-clockwise.
fn area_of(ring: &[P]) -> kentos_geometry_core::geom::arrangement::Area {
    let mut pts = ring.to_vec();
    let n = pts.len();
    let twice: f64 = (0..n)
        .map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y)
        .sum();
    if twice < 0.0 {
        pts.reverse();
    }
    kentos_geometry_core::geom::arrangement::Area {
        outer: kentos_geometry_core::geom::arrangement::Ring { pts, bulges: None },
        holes: Vec::new(),
    }
}

/// Where the plan segment from `a` to `b` (as a line) crosses the section line: its `u`; none when parallel.
fn crossing(frame: &Frame, a: P, b: P) -> Option<f64> {
    let (da, db) = (frame.d(a), frame.d(b));
    if (da - db).abs() <= 1e-12 {
        return None;
    }
    let t = da / (da - db);
    let q = P::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
    Some(frame.u(q))
}

/// Seen pieces of one layer, those on one line and touching or overlapping joined.
fn merged(mut pieces: Vec<(P, P)>) -> Vec<(P, P)> {
    let key = |a: P, b: P| -> (i64, i64, i64) {
        // The line's direction (normalised, pointing right or up) and its offset, to a micrometre.
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l = (dx * dx + dy * dy).sqrt();
        let (mut ux, mut uy) = (dx / l, dy / l);
        if ux < -1e-12 || (ux.abs() <= 1e-12 && uy < 0.0) {
            ux = -ux;
            uy = -uy;
        }
        let off = a.x * -uy + a.y * ux;
        (
            (ux * 1e6).round() as i64,
            (uy * 1e6).round() as i64,
            (off * 1e6).round() as i64,
        )
    };
    pieces.retain(|(a, b)| (b.x - a.x).hypot(b.y - a.y) > 1e-6);
    // Each line (its direction and offset, to a micrometre) with its pieces: their spans along it and ends.
    type Pieces = Vec<(f64, f64, P, P)>;
    let mut lines: BTreeMap<(i64, i64, i64), Pieces> = BTreeMap::new();
    for (a, b) in pieces {
        let k = key(a, b);
        let (ux, uy) = (k.0 as f64 / 1e6, k.1 as f64 / 1e6);
        let (ta, tb) = (a.x * ux + a.y * uy, b.x * ux + b.y * uy);
        let (ta, tb, pa, pb) = if ta <= tb {
            (ta, tb, a, b)
        } else {
            (tb, ta, b, a)
        };
        lines.entry(k).or_default().push((ta, tb, pa, pb));
    }
    let mut out = Vec::new();
    for (_, mut segs) in lines {
        segs.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut cur = segs[0];
        for s in segs.into_iter().skip(1) {
            if s.0 <= cur.1 + 1e-6 {
                if s.1 > cur.1 {
                    cur.1 = s.1;
                    cur.3 = s.3;
                }
            } else {
                out.push((cur.2, cur.3));
                cur = s;
            }
        }
        out.push((cur.2, cur.3));
    }
    out
}

/// The parts of the horizontal line at `z` from `span.0` to `span.1` that no cut ring holds (inside or on it).
fn uncut(cuts: &[sec::Cut], span: (f64, f64), z: f64) -> Vec<(f64, f64)> {
    let mut covered: Vec<(f64, f64)> = Vec::new();
    for c in cuts {
        let r = &c.ring;
        let n = r.len();
        let mut xs: Vec<f64> = Vec::new();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            if (a.y - z).abs() <= 1e-9 && (b.y - z).abs() <= 1e-9 {
                covered.push((a.x.min(b.x), a.x.max(b.x)));
            } else if (a.y - z) * (b.y - z) < 0.0 {
                xs.push(a.x + (z - a.y) * (b.x - a.x) / (b.y - a.y));
            }
        }
        xs.sort_by(f64::total_cmp);
        for w in xs.chunks(2) {
            if let [x0, x1] = w {
                covered.push((*x0, *x1));
            }
        }
    }
    covered.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    let mut at = span.0;
    for (a, b) in covered {
        if a > at + 1e-9 {
            out.push((at, a.min(span.1)));
        }
        at = at.max(b);
        if at >= span.1 {
            break;
        }
    }
    if at < span.1 - 1e-9 {
        out.push((at, span.1));
    }
    out.retain(|(a, b)| b - a > 1e-6);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_and_levels_are_written_as_turkiye_writes_them() {
        assert_eq!(title("a"), "A-A KESİTİ");
        assert_eq!(title("i"), "İ-İ KESİTİ");
        assert_eq!(title(""), "KESİT");
        assert_eq!(label("B"), "B-B");
        assert_eq!(level_words(0.0), "±0.00");
        assert_eq!(level_words(3.0), "+3.00");
        assert_eq!(level_words(-2.75), "-2.75");
        assert_eq!(block_name("A"), "MM-XXXX-KESITA");
    }

    #[test]
    fn a_marks_block_is_the_same_in_every_file() {
        let u = Uuid::parse_str("0192f5a1-7242-7000-8000-000000000001").unwrap();
        assert_eq!(block_id(u), block_id(u));
        assert_ne!(
            block_id(u),
            block_id(Uuid::parse_str("0192f5a1-7242-7000-8000-000000000002").unwrap())
        );
    }

    #[test]
    fn the_ground_stops_at_what_is_cut() {
        let cut = sec::Cut {
            solid: 0,
            ring: vec![
                P::new(1.0, -0.5),
                P::new(3.0, -0.5),
                P::new(3.0, 0.2),
                P::new(1.0, 0.2),
            ],
        };
        assert_eq!(
            uncut(&[cut], (-2.0, 6.0), 0.0),
            vec![(-2.0, 1.0), (3.0, 6.0)]
        );
    }

    #[test]
    fn seen_pieces_on_one_line_are_joined() {
        let joined = merged(vec![
            (P::new(0.0, 1.0), P::new(2.0, 1.0)),
            (P::new(3.0, 1.0), P::new(1.5, 1.0)),
            (P::new(5.0, 1.0), P::new(6.0, 1.0)),
        ]);
        assert_eq!(joined.len(), 2);
    }
}
