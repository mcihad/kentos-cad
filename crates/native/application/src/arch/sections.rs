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
//! - **Roofs** (docs/adr/8044 §5): each face a sloped slab (its ring, the roof's plane over it, the roof's thickness
//!   under it, upright), hatched as wood (TS 5319 SN23) on its covering's layer (MM-CATI-KIREMIT); a flat roof a slab
//!   of its thickness under its eaves. Their eaves' and ridges' levels join the storeys' at the left ("+7.69 Mahya",
//!   Mimarlar Odası 14.3.4 g, 14.5.4 q: "Dere mahya, saçak … kotlandırılır"), and a face cut down its fall line is
//!   given its slope ("%33").
//! - **What the model does not hold** is listed, not drawn (§3; ADR 0347 §3): the roof when it has none, the natural
//!   ground, a curved stair or wall, a block's height.
//! - **Ramps** (docs/adr/8070 §1): each run a sloped slab (the ramp's plane over its band, as thick as the slab it
//!   arrives at), each landing a flat slab at its height.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{BeamRole, Entity, OpeningForm, SlabLayerKind, Vec2, ViewMarkForm};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::section::{self as sec, Frame, Height, Solid, Void};
use kentos_geometry_core::vec2::Vec2 as P;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[path = "sections/frame.rs"]
mod frame;
use frame::{beam, column};
pub use frame::MEMBER_ELEMENT;

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
    /// A curved wall, slab or stair: the section draws its chords, not its curve (the IFC writes a wall's and a slab's
    /// arcs as arcs from [`Model::exact`], docs/adr/8071 §1).
    Curved,
    /// A wall with no height.
    NoHeight,
    /// An opening without a head: its gap reaches its wall's top.
    HeadUnknown,
    /// A stair with no slab to take its waist from: drawn solid to its floor.
    StairWaist,
}

impl Left {
    /// What the drawing's note says of it.
    pub fn words(self) -> &'static str {
        match self {
            Left::Roof => "çatı (modelde yok)",
            Left::NaturalGround => "tabii zemin (modelde yok)",
            Left::Curved => "eğri duvar ya da merdiven (düz parçalarla)",
            Left::NoHeight => "yüksekliği olmayan duvar",
            Left::HeadUnknown => "lentosu olmayan açıklık (duvarın tepesine kadar)",
            Left::StairWaist => "merdiven plağı (döşemesi yok: dolu)",
        }
    }
}

/// An opening through a wall, for what the section draws in its gap.
#[derive(Clone, Debug)]
pub struct Gap {
    /// Its outline through the wall: its start jamb's left and right face points, its end jamb's right and left.
    pub ring: Vec<P>,
    /// Its ring's edges' bulges (docs/adr/8071 §1): through a curved wall its edges along the faces are their arcs
    /// (the jambs' points are on them); empty where every edge is straight.
    pub bulges: Vec<f64>,
    pub sill: f64,
    pub head: f64,
    pub form: OpeningForm,
    /// The middle of its frame across its wall: two points on the wall's faces' line through it (its frame's middle
    /// line in plan), and the frame's depth across the wall when its type says.
    pub frame: (P, P),
    pub depth: Option<f64>,
    /// The opening's persistent id (IFC's opening and its door or window, docs/adr/8047).
    pub owner: Option<String>,
}

/// The model as the section reads it.
#[derive(Clone, Debug, Default)]
pub struct Model {
    pub solids: Vec<Solid>,
    pub looks: Vec<Look>,
    /// The persistent id of each solid's object.
    pub owners: Vec<Option<String>>,
    /// Which part of its object each solid is (docs/adr/8070 §3, §7): a roof's chimney or vent stack, a curtain wall's
    /// frame or panel; none: the object itself.
    pub parts: Vec<Option<Part>>,
    /// Each solid's footprint with its arcs (docs/adr/8071 §1): its rings' points and each edge's bulge, the source's
    /// own; none where its rings are exact as they are (no arc). `solids`' rings are these flattened for the section,
    /// which is the view's work; a writer that draws arcs (the IFC profile) and a measure take these.
    pub exact: Vec<Option<Vec<ExactRing>>>,
    /// The voids' parts over their storeys' floor slabs (docs/adr/8071 §5), taken out of those slabs' rings.
    pub slab_cuts: Vec<SlabCut>,
    pub gaps: Vec<Gap>,
    pub left: BTreeSet<(Left, Option<String>)>,
    /// The storeys: id, name, floor, height.
    pub levels: Vec<(String, String, f64, Option<f64>)>,
    /// The roofs (docs/adr/8044 §5): their eaves' and ridges' levels and their solids.
    pub roofs: Vec<RoofAt>,
    /// The roofs whose gutters and downpipes are built once every wall and column is in (docs/adr/8044 §8).
    rain: Vec<Rain>,
    /// The slabs' structure plates, by solid: what the frame's columns and beams stop under (docs/adr/8171 §2.1).
    pub structure: Vec<usize>,
}

/// A ring with its arcs (docs/adr/8071 §1): its points and each edge's bulge (to the next point), its first point not
/// repeated at its end.
pub type ExactRing = (Vec<P>, Vec<f64>);

/// A void's part over a floor slab (docs/adr/8071 §5): the slab's id, the void's (a space, docs/adr/0708) and the
/// outline taken out of the slab, with its arcs.
#[derive(Clone, Debug, PartialEq)]
pub struct SlabCut {
    pub owner: Option<String>,
    pub source: String,
    pub ring: Vec<P>,
    pub bulges: Vec<f64>,
}

/// What part of its object a solid is (docs/adr/8070 §3, §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// A roof's chimney or vent stack, by its index in the roof's `parts`.
    Roof(usize),
    /// A curtain wall's mullion (dikme), by its number on the wall.
    Mullion(usize),
    /// A curtain wall's transom (kayıt), by its number on the wall.
    Transom(usize),
    /// A curtain wall's panel.
    Panel,
    /// A stair's or a ramp's railing (docs/adr/8128), its side's (0 left, 1 right as one walks up): its top rail and
    /// its balusters, drawn, not the stair's or the ramp's body.
    Rail(usize),
}

/// A roof's gutter and downpipes waiting for the walls they run down (docs/adr/8044 §8).
#[derive(Clone, Debug)]
struct Rain {
    /// The roof's index in [`Model::roofs`].
    roof: usize,
    pts: Vec<P>,
    spec: kentos_geometry_core::geom::roof::RoofSpec,
    owner: Option<String>,
}

/// A roof as a section reads it: its eaves' level, its ridge's, its solids (its faces, or its flat slab).
#[derive(Clone, Debug, Default)]
pub struct RoofAt {
    pub eaves: f64,
    pub ridge: f64,
    /// The top of the storey it stands on: PAİY 4's saçak seviyesi ("son kat tavan döşemesinin üst kotu").
    pub seat: f64,
    pub solids: Vec<usize>,
    /// Its chimneys' and vents' tops (docs/adr/8044 §6), each with its solid and its word in the levels ("Baca").
    pub tops: Vec<(f64, usize, &'static str)>,
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

/// A covering's look (docs/adr/8070 §10): on its type's EK-2 element by its material (MM-ASTV-ALCIPAN; none: GENEL),
/// light and unhatched: a thin plate, no mass to hatch.
fn covering_look(c: &kentos_commands::Covering, material: Option<&str>) -> Look {
    let element = kentos_commands::Covering::element(&c.predefined);
    let m = material.map(str::trim).filter(|m| !m.is_empty()).unwrap_or_default();
    Look {
        key: format!("{element}:{}", fold(m)),
        layer: kentos_commands::ek4_layer(element, m),
        heavy: false,
        fill: None,
    }
}

/// A slab layer's look: its element (EK-2 Tablo 2.3) and material; insulation TS 5319's SN55, drawn cut as one zigzag
/// fitted between its sides (`geom::zigzag`, docs/adr/8071 §8).
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
    // The roof the model has is drawn (docs/adr/8044 §5); without one it is said to be missing.
    if !doc.entities().any(|e| matches!(e, Entity::Roof(_))) {
        m.left.insert((Left::Roof, None));
    }
    m.left.insert((Left::NaturalGround, None));
    let entities: Vec<(Uuid, &Entity)> = doc.entities_with_uids().collect();
    // The building's floors: a covering is none (docs/adr/8070 §10).
    let slabs: Vec<SlabAt> = entities
        .iter()
        .filter_map(|(_, e)| match e {
            Entity::Slab(s) if s.slab.covering.is_none() => Some(&s.slab),
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
    // The voids (docs/adr/0708: shafts, a stair's or a gallery's opening; no floor in them), each with its id.
    let voids: Vec<(String, &kentos_commands::SpaceFields)> = entities
        .iter()
        .filter_map(|(u, e)| match e {
            Entity::Space(sp) if sp.space.void => Some((u.to_string(), &*sp.space)),
            _ => None,
        })
        .collect();
    for (uid, e) in &entities {
        let owner = Some(uid.to_string());
        match e {
            Entity::Slab(s) => slab(&s.slab, &m.levels.clone(), owner, &voids, &mut m),
            Entity::Column(c) => column(&c.column, &slabs, owner, &mut m),
            Entity::Beam(b) => beam(doc, &b.beam, owner, &mut m),
            Entity::Roof(r) => roof(doc, r, owner, &mut m),
            Entity::Railing(r) => railing(&r.railing, owner, &mut m),
            Entity::Stair(s) => stair(doc, &s.stair, owner, &mut m),
            Entity::Ramp(r) => ramp(&r.ramp, &slabs, owner, &mut m),
            Entity::Insert(i) => fixture(doc, i, owner, &mut m),
            _ => {}
        }
    }
    for rain in std::mem::take(&mut m.rain) {
        rain_water(&rain, &mut m);
    }
    frame::joins(doc, &mut m);
    m
}

fn push(m: &mut Model, solid: Solid, look: Look, owner: Option<String>) {
    push_part(m, solid, look, owner, None);
}

/// [`push`] of a part's solid: a roof's stack, a curtain wall's frame or panel (docs/adr/8070 §3, §7).
fn push_part(m: &mut Model, solid: Solid, look: Look, owner: Option<String>, part: Option<Part>) {
    m.solids.push(solid);
    m.looks.push(look);
    m.owners.push(owner);
    m.parts.push(part);
    m.exact.push(None);
}

/// [`push`] of a solid whose footprint has arcs (docs/adr/8071 §1): `exact` its rings with them, `solid`'s rings the
/// same flattened.
fn push_exact(m: &mut Model, solid: Solid, look: Look, owner: Option<String>, exact: Vec<ExactRing>) {
    push(m, solid, look, owner);
    if let Some(last) = m.exact.last_mut() {
        *last = Some(exact);
    }
}

/// The circles a wall's lines run on where they are arcs (docs/adr/8071 §1).
fn circles_of(s: &kentos_geometry_core::entity::Shape) -> Vec<(P, f64)> {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::bulge::bulge_arc;
    let mut out: Vec<(P, f64)> = Vec::new();
    let Shape::Wall { pts, bulges, wall } = s else {
        return out;
    };
    if !bulges.as_ref().is_some_and(|b| b.iter().any(|x| *x != 0.0)) {
        return out;
    }
    let Ok(lines) = kentos_geometry_core::geom::wall::lines(pts, bulges.as_deref(), wall) else {
        return out;
    };
    for l in &lines {
        let n = l.pts.len();
        let segs = if wall.closed { n } else { n.saturating_sub(1) };
        for i in 0..segs {
            if let Some(arc) = bulge_arc(l.pts[i], l.pts[(i + 1) % n], l.bulges.get(i).copied().unwrap_or(0.0)) {
                add_circle(&mut out, (arc.c, arc.r));
            }
        }
    }
    out
}

fn add_circle(list: &mut Vec<(P, f64)>, c: (P, f64)) {
    let same = |d: &(P, f64)| (d.0.x - c.0.x).abs() < 1e-9 && (d.0.y - c.0.y).abs() < 1e-9 && (d.1 - c.1).abs() < 1e-9;
    if !list.iter().any(same) {
        list.push(c);
    }
}

/// The circles every wall's lines run on where they are arcs (docs/adr/8071 §1): what a wall's flattened rings are taken
/// back to, a wall's end cut along another's curved face too.
fn wall_circles(walls: &[(Uuid, &kentos_commands::WallFields, kentos_geometry_core::entity::Shape)]) -> Vec<(P, f64)> {
    let mut out: Vec<(P, f64)> = Vec::new();
    for (_, _, s) in walls {
        for c in circles_of(s) {
            add_circle(&mut out, c);
        }
    }
    out
}

/// The walls, joined as the plan joins them and cut by the columns on their storeys, with their openings.
fn walls(doc: &Document, entities: &[(Uuid, &Entity)], m: &mut Model) {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::column_join::{Foot, cut};
    use kentos_geometry_core::geom::wall_join::{WallIn, level, solve_with};
    let walls: Vec<(Uuid, &kentos_commands::WallFields, Shape)> = entities
        .iter()
        .filter_map(|(u, e)| match e {
            Entity::Wall(w) => Some((*u, &*w.wall, kentos_formats::walls::core_wall(&w.wall))),
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
    // The circles the walls' arcs run on (docs/adr/8071 §1).
    let circles = wall_circles(&walls);
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
    // Each wall's openings, by its persistent id, with their own.
    let mut openings: BTreeMap<String, Vec<(Uuid, &kentos_commands::OpeningFields)>> =
        BTreeMap::new();
    for (u, e) in entities {
        if let Entity::Opening(o) = e {
            openings
                .entry(o.opening.host.clone())
                .or_default()
                .push((*u, &o.opening));
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
        if let Some(grid) = &w.curtain {
            let mine = openings.get(&uid.to_string()).map_or(&[][..], |v| &v[..]);
            curtain(w, grid, (foot, top), floor, mine, owner, m);
            continue;
        }
        // Its core: the layer of the smallest rank (docs/adr/8008), whose material names the wall.
        let core = spec
            .components
            .iter()
            .min_by_key(|c| c.rank())
            .and_then(|c| c.material.clone().or_else(|| c.name.clone()));
        let base = material_look("DUVA", core.as_deref(), None);
        let mut voids = Vec::new();
        let mut opening_solids: Vec<(Solid, Look, Option<String>)> = Vec::new();
        let own = circles_of(shape);
        for (opening, o) in openings.get(&uid.to_string()).into_iter().flatten() {
            let Ok([l0, r0, l1, r1]) = <[Vec2; 4]>::try_from(o.place.clone()) else {
                continue;
            };
            // A niche or a flue (docs/adr/8070 §3): a box taken out of its wall across part of its thickness, from its
            // sill (the floor) to its head (the wall's top); cut where the line crosses it, never seen.
            if matches!(o.form, OpeningForm::Niche | OpeningForm::Flue) {
                if let Some(ring) = recess_ring(o, [l0, r0, l1, r1]) {
                    voids.push(Void {
                        ring,
                        bottom: floor + o.sill.unwrap_or(0.0),
                        top: o.head.map_or(top, |h| floor + h),
                    });
                }
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
            // Its lines on both faces of the wall, a solid of their own on its door's or window's layer (docs/adr/8043
            // §2): its outline, its frame inset by its width when its type gives one, and on a hinged one Türkiye's
            // opening lines (the joinery list's, docs/adr/8038: from the hinge side's corners to the middle of the
            // handle side; a pair from both jambs to their meeting line). Where nothing nearer hides them the section
            // and the elevation draw them; the face behind the wall is hidden as the wall is.
            let lines = opening_lines(o, [(p(l0), p(l1)), (p(r0), p(r1))], (sill, head));
            opening_solids.push((
                Solid {
                    rings: Vec::new(),
                    height: Height::Prism {
                        bottom: sill,
                        top: head,
                    },
                    voids: Vec::new(),
                    marks: lines,
                    finish: None,
                },
                opening_look(o.form),
                Some(opening.to_string()),
            ));
            let mid = |x: Vec2, y: Vec2| P::new((x.x + y.x) / 2.0, (x.y + y.y) / 2.0);
            // Through a curved wall its edges along the faces are the faces' arcs (docs/adr/8071 §1).
            let bulges = kentos_geometry_core::geom::arc_recover::along(&ring, &own);
            m.gaps.push(Gap {
                bulges: if bulges.iter().any(|b| *b != 0.0) { bulges } else { Vec::new() },
                ring,
                sill,
                head,
                form: o.form,
                frame: (mid(l0, r0), mid(l1, r1)),
                depth: o.build.as_ref().and_then(|b| b.frame_depth),
                owner: Some(opening.to_string()),
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
            // Its faces' finish (docs/adr/8043 §7): the layer's project material, else its material's name; a wall
            // without layers has none and meets what is flush with it without a line.
            let finish = comp.and_then(|c| {
                c.material_id
                    .clone()
                    .or_else(|| c.material.clone())
                    .filter(|f| !f.trim().is_empty())
            });
            for (ring, holes) in bodies {
                let mut rings = vec![ring.clone()];
                rings.extend(holes.iter().flatten().cloned());
                // Its arcs taken back from its flattened rings (docs/adr/8071 §1).
                let exact: Vec<ExactRing> = rings
                    .iter()
                    .map(|r| kentos_geometry_core::geom::arc_recover::recover(r, &circles))
                    .collect();
                let arcs = exact.iter().any(|(_, b)| b.iter().any(|x| *x != 0.0));
                let solid = Solid {
                    rings,
                    height: Height::Prism { bottom: foot, top },
                    voids: voids.clone(),
                    marks: Vec::new(),
                    finish: finish.clone(),
                };
                if arcs {
                    push_exact(m, solid, look.clone(), owner.clone(), exact);
                } else {
                    push(m, solid, look.clone(), owner.clone());
                }
            }
        }
        for (solid, look, owner) in opening_solids.drain(..) {
            push(m, solid, look, owner);
        }
    }
}

/// A door's or a window's lines on its wall's faces (docs/adr/8043 §2), each face `(a, b)` its jambs' points on it,
/// from `sill` to `head`: its outline, its frame inset by its width when its type gives one, and on a hinged one
/// Türkiye's opening lines (the joinery list's, docs/adr/8038: from the hinge side's corners to the middle of the
/// handle side; a pair from both jambs to their meeting line).
fn opening_lines(
    o: &kentos_commands::OpeningFields,
    faces: [(P, P); 2],
    (sill, head): (f64, f64),
) -> Vec<[(P, f64); 2]> {
    let mut lines: Vec<[(P, f64); 2]> = Vec::new();
    for (a, b) in faces {
        outline(&mut lines, a, b, sill, head);
        let (mut a2, mut b2, mut low, mut high) = (a, b, sill, head);
        if let Some(f) = o.frame.filter(|f| *f > 0.0) {
            let len = kentos_geometry_core::jsmath::js_hypot(b.x - a.x, b.y - a.y);
            if len > 2.0 * f && head - sill > 2.0 * f {
                let t = f / len;
                a2 = P::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
                b2 = P::new(b.x - (b.x - a.x) * t, b.y - (b.y - a.y) * t);
                low = if o.form == OpeningForm::Door { sill } else { sill + f };
                high = head - f;
                outline(&mut lines, a2, b2, low, high);
            }
        }
        if o.operation == kentos_commands::OpeningOperation::Swing {
            swing_lines(&mut lines, (a2, b2), (low, high), o.leaves, o.hand);
        }
    }
    lines
}

/// A door's or a window's lines' look: on its form's layer, light.
fn opening_look(form: OpeningForm) -> Look {
    let (layer, key) = match form {
        OpeningForm::Door => ("MM-KAPI-GENEL", "kapi"),
        _ => ("MM-PENC-GENEL", "pencere"),
    };
    Look {
        key: key.into(),
        layer: layer.into(),
        heavy: false,
        fill: None,
    }
}

/// A curtain wall's panel's thickness, metres: KentOS's own common size (docs/adr/8070 §7), an insulating glass unit's.
pub const PANEL: f64 = 0.024;

/// A curtain wall in the model (docs/adr/8070 §7): on each straight stretch of its line, its mullions (a frame's face
/// along it, its whole depth across, from its foot to its top) on the grid's vertical lines, its transoms between them
/// on the horizontal ones (its foot and top among them), and a panel in each cell, [`PANEL`] thick down the middle of
/// its depth. Both on EK-2's Giydirme Cephe (GIYC), unhatched: its frames (ÇERÇEVE) and its glass (CAM). A door, a
/// window or a bare opening fills its cells in place of their panels (docs/adr/8129): the transoms across it and the
/// mullions' part inside it are not there, its lines are drawn on the faces as a wall's door's and its gap is the
/// model's door or window.
fn curtain(
    w: &kentos_commands::WallFields,
    grid: &kentos_commands::CurtainGrid,
    (foot, top): (f64, f64),
    floor: f64,
    openings: &[(Uuid, &kentos_commands::OpeningFields)],
    owner: Option<String>,
    m: &mut Model,
) {
    use kentos_geometry_core::geom::curtain::{cell_span, divisions, frame_span};
    let left = w.justify.left_face(w.width) + w.offset;
    let right = left - w.width;
    let middle = (left + right) / 2.0;
    let face = grid.frame();
    let height = top - foot;
    if height <= 2.0 * face {
        m.left.insert((Left::NoHeight, owner));
        return;
    }
    let rows = divisions(height, grid.rows);
    let frames = Look {
        key: "GIYC:cerceve".into(),
        layer: kentos_commands::ek4_layer("GIYC", "Çerçeve"),
        heavy: false,
        fill: None,
    };
    let glass = Look {
        key: "GIYC:cam".into(),
        layer: kentos_commands::ek4_layer("GIYC", "Cam"),
        heavy: false,
        fill: None,
    };
    let block = |ring: Vec<P>, bottom: f64, top: f64| Solid {
        rings: vec![ring],
        height: Height::Prism { bottom, top },
        voids: Vec::new(),
        marks: Vec::new(),
        finish: None,
    };
    // Its doors, windows and bare openings in place of panels (docs/adr/8129): each along its base line and between
    // its sill and head (over the storey's floor), its jambs on the faces.
    #[allow(clippy::type_complexity)]
    let filled: Vec<(&Uuid, &kentos_commands::OpeningFields, (f64, f64), (f64, f64), [Vec2; 4])> = openings
        .iter()
        .filter(|(_, o)| matches!(o.form, OpeningForm::Door | OpeningForm::Window | OpeningForm::Opening))
        .filter(|(_, o)| o.broken.is_none())
        .filter_map(|(u, o)| {
            let place = <[Vec2; 4]>::try_from(o.place.clone()).ok()?;
            let (z0, z1) = (floor + o.sill.unwrap_or(0.0), o.head.map_or(top, |h| floor + h));
            (z1 > z0).then_some((u, *o, kentos_domain::opening_follow::span(o), (z0, z1), place))
        })
        .collect();
    let within = |s: f64, z: f64| {
        filled.iter().any(|(_, _, (s0, s1), (z0, z1), _)| s > s0 + 1e-9 && s < s1 - 1e-9 && z > z0 + 1e-9 && z < z1 - 1e-9)
    };
    let n = w.pts.len();
    let stretches = if w.closed { n } else { n.saturating_sub(1) };
    let (mut mullions, mut transoms) = (0usize, 0usize);
    let mut start = 0.0;
    for i in 0..stretches {
        let (a, b) = (p(w.pts[i]), p(w.pts[(i + 1) % n]));
        let length = kentos_geometry_core::jsmath::js_hypot(b.x - a.x, b.y - a.y);
        if length <= 2.0 * face {
            start += length;
            continue;
        }
        let u = P::new((b.x - a.x) / length, (b.y - a.y) / length);
        let normal = P::new(-u.y, u.x);
        let at = |s: f64, off: f64| P::new(a.x + u.x * s + normal.x * off, a.y + u.y * s + normal.y * off);
        let band = |s0: f64, s1: f64, near: f64, far: f64| vec![at(s0, near), at(s1, near), at(s1, far), at(s0, far)];
        let bays = divisions(length, grid.spacing);
        // A stretch's first mullion stands at a corner the stretch before already has, but for the line's start.
        for k in 0..bays.len() {
            if k == 0 && i > 0 {
                continue;
            }
            if k == bays.len() - 1 && w.closed && i == stretches - 1 {
                continue;
            }
            let (s0, s1) = frame_span(&bays, k, face);
            // A mullion inside a door or a window stands only under and over it.
            let mut pieces = vec![(foot, top)];
            for (_, _, (o0, o1), (z0, z1), _) in &filled {
                let line = start + bays[k];
                if line > o0 + 1e-9 && line < o1 - 1e-9 {
                    pieces = pieces
                        .into_iter()
                        .flat_map(|(lo, hi)| [(lo, hi.min(*z0)), (lo.max(*z1), hi)])
                        .filter(|(lo, hi)| hi - lo > 1e-9)
                        .collect();
                }
            }
            for (lo, hi) in pieces {
                push_part(m, block(band(s0, s1, left, right), lo, hi), frames.clone(), owner.clone(), Some(Part::Mullion(mullions)));
                mullions += 1;
            }
        }
        for k in 0..bays.len() - 1 {
            let (s0, s1) = cell_span(&bays, k, face);
            if s1 - s0 <= 1e-9 {
                continue;
            }
            let across = start + (s0 + s1) / 2.0;
            for r in 0..rows.len() {
                let (z0, z1) = frame_span(&rows, r, face);
                // A transom across a door or a window is not there (a door's foot one neither: it stands on the floor).
                if within(across, foot + (z0 + z1) / 2.0) {
                    continue;
                }
                push_part(
                    m,
                    block(band(s0, s1, left, right), foot + z0, foot + z1),
                    frames.clone(),
                    owner.clone(),
                    Some(Part::Transom(transoms)),
                );
                transoms += 1;
            }
            for r in 0..rows.len() - 1 {
                let (z0, z1) = cell_span(&rows, r, face);
                if z1 - z0 <= 1e-9 || within(across, foot + (z0 + z1) / 2.0) {
                    continue;
                }
                let t = PANEL.min(w.width / 2.0);
                push_part(
                    m,
                    block(band(s0, s1, middle + t / 2.0, middle - t / 2.0), foot + z0, foot + z1),
                    glass.clone(),
                    owner.clone(),
                    Some(Part::Panel),
                );
            }
        }
        start += length;
    }
    // The doors' and windows' lines on its faces and their gaps (the model's doors and windows), as a wall's.
    for (opening, o, _, (z0, z1), [l0, r0, l1, r1]) in filled {
        if o.form != OpeningForm::Opening {
            let lines = opening_lines(o, [(p(l0), p(l1)), (p(r0), p(r1))], (z0, z1));
            let solid = Solid {
                rings: Vec::new(),
                height: Height::Prism { bottom: z0, top: z1 },
                voids: Vec::new(),
                marks: lines,
                finish: None,
            };
            push(m, solid, opening_look(o.form), Some(opening.to_string()));
        }
        let mid = |x: Vec2, y: Vec2| P::new((x.x + y.x) / 2.0, (x.y + y.y) / 2.0);
        m.gaps.push(Gap {
            ring: vec![p(l0), p(l1), p(r1), p(r0)],
            bulges: Vec::new(),
            sill: z0,
            head: z1,
            form: o.form,
            frame: (mid(l0, r0), mid(l1, r1)),
            depth: o.build.as_ref().and_then(|b| b.frame_depth),
            owner: Some(opening.to_string()),
        });
    }
}

/// A niche's or a flue's outline in plan (docs/adr/0719, 8070 §3): the band of its wall between its jambs (`place`: the
/// start jamb's points on the wall's left and right face, then the end jamb's) across the part of the wall's thickness
/// it takes: a niche `depth` from its face (`side`), a flue `depth` across from `inset` off its face (absent: centred).
/// None without its depth.
pub fn recess_ring(o: &kentos_commands::OpeningFields, place: [Vec2; 4]) -> Option<Vec<P>> {
    use kentos_geometry_core::geom::opening::Recess;
    Recess::of(
        o.form == OpeningForm::Niche,
        o.side == kentos_commands::OpeningSide::Right,
        o.inset,
        o.depth,
    )?
    .band(place.map(p))
}

/// A hinged opening's lines on a face from `a` to `b` (its frame's inner edge, or its jambs) between `low` and `high`
/// (docs/adr/8043 §2; Türkiye's convention, the joinery list's, docs/adr/8038): one leaf from the corners at its hinge's
/// end to the middle of the other jamb; two leaves each from its own jamb to the middle of their meeting line, which
/// is drawn.
fn swing_lines(
    lines: &mut Vec<[(P, f64); 2]>,
    (a, b): (P, P),
    (low, high): (f64, f64),
    leaves: u8,
    hand: kentos_commands::OpeningHand,
) {
    let mid = (low + high) / 2.0;
    let at = |t: f64| P::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
    if leaves >= 2 {
        let m = at(0.5);
        for jamb in [a, b] {
            lines.push([(jamb, low), (m, mid)]);
            lines.push([(jamb, high), (m, mid)]);
        }
        lines.push([(m, low), (m, high)]);
        return;
    }
    let (hinge, other) = match hand {
        kentos_commands::OpeningHand::End => (b, a),
        _ => (a, b),
    };
    lines.push([(hinge, low), (other, mid)]);
    lines.push([(hinge, high), (other, mid)]);
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
    voids: &[(String, &kentos_commands::SpaceFields)],
    m: &mut Model,
) {
    use kentos_geometry_core::geom::arrangement::{Area, Ring};
    use kentos_geometry_core::geom::region::{intersect_area_sets, subtract_areas};
    let (floor, _) = floor_of(levels, s.arch.level.as_deref());
    // Its rings with their arcs, the source's (docs/adr/8071 §1); the section's flattened from them at the core's step
    // (an arc's edge was a single chord before: its vertices alone).
    let ring = |r: &kentos_commands::SpaceRing| -> ExactRing {
        let pts: Vec<P> = r.pts.iter().map(|q| p(*q)).collect();
        let n = pts.len();
        let b: Vec<f64> = (0..n).map(|i| r.bulges.as_ref().and_then(|b| b.get(i)).copied().unwrap_or(0.0)).collect();
        (pts, b)
    };
    let area = |part: &kentos_commands::SpacePart| -> Area {
        let r = |x: &kentos_commands::SpaceRing| {
            let (pts, b) = ring(x);
            Ring { bulges: b.iter().any(|v| *v != 0.0).then_some(b), pts }
        };
        Area { outer: r(&part.outer), holes: part.holes.iter().map(r).collect() }
    };
    let mut exact: Vec<ExactRing> =
        s.parts.iter().flat_map(|part| std::iter::once(&part.outer).chain(&part.holes)).map(ring).collect();
    // The voids it reads take their outlines out of it (docs/adr/8071 §5; docs/adr/0708 "no floor in it"), as the
    // quantities read them (docs/adr/0722 §5): those of its storey, every one when it has none. A covering is no floor;
    // a sloped slab has no hole. Each void's part over it is kept for the IFC's opening.
    let reads = |v: &kentos_commands::SpaceFields| s.arch.level.is_none() || v.arch.level == s.arch.level;
    let on: Vec<&(String, &kentos_commands::SpaceFields)> = voids
        .iter()
        .filter(|(_, v)| s.covering.is_none() && s.slope.is_none() && reads(v))
        .collect();
    if !on.is_empty() {
        let own: Vec<Area> = s.parts.iter().map(area).collect();
        let mut cut_any = false;
        for (uid, v) in &on {
            let shaft: Vec<Area> = v.parts.iter().map(area).collect();
            for a in intersect_area_sets(&[shaft, own.clone()]) {
                cut_any = true;
                let pts = a.outer.pts.clone();
                let n = pts.len();
                m.slab_cuts.push(SlabCut {
                    owner: owner.clone(),
                    source: uid.clone(),
                    ring: pts,
                    bulges: a.outer.bulges.clone().unwrap_or_else(|| vec![0.0; n]),
                });
            }
        }
        if cut_any {
            let shafts: Vec<Area> = on.iter().flat_map(|(_, v)| v.parts.iter().map(area)).collect();
            exact = subtract_areas(&own, &shafts)
                .iter()
                .flat_map(|a| std::iter::once(&a.outer).chain(&a.holes))
                .map(|r| {
                    let n = r.pts.len();
                    (r.pts.clone(), r.bulges.clone().unwrap_or_else(|| vec![0.0; n]))
                })
                .collect();
        }
    }
    let arcs = exact.iter().any(|(_, b)| b.iter().any(|x| *x != 0.0));
    if arcs {
        m.left.insert((Left::Curved, owner.clone()));
    }
    let rings: Vec<Vec<P>> = exact
        .iter()
        .map(|(pts, b)| kentos_geometry_core::entity::polygon_ring(pts, Some(b)))
        .collect();
    let exact = arcs.then_some(exact);
    // A sloped slab (docs/adr/8071 §6): each layer a sloped plate under the plane its arrow gives, its thicknesses
    // (square to it) standing √(1 + grade²) upright; its sides upright on its outline, every edge one.
    let k = s.upright();
    let plane = s.slope.map(|sl| {
        let (dx, dy) = (sl.to.x - sl.from.x, sl.to.y - sl.from.y);
        let l2 = dx * dx + dy * dy;
        let (a, b) = (sl.rise * dx / l2, sl.rise * dy / l2);
        (a, b, -(a * sl.from.x + b * sl.from.y))
    });
    let mut z = floor + s.offset;
    let solid = |bottom: f64, top: f64| Solid {
        rings: rings.clone(),
        height: match plane {
            Some((a, b, c)) => Height::Sloped(sec::Sloped {
                top: top + c,
                dx: a,
                dy: b,
                thickness: top - bottom,
                eaves: rings.first().cloned().unwrap_or_default(),
                roof: 0,
            }),
            None => Height::Prism { bottom, top },
        },
        voids: Vec::new(),
        marks: Vec::new(),
        finish: None,
    };
    // Each of its solids with its arcs (docs/adr/8071 §1); a sloped plate's faces drawn as its own.
    let put = |m: &mut Model, mut solid: Solid, look: Look| {
        if let Height::Sloped(sl) = &mut solid.height {
            sl.roof = SLAB_FACES + m.solids.len();
        }
        match &exact {
            Some(e) => push_exact(m, solid, look, owner.clone(), e.clone()),
            None => push(m, solid, look, owner.clone()),
        }
    };
    for l in &s.layers.over {
        put(m, solid(z - l.thickness * k, z), slab_layer_look(l.kind, l.material.as_deref()));
        z -= l.thickness * k;
    }
    // A covering's plate on its type's EK-2 element (docs/adr/8070 §10).
    let look = match &s.covering {
        Some(c) => covering_look(c, s.material.as_deref()),
        None => material_look("DOSM", s.material.as_deref(), None),
    };
    put(m, solid(z - s.thickness * k, z), look);
    if s.covering.is_none() {
        m.structure.push(m.solids.len() - 1);
    }
    z -= s.thickness * k;
    for l in &s.layers.under {
        put(m, solid(z - l.thickness * k, z), slab_layer_look(l.kind, l.material.as_deref()));
        z -= l.thickness * k;
    }
}

/// The face group of a sloped slab's plate (docs/adr/8071 §6): its own, apart from the roofs' and the ramps'.
const SLAB_FACES: usize = usize::MAX / 8;

/// A railing's look in a section (docs/adr/8045 §3): on its infill's layer (EK-2 Korkuluk: MM-KORK-CUBUK), outlined
/// light and unhatched: a balustered or glass railing is no mass to hatch, and its cut is a top rail's width.
fn railing_look(r: &kentos_commands::RailingFields) -> Look {
    Look {
        key: format!("KORK:{}", r.infill.name()),
        layer: kentos_commands::ek4_layer("KORK", r.infill.layer_word()),
        heavy: false,
        fill: None,
    }
}

/// How deep a railing's top rail is drawn in a section's or an elevation's view, metres.
pub const TOP_RAIL_DEPTH: f64 = 0.05;
pub use kentos_commands::BALUSTER_WIDTH;

/// A railing (docs/adr/8045 §3), from its floor (its storey's, moved by its offset) to its height. A solid panel is a
/// wall-like band, its top rail's underside marked on both faces. A balustered or glass railing hides nothing behind it:
/// its top rail alone is a solid (5 cm deep, cut where the line crosses it), its balusters (a bar every gap and 2 cm along
/// its line) or its glass's ends are lines of a solid without a footprint, seen where nothing nearer hides them, as a
/// door's lines on its wall (docs/adr/8043).
fn railing(r: &kentos_commands::RailingFields, owner: Option<String>, m: &mut Model) {
    use kentos_commands::RailingInfill;
    use kentos_geometry_core::entity::Shape;
    let (floor, _) = floor_of(&m.levels, r.arch.level.as_deref());
    let (bottom, top) = (floor + r.offset, floor + r.offset + r.height);
    let Shape::Railing { pts, railing: spec } = kentos_formats::arch_elements::core_railing(r)
    else {
        return;
    };
    let Some((left, right)) = kentos_geometry_core::geom::railing::faces(&pts, &spec) else {
        return;
    };
    let rings = if spec.closed {
        vec![left.clone(), right.clone()]
    } else {
        let mut band = left.clone();
        band.extend(right.iter().rev().copied());
        vec![band]
    };
    let rail = (top - TOP_RAIL_DEPTH).max(bottom);
    let closed_line = |line: &[P]| {
        let mut out = line.to_vec();
        if spec.closed
            && let Some(&first) = line.first()
        {
            out.push(first);
        }
        out
    };
    let look = railing_look(r);
    if r.infill == RailingInfill::Solid {
        let mut marks: Vec<[(P, f64); 2]> = Vec::new();
        for face in [&left, &right] {
            for w in closed_line(face).windows(2) {
                marks.push([(w[0], rail), (w[1], rail)]);
            }
        }
        let solid = Solid {
            rings,
            height: Height::Prism { bottom, top },
            voids: Vec::new(),
            marks,
            finish: None,
        };
        push(m, solid, look, owner);
        return;
    }
    let top_rail = Solid {
        rings,
        height: Height::Prism { bottom: rail, top },
        voids: Vec::new(),
        marks: Vec::new(),
        finish: None,
    };
    push(m, top_rail, look.clone(), owner.clone());
    let line = closed_line(&pts);
    let mut marks: Vec<[(P, f64); 2]> = Vec::new();
    match r.baluster_gap() {
        // A baluster every step along its line, the first half a step in.
        Some(gap) => {
            let step = gap + BALUSTER_WIDTH;
            let mut at = step / 2.0;
            let mut walked = 0.0;
            for w in line.windows(2) {
                let l = (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
                while at <= walked + l && l > 0.0 {
                    let t = (at - walked) / l;
                    let q = P::new(
                        w[0].x + (w[1].x - w[0].x) * t,
                        w[0].y + (w[1].y - w[0].y) * t,
                    );
                    marks.push([(q, bottom), (q, rail)]);
                    at += step;
                }
                walked += l;
            }
        }
        // A glass panel: its foot along its line and, open, its ends.
        None => {
            for w in line.windows(2) {
                marks.push([(w[0], bottom), (w[1], bottom)]);
            }
            if !spec.closed
                && let (Some(&a), Some(&b)) = (line.first(), line.last())
            {
                marks.push([(a, bottom), (a, rail)]);
                marks.push([(b, bottom), (b, rail)]);
            }
        }
    }
    let lines = Solid {
        rings: Vec::new(),
        height: Height::Prism { bottom, top: rail },
        voids: Vec::new(),
        marks,
        finish: None,
    };
    push(m, lines, look, owner);
}

/// A roof's look in a section (docs/adr/8044 §5): its cut hatched as wood (TS 5319 SN23, a pitched roof's structure
/// under its covering), on its covering's layer (EK-2 Çatı, CATI: MM-CATI-KIREMIT), outlined heavy.
fn roof_look(r: &kentos_commands::RoofFields) -> Look {
    let material = r.material_name();
    Look {
        key: format!("CATI:{}", fold(material)),
        layer: kentos_commands::ek4_layer("CATI", material),
        heavy: true,
        fill: Some(Fill {
            pattern: "SN23".into(),
            scale: 1.0,
            angle: 0.0,
            color: None,
        }),
    }
}

/// A roof layer's look in a section (docs/adr/8071 §8): on its EK-2 element (Çatı Kaplama, Su Yalıtımı, Çatı Kaplama
/// Tahtası, Isı Yalıtımı, Çatı Taşıyıcı) by its material (the covering's the roof's, else Genel); the insulation TS 5319
/// SN55 as a slab's (its zigzag fitted between its sides), the rafters, the roof's structure, hatched as the one
/// build-up was (SN23 "ağaç malzeme") and outlined heavy; the others outlined (a tile covering's SN53 "seramik" is not in
/// the library).
fn roof_layer_look(l: &kentos_commands::RoofLayer, r: &kentos_commands::RoofFields) -> Look {
    use kentos_commands::RoofLayerKind as K;
    let detail = match (&l.material, l.kind) {
        (Some(m), _) if !m.trim().is_empty() => m.trim().to_owned(),
        (_, K::Covering) => r.material_name().to_owned(),
        _ => "Genel".to_owned(),
    };
    let fill = |pattern: &str| {
        Some(Fill {
            pattern: pattern.into(),
            scale: 1.0,
            angle: 0.0,
            color: None,
        })
    };
    let element = l.kind.element();
    Look {
        key: format!("{element}:{}", fold(&detail)),
        layer: kentos_commands::ek4_layer(element, &detail),
        heavy: l.kind == K::Rafters,
        fill: match l.kind {
            K::Insulation => fill("SN55"),
            K::Rafters => fill("SN23"),
            _ => None,
        },
    }
}

/// A chimney's or a vent's look in a section (docs/adr/8044 §6): masonry, hatched as the plan's brick (ANSI31), on EK-2's
/// own element (Baca: MM-BACA-GENEL), outlined heavy.
fn chimney_look() -> Look {
    Look {
        key: "BACA:genel".into(),
        layer: kentos_commands::ek4_layer("BACA", "Genel"),
        heavy: true,
        fill: Some(Fill {
            pattern: "ANSI31".into(),
            scale: 1.0,
            angle: 0.0,
            color: None,
        }),
    }
}

/// A roof (docs/adr/8044 §5): each face a sloped slab, its top the roof's plane over it (the eaves at its storey's
/// top and its offset), the roof's thickness under it; a flat roof a slab of its thickness under its eaves. Its eaves'
/// and ridge's levels are kept for the drawing.
fn roof(doc: &Document, r: &kentos_commands::RoofEntity, owner: Option<String>, m: &mut Model) {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::roof as core;
    let Shape::Roof { pts, roof: spec } = crate::geometry::roof_shape(r, doc.settings()) else {
        return;
    };
    let Ok(faces) = core::face_planes(&pts, &spec) else {
        return;
    };
    let thickness = r.roof.thickness_m();
    let look = roof_look(&r.roof);
    let finish = Some(r.roof.material_name().to_owned());
    let k = m.roofs.len();
    let mut at = RoofAt {
        eaves: spec.base,
        ridge: spec.base + core::ridge_rise(&pts, &spec).unwrap_or(0.0),
        seat: spec.base - r.roof.offset,
        solids: Vec::new(),
        tops: Vec::new(),
    };
    let solid = |rings: Vec<Vec<P>>, height: Height| Solid {
        rings,
        height,
        voids: Vec::new(),
        marks: Vec::new(),
        finish: finish.clone(),
    };
    // Its chimneys and vents (docs/adr/8044 §6): from the top of the storey it stands on up to their tops, through it:
    // where the line cuts one, the roof's band stops at its sides.
    let stacks: Vec<(usize, &core::Part, Vec<P>, f64)> = spec
        .parts
        .iter()
        .enumerate()
        .filter_map(|(k, part)| {
            let ring = core::part_ring(&pts, &spec, part)?;
            let top = core::part_top(&pts, &spec, part)?;
            (top > at.seat + 1e-6).then_some((k, part, ring, top))
        })
        .collect();
    let holes: Vec<sec::Void> = stacks
        .iter()
        .map(|(_, _, ring, top)| sec::Void {
            ring: ring.clone(),
            bottom: at.seat,
            top: *top,
        })
        .collect();
    // Its layers top down (docs/adr/8071 §8), each a plate under the one over it: how far down from its top and how
    // thick, across its face (upright on a face sloping g: × √(1 + g²)), its look and its finish; none: one build-up of
    // its thickness, upright (docs/adr/8044 §1).
    #[allow(clippy::type_complexity)]
    let (layered, plates): (bool, Vec<(f64, f64, Look, Option<String>)>) = if r.roof.layers.is_empty() {
        (false, vec![(0.0, thickness, look.clone(), finish.clone())])
    } else {
        let mut down = 0.0;
        let plates = r
            .roof
            .layers
            .iter()
            .enumerate()
            .map(|(n, l)| {
                let plate = (down, l.thickness, roof_layer_look(l, &r.roof), if n == 0 { finish.clone() } else { None });
                down += l.thickness;
                plate
            })
            .collect();
        (true, plates)
    };
    if faces.is_empty() {
        for (down, t, look, finish) in &plates {
            at.solids.push(m.solids.len());
            let mut slab = solid(
                vec![pts.clone()],
                Height::Prism {
                    bottom: spec.base - down - t,
                    top: spec.base - down,
                },
            );
            slab.voids = holes.clone();
            slab.finish = finish.clone();
            push(m, slab, look.clone(), owner.clone());
        }
    } else {
        // Its snow guard's line on each sloping face (docs/adr/8044 §8; Sivas İmar Yönetmeliği 42/5).
        let guards = spec
            .snow_guard
            .filter(|d| *d > 1e-9)
            .map_or_else(Vec::new, |d| core::along_eaves(&pts, &spec, -d));
        for (f, (c, dx, dy)) in faces {
            // Its windows' outlines on its top (docs/adr/8044 §6): seen where it is, never cut.
            let plane = |q: P| c + dx * q.x + dy * q.y;
            let mut marks: Vec<[(P, f64); 2]> = spec
                .parts
                .iter()
                .filter(|part| core::part_face(&pts, &spec, part) == Some(f.edge))
                .filter_map(|part| core::part_ring(&pts, &spec, part))
                .flat_map(|ring| {
                    let n = ring.len();
                    (0..n)
                        .map(|i| {
                            let (a, b) = (ring[i], ring[(i + 1) % n]);
                            [(a, plane(a)), (b, plane(b))]
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            marks.extend(
                guards
                    .iter()
                    .filter(|(i, _)| *i == f.edge)
                    .map(|(_, [a, b])| [(*a, plane(*a)), (*b, plane(*b))]),
            );
            // A layer's thickness across the face, upright on it.
            let upright = if layered { (1.0 + dx * dx + dy * dy).sqrt() } else { 1.0 };
            for (n, (down, t, look, finish)) in plates.iter().enumerate() {
                at.solids.push(m.solids.len());
                let sloped = sec::Sloped {
                    top: c - down * upright,
                    dx,
                    dy,
                    thickness: t * upright,
                    eaves: pts.clone(),
                    roof: k,
                };
                let mut face = solid(vec![f.ring.clone()], Height::Sloped(sloped));
                // Its windows and snow guard on its top layer.
                if n == 0 {
                    face.marks = marks.clone();
                }
                face.voids = holes.clone();
                face.finish = finish.clone();
                push(m, face, look.clone(), owner.clone());
            }
        }
    }
    for (k, part, ring, top) in stacks {
        at.solids.push(m.solids.len());
        at.tops.push((
            top,
            m.solids.len(),
            match part.kind {
                core::PartKind::Vent => "Hava bacası",
                _ => "Baca",
            },
        ));
        let height = Height::Prism {
            bottom: at.seat,
            top,
        };
        // Masonry, not the roof's covering: an elevation does not name its faces "Kiremit".
        let mut stack = solid(vec![ring], height);
        stack.finish = None;
        push_part(m, stack, chimney_look(), owner.clone(), Some(Part::Roof(k)));
    }
    if spec.gutter.is_some() {
        m.rain.push(Rain {
            roof: k,
            pts,
            spec,
            owner,
        });
    }
    m.roofs.push(at);
}

/// A gutter's or a downpipe's look in a section (docs/adr/8044 §8): sheet metal or PVC, unhatched, outlined light, on
/// EK-2's own element (Oluk: MM-OLUK-GENEL; Yağmur İniş Borusu: MM-YIBO-GENEL).
fn rain_look(element: &str) -> Look {
    Look {
        key: format!("{element}:genel"),
        layer: kentos_commands::ek4_layer(element, "Genel"),
        heavy: false,
        fill: None,
    }
}

/// A roof's gutter and downpipes (docs/adr/8044 §8; Mimarlık Hizmetleri Şartnamesi, görünüşler 14.3.5 e "Yağmur
/// olukları inişleri … gösterilir" and, uygulama projesinde, h "Yağmur inişleri ve olukları … gösterilir"): the gutter
/// a half-round band hanging from the eaves, its top at their level; a downpipe from under it back to the wall in a 45°
/// swan neck, then down the wall to the graded ground (±0.00); without a wall square in from it within
/// [`PIPE_REACH`] (a roof drawn by its corners, a pipe set past the building's corner) straight down. The wall is the
/// face the drawing has, its finish included, half a metre under the storey's top the roof stands on. Neither is the
/// roof's: no level is written for them, no face of theirs is named.
fn rain_water(rain: &Rain, m: &mut Model) {
    use kentos_geometry_core::geom::roof as core;
    use kentos_geometry_core::jsmath::js_hypot;
    let Rain {
        roof: k,
        pts,
        spec,
        owner,
    } = rain;
    let Some(g) = spec.gutter.filter(|g| *g > 1e-9) else {
        return;
    };
    let bottom = spec.base - g * core::GUTTER_DEPTH;
    let probe = m.roofs.get(*k).map_or(spec.base, |r| r.seat) - 0.5;
    let plain = |rings: Vec<Vec<P>>, height: Height| Solid {
        rings,
        height,
        voids: Vec::new(),
        marks: Vec::new(),
        finish: None,
    };
    let mut made: Vec<(Solid, Look)> = Vec::new();
    for (_, band) in core::gutter_bands(pts, spec) {
        let height = Height::Prism {
            bottom,
            top: spec.base,
        };
        made.push((plain(vec![band], height), rain_look("OLUK")));
    }
    for part in spec
        .parts
        .iter()
        .filter(|p| p.kind == core::PartKind::Downpipe)
    {
        let wall = |from: P, inward: P| face_ahead(m, from, inward, probe, PIPE_REACH);
        let Some(way) = core::downpipe_way(pts, spec, part, wall) else {
            continue;
        };
        let half = part.width / 2.0;
        let (along, across) = (way.along, P::new(-way.along.y, way.along.x));
        let at = |c: P, s: f64, t: f64| {
            P::new(
                c.x + (along.x * s + across.x * t) * half,
                c.y + (along.y * s + across.y * t) * half,
            )
        };
        let back = js_hypot(way.down.x - way.outlet.x, way.down.y - way.outlet.y);
        let mut top = bottom;
        if back > 1e-6 {
            // The swan neck: its top at the gutter's bottom over the outlet, falling 1:1 to the wall; upright it is
            // its diameter × √2 thick.
            let v = P::new(
                (way.down.x - way.outlet.x) / back,
                (way.down.y - way.outlet.y) / back,
            );
            let thickness = part.width * std::f64::consts::SQRT_2;
            let mut ring = vec![
                at(way.outlet, -1.0, 0.0),
                at(way.down, -1.0, 0.0),
                at(way.down, 1.0, 0.0),
                at(way.outlet, 1.0, 0.0),
            ];
            if area(&ring) < 0.0 {
                ring.reverse();
            }
            let sloped = sec::Sloped {
                top: bottom + v.x * way.outlet.x + v.y * way.outlet.y,
                dx: -v.x,
                dy: -v.y,
                thickness,
                eaves: ring.clone(),
                roof: *k,
            };
            made.push((plain(vec![ring], Height::Sloped(sloped)), rain_look("YIBO")));
            top = bottom - back - thickness / 2.0;
        }
        if top > 1e-6 {
            let ring = vec![
                at(way.down, -1.0, -1.0),
                at(way.down, 1.0, -1.0),
                at(way.down, 1.0, 1.0),
                at(way.down, -1.0, 1.0),
            ];
            let height = Height::Prism { bottom: 0.0, top };
            made.push((plain(vec![ring], height), rain_look("YIBO")));
        }
    }
    for (solid, look) in made {
        push(m, solid, look, owner.clone());
    }
}

/// How far a downpipe looks in for the wall it runs down, metres: PAİY 42's widest eaves (1.00 m), the widest gutter
/// (0.50 m) and as much again.
const PIPE_REACH: f64 = 2.0;

/// How far from `from` along `dir` (a unit vector) the nearest solid standing at height `z` begins, within `reach`: a
/// wall's or a column's face, its finish included; none when nothing stands there.
fn face_ahead(m: &Model, from: P, dir: P, z: f64, reach: f64) -> Option<f64> {
    let mut best: Option<f64> = None;
    for s in &m.solids {
        let Height::Prism { bottom, top } = s.height else {
            continue;
        };
        if z < bottom || z > top {
            continue;
        }
        for ring in &s.rings {
            let n = ring.len();
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                let e = P::new(b.x - a.x, b.y - a.y);
                let den = dir.x * e.y - dir.y * e.x;
                if den.abs() < 1e-12 {
                    continue;
                }
                let w = P::new(a.x - from.x, a.y - from.y);
                let t = (w.x * e.y - w.y * e.x) / den;
                let u = (w.x * dir.y - w.y * dir.x) / den;
                if t > 1e-9
                    && t <= reach
                    && (-1e-9..=1.0 + 1e-9).contains(&u)
                    && best.is_none_or(|b| t < b)
                {
                    best = Some(t);
                }
            }
        }
    }
    best
}

/// A ring's signed area, counter-clockwise positive.
fn area(ring: &[P]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

/// A stair (docs/adr/8042 §3): its body as the geometry core builds it (`stair_body`: each straight leg a flight, a
/// balanced stair's winders and a curved stair's treads one by one, each landing a slab), as thick as the slab it
/// arrives at (`Document::stair_waist`, docs/adr/8080); without one, solid down to its floor. A curved stair's arcs are
/// true (`Model::exact`); the section draws their chords.
fn stair(doc: &Document, s: &kentos_commands::StairFields, owner: Option<String>, m: &mut Model) {
    use kentos_geometry_core::entity::{Shape, polygon_ring};
    use kentos_geometry_core::geom::stair_body::{StairPart, stair_body};
    let Shape::Stair { pts, stair: spec } = kentos_formats::arch_elements::core_stair(s) else {
        return;
    };
    if pts.len() < 2 {
        m.left.insert((Left::Curved, owner));
        return;
    }
    let (base, _) = floor_of(&m.levels, s.arch.level.as_deref());
    let waist = doc.stair_waist(s);
    if waist.is_none() {
        m.left.insert((Left::StairWaist, owner.clone()));
    }
    if spec.bulge != 0.0 {
        m.left.insert((Left::Curved, owner.clone()));
    }
    let look = concrete_look();
    for part in stair_body(&pts, &spec, base, waist) {
        match part {
            StairPart::Flight { ring, flight } => push(
                m,
                Solid {
                    rings: vec![ring],
                    height: Height::Flight(flight),
                    voids: Vec::new(),
                    marks: Vec::new(),
                    finish: None,
                },
                look.clone(),
                owner.clone(),
            ),
            StairPart::Prism {
                ring,
                bulges: Some(bulges),
                bottom,
                top,
            } => {
                let solid = Solid {
                    rings: vec![polygon_ring(&ring, Some(&bulges))],
                    height: Height::Prism { bottom, top },
                    voids: Vec::new(),
                    marks: Vec::new(),
                    finish: None,
                };
                push_exact(m, solid, look.clone(), owner.clone(), vec![(ring, bulges)]);
            }
            StairPart::Prism {
                ring,
                bulges: None,
                bottom,
                top,
            } => push(
                m,
                Solid {
                    rings: vec![ring],
                    height: Height::Prism { bottom, top },
                    voids: Vec::new(),
                    marks: Vec::new(),
                    finish: None,
                },
                look.clone(),
                owner.clone(),
            ),
        }
    }
    // Its railings over it (docs/adr/8128): a straight stair's; a curved stair's and a balanced stair's winders' none.
    let step = kentos_commands::RAILING_GAP + BALUSTER_WIDTH;
    let runs = kentos_geometry_core::geom::stair_rails::stair_rails(&pts, &spec, base, step);
    push_rails(m, runs, owner);
}

/// A stair's or a ramp's railings (docs/adr/8128) drawn as a balustered railing is (ADR 8045 §3), on its layer: each
/// piece's top rail a solid `TOP_RAIL_DEPTH` deep across and down, its top [`kentos_commands::RAILING_HEIGHT`] over the
/// piece's walking plane (sloped with it over a flight or a ramp's run); its balusters lines of a solid without a
/// footprint, each from its foot to the top rail's underside. Parts of their side's railing, not the stair's or the
/// ramp's body.
fn push_rails(m: &mut Model, runs: Vec<kentos_geometry_core::geom::stair_rails::RailRun>, owner: Option<String>) {
    use kentos_commands::RailingInfill;
    let (h, d) = (kentos_commands::RAILING_HEIGHT, TOP_RAIL_DEPTH);
    let look = Look {
        key: format!("KORK:{}", RailingInfill::Balusters.name()),
        layer: kentos_commands::ek4_layer("KORK", RailingInfill::Balusters.layer_word()),
        heavy: false,
        fill: None,
    };
    for run in runs {
        let part = Some(Part::Rail(usize::from(!run.left)));
        let (ex, ey) = (run.b.x - run.a.x, run.b.y - run.a.y);
        let l = ex.hypot(ey);
        if l <= 1e-9 {
            continue;
        }
        // Across it, half the rail's depth either side of its line.
        let (nx, ny) = (-ey / l * d / 2.0, ex / l * d / 2.0);
        let band = vec![
            P::new(run.a.x - nx, run.a.y - ny),
            P::new(run.b.x - nx, run.b.y - ny),
            P::new(run.b.x + nx, run.b.y + ny),
            P::new(run.a.x + nx, run.a.y + ny),
        ];
        let height = if run.plane.is_flat() {
            Height::Prism { bottom: run.plane.top + h - d, top: run.plane.top + h }
        } else {
            Height::Sloped(sec::Sloped {
                top: run.plane.top + h,
                dx: run.plane.dx,
                dy: run.plane.dy,
                thickness: d,
                // Every edge of it has its face; no roof's hip or ridge is taken from it.
                eaves: band.clone(),
                roof: RAMP_FACES + m.solids.len(),
            })
        };
        let rail = Solid { rings: vec![band], height, voids: Vec::new(), marks: Vec::new(), finish: None };
        push_part(m, rail, look.clone(), owner.clone(), part);
        if run.balusters.is_empty() {
            continue;
        }
        let head = |q: P| run.plane.z(q) + h - d;
        let marks: Vec<[(P, f64); 2]> = run.balusters.iter().map(|&(q, foot)| [(q, foot), (q, head(q))]).collect();
        let bottom = run.balusters.iter().map(|b| b.1).fold(f64::INFINITY, f64::min);
        let top = run.balusters.iter().map(|b| head(b.0)).fold(f64::NEG_INFINITY, f64::max);
        let lines = Solid { rings: Vec::new(), height: Height::Prism { bottom, top }, voids: Vec::new(), marks, finish: None };
        push_part(m, lines, look.clone(), owner.clone(), part);
    }
}

/// A placed block's outline in plan (docs/adr/8070 §5): the box of what its block draws with the insert's values,
/// placed as the insert is (moved, turned, scaled, mirrored); none for a block that draws nothing.
pub fn fixture_outline(doc: &Document, i: &kentos_commands::InsertEntity) -> Option<Vec<P>> {
    let [x0, y0, x1, y1] = crate::blocks::fixture_housing_bounds(doc.block(i.block)?)
        .or_else(|| crate::blocks::definition_box(doc, i.block, Some(&i.parameters)))?;
    let m = kentos_geometry_core::block::placement_xy(
        P::new(i.p.x, i.p.y),
        i.scale,
        i.y_scale.unwrap_or(i.scale),
        i.rotation,
        i.mirror,
    );
    Some(
        [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
            .iter()
            .map(|(x, y)| P::new(m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]))
            .collect(),
    )
}

/// A block that is a building element with a body (docs/adr/8070 §5): a box over its outline from its underside over its
/// storey's floor up its height, outlined light on its class's EK-2 element (Tefriş's Mobilya and Vitrifiye, Asansör);
/// cut where the line crosses it, seen past it. A block that is no building element, or has no height, is not in the
/// model.
fn fixture(doc: &Document, i: &kentos_commands::InsertEntity, owner: Option<String>, m: &mut Model) {
    use kentos_commands::BlockElementClass as C;
    let Some(el) = doc
        .block(i.block)
        .and_then(|d| d.card.as_ref())
        .and_then(|c| c.element.as_ref())
    else {
        return;
    };
    let Some(height) = el.height else { return };
    let Some(ring) = fixture_outline(doc, i) else {
        return;
    };
    let (floor, _) = floor_of(&m.levels, i.arch.level.as_deref());
    let bottom = floor + el.elevation.unwrap_or(0.0);
    let (element, word) = match el.class {
        C::Furniture => ("TEFR", "Mobilya"),
        C::SanitaryTerminal => ("TEFR", "Vitrifiye"),
        C::ElectricAppliance => ("TEFR", "Elektrikli cihaz"),
        C::TransportElement => ("ASAN", "Genel"),
    };
    push(
        m,
        Solid {
            rings: vec![ring],
            height: Height::Prism {
                bottom,
                top: bottom + height,
            },
            voids: Vec::new(),
            marks: Vec::new(),
            finish: None,
        },
        Look {
            key: format!("{element}:{}", fold(word)),
            layer: kentos_commands::ek4_layer(element, word),
            heavy: false,
            fill: None,
        },
        owner,
    );
}

/// The index a ramp's run takes as its sloped slab's roof (docs/adr/8070 §1): far past any roof's, so a section never
/// takes an edge of it for a roof's hip or ridge drawn once.
const RAMP_FACES: usize = usize::MAX / 4;

/// A ramp's body (docs/adr/8070 §1): each sloped run a sloped slab whose plane rises from the height at the run's start
/// to the one at its end (the rise shared out over the runs by their lengths, as the plan's arrow and levels read it),
/// and each landing a flat slab at the height it stands at. Both are as thick as the slab the ramp arrives at, else
/// the one it starts from (a stair's waist), else a slab's own thickness: under a ramp on the ground lies its fill,
/// which is ground, and the model holds no ground (EK-4 4.30).
fn ramp(r: &kentos_commands::RampFields, slabs: &[SlabAt], owner: Option<String>, m: &mut Model) {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::arch::{Walk, band, ramp_landings};
    use kentos_geometry_core::geom::stair_rails::{Plane, Stretch, rail_runs};
    let Shape::Ramp { pts, ramp: spec } = kentos_formats::arch_elements::core_ramp(r) else {
        return;
    };
    if pts.len() < 2 || spec.width.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return;
    }
    let walk = Walk::of(&pts);
    let landings = ramp_landings(&walk, &spec);
    // Its sloped length before walking distance s: the distance less the landings' parts before it.
    let run_before = |s: f64| -> f64 {
        s - landings
            .iter()
            .map(|(a, b)| (s.min(*b) - a).max(0.0))
            .sum::<f64>()
    };
    let total = run_before(walk.length());
    let base = r.base.unwrap_or(0.0);
    let height = |s: f64| -> f64 {
        if total > 1e-12 {
            base + r.rise * run_before(s) / total
        } else {
            base
        }
    };
    let (start, end) = (pts[0], pts[pts.len() - 1]);
    let arrive = base + r.rise;
    let waist = slabs
        .iter()
        .find(|sl| sl.holds(end) && (sl.top - arrive).abs() <= 0.5)
        .or_else(|| {
            slabs
                .iter()
                .find(|sl| sl.holds(start) && (sl.top - base).abs() <= 0.5)
        })
        .map_or(kentos_commands::SLAB_THICKNESS, |sl| sl.thickness);
    let look = concrete_look();
    let half = spec.width / 2.0;
    // Its stretches for its railings (docs/adr/8128): each run's surface, each landing's floor.
    let mut stretches: Vec<Stretch> = Vec::new();
    for k in 0..pts.len() - 1 {
        let (a, b) = (pts[k], pts[k + 1]);
        let (sa, sb) = (walk.at[k], walk.at[k + 1]);
        let len = sb - sa;
        if len <= 1e-9 {
            continue;
        }
        let dir = P::new((b.x - a.x) / len, (b.y - a.y) / len);
        let across = P::new(-dir.y, dir.x);
        // Its run: from the end of a landing that holds its start (else its start) to the start of one that holds its
        // end (else its end); landings stand only at the line's corners.
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
        let (z0, z1) = (height(lo), height(hi));
        let grade = (z1 - z0) / (hi - lo);
        let (dx, dy) = (grade * dir.x, grade * dir.y);
        let sloped = sec::Sloped {
            top: z0 - dx * fa.x - dy * fa.y,
            dx,
            dy,
            // Its slab's thickness across it, measured upright as a sloped slab's is.
            thickness: waist * (1.0 + grade * grade).sqrt(),
            // Every edge of a run has its face: its sides, and its ends where it meets a landing or a floor.
            eaves: ring.clone(),
            roof: RAMP_FACES + m.solids.len(),
        };
        stretches.push(Stretch {
            from: lo,
            to: hi,
            segment: Some(k),
            plane: Plane { top: sloped.top, dx, dy },
            steps: Vec::new(),
        });
        push(
            m,
            Solid {
                rings: vec![ring],
                height: Height::Sloped(sloped),
                voids: Vec::new(),
                marks: Vec::new(),
                finish: None,
            },
            look.clone(),
            owner.clone(),
        );
    }
    let outline = band(&pts, half);
    for (x, y) in &landings {
        let top = height(*x);
        stretches.push(Stretch { from: *x, to: *y, segment: None, plane: Plane::flat(top), steps: Vec::new() });
        let Some(ring) = kentos_geometry_core::geom::stair_body::band_cut(&walk, &outline, *x, *y) else {
            continue;
        };
        push(
            m,
            Solid {
                rings: vec![ring],
                height: Height::Prism {
                    bottom: top - waist,
                    top,
                },
                voids: Vec::new(),
                marks: Vec::new(),
                finish: None,
            },
            look.clone(),
            owner.clone(),
        );
    }
    // Its railings over it (docs/adr/8128): over each run its surface, over each landing its floor.
    let step = kentos_commands::RAILING_GAP + BALUSTER_WIDTH;
    push_rails(m, rail_runs(&walk, spec.railings, half, &stretches, step), owner);
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

/// What a view mark draws (docs/adr/8043 §1): a section through its line, or an elevation of what faces its bubble.
#[derive(Clone, Debug, PartialEq)]
pub enum View {
    /// A section by its mark's words ("B").
    Section(String),
    /// An elevation by the direction its facade faces.
    Elevation(Facing),
}

/// The direction a facade faces, as Türkiye names its elevation ("Kuzey görünüşü"), with its three letters in EK-2's
/// place field (2.9.3: for an elevation the author's three characters).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facing {
    pub word: &'static str,
    pub upper: &'static str,
    pub code: &'static str,
}

/// The eight directions clockwise from north.
pub const FACINGS: [Facing; 8] = [
    Facing {
        word: "Kuzey",
        upper: "KUZEY",
        code: "KUZ",
    },
    Facing {
        word: "Kuzeydoğu",
        upper: "KUZEYDOĞU",
        code: "KDG",
    },
    Facing {
        word: "Doğu",
        upper: "DOĞU",
        code: "DOG",
    },
    Facing {
        word: "Güneydoğu",
        upper: "GÜNEYDOĞU",
        code: "GDG",
    },
    Facing {
        word: "Güney",
        upper: "GÜNEY",
        code: "GUN",
    },
    Facing {
        word: "Güneybatı",
        upper: "GÜNEYBATI",
        code: "GBT",
    },
    Facing {
        word: "Batı",
        upper: "BATI",
        code: "BAT",
    },
    Facing {
        word: "Kuzeybatı",
        upper: "KUZEYBATI",
        code: "KBT",
    },
];

/// The plan's north (docs/adr/8043 §1): the drawing's north arrow's (its “Kuzey oku” turned), else the drawing's up.
pub fn plan_north(doc: &Document) -> P {
    let arrow = doc.block_named(kentos_commands::NORTH_BLOCK).map(|b| b.id);
    arrow
        .and_then(|id| {
            doc.entities().find_map(|e| match e {
                Entity::Insert(i) if i.block == id => Some(i.rotation),
                _ => None,
            })
        })
        .map_or(P::new(0.0, 1.0), |r| P::new(-r.sin(), r.cos()))
}

/// The direction an elevation mark's facade faces: back at the mark (its look, degrees from east, turned round),
/// named by the nearest of the eight directions from the plan's north.
pub fn facing(doc: &Document, mark: &kentos_commands::ViewMarkFields) -> Facing {
    let a = mark.angle.unwrap_or(90.0).to_radians();
    FACINGS[rank_of(plan_north(doc), P::new(-a.cos(), -a.sin()))]
}

/// Which of the eight directions (an index into [`FACINGS`]) `face` points to, from the plan's north `n`.
fn rank_of(n: P, face: P) -> usize {
    let east = P::new(n.y, -n.x);
    let bearing = (face.x * east.x + face.y * east.y)
        .atan2(face.x * n.x + face.y * n.y)
        .to_degrees();
    ((bearing.rem_euclid(360.0) + 22.5) / 45.0).floor() as usize % 8
}

/// The building's sides (docs/adr/8043 §9): its outline (the union of the model's footprints, holes left out) edge by
/// edge, each by the direction it faces among [`FACINGS`] from the plan's north, with their length; the ones at least
/// 2 m and a fifth of the longest direction's, north first. A corner cut 1.20 m across beside 12 m walls is part of
/// its neighbours' elevations, not a side of its own.
pub fn building_facings(doc: &Document) -> Vec<(usize, f64)> {
    let m = model(doc);
    let areas: Vec<kentos_geometry_core::geom::arrangement::Area> = m
        .solids
        .iter()
        .filter_map(|s| s.rings.first())
        .filter(|r| r.len() >= 3)
        .map(|r| area_of(r))
        .collect();
    let n = plan_north(doc);
    let mut length = [0.0_f64; 8];
    for a in kentos_geometry_core::geom::region::union_areas(&areas) {
        let ring = area_of(&a.outer.pts).outer.pts;
        for i in 0..ring.len() {
            let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
            let (dx, dy) = (q.x - p.x, q.y - p.y);
            let len = kentos_geometry_core::jsmath::js_hypot(dx, dy);
            if len > 1e-9 {
                // Counter-clockwise: outside is on the right.
                length[rank_of(n, P::new(dy / len, -dx / len))] += len;
            }
        }
    }
    let longest = length.iter().copied().fold(0.0, f64::max);
    (0..8)
        .filter(|k| length[*k] >= 2.0 && length[*k] >= 0.2 * longest)
        .map(|k| (k, length[k]))
        .collect()
}

/// What the mark draws; none for a detail mark.
pub fn view_of(doc: &Document, mark: &kentos_commands::ViewMarkFields) -> Option<View> {
    match mark.form {
        ViewMarkForm::Section => Some(View::Section(mark.mark.clone())),
        ViewMarkForm::Elevation => Some(View::Elevation(facing(doc, mark))),
        ViewMarkForm::Detail => None,
    }
}

/// How far past the building the four marks of [`facade_marks`] stand, metres: past the grids' bubbles (2–2.5 m off
/// the walls at 1/50).
pub const FACADE_MARK_OFF: f64 = 4.0;

/// Four elevation marks around the building (docs/adr/8043 §8), for a drawing that has none: past the middle of each
/// side of the model's extent as the plan's north turns it, [`FACADE_MARK_OFF`] off, each looking at the building: the
/// north facade's first, then the east, the south and the west, numbered 1–4. None for a model with nothing in it.
pub fn facade_marks(doc: &Document) -> Vec<kentos_commands::NewViewMark> {
    let m = model(doc);
    let n = plan_north(doc);
    let e = P::new(n.y, -n.x);
    let (mut e0, mut e1, mut n0, mut n1) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for q in m.solids.iter().flat_map(|s| s.rings.iter().flatten()) {
        let (a, b) = (q.x * e.x + q.y * e.y, q.x * n.x + q.y * n.y);
        e0 = e0.min(a);
        e1 = e1.max(a);
        n0 = n0.min(b);
        n1 = n1.max(b);
    }
    if e0 > e1 || n0 > n1 {
        return Vec::new();
    }
    let (em, nm) = ((e0 + e1) / 2.0, (n0 + n1) / 2.0);
    let at = |a: f64, b: f64| P::new(e.x * a + n.x * b, e.y * a + n.y * b);
    let back = |v: P| P::new(-v.x, -v.y);
    [
        (at(em, n1 + FACADE_MARK_OFF), back(n)),
        (at(e1 + FACADE_MARK_OFF, nm), back(e)),
        (at(em, n0 - FACADE_MARK_OFF), n),
        (at(e0 - FACADE_MARK_OFF, nm), e),
    ]
    .into_iter()
    .enumerate()
    .map(|(k, (p, look))| kentos_commands::NewViewMark {
        form: ViewMarkForm::Elevation,
        points: vec![Vec2 { x: p.x, y: p.y }],
        radius: None,
        at: None,
        angle: Some(look.y.atan2(look.x).to_degrees().rem_euclid(360.0)),
        flip: false,
        mark: (k + 1).to_string(),
        target: None,
        size: None,
    })
    .collect()
}

/// Where a facade stands among [`FACINGS`] (north first, clockwise): the order Tümü draws them in.
pub fn facing_rank(doc: &Document, mark: &kentos_commands::ViewMarkFields) -> usize {
    let code = facing(doc, mark).code;
    FACINGS.iter().position(|f| f.code == code).unwrap_or(FACINGS.len())
}

/// How a mark's drawing is called: "B-B kesiti", "Kuzey görünüşü".
pub fn view_name(doc: &Document, mark: &kentos_commands::ViewMarkFields) -> String {
    match view_of(doc, mark) {
        Some(View::Elevation(f)) => format!("{} görünüşü", f.word),
        _ => format!("{} kesiti", label(&mark.mark)),
    }
}

/// Its block's name by EK-4 Tablo 4.4: "MM-XXXX-KESITB", "MM-XXXX-GORUNUSKUZEY".
pub fn view_block_name(doc: &Document, mark: &kentos_commands::ViewMarkFields) -> String {
    match view_of(doc, mark) {
        Some(View::Elevation(f)) => kentos_commands::ek4_layer(
            kentos_commands::EK4_GENERAL,
            &format!("GORUNUS{}", kentos_commands::ek4_word(f.word, 9)),
        ),
        _ => block_name(&mark.mark),
    }
}

/// Where a mark's drawing is drawn from (docs/adr/8043 §1): a section's line; an elevation's line through its bubble
/// across where it looks, as long as the model is wide there and 1 m on, the viewer at its left end looking where the
/// mark looks; none for a detail mark, or an elevation of a model with nothing in it.
pub fn frame_for(m: &Model, mark: &kentos_commands::ViewMarkFields) -> Option<Frame> {
    match mark.form {
        ViewMarkForm::Section => frame_of(mark),
        ViewMarkForm::Elevation => {
            let c = p(*mark.points.first()?);
            let a = mark.angle.unwrap_or(90.0).to_radians();
            let look = P::new(a.cos(), a.sin());
            let t = P::new(look.y, -look.x);
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for q in m.solids.iter().flat_map(|s| s.rings.iter().flatten()) {
                let v = (q.x - c.x) * t.x + (q.y - c.y) * t.y;
                lo = lo.min(v);
                hi = hi.max(v);
            }
            if lo > hi {
                return None;
            }
            let (lo, hi) = (lo - 1.0, hi + 1.0);
            Frame::new(
                P::new(c.x + t.x * lo, c.y + t.y * lo),
                P::new(c.x + t.x * hi, c.y + t.y * hi),
                false,
                None,
            )
        }
        ViewMarkForm::Detail => None,
    }
}

/// The mark's drawing from the model as it is now; none for a detail mark or what [`frame_for`] cannot frame.
pub fn draw_mark(doc: &Document, mark: &kentos_commands::ViewMarkFields) -> Option<Drawn> {
    let m = model(doc);
    let frame = frame_for(&m, mark)?;
    let view = view_of(doc, mark)?;
    Some(draw_view(doc, &m, &frame, &view))
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
    draw_view(
        doc,
        &model(doc),
        frame,
        &View::Section(mark_words.to_owned()),
    )
}

/// [`draw`] of a view (docs/adr/8043): an elevation's title names its facade, what is seen under the graded ground is
/// dashed (Mimarlar Odası: "zemin altında kalan yapı kısımlarının dış hatları kesik çizgilerle"), no room is named.
pub fn draw_view(doc: &Document, m: &Model, frame: &Frame, view: &View) -> Drawn {
    let elevation = matches!(view, View::Elevation(_));
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
    // TS 5319's insulation fitted to each cut layer (docs/adr/8014, 8071 §8): one zigzag between its two long sides, as
    // a wall's in plan; the library's rows, tiled at 2 mm, break into pieces in a thinner or a sloping band.
    let least = kentos_geometry_core::geom::wall::ZIGZAG_MIN_MM * plot / 1000.0;
    let mut zigzags: Vec<(String, Option<String>, Vec<P>)> = Vec::new();
    for c in &s.cut {
        let look = &m.looks[c.solid];
        let area = area_of(&c.ring);
        outlines
            .entry(look.key.clone())
            .or_insert_with(|| (look.clone(), Vec::new()))
            .1
            .push(area.clone());
        if let Some(f) = look
            .fill
            .as_ref()
            .filter(|f| f.pattern.eq_ignore_ascii_case(kentos_geometry_core::geom::wall::FITTED_INSULATION))
        {
            for line in kentos_geometry_core::geom::zigzag::fitted(&c.ring, least) {
                zigzags.push((format!("{}-T", look.layer), f.color.clone(), line));
            }
        } else if let Some(f) = &look.fill {
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
    for (layer, color, line) in &zigzags {
        let pts: Vec<Value> = line.iter().map(|q| json!({"x": q.x, "y": q.y})).collect();
        let mut v = json!({"kind": "polyline", "pts": pts, "closed": false, "lineWeight": HATCH});
        if let Some(c) = color {
            v["color"] = json!(c);
        }
        out.put(layer, v);
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
    // What is seen: on its material's layer with its depth, 1 nearest … 4 farthest (EK-4 4.14.3). A section measures
    // the depth from its plane over what it sees; an elevation from its nearest face, in slices at least a metre deep
    // (docs/adr/8043 §4): measured from the bubble 3.5 m off, the whole facade fell in the fourth slice at the faintest
    // pen, and over the facade's own depth a column 2.5 cm proud of the plaster was the first slice, the windows the
    // fourth.
    let far = s
        .seen
        .iter()
        .map(|e| e.depth)
        .fold(0.0, kentos_geometry_core::jsmath::js_max);
    let (near, span) = if elevation {
        let near = s
            .seen
            .iter()
            .map(|e| e.depth)
            .fold(f64::INFINITY, kentos_geometry_core::jsmath::js_min)
            .min(far);
        (near, (far - near).max(4.0))
    } else {
        (0.0, far)
    };
    let mut seen: BTreeMap<String, Vec<(P, P)>> = BTreeMap::new();
    for e in &s.seen {
        let rank = if span > 1e-6 {
            (1.0 + (4.0 * (e.depth - near) / span).floor()).clamp(1.0, 4.0) as u8
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
            if !elevation || (a.y >= -1e-9 && b.y >= -1e-9) {
                out.line(&layer, a, b, weight);
                continue;
            }
            // Under the graded ground (±0.00) dashed, above it as it is.
            let at = |z: f64| {
                let t = if (b.y - a.y).abs() > 1e-12 {
                    (z - a.y) / (b.y - a.y)
                } else {
                    0.0
                };
                P::new(a.x + (b.x - a.x) * t, z)
            };
            let ((p0, p1), over) = if a.y < -1e-9 && b.y < -1e-9 {
                ((a, b), None)
            } else if a.y < -1e-9 {
                ((a, at(0.0)), Some((at(0.0), b)))
            } else {
                ((at(0.0), b), Some((a, at(0.0))))
            };
            if let Some((q0, q1)) = over {
                out.line(&layer, q0, q1, weight);
            }
            out.put(
                &layer,
                json!({"kind": "line", "a": {"x": p0.x, "y": p0.y}, "b": {"x": p1.x, "y": p1.y},
                       "lineType": "dashed", "lineWeight": weight}),
            );
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
    let mut marks: Vec<(f64, Option<&str>)> = m.levels.iter().map(|l| (l.2, None)).collect();
    if let Some(top) = m
        .levels
        .iter()
        .filter_map(|l| l.3.map(|h| l.2 + h))
        .fold(None, |a: Option<f64>, b| Some(a.map_or(b, |a| a.max(b))))
    {
        marks.push((top, None));
    }
    marks.push((0.0, None));
    // A roof the drawing shows (docs/adr/8044 §5; Mimarlar Odası 14.3.4 g: "Dere mahya, saçak … kotlandırılır"): the top
    // of the storey it stands on is the saçak seviyesi (PAİY 4: "son kat tavan döşemesinin üst kotu"), its ridge's top
    // the mahya.
    let shown: BTreeSet<usize> = s
        .cut
        .iter()
        .map(|c| c.solid)
        .chain(s.seen.iter().map(|e| e.solid))
        .collect();
    for r in m
        .roofs
        .iter()
        .filter(|r| r.solids.iter().any(|i| shown.contains(i)))
    {
        marks.push((r.seat, Some("Saçak")));
        if r.ridge > r.eaves + 1e-6 {
            marks.push((r.ridge, Some("Mahya")));
        }
        // Its chimneys' and vents' tops the drawing shows (14.3.4 g: "bacalar kotlandırılır").
        marks.extend(
            r.tops
                .iter()
                .filter(|(_, solid, _)| shown.contains(solid))
                .map(|(top, _, word)| (*top, Some(*word))),
        );
    }
    marks.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.is_some().cmp(&a.1.is_some())));
    marks.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-6);
    // The heights chain at the right and the finishes' scan: the storeys', the eaves' and the ridge's levels, not a
    // chimney's or a vent's top (its level is written; a 16 cm link between a vent's top and the ridge only crowds it).
    let zs: Vec<f64> = marks
        .iter()
        .filter(|(_, word)| !matches!(word, Some("Baca" | "Hava bacası")))
        .map(|m| m.0)
        .collect();
    let x0 = bx0 - 1.0;
    let words: Vec<String> = marks
        .iter()
        .map(|(z, word)| match word {
            Some(w) => format!("{} {w}", level_words(*z)),
            None => level_words(*z),
        })
        .collect();
    // A level nearer the next one up than its words are tall (a vent's top 16 cm under the ridge) is moved left by its
    // words' width, beside it, its line drawn on to the building: printed over each other neither could be read.
    let mut shift = vec![0.0; marks.len()];
    for k in (0..marks.len().saturating_sub(1)).rev() {
        if marks[k + 1].0 - marks[k].0 < text * 2.2 {
            shift[k] = shift[k + 1] + words[k].chars().count() as f64 * text * 0.6 + text;
        }
    }
    for (k, (z, _)) in marks.iter().enumerate() {
        let x = x0 - shift[k];
        let tip = P::new(x, *z);
        let w = text * 0.6;
        out.line(
            "MM-XXXX-OLCU",
            P::new(x - 0.9, *z),
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
            P::new(x - 0.8, *z + w * 1.2),
            P::new(x + 0.1, *z + w * 1.2),
            FAINT,
        );
        out.text(
            "MM-XXXX-OLCU",
            P::new(x - 0.75, *z + w * 1.2 + text * 0.4),
            &words[k],
            text,
            "bottomLeft",
        );
        hi.y = hi.y.max(*z);
    }
    // A roof's face cut down its fall line is given its slope over its band (Mimarlar Odası 14.3.4 g: "Çatı eğimi …
    // belirtilir"); cut across it at an angle the band's slope is not the roof's and nothing is written.
    for c in &s.cut {
        let Height::Sloped(sl) = &m.solids[c.solid].height else {
            continue;
        };
        // A downpipe's swan neck slopes too, but it is not a roof's face (docs/adr/8044 §8).
        if !m.roofs.iter().any(|r| r.solids.contains(&c.solid)) {
            continue;
        }
        let ku = sl.dx * frame.right.x + sl.dy * frame.right.y;
        let grade = sl.dx.hypot(sl.dy);
        if ku.abs() <= 1e-6 || (ku.abs() - grade).abs() > 1e-6 * grade.max(1.0) {
            continue;
        }
        let (u0, u1) = c
            .ring
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), q| {
                (a.min(q.x), b.max(q.x))
            });
        if u1 - u0 < 1.0 {
            continue;
        }
        let mid = (u0 + u1) / 2.0;
        let top = sl.z(frame.at(mid, 0.0));
        let turn = ku.atan();
        let lift = text * 0.5;
        out.put(
            "MM-XXXX-ETIKET",
            json!({"kind": "text",
                   "p": {"x": mid - turn.sin() * lift, "y": top + turn.cos() * lift},
                   "text": kentos_geometry_core::geom::roof::slope_words(grade * 100.0),
                   "height": text, "rotation": turn.to_degrees(), "align": "bottomCenter", "lineWeight": FAINT}),
        );
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
        // Read from the right, turned a quarter (a text's turn is in degrees: given π/2 it lay almost flat, M42–M43).
        out.put(
            "MM-XXXX-OLCU",
            json!({"kind": "text", "p": {"x": xd - 0.1, "y": (a + b) / 2.0}, "text": words, "height": text,
                   "rotation": 90.0, "align": "bottomCenter", "lineWeight": FAINT}),
        );
    }
    hi.x = hi.x.max(xd + 0.5);
    // An elevation's finishes (docs/adr/8043 §10; Mimarlar Odası: "Cephe kaplama malzemesi ve renkler yazılır"): what
    // the facade shows, looked at every 25 cm over the building from ±0.00 up, the openings left out, by the finish of
    // the face seen there; each finish seen over a quarter of a square metre named once, right of the heights.
    if elevation {
        let names: BTreeMap<String, String> = doc
            .settings()
            .arch
            .as_ref()
            .map(|a| {
                a.materials
                    .iter()
                    .map(|mat| (mat.id.clone(), mat.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let gaps: Vec<(f64, f64, f64, f64)> = m
            .gaps
            .iter()
            .filter(|g| !g.ring.is_empty())
            .map(|g| {
                let us = g.ring.iter().map(|q| frame.u(*q));
                (
                    us.clone().fold(f64::INFINITY, f64::min),
                    us.fold(f64::NEG_INFINITY, f64::max),
                    g.sill,
                    g.head,
                )
            })
            .collect();
        let top = zs.last().copied().unwrap_or(hi.y);
        let step = 0.25;
        let mut seen_finish: BTreeMap<String, Vec<P>> = BTreeMap::new();
        let mut u = bx0 + step / 2.0;
        while u < bx1 {
            let mut z = step / 2.0;
            while z < top {
                let opening = gaps
                    .iter()
                    .any(|(u0, u1, s0, s1)| u > *u0 && u < *u1 && z > *s0 && z < *s1);
                if !opening
                    && let Some((k, _)) = sec::seen_at(frame, &m.solids, P::new(u, z))
                    && let Some(f) = m.solids[k].finish.as_deref()
                {
                    let name = names.get(f).cloned().unwrap_or_else(|| f.to_owned());
                    seen_finish.entry(name).or_default().push(P::new(u, z));
                }
                z += step;
            }
            u += step;
        }
        // The highest finish first; each leader from its seen point nearest its words, so it stays short and runs
        // across nothing but the facade's edge (from the middle, the first leader crossed a window).
        let mut finishes: Vec<(String, Vec<P>)> = seen_finish
            .into_iter()
            .filter(|(_, at)| at.len() >= 4)
            .collect();
        let highest = |at: &[P]| at.iter().map(|q| q.y).fold(f64::NEG_INFINITY, f64::max);
        finishes.sort_by(|a, b| highest(&b.1).total_cmp(&highest(&a.1)).then(a.0.cmp(&b.0)));
        let lx = xd + 1.2;
        for (k, (name, at)) in finishes.iter().enumerate() {
            let ly = top - 0.4 - k as f64 * 3.0 * text;
            let elbow = P::new(lx - 0.2, ly);
            let Some(from) = at.iter().copied().min_by(|a, b| {
                let da = (a.x - elbow.x).powi(2) + (a.y - elbow.y).powi(2);
                let db = (b.x - elbow.x).powi(2) + (b.y - elbow.y).powi(2);
                da.total_cmp(&db)
            }) else {
                continue;
            };
            out.line("MM-XXXX-ETIKET", from, elbow, FAINT);
            out.line("MM-XXXX-ETIKET", elbow, P::new(lx - 0.05, ly), FAINT);
            out.put(
                "MM-XXXX-ETIKET",
                json!({"kind": "circle", "c": {"x": from.x, "y": from.y}, "r": 0.3 * text, "lineWeight": FAINT}),
            );
            out.text("MM-XXXX-ETIKET", P::new(lx, ly), name, text, "middleLeft");
            hi.x = hi.x.max(lx + 0.6 * text * name.chars().count() as f64);
        }
    }
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
    for e in doc.entities().filter(|_| !elevation) {
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
        &match view {
            View::Section(words) => title(words),
            View::Elevation(f) => format!("{} GÖRÜNÜŞÜ", f.upper),
        },
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
        left: m.left.iter().cloned().collect(),
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
